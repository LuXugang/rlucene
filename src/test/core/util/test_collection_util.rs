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
// Migrated from src/core/util/collection_util.rs

use crate::core::util::ReverseOrder;
use crate::core::util::collection_util::CollectionUtil;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::lucene_test_case::{at_least, random};
use rand::Rng;
use rand::RngExt;
#[allow(dead_code)] // for quick search
struct TestCollectionUtil;
fn create_random_list<R>(random: &mut R, max_size: usize) -> Vec<i32>
where
  R: Rng + ?Sized,
{
  let len = random.random_range(1..=max_size);
  (0..len)
    .map(|_| random.random_range(0..len as i32))
    .collect()
}
#[test]
fn test_intro_sort() -> Result<()> {
  let mut random = random();
  for _ in 0..at_least(&mut random, 100) {
    let mut list1 = create_random_list(&mut random, 2000);
    let mut list2 = list1.clone();
    CollectionUtil::intro_sort(&mut list1)?;
    list2.sort();
    assert_eq!(list1, list2);
    let mut list1 = create_random_list(&mut random, 2000);
    let mut list2 = list1.clone();
    CollectionUtil::intro_sort_with_comparator(&mut list1, ReverseOrder::new())?;
    list2.sort_by(|a, b| b.cmp(a));
    assert_eq!(list1, list2);
    CollectionUtil::intro_sort(&mut list1)?;
    list2.sort();
    assert_eq!(list1, list2);
  }

  Ok(())
}

#[test]
fn test_tim_sort() -> Result<()> {
  let mut random = random();
  for _ in 0..at_least(&mut random, 100) {
    let mut list1 = create_random_list(&mut random, 2000);
    let mut list2 = list1.clone();
    CollectionUtil::tim_sort(&mut list1)?;
    list2.sort();
    assert_eq!(list1, list2);

    let mut list1 = create_random_list(&mut random, 2000);
    let mut list2 = list1.clone();
    CollectionUtil::tim_sort_with_comparator(&mut list1, ReverseOrder::new())?;
    list2.sort_by(|a, b| b.cmp(a));
    assert_eq!(list1, list2);

    CollectionUtil::tim_sort(&mut list1)?;
    list2.sort();
    assert_eq!(list1, list2);
  }

  Ok(())
}
#[test]
fn test_empty_list_sort() -> Result<()> {
  let mut vec: Vec<i32> = Vec::new();
  CollectionUtil::intro_sort(&mut vec)?;
  CollectionUtil::tim_sort(&mut vec)?;
  CollectionUtil::intro_sort_with_comparator(&mut vec, ReverseOrder::new())?;
  CollectionUtil::tim_sort_with_comparator(&mut vec, ReverseOrder::new())?;

  use std::collections::VecDeque;

  let list: VecDeque<i32> = VecDeque::new();
  let mut vec2: Vec<i32> = list.into_iter().collect();
  CollectionUtil::intro_sort(&mut vec2)?;
  CollectionUtil::tim_sort(&mut vec2)?;
  CollectionUtil::intro_sort_with_comparator(&mut vec2, ReverseOrder::new())?;
  CollectionUtil::tim_sort_with_comparator(&mut vec2, ReverseOrder::new())?;

  Ok(())
}

#[test]
fn test_one_element_list_sort() -> Result<()> {
  let mut list = Vec::new();
  list.push(1);
  CollectionUtil::intro_sort(&mut list)?;
  CollectionUtil::tim_sort(&mut list)?;
  CollectionUtil::intro_sort_with_comparator(&mut list, ReverseOrder::new())?;
  CollectionUtil::tim_sort_with_comparator(&mut list, ReverseOrder::new())?;
  Ok(())
}
