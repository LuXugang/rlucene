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
use crate::test::core::util::lucene_test_case::random;
use rand::RngExt;

use crate::core::util::automation::int_set::IntSet;
use crate::core::util::automation::state_set::StateSet;
use crate::core::util::error::lucene_error::Result;
#[allow(dead_code)] // for quick search
struct TestIntSet;
#[test]
fn test_freeze_equality_small_set() {
  test_freeze_equality(10)
}

#[test]
fn test_freeze_equality_large_set() {
  test_freeze_equality(100)
}

fn test_freeze_equality(size: i32) {
  let mut random = random();
  let mut state_set = StateSet::new(0);
  for i in 0..size {
    let val = random.random_range(0..=i);
    state_set.incr(val);
  }
  let mut frozen0 = state_set.freeze(0);
  assert_equal(&mut state_set, &mut frozen0);

  let state = random.random();
  let mut frozen1 = state_set.freeze(state);
  assert_equal(&mut state_set, &mut frozen1);
  assert_equal(&mut frozen0, &mut frozen1);
}

fn assert_equal(state_set1: &mut impl IntSet, state2: &mut impl IntSet) {
  assert!(
    state_set1.long_hash_code() == state2.long_hash_code()
      && state_set1.get_array() == state2.get_array()
  );
}
#[test]
fn test_map_cutover() -> Result<()> {
  let mut set = StateSet::new(10);
  for i in 0..35 {
    // No duplicates so there are enough elements to trigger impl cutover
    set.incr(i);
  }
  assert!(set.size() > 32);
  for i in 0..35 {
    // This is pretty much the worst case, perf wise
    set.decr(i);
  }

  assert_eq!(set.size(), 0);
  Ok(())
}
#[test]
fn test_modify() -> Result<()> {
  let mut set = StateSet::new(2);
  set.incr(1);
  set.incr(2);
  let mut set2 = set.freeze(0);
  assert_equal(&mut set, &mut set2);

  set.incr(1);
  assert_equal(&mut set, &mut set2);

  set.decr(1);
  assert_equal(&mut set, &mut set2);

  set.decr(1);
  assert_ne!(
    (set.long_hash_code(), set.get_array()),
    (set2.long_hash_code(), set2.get_array())
  );

  Ok(())
}
#[test]
fn test_hash_code() -> Result<()> {
  let mut set = StateSet::new(1000);
  let mut set2 = StateSet::new(100);
  for i in 0..100 {
    set.incr(i);
    set2.incr(99 - i);
  }
  assert_eq!(set.long_hash_code(), set2.long_hash_code());
  Ok(())
}
