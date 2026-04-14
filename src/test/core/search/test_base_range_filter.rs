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
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::new_directory_shared;
use rand::Rng;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub struct TestBaseRangeFilter;

struct TestIndex {
  max_r: i32,
  min_r: i32,
  allow_negative_random_ints: bool,
  index: Arc<DirEnum>,
}

impl TestIndex {
  fn new<R>(
    random: &mut R,
    min_r: i32,
    max_r: i32,
    allow_negative_random_ints: bool,
  ) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    Ok(Self {
      min_r,
      max_r,
      allow_negative_random_ints,
      index: new_directory_shared(random)?,
    })
  }
}
pub fn pad(n: i32) -> String {
  let mut b = String::with_capacity(40);
  let mut p = "0";
  let mut n = n;

  if n < 0 {
    p = "-";
    n = i32::MAX + n + 1;
  }

  b.push_str(p);

  let s = n.to_string();
  for _ in s.len()..=i32::MAX.to_string().len() {
    b.push('0');
  }
  b.push_str(&s);

  b
}
#[test]
fn test_pad() {
  let tests = [
    -9_999_999,
    -99_560,
    -100,
    -3,
    -1,
    0,
    3,
    9,
    10,
    1000,
    999_999_999,
  ];

  for i in 0..tests.len() - 1 {
    let a = tests[i];
    let b = tests[i + 1];
    let aa = pad(a);
    let bb = pad(b);
    let label = format!("{a}:{aa} vs {b}:{bb}");

    assert_eq!(aa.len(), bb.len(), "length of {label}");
    assert!(aa < bb, "compare less than {label}");
  }
}
