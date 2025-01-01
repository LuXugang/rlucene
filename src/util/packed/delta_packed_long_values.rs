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
use crate::util::error::data_io_error_enum::RuntimeError;
use crate::util::packed::monotonic_long_values::{MonotonicLongValues, MonotonicLongValuesBuilder};
use crate::util::packed::packed_long_values::INITIAL_PAGE_COUNT;

pub struct DeltaPackedLongValues {
    pub(crate) sub_long_value: Option<MonotonicLongValues>,
    pub(crate) mins: Vec<i64>,
}

impl DeltaPackedLongValues {
    #[allow(dead_code)]
    const BASE_RAM_BYTES_USED: u64 = 0;
    pub(crate) fn new(mins: Vec<i64>, sub_reader: Option<MonotonicLongValues>) -> Self {
        Self {
            sub_long_value: sub_reader,
            mins,
        }
    }
    pub(crate) fn decode_block(
        &mut self,
        block: usize,
        dest: &mut [i64],
        count: u32,
    ) -> Result<u32, RuntimeError> {
        let min = self.mins[block];
        for item in dest.iter_mut().take(count as usize) {
            *item += min;
        }
        match self.sub_long_value {
            Some(ref mut sub) => Ok(sub.decode_block(block, dest, count)?),
            _ => Ok(count),
        }
    }

    pub(crate) fn get_value(
        &mut self,
        block: usize,
        element: usize,
        _value: u64,
    ) -> Result<i64, RuntimeError> {
        let current = self.mins[block];
        match self.sub_long_value {
            Some(ref mut reader) => Ok(reader.get_value(block, element, current as u64)?),
            None => Ok(current),
        }
    }
}

pub struct DeltaPackedLongValuesBuilder {
    pub(crate) sub_builder: Option<MonotonicLongValuesBuilder>,
    pub(crate) mins: Vec<i64>,
}
impl Default for DeltaPackedLongValuesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DeltaPackedLongValuesBuilder {
    // TODO
    #[allow(dead_code)]
    const BASE_RAM_BYTES_USED: u64 = 0;
    pub fn new() -> DeltaPackedLongValuesBuilder {
        Self::new_with_sub_builder(None)
    }
    pub fn new_with_sub_builder(
        sub_builder: Option<MonotonicLongValuesBuilder>,
    ) -> DeltaPackedLongValuesBuilder {
        Self {
            sub_builder,
            mins: vec![0; INITIAL_PAGE_COUNT],
        }
    }

    pub fn build(mut self, values_off: usize) -> Result<DeltaPackedLongValues, RuntimeError> {
        let sub_reader = if self.sub_builder.is_some() {
            Some(self.sub_builder.take().unwrap().build(values_off)?)
        } else {
            None
        };
        let _ = self.mins.split_off(values_off);
        // TODO:
        let _ram_bytes_used = 0;
        Ok(DeltaPackedLongValues::new(
            std::mem::take(&mut self.mins),
            sub_reader,
        ))
    }
    pub(crate) fn pack(
        &mut self,
        values: &mut [i64],
        num_values: u32,
        block: usize,
    ) -> Result<(), RuntimeError> {
        if self.sub_builder.is_some() {
            self.sub_builder
                .as_mut()
                .unwrap()
                .pack(values, num_values, block)?;
        }
        let mut min = values[0];
        for &value in values.iter().take(num_values as usize).skip(1) {
            min = min.min(value);
        }
        for value in values.iter_mut().take(num_values as usize) {
            *value -= min;
        }
        self.mins[block] = min;
        Ok(())
    }

    pub(crate) fn grow(&mut self, new_block_count: u32) {
        if let Some(ref mut builder) = self.sub_builder {
            builder.grow(new_block_count)
        }
        if new_block_count as usize >= self.mins.len() {
            for _i in 0..new_block_count as usize / 2 {
                self.mins.push(0);
            }
        }
        //TODO
        // self.sub_builder.ram_bytes_used = 0;
    }
    #[allow(dead_code)]
    fn base_ram_bytes_used(&self) -> u64 {
        todo!()
    }
}
