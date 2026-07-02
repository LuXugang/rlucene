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
// Migrated from src/core/util/radix_selector.rs

use crate::test_framework::core::util::lucene_test_case::random;
use std::cmp::{Ordering, min};

use rand::Rng;
use rand::RngExt;

use crate::core::index::BytesRef;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::radix_selector::{RadixSelector, RadixSelectorBase};
use crate::core::util::selector::Selector;
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestRadixSelector;
#[test]
pub fn test_select() -> Result<()> {
  let mut random = random();
  for _ in 0..100 {
    do_test_select(&mut random)?;
  }
  Ok(())
}

fn do_test_select<R>(random: &mut R) -> Result<()>
where
  R: Rng + ?Sized,
{
  let from = random.random_range(0..5) as usize;
  let to = from + TestUtil::next_usize(random, 1, 10000);
  let max_len = TestUtil::next_usize(random, 1, 12);
  let arr_len = from + to + random.random_range(0..5);
  let mut arr: Vec<BytesRef<Vec<u8>>> = Vec::with_capacity(arr_len);
  for _ in 0..arr_len {
    let byte_len = TestUtil::next_usize(random, 0, max_len);
    let mut bytes = vec![0u8; byte_len];
    random.fill_bytes(&mut bytes);
    arr.push(BytesRef::from_bytes(bytes));
  }
  do_test(random, &arr, from, to, max_len)
}

#[test]
pub fn test_shared_prefixes() -> Result<()> {
  let mut random = random();
  for _ in 0..100 {
    do_test_shared_prefixes(&mut random)?;
  }
  Ok(())
}

pub fn do_test_shared_prefixes<R>(random: &mut R) -> Result<()>
where
  R: Rng + ?Sized,
{
  let from = random.random_range(0..5);
  let to = from + TestUtil::next_usize(random, 1, 10000);
  let max_len = TestUtil::next_usize(random, 1, 12);
  let arr_len = from + to + random.random_range(0..5);
  let mut arr: Vec<BytesRef<Vec<u8>>> = Vec::with_capacity(arr_len);
  for _ in 0..arr_len {
    let byte_len = TestUtil::next_usize(random, 0, max_len);
    let mut bytes = vec![0u8; byte_len];
    random.fill_bytes(&mut bytes);
    arr.push(BytesRef::from_bytes(bytes));
  }
  let shared_prefix_length = min(arr[0].length, TestUtil::next_usize(random, 1, max_len));
  for i in 1..arr.len() {
    let copy_len = min(shared_prefix_length, arr[i].length);
    let offset_1 = arr[i].offset;
    let offset_2 = arr[0].offset;
    arr[i]
      .bytes
      .copy_within(offset_2..offset_2 + copy_len, offset_1);
  }
  do_test(random, &arr, from, to, max_len)
}

pub fn do_test<R>(
  random: &mut R,
  arr: &[BytesRef<Vec<u8>>],
  from: usize,
  to: usize,
  max_len: usize,
) -> Result<()>
where
  R: Rng + ?Sized,
{
  let k = TestUtil::next_usize(random, from, to - 1);

  let mut expected = arr.to_vec();
  expected[from..to].sort();

  let mut actual = arr.to_vec();
  let enforced_max_len = if random.random_bool(0.5) {
    max_len
  } else {
    i32::MAX as usize
  };

  let selector_impl = RadixSelectorMock {
    actual,
    enforced_max_len,
  };

  let mut selector = RadixSelector::new(enforced_max_len, selector_impl);
  Selector::select(&mut selector, from, to, k)?;
  actual = selector.get_sub_selector().actual.clone();

  assert_eq!(expected[k], actual[k]);
  for i in 0..actual.len() {
    if i < from || i >= to {
      assert_eq!(&arr[i], &actual[i]);
    } else if i <= k {
      assert_ne!(actual[i].cmp(&actual[k]), Ordering::Greater);
    } else {
      assert_ne!(actual[i].cmp(&actual[k]), Ordering::Less);
    }
  }
  Ok(())
}

struct RadixSelectorMock {
  enforced_max_len: usize,
  actual: Vec<BytesRef<Vec<u8>>>,
}

impl Selector for RadixSelectorMock {
  fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    self.actual.swap(i, j);
    Ok(())
  }
}

impl RadixSelectorBase for RadixSelectorMock {
  fn byte_at(&mut self, i: usize, k: usize) -> Result<i32> {
    assert!(k < self.enforced_max_len);
    let b = self.actual[i].clone();
    if k < b.length {
      Ok(b.bytes[k] as i32)
    } else {
      Ok(-1)
    }
  }
}
