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
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::packed::monotonic_block_packed_reader::expected;
use crate::core::util::packed::packed_long_values::INITIAL_PAGE_COUNT;

pub(crate) struct MonotonicLongValues {
  averages: Vec<f32>,
}

impl MonotonicLongValues {
  //TODO
  const BASE_RAM_BYTES_USED: u64 = 0;

  pub(crate) fn new(averages: Vec<f32>) -> Self {
    Self { averages }
  }
  pub(crate) fn decode_block(&self, block: i32, dest: &mut [i64], count: i32) -> i32 {
    let average = self.averages[block as usize];
    for (i, item) in dest.iter_mut().enumerate().take(count as usize) {
      debug_assert!(i <= i32::MAX as usize);
      *item = item.wrapping_add(expected(0, average, i as i32));
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
  const BASE_RAM_BYTES_USED: u64 = 0;

  pub(crate) fn new() -> Self {
    Self {
      averages: vec![0.0; INITIAL_PAGE_COUNT as usize],
    }
  }

  pub(crate) fn build(mut self, values_off: i32) -> Result<MonotonicLongValues> {
    let _ = self.averages.split_off(values_off as usize);

    // TODO: memory calculation not implement
    let _ram_bytes_used = 0;

    Ok(MonotonicLongValues::new(std::mem::take(&mut self.averages)))
  }
  pub(crate) fn base_ram_bytes_used(&self) -> u64 {
    // TODO: memory calculation not implement
    Self::BASE_RAM_BYTES_USED
  }

  pub(crate) fn pack(&mut self, values: &mut [i64], num_values: i32, block: i32) {
    let average = if num_values == 1 {
      0.0
    } else {
      values[num_values as usize - 1].wrapping_sub(values[0]) as f32 / (num_values - 1) as f32
    };

    for (i, value) in values.iter_mut().enumerate().take(num_values as usize) {
      debug_assert!(i <= i32::MAX as usize);
      *value = value.wrapping_sub(expected(0, average, i as i32));
    }
    self.averages[block as usize] = average;
  }

  pub(crate) fn grow(&mut self, new_block_count: i32) -> Result<()> {
    // TODO: memory calculation not implement
    ArrayUtil::grow_exact(&mut self.averages, new_block_count as usize)
  }
}
