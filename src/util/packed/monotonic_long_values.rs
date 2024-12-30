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
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::packed::delta_packed_long_values::{
    DeltaPackedLongValues, DeltaPackedLongValuesBuilder,
};
use crate::util::packed::monotonic_block_packed_reader::MonotonicBlockPackedReader;
use crate::util::packed::packed_long_values::{PackedLongValuesBase1, PackedLongValuesBase2};
use crate::util::packed::read_enum::PackedIntsReadEnum;
use crate::util::packed::Reader;

pub struct MonotonicLongValues {
    base: DeltaPackedLongValues,
    averages: Vec<f32>,
}

impl MonotonicLongValues {
    //TODO
    const BASE_RAM_BYTES_USED: u64 = 0;

    pub fn new(
        page_shift: u32,
        page_mask: u32,
        values: Vec<PackedIntsReadEnum>,
        mins: Vec<i64>,
        averages: Vec<f32>,
        size: u64,
        ram_bytes_used: u64,
    ) -> Self {
        let length = values.len();
        let base =
            DeltaPackedLongValues::new(page_shift, page_mask, values, mins, size, ram_bytes_used);
        debug_assert!(length == averages.len(),);
        Self { base, averages }
    }
}

impl PackedLongValuesBase1 for MonotonicLongValues {
    fn decode_block(&mut self, block: usize, dest: &mut [i64]) -> Result<u32, DataIOError> {
        let count = self.base.decode_block(block, dest)?;
        let average = self.averages[block];
        for i in 0..count as usize {
            dest[i] += MonotonicBlockPackedReader::expected(0, average, i);
        }
        Ok(count)
    }

    fn get_value(&mut self, block: usize, element: usize) -> Result<i64, DataIOError> {
        let base_value = self.base.base.values[block].get(element)?;
        Ok(MonotonicBlockPackedReader::expected(
            self.base.mins[block],
            self.averages[block],
            element,
        ) + base_value)
    }
}

pub struct MonotonicLongValuesBuilder {
    base_builder: DeltaPackedLongValuesBuilder,
    averages: Vec<f32>,
}

impl MonotonicLongValuesBuilder {
    //TODO
    const BASE_RAM_BYTES_USED: u64 = 0;

    pub fn new(page_size: u32, acceptable_overhead_ratio: f32) -> Result<Self, DataIOError> {
        let base_builder = DeltaPackedLongValuesBuilder::new(page_size, acceptable_overhead_ratio)?;
        let length = base_builder.base_builder.values.len();
        Ok(Self {
            base_builder,
            averages: vec![0.0; length],
        })
    }

    pub fn build(mut self) -> Result<MonotonicLongValues, DataIOError> {
        self.base_builder.base_builder.finish()?;

        let values = self
            .base_builder
            .base_builder
            .values
            .split_off(self.base_builder.base_builder.values_off);
        let mins = self
            .base_builder
            .mins
            .split_off(self.base_builder.base_builder.values_off);
        let averages = self
            .averages
            .split_off(self.base_builder.base_builder.values_off);

        // TODO
        let ram_bytes_used = 0;

        Ok(MonotonicLongValues::new(
            self.base_builder.base_builder.page_shift,
            self.base_builder.base_builder.page_mask,
            values,
            mins,
            averages,
            self.base_builder.base_builder.size,
            ram_bytes_used,
        ))
    }
}

impl PackedLongValuesBase2 for MonotonicLongValuesBuilder {
    fn base_ram_bytes_used(&self) -> u64 {
        // TODO
        Self::BASE_RAM_BYTES_USED
    }

    fn pack(
        &mut self,
        values: &mut [i64],
        num_values: u32,
        block: usize,
        acceptable_overhead_ratio: f32,
    ) -> Result<(), DataIOError> {
        let average = if num_values == 1 {
            0.0
        } else {
            (values[num_values as usize - 1] - values[0]) as f32 / (num_values - 1) as f32
        };

        for i in 0..num_values as usize {
            values[i] -= MonotonicBlockPackedReader::expected(0, average, i);
        }

        self.base_builder
            .pack(values, num_values, block, acceptable_overhead_ratio)?;
        self.averages[block] = average;
        Ok(())
    }

    fn grow(&mut self, new_block_count: u32) {
        self.base_builder.grow(new_block_count);
        if new_block_count as usize > self.averages.len() {
            for _i in 0..new_block_count as usize - self.averages.len() {
                self.averages.push(0.0);
            }
        }
    }
}
