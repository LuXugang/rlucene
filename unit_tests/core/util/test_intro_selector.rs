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
// Migrated from src/core/util/intro_selector.rs

use crate::test_framework::core::util::lucene_test_case::random;
use rand::{Rng, RngExt};

use crate::core::util::error::lucene_error::Result;
use crate::core::util::selector::Selector;
use crate::core::util::{IntroSelector, IntroSelectorBase, IntroSelectorBaseDefault, ToInt};
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
pub struct TestIntroSelector;

#[test]
pub fn test_select() -> Result<()> {
  let mut random = random();
  for _ in 0..100 {
    do_test_select(&mut random)?;
  }
  Ok(())
}

pub fn do_test_select<R>(random: &mut R) -> Result<()>
where
  R: Rng + ?Sized,
{
  let from = random.random_range(0..5);
  let to = from + TestUtil::next_usize(random, 1, 10000);
  let max = if random.random_bool(0.5) {
    random.random_range(0..100)
  } else {
    random.random_range(0..100000)
  };

  let arr: Vec<i32> = if max == 0 {
    vec![0; to + random.random_range(0..5)]
  } else {
    (0..(to + random.random_range(0..5)))
      .map(|_| TestUtil::next_int(random, 0, max))
      .collect()
  };

  let k = TestUtil::next_usize(random, from, to - 1);
  let mut expected = arr.clone();
  let mut actual = arr.clone();
  expected[from..to].sort();
  let sub_selector = IntroSelectorMock::new(&mut actual);
  let mut selector = IntroSelector::new(sub_selector);
  if random.random_bool(0.5) {
    Selector::select(&mut selector, from, to, k)?;
  } else {
    IntroSelector::select(&mut selector, from, to, k, random.random_range(0..3))?;
  }
  assert_eq!(expected[k], actual[k]);
  for i in 0..actual.len() {
    if i < from || i >= to {
      assert_eq!(arr[i], actual[i]);
    } else if i <= k {
      assert!(actual[i] <= actual[k]);
    } else {
      assert!(actual[i] >= actual[k]);
    }
  }
  Ok(())
}

pub struct IntroSelectorMock<'a> {
  pivot: i32,
  actual: &'a mut Vec<i32>,
}
impl<'a> IntroSelectorMock<'a> {
  fn new(actual: &'a mut Vec<i32>) -> IntroSelectorMock<'a> {
    IntroSelectorMock { pivot: 0, actual }
  }
}
impl Selector for IntroSelectorMock<'_> {
  fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    self.actual.swap(i, j);
    Ok(())
  }
}

impl IntroSelectorBaseDefault for IntroSelectorMock<'_> {
  fn set_pivot(&mut self, i: usize) -> Result<()> {
    self.pivot = self.actual[i];
    Ok(())
  }

  fn compare_pivot(&mut self, j: usize) -> Result<i32> {
    Ok(self.pivot.cmp(&self.actual[j]).to_int())
  }
}

impl IntroSelectorBase for IntroSelectorMock<'_> {}
