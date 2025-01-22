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
use crate::common::my_random;
use crate::util::test_error::TestError;
use crate::util::TestUtil;
use rand::Rng;
use rlucene::index::{BytesRef, BytesRefBuilder};
use rlucene::util::bytes_ref_array::BytesRefArray;
use rlucene::util::bytes_ref_iterator::BytesRefIterator;
use rlucene::util::{new_counter, Natural, NaturalOrder, SortableBytesRefArray};
use std::sync::{Arc, Mutex};

#[allow(dead_code)] // for quick search
struct TestBytesRefArray;
#[test]
fn test_append() -> Result<(), TestError> {
    let mut random = my_random("test_append".to_string());
    let counter = Arc::new(Mutex::new(new_counter(false)));
    let mut list = BytesRefArray::new(counter)?;
    let mut string_list = Vec::new();

    for j in 0..2 {
        if j > 0 && random.gen_bool(0.5) {
            list.clear()?;
            string_list.clear();
        }

        let entries = random.gen_range(500..10000);
        let mut spare = BytesRefBuilder::new();
        let init_size = list.size();

        for i in 0..entries {
            let random_realistic_unicode_string =
                TestUtil::random_realistic_unicode_string(&mut random);
            spare.copy_chars_with_string(&random_realistic_unicode_string);
            assert_eq!(i + init_size, list.append(spare.get())?);
            string_list.push(random_realistic_unicode_string);
        }

        for i in 0..entries {
            assert_eq!(
                string_list[i as usize],
                list.get(&mut spare, i).unwrap().utf8_to_string()?,
                "entry {} doesn't match",
                i
            );
        }

        // Check random access
        for _i in 0..entries {
            let e = random.gen_range(0..entries);
            assert_eq!(
                string_list[e as usize],
                list.get(&mut spare, e).unwrap().utf8_to_string()?,
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
                assert_eq!(*string, value.unwrap().utf8_to_string()?,);
            }
        }
    }
    Ok(())
}
#[test]
fn test_sort() -> Result<(), TestError> {
    let mut random = my_random("test_sort".to_string());
    let counter = Arc::new(Mutex::new(new_counter(false)));
    let mut list = BytesRefArray::new(counter)?;
    let mut string_list = Vec::new();

    for j in 0..5 {
        if j > 0 && random.gen_bool(0.5) {
            list.clear()?;
            string_list.clear();
        }

        let entries = random.gen_range(200..1000);
        let mut spare = BytesRefBuilder::new();
        let init_size = list.size();

        for i in 0..entries {
            let random_realistic_unicode_string =
                TestUtil::random_realistic_unicode_string(&mut random);
            spare.copy_chars_with_string(&random_realistic_unicode_string);
            assert_eq!(init_size + i, list.append(spare.get())?);
            string_list.push(random_realistic_unicode_string);
        }

        string_list.sort_by(|a, b| TestUtil::string_codepoint_comparator(a, b));
        {
            let mut iter1 = SortableBytesRefArray::iterator(&mut list, Natural::default())?;

            let mut i = 0;
            while let Some(next) = iter1.next()? {
                assert_eq!(
                    string_list[i],
                    next.utf8_to_string()?, // 转换为 UTF-8 字符串
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

        let mut iter2 = SortableBytesRefArray::iterator(&mut list, NaturalOrder::default())?;
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
fn test_stable_sort() -> Result<(), TestError> {
    let mut random = my_random("test_stable_sort".to_string());

    let counter = Arc::new(Mutex::new(new_counter(false)));
    let mut list = BytesRefArray::new(counter)?;

    let mut string_list = Vec::new();

    for j in 0..5 {
        if j > 0 && random.gen_bool(0.5) {
            list.clear()?;
            string_list.clear();
        }

        let entries = random.gen_range(200..1000);

        let mut values = Vec::new();
        for _ in 0..20 {
            values.push(TestUtil::random_realistic_unicode_string(&mut random));
        }

        let mut spare = BytesRefBuilder::new();
        let init_size = list.size();
        for i in 0..entries {
            let random_realistic_unicode_string = values[random.gen_range(0..values.len())].clone();
            spare.copy_chars_with_string(&random_realistic_unicode_string);
            assert_eq!(init_size + i, list.append(spare.get())?);
            string_list.push(random_realistic_unicode_string);
        }

        string_list.sort_by(|a, b| TestUtil::string_codepoint_comparator(a, b));

        let sort_state = if random.gen_bool(0.5) {
            list.sort(NaturalOrder::default(), true)?
        } else {
            list.sort(Natural::default(), true)?
        };
        let mut iter = list.iterator_with_state(Arc::new(sort_state));
        let mut i = 0;
        let mut last_ord = -1;
        let mut last: Option<BytesRef> = None;

        while let Some(next) = iter.next()? {
            assert_eq!(
                string_list[i],
                next.utf8_to_string()?,
                "entry {} doesn't match",
                i
            );

            if let Some(last_ref) = &last {
                if next == *last_ref {
                    let ord = iter.ord();
                    assert!(ord > last_ord, "sort not stable: {} <= {}", ord, last_ord);
                }
            }

            last = Some(BytesRef::deep_copy_of(&next));
            last_ord = iter.ord();
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
