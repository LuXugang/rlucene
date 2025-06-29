/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/
use crate::store::DataOutput;
use crate::util::error::lucene_error::Result;
use crate::util::packed::abstract_block_packed_writer::{
    write_values, AbstractBlockPackedWriterBase,
};
use crate::util::packed::monotonic_block_packed_reader::expected;
use crate::util::packed::PackedInts;
/// A writer for large monotonically increasing sequences of positive longs.
///
/// The sequence is divided into fixed-size blocks, and for each block, values
/// are modeled after a linear function `f(x) = A * x + B`. The block encodes
/// deltas from the expected values computed from this function using as few
/// bits as possible.
///
/// # Format
///
/// - `<Block>^BlockCount`
/// - `BlockCount`: ⌈ ValueCount / BlockSize ⌉
/// - `Block`: `<Header, (Ints)>`
/// - `Header`: `<B, A, BitsPerValue>`
///   - `B`: The `B` from `f(x) = A * x + B` encoded using
///     [`BitUtil::zig_zag_encode_i64`](crate::util::bit_util::BitUtil::zig_zag_encode_i64)
///     with [`DataOutput::write_vlong`].
///   - `A`: The `A` from `f(x) = A * x + B` encoded using [`f32::to_bits`] and
///     written as a 4-byte integer with [`DataOutput::write_int`].
///   - `BitsPerValue`: A variable-length integer written with
///     [`DataOutput::write_vint`].
/// - `Ints`: If `BitsPerValue` is `0`, then there is nothing to read, and all
///   values perfectly match the result of the function. Otherwise, these are
///   the packed deltas from the expected values (computed from the function)
///   using exactly `BitsPerValue` bits per value.
///
/// # See Also
/// - [`MonotonicBlockPackedReader`](crate::util::packed::monotonic_block_packed_reader::MonotonicBlockPackedReader)
///
/// # Note
/// This is an internal implementation detail of the Lucene-like system.
pub struct MonotonicBlockPackedWriter;
impl AbstractBlockPackedWriterBase for MonotonicBlockPackedWriter {
    fn flush(
        &mut self,
        out: &mut impl DataOutput,
        off: &mut i32,
        values: &mut [i64],
        blocks: &mut Vec<u8>,
    ) -> Result<()> {
        debug_assert!(*off > 0);
        let avg = if *off == 1 {
            0.0f32
        } else {
            (values[*off as usize - 1] - values[0]) as f32 / (*off as f32 - 1.0)
        };

        let mut min = values[0];
        // adjust min so that all deltas will be positive
        for (i, &actual) in values.iter().enumerate().skip(1).take(*off as usize - 1) {
            debug_assert!(i <= i32::MAX as usize);
            let expected = expected(min, avg, i as i32);
            if expected > actual {
                min -= expected - actual;
            }
        }
        let mut max_delta = 0;
        for (i, value) in values.iter_mut().take(*off as usize).enumerate() {
            debug_assert!(i <= i32::MAX as usize);
            *value -= expected(min, avg, i as i32);
            max_delta = max_delta.max(*value);
        }
        out.write_zlong(min)?;
        out.write_int(avg.to_bits() as i32)?;

        if max_delta == 0 {
            out.write_vint(0)?;
        } else {
            let bits_required = PackedInts::bits_required(max_delta)?;
            out.write_vint(bits_required)?;
            write_values(bits_required, out, blocks, values, *off)?;
        }
        *off = 0;
        Ok(())
    }

    fn add(&mut self, value: i64) {
        debug_assert!(value >= 0);
    }
}
