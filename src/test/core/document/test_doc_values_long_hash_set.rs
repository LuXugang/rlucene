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

use crate::core::document::doc_values_long_hash_set::DocValuesLongHashSet;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::lucene_test_case::{at_least, random};
use rand::Rng;
use rand::RngExt;
use std::collections::HashSet;
#[allow(dead_code)] // for quick search
struct TestDocValuesLongHashSet;
fn assert_eq_set<R>(random: &mut R, set1: &HashSet<i64>, long_hash_set: &DocValuesLongHashSet)
where
  R: Rng + ?Sized,
{
  assert_eq!(set1.len() as i32, long_hash_set.size);
  let set2 = long_hash_set.stream();
  assert_eq!(set1, &set2);
  if !set1.is_empty() {
    let mut set3 = set1.clone();
    let removed = *set3.iter().next().unwrap();
    loop {
      let next = random.random();

      if next != removed && set3.insert(next) {
        assert!(!long_hash_set.contains(next));
        break;
      }
    }
    assert_ne!(set3, long_hash_set.stream());
  }
  assert!(set1.iter().all(|v| long_hash_set.contains(*v)));
}
fn assert_not_eq_set(set1: &HashSet<i64>, long_hash_set: &DocValuesLongHashSet) {
  let set2 = long_hash_set.stream();
  assert_ne!(set1, &set2);
  let mut sorted: Vec<i64> = set1.iter().copied().collect();
  sorted.sort_unstable();
  let set3 =
    DocValuesLongHashSet::new(&sorted).expect("DocValuesLongHashSet construction must succeed");
  let set3_stream = set3.stream();
  assert_ne!(set2, set3_stream);
  assert!(!set1.iter().all(|v| long_hash_set.contains(*v)));
}
#[test]
fn test_empty() -> Result<()> {
  let mut random = random();
  let set1 = HashSet::new();
  let set2 = DocValuesLongHashSet::new(&[])?;
  assert_eq!(set2.size, 0);
  assert_eq!(set2.min_value, i64::MAX);
  assert_eq!(set2.max_value, i64::MIN);
  assert_eq_set(&mut random, &set1, &set2);
  Ok(())
}
#[test]
fn test_one_value() -> Result<()> {
  let mut random = random();

  let set1 = [42_i64].into_iter().collect();
  let set2 = DocValuesLongHashSet::new(&[42_i64])?;

  assert_eq!(set2.size, 1);
  assert_eq!(set2.min_value, 42);
  assert_eq!(set2.max_value, 42);

  assert_eq_set(&mut random, &set1, &set2);

  let set1 = [i64::MIN].into_iter().collect();
  let set2 = DocValuesLongHashSet::new(&[i64::MIN])?;

  assert_eq!(set2.size, 1);
  assert_eq!(set2.min_value, i64::MIN);
  assert_eq!(set2.max_value, i64::MIN);

  assert_eq_set(&mut random, &set1, &set2);

  Ok(())
}
#[test]
fn test_two_values() -> Result<()> {
  let mut random = random();

  let set1 = [42_i64, i64::MAX].into_iter().collect();
  let set2 = DocValuesLongHashSet::new(&[42_i64, i64::MAX])?;
  assert_eq!(set2.size, 2);
  assert_eq!(set2.min_value, 42);
  assert_eq!(set2.max_value, i64::MAX);
  assert_eq_set(&mut random, &set1, &set2);

  let set1 = [i64::MIN, 42_i64].into_iter().collect();
  let set2 = DocValuesLongHashSet::new(&[i64::MIN, 42_i64])?;
  assert_eq!(set2.size, 2);
  assert_eq!(set2.min_value, i64::MIN);
  assert_eq!(set2.max_value, 42);
  assert_eq_set(&mut random, &set1, &set2);

  Ok(())
}

#[test]
fn test_same_value() -> Result<()> {
  let set2 = DocValuesLongHashSet::new(&[42_i64, 42_i64])?;
  assert_eq!(set2.size, 1);
  assert_eq!(set2.min_value, 42);
  assert_eq!(set2.max_value, 42);
  Ok(())
}

#[test]
fn test_same_missing_placeholder() -> Result<()> {
  let set2 = DocValuesLongHashSet::new(&[i64::MIN, i64::MIN])?;
  assert_eq!(set2.size, 1);
  assert_eq!(set2.min_value, i64::MIN);
  assert_eq!(set2.max_value, i64::MIN);
  Ok(())
}

#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 10);

  for _ in 0..iters {
    let v = random.random_range(0..16);
    let len = random.random_range(0..(1 << v));
    let mut values = vec![0_i64; len];

    for i in 0..len {
      if i == 0 || random.random_range(0..10) < 9 {
        values[i] = random.random();
      } else {
        let idx = random.random_range(0..i);
        values[i] = values[idx];
      }
    }

    if len > 0 && random.random_bool(0.5) {
      values[len / 2] = i64::MIN;
    }
    let set1 = values.iter().copied().collect();
    values.sort_unstable();
    let set2 = DocValuesLongHashSet::new(&values)?;
    assert_eq_set(&mut random, &set1, &set2);
  }
  Ok(())
}
