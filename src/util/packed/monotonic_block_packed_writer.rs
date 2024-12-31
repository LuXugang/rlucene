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
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::packed::abstract_block_packed_writer::{
    write_values, AbstractBlockPackedWriterBase,
};
use crate::util::packed::monotonic_block_packed_reader::MonotonicBlockPackedReader;
use crate::util::packed::PackedInts;

pub struct MonotonicBlockPackedWriter;
impl AbstractBlockPackedWriterBase for MonotonicBlockPackedWriter {
    fn flush<T: DataOutput>(
        &mut self,
        out: &mut T,
        off: &mut usize,
        values: &mut [i64],
        blocks: &mut Vec<u8>,
    ) -> Result<(), DataIOError> {
        let avg = if *off == 1 {
            0.0f32
        } else {
            (values[*off - 1] - values[0]) as f32 / (*off as f32 - 1.0)
        };

        let mut min = values[0];
        // adjust min so that all deltas will be positive
        for i in 1..*off {
            let actual = values[i];
            let expected = MonotonicBlockPackedReader::expected(min, avg, i);
            if expected > actual {
                min -= expected - actual;
            }
        }
        let mut max_delta = 0;
        for i in 0..*off {
            values[i] -= MonotonicBlockPackedReader::expected(min, avg, i);
            max_delta = max_delta.max(values[i]);
        }
        out.write_zlong(min)?;
        out.write_int(avg.to_bits() as i32)?;

        if max_delta == 0 {
            out.write_vint(0)?;
        } else {
            let bits_required = PackedInts::bits_required(max_delta)?;
            out.write_vint(bits_required as i32)?;
            write_values(bits_required, out, blocks, values, *off)?;
        }
        *off = 0;
        Ok(())
    }

    fn add(&mut self, value: i64) {
        debug_assert!(value >= 0);
    }
}
