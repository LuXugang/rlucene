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
use crate::util::packed::monotonic_block_packed_reader::MonotonicBlockPackedReader;
use crate::util::packed::packed_long_values::INITIAL_PAGE_COUNT;

pub struct MonotonicLongValues {
    averages: Vec<f32>,
}

impl MonotonicLongValues {
    //TODO
    #[allow(dead_code)]
    const BASE_RAM_BYTES_USED: u64 = 0;

    pub fn new(averages: Vec<f32>) -> Self {
        Self { averages }
    }
    pub(crate) fn decode_block(
        &mut self,
        block: usize,
        dest: &mut [i64],
        count: u32,
    ) -> Result<u32, DataIOError> {
        let average = self.averages[block];
        for (i, item) in dest.iter_mut().enumerate().take(count as usize) {
            *item += MonotonicBlockPackedReader::expected(0, average, i);
        }
        Ok(count)
    }

    pub(crate) fn get_value(
        &mut self,
        block: usize,
        element: usize,
        value: u64,
    ) -> Result<i64, DataIOError> {
        Ok(MonotonicBlockPackedReader::expected(
            value as i64,
            self.averages[block],
            element,
        ))
    }
}

pub struct MonotonicLongValuesBuilder {
    averages: Vec<f32>,
}

impl Default for MonotonicLongValuesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicLongValuesBuilder {
    //TODO
    #[allow(dead_code)]
    const BASE_RAM_BYTES_USED: u64 = 0;

    pub fn new() -> Self {
        Self {
            averages: vec![0.0; INITIAL_PAGE_COUNT],
        }
    }

    pub fn build(mut self, values_off: usize) -> Result<MonotonicLongValues, DataIOError> {
        let _ = self.averages.split_off(values_off);

        // TODO
        let _ram_bytes_used = 0;

        Ok(MonotonicLongValues::new(std::mem::take(&mut self.averages)))
    }
    #[allow(dead_code)]
    fn base_ram_bytes_used(&self) -> u64 {
        // TODO
        Self::BASE_RAM_BYTES_USED
    }

    pub(crate) fn pack(
        &mut self,
        values: &mut [i64],
        num_values: u32,
        block: usize,
    ) -> Result<(), DataIOError> {
        let average = if num_values == 1 {
            0.0
        } else {
            (values[num_values as usize - 1] - values[0]) as f32 / (num_values - 1) as f32
        };

        for (i, value) in values.iter_mut().enumerate().take(num_values as usize) {
            *value -= MonotonicBlockPackedReader::expected(0, average, i);
        }
        self.averages[block] = average;
        Ok(())
    }

    pub(crate) fn grow(&mut self, new_block_count: u32) {
        if new_block_count as usize >= self.averages.len() {
            for _i in 0..new_block_count as usize / 2 {
                self.averages.push(0.0);
            }
        }
    }
}
