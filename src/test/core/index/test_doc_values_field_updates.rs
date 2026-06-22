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
use crate::test::core::util::lucene_test_case::{random, rarely};
use rand::RngExt;
use rand::prelude::SliceRandom;

use crate::core::index::doc_values_field_updates::merged_iterator;
use crate::core::index::doc_values_field_updates::{
  DocValuesFieldIterator, DocValuesFieldUpdates, DocValuesFieldUpdatesBase,
  SingleValueDocValuesFieldUpdates, SingleValueDocValuesFieldUpdatesBase,
};
use crate::core::index::numeric_doc_values_field_updates::{
  NumericDocValuesFieldUpdates, SingleValueNumericDocValuesFieldUpdates,
};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)] // for quick search
struct TestDocValuesFieldUpdates;
#[test]
fn test_merge_iterator() -> Result<()> {
  let mut random = random();
  let sub_update1 = NumericDocValuesFieldUpdates::new()?;
  let mut updates1 = DocValuesFieldUpdates::new(6, 0, "test", sub_update1.sub_type(), sub_update1)?;
  let sub_update2 = NumericDocValuesFieldUpdates::new()?;
  let mut updates2 = DocValuesFieldUpdates::new(6, 1, "test", sub_update2.sub_type(), sub_update2)?;
  let sub_update3 = NumericDocValuesFieldUpdates::new()?;
  let mut updates3 = DocValuesFieldUpdates::new(6, 2, "test", sub_update3.sub_type(), sub_update3)?;
  let sub_update4 = NumericDocValuesFieldUpdates::new()?;
  let mut updates4 = DocValuesFieldUpdates::new(6, 3, "test", sub_update4.sub_type(), sub_update4)?;

  updates1.add_value(0, 1)?;
  updates1.add_value(4, 0)?;
  updates1.add_value(1, 4)?;
  updates1.add_value(2, 5)?;
  updates1.add_value(4, 9)?;
  assert!(updates1.any());

  updates2.add_value(0, 18)?;
  updates2.add_value(1, 7)?;
  updates2.add_value(2, 19)?;
  updates2.add_value(5, 24)?;
  assert!(updates2.any());

  updates3.add_value(2, 42)?;
  assert!(updates3.any());
  assert!(!updates4.any());

  // Finish updates
  updates1.finish()?;
  updates2.finish()?;
  updates3.finish()?;
  updates4.finish()?;

  // Create iterators
  let mut iterators = vec![
    updates1.iterator()?,
    updates2.iterator()?,
    updates3.iterator()?,
    updates4.iterator()?,
  ];

  // Shuffle iterators (simulate randomness)
  iterators.shuffle(&mut random);

  // Merge iterators
  let merged_iterator_result = merged_iterator(iterators)?;
  assert!(merged_iterator_result.is_some());
  let mut merged_iterator = merged_iterator_result.unwrap();

  // Verify merged iterator results
  assert_eq!(merged_iterator.next_doc()?, 0);
  assert_eq!(merged_iterator.long_value()?, 18);

  assert_eq!(merged_iterator.next_doc()?, 1);
  assert_eq!(merged_iterator.long_value()?, 7);

  assert_eq!(merged_iterator.next_doc()?, 2);
  assert_eq!(merged_iterator.long_value()?, 42);

  assert_eq!(merged_iterator.next_doc()?, 4);
  assert_eq!(merged_iterator.long_value()?, 9);

  assert_eq!(merged_iterator.next_doc()?, 5);
  assert_eq!(merged_iterator.long_value()?, 24);

  assert_eq!(merged_iterator.next_doc()?, NO_MORE_DOCS);
  Ok(())
}
#[test]
fn test_update_and_reset_same_doc() -> Result<()> {
  let sub_update = NumericDocValuesFieldUpdates::new()?;
  let mut updates = DocValuesFieldUpdates::new(2, 0, "test", sub_update.sub_type(), sub_update)?;

  updates.add_value(0, 1)?;
  updates.reset(0)?;
  updates.finish()?;

  let mut iterator = updates.iterator()?;
  assert_eq!(iterator.next_doc()?, 0);
  assert!(!iterator.has_value()?);
  assert_eq!(iterator.next_doc()?, NO_MORE_DOCS);

  Ok(())
}
#[test]
fn test_update_and_reset_update_same_doc() -> Result<()> {
  let sub_update = NumericDocValuesFieldUpdates::new()?;
  let mut updates = DocValuesFieldUpdates::new(3, 0, "test", sub_update.sub_type(), sub_update)?;

  updates.add_value(0, 1)?;
  updates.reset(0)?;
  updates.add_value(0, 2)?;
  updates.finish()?;

  let mut iterator = updates.iterator()?;
  assert_eq!(iterator.next_doc()?, 0);
  assert!(iterator.has_value()?);
  assert_eq!(iterator.long_value()?, 2);
  assert_eq!(iterator.next_doc()?, NO_MORE_DOCS);

  Ok(())
}
#[test]
fn test_updates_and_reset_random() -> Result<()> {
  let mut random = random();

  let sub_update = NumericDocValuesFieldUpdates::new()?;
  let mut updates = DocValuesFieldUpdates::new(10, 0, "test", sub_update.sub_type(), sub_update)?;

  let num_updates = 10 + random.random_range(0..100);
  let mut values: [Option<i32>; 5] = [None; 5];

  for (i, value) in values.iter_mut().enumerate() {
    if random.random_bool(0.5) {
      *value = None;
      updates.reset(i as i32)?;
    } else {
      let val = random.random_range(0..100);
      *value = Some(val);
      updates.add_value(i as i32, val as i64)?;
    }
  }

  for _ in 0..num_updates {
    let doc_id = random.random_range(0..5);
    if random.random_bool(0.5) {
      values[doc_id] = None;
      updates.reset(doc_id as i32)?;
    } else {
      let value = random.random_range(0..100);
      values[doc_id] = Some(value);
      updates.add_value(doc_id as i32, value as i64)?;
    }
  }

  updates.finish()?;

  // Test iterator could be reused multiple times
  let iter = random.random_range(0..2);
  for _ in 0..iter {
    let mut iterator = updates.iterator()?;
    let mut idx = 0;

    while iterator.next_doc()? != NO_MORE_DOCS {
      assert_eq!(idx, iterator.doc_id() as usize);
      if values[idx].is_none() {
        assert!(!iterator.has_value()?);
      } else {
        assert!(iterator.has_value()?);
        assert_eq!(values[idx].unwrap() as i64, iterator.long_value()?);
      }
      idx += 1;
    }
  }

  Ok(())
}
#[test]
fn test_shared_value_updates() -> Result<()> {
  let mut random = random();

  let del_gen = random.random::<i64>();
  let max_doc: i32 = 1 + random.random_range(0..1000);
  let value = random.random::<i64>();

  let sub_update1 = SingleValueNumericDocValuesFieldUpdates::new(value);
  let sub_type = sub_update1.sub_type();
  let sub_update2 = SingleValueDocValuesFieldUpdates::new(sub_update1, max_doc, del_gen, sub_type)?;
  let mut update = DocValuesFieldUpdates::new(max_doc, del_gen, "foo", sub_type, sub_update2)?;
  assert_eq!(value, update.sub_update.long_value()?);

  let mut values: Vec<Option<bool>> = vec![None; max_doc as usize];
  let mut any = false;
  let no_reset = random.random_bool(0.5);

  for (i, tmp_value) in values.iter_mut().enumerate() {
    if random.random_bool(0.5) {
      *tmp_value = Some(true);
      any = true;
      update.add_value(i as i32, value)?;
    } else if random.random_bool(0.5) && !no_reset {
      *tmp_value = None;
      any = true;
      update.reset(i as i32)?;
    } else {
      *tmp_value = Some(false);
    }
  }

  if !no_reset {
    for (i, tmp_value) in values.iter_mut().enumerate() {
      if rarely(&mut random) {
        if tmp_value.is_none() {
          *tmp_value = Some(true);
          update.add_value(i as i32, value)?;
        } else if *tmp_value == Some(true) {
          *tmp_value = None;
          update.reset(i as i32)?;
        }
      }
    }
  }

  update.finish()?;
  assert_eq!(any, update.any());
  let mut iterator = update.iterator()?;
  assert_eq!(del_gen, iterator.del_gen());

  let mut index = 0;

  while iterator.next_doc()? != NO_MORE_DOCS {
    let doc = iterator.doc_id() as usize;

    if index < doc {
      values[index..doc]
        .iter()
        .for_each(|value| assert_eq!(*value, Some(false)));
      index = doc;
    }

    if index == doc {
      if values[index].is_none() {
        assert!(!iterator.has_value()?);
      } else {
        assert!(iterator.has_value()?);
        assert_eq!(value, iterator.long_value()?);
      }
      index += 1;
    }
  }

  Ok(())
}
