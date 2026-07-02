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
// Migrated from src/core/util/lsb_radix_sorter.rs

use crate::test_framework::core::util::lucene_test_case::random;
use rand::Rng;
use rand::RngExt;

use crate::core::util::error::lucene_error::Result;
use crate::core::util::lsb_radix_sorter::LSBRadixSorter;
use crate::core::util::packed::PackedInts;
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestLSBRadixSorter;
fn test<R>(random: &mut R, sorter: &mut LSBRadixSorter, max_len: usize) -> Result<()>
where
  R: Rng + ?Sized,
{
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

fn test_with_range<R>(
  random: &mut R,
  sorter: &mut LSBRadixSorter,
  arr: &mut [i32],
  len: usize,
) -> Result<()>
where
  R: Rng + ?Sized,
{
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
  sorter.sort(num_bits as usize, arr, len)?;
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
