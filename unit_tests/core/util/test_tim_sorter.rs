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
// Migrated from src/core/util/tim_sorter.rs

use crate::test_framework::core::util::lucene_test_case::random;
use rand::Rng;

use crate::core::util::array_tim_sorter::ArrayTimSorter;
use crate::core::util::{NaturalOrder, Sorter};
use crate::test_framework::core::util::test_util::TestUtil;
use crate::util_tests::base_sort_test_case::{BaseSortTestCase, Entry};
#[allow(dead_code)] // for quick search
struct TestTimSorter;

impl TestTimSorter {
  fn default() -> Self {
    TestTimSorter {}
  }
}

impl BaseSortTestCase for TestTimSorter {
  fn new_sorter<R>(&self, random: &mut R, arr: &mut Vec<Entry>) -> impl Sorter
  where
    R: Rng + ?Sized,
  {
    let arr_len = arr.len();
    let max_temp_slots = TestUtil::next_usize(random, 0, arr_len);
    ArrayTimSorter::new(arr, NaturalOrder::new(), max_temp_slots)
  }

  fn get_stable(&self) -> bool {
    true
  }
}

#[test]
fn test_empty() {
  let mut random = random();
  let case = TestTimSorter::default();
  case.test_empty(&mut random);
}
#[test]
fn test_one() {
  let mut random = random();
  let case = TestTimSorter::default();
  case.test_one(&mut random);
}
#[test]
fn test_two() {
  let mut random = random();
  let case = TestTimSorter::default();
  case.test_two(&mut random);
}
#[test]
fn test_random() {
  let mut random = random();
  let case = TestTimSorter::default();
  case.test_random(&mut random);
}
#[test]
fn test_random_low_cardinality() {
  let mut random = random();
  let case = TestTimSorter::default();
  case.test_random_low_cardinality(&mut random);
}
#[test]
fn test_ascending() {
  let mut random = random();
  let case = TestTimSorter::default();
  case.test_ascending(&mut random);
}
#[test]
fn test_ascending_sequences() {
  let mut random = random();
  let case = TestTimSorter::default();
  case.test_ascending_sequences(&mut random);
}
#[test]
fn test_descending() {
  let mut random = random();
  let case = TestTimSorter::default();
  case.test_descending(&mut random);
}
#[test]
fn test_strictly_descending() {
  let mut random = random();
  let case = TestTimSorter::default();
  case.test_strictly_descending(&mut random);
}
