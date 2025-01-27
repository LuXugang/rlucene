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
use crate::util::lucene_test_case::{random, rarely};
use crate::util::test_error::TestError;
use crate::util::TestUtil;
use rand::rngs::StdRng;
use rand::Rng;
use rlucene::index::buffered_updates::BufferedUpdates;
use rlucene::index::doc_values_type::DocValuesType;
use rlucene::index::doc_values_update::{
    BinaryDocValuesUpdate, DocValuesUpdate, DocValuesUpdateEnum, NumericDocValuesUpdate,
};
use rlucene::index::field_updates_buffer::FieldUpdatesBuffer;
use rlucene::index::term::Term;
use rlucene::index::BytesRef;
use rlucene::util::CounterEnum;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[allow(dead_code)] // for quick search
pub struct TestFieldUpdatesBuffer;

#[test]
pub fn test_basics() -> Result<(), TestError> {
    let counter = Arc::new(Mutex::new(CounterEnum::new_counter(false)));
    let update = DocValuesUpdate::new(
        DocValuesType::Numeric,
        Term::from_text("id".to_string(), "1"),
        "age".to_string(),
        BufferedUpdates::MAX_INT,
        DocValuesUpdateEnum::Numeric(NumericDocValuesUpdate::new(Option::from(6))),
    );
    let mut buffer = FieldUpdatesBuffer::from_numeric_update(counter.clone(), update, 15)?;
    buffer.add_update_with_long(Term::from_text("id".to_string(), "10"), 6, 15)?;
    assert!(buffer.has_single_value());
    buffer.add_update_with_long(Term::from_text("id".to_string(), "8"), 12, 15)?;
    assert!(!buffer.has_single_value());
    buffer.add_update_with_long(Term::from_text("some_other_field".to_string(), "8"), 13, 17)?;
    assert!(!buffer.has_single_value());
    buffer.add_update_with_long(Term::from_text("id".to_string(), "8"), 12, 16)?;
    assert!(!buffer.has_single_value());
    assert!(buffer.is_numeric());
    assert_eq!(buffer.get_max_numeric(), 13);
    assert_eq!(buffer.get_min_numeric(), 6);
    buffer.finish()?;
    let mut iterator = buffer.iterator()?;
    let mut count = 0;
    while let Some(value) = iterator.next_value()? {
        match count {
            0 => {
                assert_eq!(value.term_field, "id");
                assert_eq!(value.term_value.unwrap().utf8_to_string()?, "1");
                assert_eq!(value.numeric_value, 6);
                assert_eq!(value.doc_up_to, 15);
            }
            1 => {
                assert_eq!(value.term_field, "id");
                assert_eq!(value.term_value.unwrap().utf8_to_string()?, "10");
                assert_eq!(value.numeric_value, 6);
                assert_eq!(value.doc_up_to, 15);
            }
            2 => {
                assert_eq!(value.term_field, "id");
                assert_eq!(value.term_value.unwrap().utf8_to_string()?, "8");
                assert_eq!(value.numeric_value, 12);
                assert_eq!(value.doc_up_to, 15);
            }
            3 => {
                assert_eq!(value.term_field, "some_other_field");
                assert_eq!(value.term_value.unwrap().utf8_to_string()?, "8");
                assert_eq!(value.numeric_value, 13);
                assert_eq!(value.doc_up_to, 17);
            }
            4 => {
                assert_eq!(value.term_field, "id");
                assert_eq!(value.term_value.unwrap().utf8_to_string()?, "8");
                assert_eq!(value.numeric_value, 12);
                assert_eq!(value.doc_up_to, 16);
            }
            _ => unreachable!(),
        }
        count += 1;
    }
    Ok(())
}
#[test]
fn test_update_share_values() -> Result<(), TestError> {
    let mut random = random();
    let counter = Arc::new(Mutex::new(CounterEnum::new_counter(false)));
    let int_value = random.gen::<i32>();
    let value_for_three = random.gen_bool(0.5);
    let sub_update =
        DocValuesUpdateEnum::Numeric(NumericDocValuesUpdate::new(Option::from(int_value as i64)));
    let update = DocValuesUpdate::new(
        DocValuesType::Numeric,
        Term::from_text("id".to_string(), "0"),
        "enabled".to_string(),
        BufferedUpdates::MAX_INT,
        sub_update,
    );
    let mut buffer = FieldUpdatesBuffer::from_numeric_update(counter.clone(), update, i32::MAX)?;
    buffer.add_update_with_long(
        Term::from_text("id".to_string(), "1"),
        int_value as i64,
        i32::MAX,
    )?;
    buffer.add_update_with_long(
        Term::from_text("id".to_string(), "2"),
        int_value as i64,
        i32::MAX,
    )?;
    if value_for_three {
        buffer.add_update_with_long(
            Term::from_text("id".to_string(), "3"),
            int_value as i64,
            i32::MAX,
        )?;
    } else {
        buffer.add_no_value(Term::from_text("id".to_string(), "3"), i32::MAX)?;
    }
    buffer.add_update_with_long(
        Term::from_text("id".to_string(), "4"),
        int_value as i64,
        i32::MAX,
    )?;
    buffer.finish()?;

    let mut iterator = buffer.iterator()?;
    let mut count = 0;
    while let Some(value) = iterator.next_value()? {
        let has_value = count != 3 || value_for_three;
        assert_eq!(
            count.to_string(),
            value.term_value.unwrap().utf8_to_string()?
        );
        assert_eq!("id", value.term_field);
        assert_eq!(has_value, value.has_value);
        if has_value {
            assert_eq!(int_value as i64, value.numeric_value);
        } else {
            assert_eq!(0, value.numeric_value);
        }
        assert_eq!(i32::MAX, value.doc_up_to);
        count += 1;
    }
    assert!(buffer.is_numeric());
    Ok(())
}
#[test]
pub fn test_update_share_values_binary() -> Result<(), TestError> {
    let mut random = random();
    let counter = Arc::new(Mutex::new(CounterEnum::new_counter(false)));
    let value_for_three = random.gen_bool(0.5);
    let sub_update = DocValuesUpdateEnum::Binary(BinaryDocValuesUpdate::new(Option::from(
        BytesRef::from_string(""),
    )));
    let update = DocValuesUpdate::new(
        DocValuesType::Binary,
        Term::from_text("id".to_string(), "0"),
        "enabled".to_string(),
        BufferedUpdates::MAX_INT,
        sub_update,
    );
    let mut buffer = FieldUpdatesBuffer::from_binary_update(counter.clone(), update, i32::MAX)?;
    buffer.add_update_with_bytes_ref(
        Term::from_text("id".to_string(), "1"),
        &BytesRef::from_string(""),
        i32::MAX,
    )?;
    buffer.add_update_with_bytes_ref(
        Term::from_text("id".to_string(), "2"),
        &BytesRef::from_string(""),
        i32::MAX,
    )?;
    if value_for_three {
        buffer.add_update_with_bytes_ref(
            Term::from_text("id".to_string(), "3"),
            &BytesRef::from_string(""),
            i32::MAX,
        )?;
    } else {
        buffer.add_no_value(Term::from_text("id".to_string(), "3"), i32::MAX)?;
    }

    buffer.add_update_with_bytes_ref(
        Term::from_text("id".to_string(), "4"),
        &BytesRef::from_string(""),
        i32::MAX,
    )?;
    buffer.finish()?;
    let mut iterator = buffer.iterator()?;
    let mut count = 0;
    while let Some(value) = iterator.next_value()? {
        let has_value = count != 3 || value_for_three;
        assert_eq!(
            count.to_string(),
            value.term_value.unwrap().utf8_to_string()?
        );
        assert_eq!("id", value.term_field);
        assert_eq!(has_value, value.has_value);

        if has_value {
            assert_eq!(BytesRef::from_string(""), value.binary_value.unwrap());
        } else {
            assert!(value.binary_value.is_none());
        }
        assert_eq!(i32::MAX, value.doc_up_to);
        count += 1;
    }
    Ok(())
}
pub fn random_from<T>(items: Vec<T>) -> T
where
    T: Clone,
{
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..items.len());
    items[index].clone()
}
pub fn get_random_binary_update(random: &mut StdRng, doc_id_up_to: i32) -> DocValuesUpdate {
    let term_field = random_from(vec!["id", "_id", "some_other_field"]);
    let doc_id = random.gen_range(0..10).to_string();

    let value = if rarely(random) {
        None
    } else {
        Some(BytesRef::from_string(
            &TestUtil::random_realistic_unicode_string(random),
        ))
    };

    let sub_update = DocValuesUpdateEnum::Binary(BinaryDocValuesUpdate::new(value));
    let mut update = DocValuesUpdate::new(
        DocValuesType::Binary,
        Term::from_text(term_field.to_string(), &doc_id),
        "enabled".to_string(),
        BufferedUpdates::MAX_INT,
        sub_update,
    );
    if rarely(random) {
        let result = update.prepare_for_apply(doc_id_up_to);
        result.unwrap_or(update)
    } else {
        update
    }
}
pub fn get_random_numeric_update(random: &mut StdRng, doc_id_up_to: i32) -> DocValuesUpdate {
    let term_field = random_from(vec!["id", "_id", "some_other_field"]);
    let doc_id = random.gen_range(0..10).to_string();

    let value = if rarely(random) {
        None
    } else {
        Some(random.gen_range(0..100) as i64)
    };

    let sub_update = DocValuesUpdateEnum::Numeric(NumericDocValuesUpdate::new(value));
    let mut update = DocValuesUpdate::new(
        DocValuesType::Numeric,
        Term::from_text(term_field.to_string(), &doc_id),
        "numeric".to_string(),
        BufferedUpdates::MAX_INT,
        sub_update,
    );

    if rarely(random) {
        let result = update.prepare_for_apply(doc_id_up_to);
        result.unwrap_or(update)
    } else {
        update
    }
}

#[test]
pub fn test_binary_random() -> Result<(), TestError> {
    let mut random = random();
    let mut updates = Vec::new();
    let num_updates = 1 + random.gen_range(0..1000);
    let counter = Arc::new(Mutex::new(CounterEnum::new_counter(false)));

    let mut random_update = get_random_binary_update(&mut random, 0);
    updates.push(random_update.clone());

    let doc_id_up_to = random_update.doc_id_up_to;
    let mut buffer =
        FieldUpdatesBuffer::from_binary_update(counter.clone(), random_update, doc_id_up_to)?;

    for i in 0..num_updates {
        random_update = get_random_binary_update(&mut random, i + 1);
        let doc_id_up_to = random_update.doc_id_up_to;
        updates.push(random_update.clone());

        if random_update.has_value {
            buffer.add_update_with_bytes_ref(
                random_update.term,
                &random_update.sub_update.get_binary().unwrap().get_value(),
                doc_id_up_to,
            )?;
        } else {
            buffer.add_no_value(random_update.term, doc_id_up_to)?;
        }
    }
    buffer.finish()?;

    let mut iterator = buffer.iterator()?;
    let mut count = 0;

    while let Some(value) = iterator.next_value()? {
        let random_update = &updates[count];
        count += 1;
        assert_eq!(
            random_update.term.bytes.utf8_to_string()?,
            value.term_value.unwrap().utf8_to_string()?
        );
        assert_eq!(random_update.term.field, value.term_field);
        assert_eq!(random_update.has_value, value.has_value, "count: {}", count);

        if random_update.has_value {
            assert_eq!(
                random_update.sub_update.get_binary().unwrap().get_value(),
                value.binary_value.unwrap()
            );
        } else {
            assert!(value.binary_value.is_none());
        }
        assert_eq!(random_update.doc_id_up_to, value.doc_up_to);
    }

    Ok(())
}
#[test]
pub fn test_numeric_random() -> Result<(), TestError> {
    let mut random = random();
    let mut updates = Vec::new();
    let num_updates = 1 + random.gen_range(0..1000);
    let counter = Arc::new(Mutex::new(CounterEnum::new_counter(false)));

    let mut random_update = get_random_numeric_update(&mut random, 0);
    updates.push(random_update.clone());

    let doc_id_up_to = random_update.doc_id_up_to;
    let mut buffer =
        FieldUpdatesBuffer::from_numeric_update(counter.clone(), random_update, doc_id_up_to)?;

    let mut last_update: Option<DocValuesUpdate> = None;
    for i in 0..num_updates {
        random_update = get_random_numeric_update(&mut random, i + 1);
        // last
        if i == num_updates - 1 {
            last_update = Some(random_update.clone());
        }
        let doc_id_up_to = random_update.doc_id_up_to;
        updates.push(random_update.clone());

        if random_update.has_value {
            buffer.add_update_with_long(
                random_update.term,
                random_update.sub_update.get_numeric().unwrap().get_value(),
                doc_id_up_to,
            )?;
        } else {
            buffer.add_no_value(random_update.term, doc_id_up_to)?;
        }
    }
    buffer.finish()?;
    assert!(last_update.is_some());
    let last_update = last_update.unwrap();
    let terms_sorted = last_update.has_value
        && updates.iter().all(|update| {
            update.field == last_update.field
                && update.has_value
                && update.sub_update.get_numeric().unwrap().get_value()
                    == last_update.sub_update.get_numeric().unwrap().get_value()
        });

    assert_buffer_updates(&buffer, &mut updates, terms_sorted)?;

    Ok(())
}
#[test]
pub fn test_no_numeric_value() -> Result<(), TestError> {
    let update = DocValuesUpdate::new(
        DocValuesType::Numeric,
        Term::from_text("id".to_string(), "1"),
        "age".to_string(),
        BufferedUpdates::MAX_INT,
        DocValuesUpdateEnum::Numeric(NumericDocValuesUpdate::new(None)),
    );

    let counter = Arc::new(Mutex::new(CounterEnum::new_counter(false)));
    let doc_id_up_to = update.doc_id_up_to;
    let buffer = FieldUpdatesBuffer::from_numeric_update(counter.clone(), update, doc_id_up_to)?;

    assert_eq!(buffer.get_min_numeric(), 0);
    assert_eq!(buffer.get_max_numeric(), 0);

    Ok(())
}
#[test]
pub fn test_sort_and_dedup_numeric_updates_by_terms() -> Result<(), TestError> {
    let mut random = random();
    let mut updates = Vec::new();
    let num_updates = 1 + random.gen_range(0..1000);
    let counter = Arc::new(Mutex::new(CounterEnum::new_counter(false)));

    let term_field = random_from(vec!["id", "_id", "some_other_field"]);
    let doc_value = 1 + random.gen_range(0..1000);

    let mut random_update = DocValuesUpdate::new(
        DocValuesType::Numeric,
        Term::from_text(
            term_field.to_string(),
            &random.gen_range(0..1000).to_string(),
        ),
        "numeric".to_string(),
        BufferedUpdates::MAX_INT,
        DocValuesUpdateEnum::Numeric(NumericDocValuesUpdate::new(Some(doc_value))),
    );
    if let Some(v) = random_update.prepare_for_apply(0) {
        random_update = v
    }
    updates.push(random_update.clone());
    let doc_id_up_to = random_update.doc_id_up_to;
    let mut buffer =
        FieldUpdatesBuffer::from_numeric_update(counter.clone(), random_update, doc_id_up_to)?;

    for i in 0..num_updates {
        random_update = DocValuesUpdate::new(
            DocValuesType::Numeric,
            Term::from_text(
                term_field.to_string(),
                &random.gen_range(0..1000).to_string(),
            ),
            "numeric".to_string(),
            BufferedUpdates::MAX_INT,
            DocValuesUpdateEnum::Numeric(NumericDocValuesUpdate::new(Some(doc_value))),
        );
        if let Some(v) = random_update.prepare_for_apply(i + 1) {
            random_update = v
        }
        updates.push(random_update.clone());
        buffer.add_update_with_long(random_update.term, doc_value, random_update.doc_id_up_to)?;
    }

    buffer.finish()?;

    // We can now assert that the buffer updates are correct after sorting and deduplication
    assert_buffer_updates(&buffer, &mut updates, true)?;

    Ok(())
}

fn assert_buffer_updates(
    buffer: &FieldUpdatesBuffer,
    updates: &mut [DocValuesUpdate],
    term_sorted: bool,
) -> Result<(), TestError> {
    let mut updates = updates.to_owned();
    if term_sorted {
        updates.sort_by(|a, b| a.term.bytes.cmp(&b.term.bytes));
        let mut by_terms: BTreeMap<BytesRef, DocValuesUpdate> = BTreeMap::new();

        for update in updates.iter() {
            by_terms
                .entry(update.term.bytes.clone())
                .or_insert_with(|| update.clone());
        }

        updates = by_terms.into_values().collect();
    }

    let mut iterator = buffer.iterator()?;
    let mut count = 0;
    let mut min = i64::MAX;
    let mut max = i64::MIN;
    let mut has_at_least_one_value = false;

    while let Some(value) = iterator.next_value()? {
        let v = buffer.get_numeric_value(count);
        let expected_update = &updates[count as usize];
        count += 1;
        assert_eq!(
            expected_update.term.bytes.utf8_to_string()?,
            value.term_value.unwrap().utf8_to_string()?
        );
        assert_eq!(expected_update.term.field, value.term_field);
        assert_eq!(expected_update.has_value, value.has_value);

        if expected_update.has_value {
            let expected_value = expected_update
                .sub_update
                .get_numeric()
                .unwrap()
                .get_value();
            assert_eq!(expected_value, value.numeric_value);
            min = min.min(expected_value);
            max = max.max(expected_value);
            has_at_least_one_value = true;
        } else {
            assert_eq!(0, value.numeric_value);
            assert_eq!(0, v)
        }
    }
    if has_at_least_one_value {
        assert_eq!(max, buffer.get_max_numeric());
        assert_eq!(min, buffer.get_min_numeric());
    } else {
        assert_eq!(0, buffer.get_min_numeric());
        assert_eq!(0, buffer.get_max_numeric());
    }
    assert_eq!(updates.len() as i32, count);
    Ok(())
}
