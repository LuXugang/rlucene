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
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::Result;
use crate::util::packed::monotonic_block_packed_reader::expected;
use crate::util::packed::packed_long_values::INITIAL_PAGE_COUNT;

pub(crate) struct MonotonicLongValues {
    averages: Vec<f32>,
}

impl MonotonicLongValues {
    //TODO
    #[allow(dead_code)]
    const BASE_RAM_BYTES_USED: u64 = 0;

    pub(crate) fn new(averages: Vec<f32>) -> Self {
        Self { averages }
    }
    pub(crate) fn decode_block(&self, block: i32, dest: &mut [i64], count: i32) -> i32 {
        let average = self.averages[block as usize];
        for (i, item) in dest.iter_mut().enumerate().take(count as usize) {
            debug_assert!(i <= i32::MAX as usize);
            *item += expected(0, average, i as i32);
        }
        count
    }

    pub(crate) fn get_value(&self, block: i32, element: i32, value: u64) -> i64 {
        expected(value as i64, self.averages[block as usize], element)
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

    pub(crate) fn new() -> Self {
        Self {
            averages: vec![0.0; INITIAL_PAGE_COUNT as usize],
        }
    }

    pub(crate) fn build(mut self, values_off: i32) -> Result<MonotonicLongValues> {
        let _ = self.averages.split_off(values_off as usize);

        // TODO
        let _ram_bytes_used = 0;

        Ok(MonotonicLongValues::new(std::mem::take(&mut self.averages)))
    }
    #[allow(dead_code)]
    pub(crate) fn base_ram_bytes_used(&self) -> u64 {
        // TODO
        Self::BASE_RAM_BYTES_USED
    }

    pub(crate) fn pack(&mut self, values: &mut [i64], num_values: i32, block: i32) {
        let average = if num_values == 1 {
            0.0
        } else {
            (values[num_values as usize - 1] - values[0]) as f32 / (num_values - 1) as f32
        };

        for (i, value) in values.iter_mut().enumerate().take(num_values as usize) {
            debug_assert!(i <= i32::MAX as usize);
            *value -= expected(0, average, i as i32);
        }
        self.averages[block as usize] = average;
    }

    pub(crate) fn grow(&mut self, new_block_count: i32) -> Result<()> {
        // TODO: memory calculation not implemented
        ArrayUtil::grow_exact(&mut self.averages, new_block_count as usize)
    }
}
