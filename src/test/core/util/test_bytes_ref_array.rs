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
// Migrated from src/core/util/bytes_ref_array.rs

use crate::test::core::util::lucene_test_case::{at_least_usize, random};
use std::sync::Arc;

use rand::RngExt;

use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::{
  AtomicCounter, BytesRefArray, IndexedBytesRefIterator, Natural, NaturalOrder,
  SortableBytesRefArray,
};
use crate::test::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestBytesRefArray;
#[test]
fn test_append() -> Result<()> {
  let mut random = random();
  let counter = Arc::new(AtomicCounter::new());
  let mut list = BytesRefArray::new(counter)?;
  let mut string_list = Vec::new();

  for j in 0..2 {
    if j > 0 && random.random_bool(0.5) {
      list.clear();
      string_list.clear();
    }

    let entries = at_least_usize(&mut random, 500);
    let mut spare = BytesRefBuilder::new();
    let init_size = list.size();
    for i in 0..entries {
      let random_realistic_unicode_string = TestUtil::random_realistic_unicode_string(&mut random);
      spare.copy_chars_from_string(&random_realistic_unicode_string);
      assert_eq!(i + init_size, list.append(spare.get_bytes_mut_ref())?);
      string_list.push(random_realistic_unicode_string);
    }
    for (i, expected) in string_list.iter().take(entries).enumerate() {
      assert_eq!(
        *expected,
        list
          .get(&mut spare, i)
          .expect("not fail")
          .utf8_to_string()?,
        "entry {} doesn't match",
        i
      );
    }

    // Check random access
    for _i in 0..entries {
      let e = random.random_range(0..entries);
      assert_eq!(
        string_list[e],
        list
          .get(&mut spare, e)
          .expect("not fail")
          .utf8_to_string()?,
        "entry {} doesn't match",
        e
      );
    }

    // Check iterator multiple times
    for _ in 0..2 {
      let mut iterator = list.iterator();
      for string in &string_list {
        let value = iterator.next()?;
        assert!(value.is_some());
        assert_eq!(*string, value.expect("not fail").utf8_to_string()?,);
      }
    }
  }
  Ok(())
}
#[test]
fn test_sort() -> Result<()> {
  let mut random = random();
  let counter = Arc::new(AtomicCounter::new());
  let mut list = BytesRefArray::new(counter)?;
  let mut string_list = Vec::new();

  for j in 0..5 {
    if j > 0 && random.random_bool(0.5) {
      list.clear();
      string_list.clear();
    }

    let entries = at_least_usize(&mut random, 200);
    let mut spare = BytesRefBuilder::new();
    let init_size = list.size();

    for i in 0..entries {
      let random_realistic_unicode_string = TestUtil::random_realistic_unicode_string(&mut random);
      spare.copy_chars_from_string(&random_realistic_unicode_string);
      assert_eq!(init_size + i, list.append(spare.get_bytes_mut_ref())?);
      string_list.push(random_realistic_unicode_string);
    }

    string_list.sort_by(|a, b| TestUtil::string_codepoint_comparator(a, b));
    {
      let mut iter1 = SortableBytesRefArray::iterator(&list, Natural::default())?;

      let mut i = 0;
      while let Some(next) = iter1.next()? {
        assert_eq!(
          string_list[i],
          next.utf8_to_string()?,
          "entry {} doesn't match",
          i
        );
        i += 1;
      }
      assert!(iter1.next()?.is_none());
      assert_eq!(
        i,
        string_list.len(),
        "Iterated count doesn't match sorted list size"
      );
    }

    let mut iter2 = SortableBytesRefArray::iterator(&list, NaturalOrder)?;
    let mut i = 0;
    while let Some(next) = iter2.next()? {
      assert_eq!(
        string_list[i],
        next.utf8_to_string()?,
        "entry {} doesn't match",
        i
      );
      i += 1;
    }
    assert!(iter2.next()?.is_none());
    assert_eq!(
      i,
      string_list.len(),
      "Iterated count doesn't match sorted list size"
    );
  }

  Ok(())
}
#[test]
fn test_stable_sort() -> Result<()> {
  let mut random = random();

  let counter = Arc::new(AtomicCounter::new());
  let mut list = BytesRefArray::new(counter)?;

  let mut string_list = Vec::new();

  for j in 0..5 {
    if j > 0 && random.random_bool(0.5) {
      list.clear();
      string_list.clear();
    }

    let entries = at_least_usize(&mut random, 200);

    let mut values = Vec::new();
    for _ in 0..20 {
      values.push(TestUtil::random_realistic_unicode_string(&mut random));
    }

    let mut spare = BytesRefBuilder::new();
    let init_size = list.size();
    for i in 0..entries {
      let random_realistic_unicode_string = values[random.random_range(0..values.len())].clone();
      spare.copy_chars_from_string(&random_realistic_unicode_string);
      assert_eq!(init_size + i, list.append(spare.get_bytes_mut_ref())?);
      string_list.push(random_realistic_unicode_string);
    }

    string_list.sort_by(|a, b| TestUtil::string_codepoint_comparator(a, b));

    let sort_state = if random.random_bool(0.5) {
      list.sort(NaturalOrder, true)?
    } else {
      list.sort(Natural::default(), true)?
    };
    let mut iter = list.iterator_with_state(Arc::new(sort_state));
    let mut i = 0;
    let mut last_ord = None;
    let mut last = None;

    while let Some(next) = iter.next()? {
      let next = next.into_owned();
      assert_eq!(
        string_list[i],
        next.utf8_to_string()?,
        "entry {} doesn't match",
        i
      );

      if let Some(last_ref) = &last
        && next == *last_ref
      {
        let ord = iter.ord();
        assert!(last_ord.is_none() || Some(ord) > last_ord);
      }

      last = Some(BytesRef::deep_copy_of(&next));
      last_ord = Some(iter.ord());
      i += 1;
    }

    assert!(iter.next()?.is_none());
    assert_eq!(
      i,
      string_list.len(),
      "Iterated count doesn't match sorted list size"
    );
  }

  Ok(())
}
