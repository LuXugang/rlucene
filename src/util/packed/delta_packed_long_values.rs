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
use crate::util::packed::packed_long_values::{
    PackedLongValues, PackedLongValuesBase1, PackedLongValuesBase2, PackedLongValuesBuilder,
};
use crate::util::packed::read_enum::PackedIntsReadEnum;
use crate::util::packed::Reader;

pub struct DeltaPackedLongValues {
    pub(crate) base: PackedLongValues,
    pub(crate) mins: Vec<i64>,
}

impl DeltaPackedLongValues {
    const BASE_RAM_BYTES_USED: u64 = 0;
    pub(crate) fn new(
        page_shift: u32,
        page_mask: u32,
        values: Vec<PackedIntsReadEnum>,
        mins: Vec<i64>,
        size: u64,
        ram_bytes_used: u64,
    ) -> Self {
        let length = values.len();
        let base = PackedLongValues::new(page_shift, page_mask, values, size, ram_bytes_used);
        debug_assert!(length == mins.len(),);
        Self { base, mins }
    }
}
impl PackedLongValuesBase1 for DeltaPackedLongValues {
    fn decode_block(&mut self, block: usize, dest: &mut [i64]) -> Result<u32, DataIOError> {
        let count = self.base.decode_block(block, dest)?;
        let min = self.mins[block];
        for i in 0..count as usize {
            dest[i] += min;
        }
        Ok(count)
    }

    fn get_value(&mut self, block: usize, element: usize) -> Result<i64, DataIOError> {
        Ok(self.mins[block] + self.base.values[block].get(element)?)
    }
}

pub struct DeltaPackedLongValuesBuilder {
    pub(crate) base_builder: PackedLongValuesBuilder,
    pub(crate) mins: Vec<i64>,
}
impl DeltaPackedLongValuesBuilder {
    // TODO
    const BASE_RAM_BYTES_USED: u64 = 0;
    pub fn new(
        page_size: u32,
        acceptable_overhead_ratio: f32,
    ) -> Result<DeltaPackedLongValuesBuilder, DataIOError> {
        let base_builder = PackedLongValuesBuilder::new(page_size, acceptable_overhead_ratio)?;
        let length = base_builder.values.len();
        Ok(Self {
            base_builder,
            mins: Vec::with_capacity(length),
        })
    }
    pub fn build(mut self) -> Result<DeltaPackedLongValues, DataIOError> {
        self.base_builder.finish()?;
        let values = self
            .base_builder
            .values
            .split_off(self.base_builder.values_off as usize);
        let mins = self.mins.split_off(self.base_builder.values_off as usize);
        // TODO:
        let ram_bytes_used = 0;
        //TODO
        let ram_bytes_used = 0;
        Ok(DeltaPackedLongValues::new(
            self.base_builder.page_shift,
            self.base_builder.page_mask,
            values,
            mins,
            self.base_builder.size,
            ram_bytes_used,
        ))
    }
}
impl PackedLongValuesBase2 for DeltaPackedLongValuesBuilder {
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
        let mut min = values[0];
        for &value in values.iter().take(num_values as usize).skip(1) {
            min = min.min(value);
        }
        for value in values.iter_mut().take(num_values as usize) {
            *value -= min;
        }
        self.base_builder
            .pack(values, num_values, block, acceptable_overhead_ratio)?;
        self.mins[block] = min;
        Ok(())
    }

    fn grow(&mut self, new_block_count: u32) {
        self.base_builder.grow(new_block_count);
        if new_block_count as usize > self.mins.len() {
            for _i in 0..new_block_count as usize - self.mins.len() {
                self.mins.push(0);
            }
        }
        //TODO
        self.base_builder.ram_bytes_used = 0;
    }
}
