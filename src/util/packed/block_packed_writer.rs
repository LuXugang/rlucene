/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate::store::DataOutput;
use crate::util::bit_util::BitUtil;
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::packed::abstract_block_packed_writer::{
    write_values, write_vlong, AbstractBlockPackedWriterBase, BPV_SHIFT, MIN_VALUE_EQUALS_0,
};
use crate::util::packed::PackedInts;

/// A writer for large sequences of longs.
///
/// The sequence is divided into fixed-size blocks, and for each block, the difference between each
/// value and the minimum value of the block is encoded using as few bits as possible. Memory usage
/// of this struct is proportional to the block size. Each block has an overhead between 1 and 10
/// bytes to store the minimum value and the number of bits per value of the block.
///
/// # Format
///
/// - `<Block>`<sup>BlockCount</sup>
/// - `BlockCount`: ⌈ValueCount / BlockSize⌉
/// - `Block`: `<Header, (Ints)>`
/// - `Header`: `<Token, (MinValue)>`
/// - `Token`: A single byte, where the first 7 bits are the number of bits per value
///   (`bits_per_value`). If the 8th bit is 1, then `MinValue` is `0`, otherwise `MinValue` needs to
///   be decoded.
/// - `MinValue`: A ZigZag-encoded variable-length long whose value is added to every integer
///   in the block to restore the original values.
/// - `Ints`: If `bits_per_value` is `0`, then all integers are equal to `MinValue`. Otherwise:
///   `BlockSize` integers are stored as packed integers using exactly `bits_per_value` bits
///   per value.
///
/// # See Also
/// - [`BlockPackedReaderIterator`](crate::util::packed::block_packed_reader_iterator::BlockPackedReaderIterator)
#[derive(Default)]
pub struct BlockPackedWriter;
impl AbstractBlockPackedWriterBase for BlockPackedWriter {
    fn flush<T: DataOutput>(
        &mut self,
        out: &mut T,
        off: &mut usize,
        values: &mut [i64],
        blocks: &mut Vec<u8>,
    ) -> Result<(), DataIOError> {
        let mut min = i64::MAX;
        let mut max = i64::MIN;
        for &value in &values[..*off] {
            min = min.min(value);
            max = max.max(value);
        }

        let delta = max - min;
        let bits_required = if delta == 0 {
            0
        } else {
            PackedInts::unsigned_bits_required(delta)
        };

        let mut min_adjusted = min;
        if bits_required == 64 {
            min_adjusted = 0;
        } else if min > 0 {
            min_adjusted = (max - PackedInts::max_value(bits_required)).max(0);
        }

        let token = (bits_required << BPV_SHIFT)
            | if min_adjusted == 0 {
                MIN_VALUE_EQUALS_0
            } else {
                0
            };
        debug_assert!(token <= u8::MAX as u32);
        out.write_byte(token as u8)?;

        if min_adjusted != 0 {
            write_vlong(out, BitUtil::zig_zag_encode_i64(min_adjusted) - 1)?;
        }

        if bits_required > 0 {
            if min_adjusted != 0 {
                for value in values.iter_mut().take(*off) {
                    *value -= min_adjusted;
                }
            }
            write_values(bits_required, out, blocks, values, *off)?;
        }
        *off = 0;
        Ok(())
    }
}
