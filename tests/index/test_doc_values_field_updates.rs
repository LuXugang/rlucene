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
use crate::common::{my_random, rarely};
use crate::util::test_error::TestError;
use rand::seq::SliceRandom;
use rand::Rng;
use rlucene::index::doc_values_field_updates::{
    merged_iterator, DocValuesFieldUpdates, DocValuesFieldUpdatesBase, Iterator,
    SingleValueDocValuesFieldUpdates, SingleValueDocValuesFieldUpdatesBase,
};
use rlucene::index::numeric_doc_values_field_updates::{
    NumericDocValuesFieldUpdates, SingleValueNumericDocValuesFieldUpdates,
};
use rlucene::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};

#[allow(dead_code)] // for quick search
pub struct TestDocValuesFieldUpdates;
#[test]
fn test_merge_iterator() -> Result<(), TestError> {
    let mut random = my_random("test_merge_iterator".to_string());
    let sub_update1 = NumericDocValuesFieldUpdates::new()?;
    let mut updates1 = DocValuesFieldUpdates::new(
        6,
        0,
        "test".to_string(),
        sub_update1.sub_type(),
        sub_update1,
    )?;
    let sub_update2 = NumericDocValuesFieldUpdates::new()?;
    let mut updates2 = DocValuesFieldUpdates::new(
        6,
        1,
        "test".to_string(),
        sub_update2.sub_type(),
        sub_update2,
    )?;
    let sub_update3 = NumericDocValuesFieldUpdates::new()?;
    let mut updates3 = DocValuesFieldUpdates::new(
        6,
        2,
        "test".to_string(),
        sub_update3.sub_type(),
        sub_update3,
    )?;
    let sub_update4 = NumericDocValuesFieldUpdates::new()?;
    let mut updates4 = DocValuesFieldUpdates::new(
        6,
        3,
        "test".to_string(),
        sub_update4.sub_type(),
        sub_update4,
    )?;

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
fn test_update_and_reset_same_doc() -> Result<(), TestError> {
    let sub_update = NumericDocValuesFieldUpdates::new()?;
    let mut updates =
        DocValuesFieldUpdates::new(2, 0, "test".to_string(), sub_update.sub_type(), sub_update)?;

    updates.add_value(0, 1)?;
    updates.reset(0)?;
    updates.finish()?;

    let mut iterator = updates.iterator()?;
    assert_eq!(iterator.next_doc()?, 0);
    assert!(!iterator.has_value());
    assert_eq!(iterator.next_doc()?, NO_MORE_DOCS);

    Ok(())
}
#[test]
fn test_update_and_reset_update_same_doc() -> Result<(), TestError> {
    let sub_update = NumericDocValuesFieldUpdates::new()?;
    let mut updates =
        DocValuesFieldUpdates::new(3, 0, "test".to_string(), sub_update.sub_type(), sub_update)?;

    updates.add_value(0, 1)?;
    updates.reset(0)?;
    updates.add_value(0, 2)?;
    updates.finish()?;

    let mut iterator = updates.iterator()?;
    assert_eq!(iterator.next_doc()?, 0);
    assert!(iterator.has_value());
    assert_eq!(iterator.long_value()?, 2);
    assert_eq!(iterator.next_doc()?, NO_MORE_DOCS);

    Ok(())
}
#[test]
fn test_updates_and_reset_random() -> Result<(), TestError> {
    let mut random = my_random("test_updates_and_reset_random".to_string());

    let sub_update = NumericDocValuesFieldUpdates::new()?;
    let mut updates =
        DocValuesFieldUpdates::new(10, 0, "test".to_string(), sub_update.sub_type(), sub_update)?;

    let num_updates = 10 + random.gen_range(0..100);
    let mut values: [Option<i32>; 5] = [None; 5];

    for i in 0..5 {
        if random.gen_bool(0.5) {
            values[i] = None;
            updates.reset(i as u32)?;
        } else {
            let value = random.gen_range(0..100);
            values[i] = Some(value);
            updates.add_value(i as u32, value as i64)?;
        }
    }

    for _ in 0..num_updates {
        let doc_id = random.gen_range(0..5);
        if random.gen_bool(0.5) {
            values[doc_id] = None;
            updates.reset(doc_id as u32)?;
        } else {
            let value = random.gen_range(0..100);
            values[doc_id] = Some(value);
            updates.add_value(doc_id as u32, value as i64)?;
        }
    }

    updates.finish()?;

    let mut iterator = updates.iterator()?;
    let mut idx = 0;

    while iterator.next_doc()? != NO_MORE_DOCS {
        assert_eq!(idx, iterator.doc_id() as usize);
        if values[idx].is_none() {
            assert!(!iterator.has_value());
        } else {
            assert!(iterator.has_value());
            assert_eq!(values[idx].unwrap() as i64, iterator.long_value()?);
        }
        idx += 1;
    }

    Ok(())
}
#[test]
fn test_shared_value_updates() -> Result<(), TestError> {
    let mut random = my_random("test_shared_value_updates".to_string());

    let del_gen = random.gen::<u64>();
    let max_doc: u32 = 1 + random.gen_range(0..1000);
    let value = random.gen::<i64>();

    let sub_update1 = SingleValueNumericDocValuesFieldUpdates::new(value);
    let sub_type = sub_update1.sub_type();
    let sub_update2 =
        SingleValueDocValuesFieldUpdates::new(sub_update1, max_doc, del_gen, sub_type)?;
    let mut update =
        DocValuesFieldUpdates::new(max_doc, del_gen, "foo".to_string(), sub_type, sub_update2)?;

    assert_eq!(value, update.sub_update.long_value()?);

    let mut values: Vec<Option<bool>> = vec![None; max_doc as usize];
    let mut any = false;
    let no_reset = random.gen_bool(0.5);

    for i in 0..max_doc as usize {
        if random.gen_bool(0.5) {
            values[i] = Some(true);
            any = true;
            update.add_value(i as u32, value)?;
        } else if random.gen_bool(0.5) && !no_reset {
            values[i] = None;
            any = true;
            update.reset(i as u32)?;
        } else {
            values[i] = Some(false);
        }
    }

    if !no_reset {
        for i in 0..values.len() {
            if rarely(&mut random) {
                if values[i].is_none() {
                    values[i] = Some(true);
                    update.add_value(i as u32, value)?;
                } else if values[i] == Some(true) {
                    values[i] = None;
                    update.reset(i as u32)?;
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
            for idx in index..doc {
                assert_eq!(values[idx], Some(false));
            }
            index = doc;
        }

        if index == doc {
            if values[index].is_none() {
                assert!(!iterator.has_value());
            } else {
                assert!(iterator.has_value());
                assert_eq!(value, iterator.long_value()?);
            }
            index += 1;
        }
    }

    Ok(())
}
