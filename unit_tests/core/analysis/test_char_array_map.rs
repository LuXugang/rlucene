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
use crate::core::analysis::char_array_map::CharArrayMap;
use crate::test::support::core::util::lucene_test_case::{random, random_multiplier};
use rand::Rng;
use rand::RngExt;
use std::collections::HashMap;
#[allow(dead_code)]
struct TestCharArrayMap;

fn do_random<R>(random: &mut R, iter: i32, ignore_case: bool)
where
  R: Rng + ?Sized,
{
  let mut cmap = CharArrayMap::new(ignore_case);
  let mut hmap: HashMap<String, i32> = HashMap::new();

  for _ in 0..iter {
    let len = random.random_range(0..5);
    let key: Vec<char> = (0..len)
      .map(|_| random.random_range(0..127) as u8 as char)
      .collect();
    let key_str: String = key.iter().collect();
    let hmap_key = if ignore_case {
      key_str.to_lowercase()
    } else {
      key_str.clone()
    };

    let val: i32 = random.random();

    let o1 = cmap.put(&key, val);
    let o2 = hmap.insert(hmap_key.clone(), val);
    assert_eq!(o1, o2, "put return value mismatch");
    assert_eq!(
      val,
      cmap.put_str(&key_str, val).unwrap(),
      "put_str mismatch"
    );
    assert_eq!(
      val,
      *cmap.get(&key, 0, key.len() as i32).unwrap(),
      "get(&[char], off, len) mismatch"
    );
    assert_eq!(
      Some(&val),
      cmap.get(&key, 0, key.len() as i32),
      "get(&[char]) mismatch"
    );
    assert_eq!(Some(&val), cmap.get_str(&key_str), "get(&str) mismatch");
    assert_eq!(hmap.len(), cmap.size());
  }
}
#[test]
fn test_char_array_map() {
  let mut random = random();
  let num = 5 * random_multiplier();
  for i in 0..num {
    do_random(&mut random, i, false);
    do_random(&mut random, i, true);
  }
}

#[test]
fn test_put_all_variants() {
  use std::collections::HashMap;
  let mut cmap = CharArrayMap::new(true);

  let mut v1: HashMap<Vec<char>, i32> = HashMap::new();
  v1.insert(vec!['a'], 1);
  cmap.put_all(v1);
  assert!(cmap.contains_key_str("a"));

  let mut v2: HashMap<String, i32> = HashMap::new();
  v2.insert("b".to_string(), 2);
  cmap.put_all_str(v2);
  assert!(cmap.contains_key_str("b"));

  let mut v3: HashMap<i32, i32> = HashMap::new();
  v3.insert(3, 3);
  cmap.put_all_any(v3);
  assert!(cmap.contains_key_str("3"));
}
#[test]
fn test_methods() {
  // TODO
}
#[test]
fn test_modify_on_unmodifiable() -> crate::core::util::error::lucene_error::Result<()> {
  test_not_required_in_rust_lucene!();
}
#[test]
fn test_to_string() {
  let mut cm = CharArrayMap::new(false);
  cm.put("test".chars().collect::<Vec<char>>(), 1);
  assert_eq!("{test=1}", cm.to_string());
  cm.put("test2".chars().collect::<Vec<char>>(), 2);
  assert!(cm.to_string().contains(", "));
}
