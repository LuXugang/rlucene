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

use crate::core::util::SliceCopyOps;
use crate::core::util::array_util::ArrayUtil;

/// A LSB Radix sorter for unsigned int values.
pub struct LSBRadixSorter {
  histogram: [i32; HISTOGRAM_SIZE],
  buffer: Vec<i32>,
}
impl Default for LSBRadixSorter {
  fn default() -> Self {
    Self::new()
  }
}

impl LSBRadixSorter {
  pub fn new() -> Self {
    LSBRadixSorter {
      histogram: [0; HISTOGRAM_SIZE],
      buffer: Vec::new(),
    }
  }
  fn build_histogram(array: &[i32], len: usize, histogram: &mut [i32; 256], shift: usize) {
    for &v in &array[..len] {
      let b = ((v as u32) >> shift) & 0xFF;
      histogram[b as usize] += 1;
    }
  }
  fn sum_histogram(histogram: &mut [i32; 256]) {
    let mut accum = 0;
    for h in histogram.iter_mut() {
      let count = *h;
      *h = accum;
      accum += count;
    }
  }
  fn reorder(
    array: &[i32],
    len: usize,
    histogram: &mut [i32; 256],
    shift: usize,
    dest: &mut [i32],
  ) {
    for &v in &array[..len] {
      let b = ((v as u32) >> shift) & 0xFF;
      let idx = histogram[b as usize] as usize;
      dest[idx] = v;
      histogram[b as usize] += 1;
    }
  }
  fn sort_pass(
    array: &[i32],
    len: usize,
    histogram: &mut [i32; 256],
    shift: usize,
    dest: &mut [i32],
  ) -> bool {
    histogram.fill(0);
    Self::build_histogram(array, len, histogram, shift);
    if histogram[0] == len as i32 {
      return false;
    }
    Self::sum_histogram(histogram);
    Self::reorder(array, len, histogram, shift, dest);
    true
  }

  fn insertion_sort(array: &mut [i32], off: usize, len: usize) {
    let end = off + len;
    for i in off + 1..end {
      for j in (off + 1..=i).rev() {
        if array[j - 1] > array[j] {
          array.swap(j - 1, j);
        } else {
          break;
        }
      }
    }
  }
  /// Sorts `array[0..len]` in place.
  ///
  /// - `num_bits`: how many bits are required to store any of the values in
  ///   `array[0..len]`. Pass `32` if unknown.
  pub fn sort(&mut self, num_bits: usize, array: &mut [i32], len: usize) {
    if len < INSERTION_SORT_THRESHOLD {
      Self::insertion_sort(array, 0, len);
      return;
    }

    if let Some(new_array) = ArrayUtil::grow_no_copy(&self.buffer, len) {
      self.buffer = new_array;
    }
    let mut swapped = false;
    let mut arr: &mut [i32] = array;
    let mut buf: &mut [i32] = &mut self.buffer;

    for shift in (0..num_bits).step_by(8) {
      if Self::sort_pass(arr, len, &mut self.histogram, shift, buf) {
        std::mem::swap(&mut arr, &mut buf);
        swapped = !swapped;
      }
    }
    if swapped {
      array.copy_from(&self.buffer[..len], 0);
    }
  }
}

const INSERTION_SORT_THRESHOLD: usize = 30;
const HISTOGRAM_SIZE: usize = 256;

#[cfg(test)]
mod tests {
  use rand::Rng;
  use rand::RngExt;

  use crate::core::util::error::lucene_error::Result;
  use crate::core::util::lsb_radix_sorter::LSBRadixSorter;
  use crate::core::util::packed::PackedInts;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
  use crate::test::core::util::test_util::TestUtil;

  #[allow(dead_code)] // for quick search
  struct TestLSBRadixSorter;
  fn test<R: Rng + ?Sized>(
    random: &mut R,
    sorter: &mut LSBRadixSorter,
    max_len: usize,
  ) -> Result<()> {
    for _ in 0..10 {
      let len = TestUtil::next_usize(random, 0, max_len);
      let tail = random.random_range(0..10);
      let mut arr = vec![0i32; len + tail];

      let num_bits = random.random_range(0..31);
      let max_value = (1 << num_bits) - 1;

      for val in arr.iter_mut() {
        *val = TestUtil::next_int(random, 0, max_value);
      }

      test_with_range(random, sorter, &mut arr, len)?;
    }

    Ok(())
  }

  fn test_with_range<R: Rng + ?Sized>(
    random: &mut R,
    sorter: &mut LSBRadixSorter,
    arr: &mut [i32],
    len: usize,
  ) -> Result<()> {
    let mut expected = arr[..len].to_vec();
    expected.sort_unstable();

    let mut num_bits = 0;
    for &val in &arr[..len] {
      let v = PackedInts::bits_required(val as i64)?;
      num_bits = num_bits.max(v);
    }

    if random.random_bool(0.5) {
      num_bits = TestUtil::next_int(random, num_bits, 32);
    }
    sorter.sort(num_bits as usize, arr, len);
    let actual = arr[..len].to_vec();
    assert_eq!(expected, actual);
    Ok(())
  }
  #[test]
  fn test_empty() -> Result<()> {
    let mut random = random();
    test(&mut random, &mut LSBRadixSorter::new(), 0)
  }
  #[test]
  fn test_one() -> Result<()> {
    let mut random = random();
    test(&mut random, &mut LSBRadixSorter::new(), 1)
  }

  #[test]
  fn test_two() -> Result<()> {
    let mut random = random();
    test(&mut random, &mut LSBRadixSorter::new(), 2)
  }

  #[test]
  fn test_simple() -> Result<()> {
    let mut random = random();
    test(&mut random, &mut LSBRadixSorter::new(), 100)
  }

  #[test]
  fn test_random() -> Result<()> {
    let mut random = random();
    test(&mut random, &mut LSBRadixSorter::new(), 10_000)
  }
  #[test]
  fn test_sorted() -> Result<()> {
    let mut random = random();
    let mut sorter = LSBRadixSorter::new();

    for _ in 0..10 {
      let mut arr = vec![0i32; 10_000];
      let mut a = 0;
      for val in arr.iter_mut() {
        a += random.random_range(0..10);
        *val = a;
      }

      let len = TestUtil::next_int(&mut random, 0, arr.len() as i32) as usize;
      test_with_range(&mut random, &mut sorter, &mut arr, len)?;
    }

    Ok(())
  }
}
