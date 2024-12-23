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
use rlucene::index::BytesRefBuilder;
use rlucene::util::bytes_ref_array::BytesRefArray;
use rlucene::util::bytes_ref_iterator::BytesRefIterator;
use rlucene::util::{new_counter, CounterEnum, SortableBytesRefArray};
use std::sync::{Arc, Mutex};

#[allow(dead_code)] // for quick search
struct TestBytesRefArray;
#[test]
fn test_append() -> Result<(), TestError> {
    let mut random = my_random("test_append".to_string());
    let counter = Arc::new(Mutex::new(new_counter(false)));
    let mut list = BytesRefArray::new(counter);
    let mut string_list = Vec::new();

    for j in 0..2 {
        if j > 0 && random.gen_bool(0.5) {
            list.clear();
            string_list.clear();
        }

        let entries = random.gen_range(500..10000);
        let mut spare = BytesRefBuilder::new();
        let init_size = list.size();

        for i in 0..entries {
            let random_realistic_unicode_string =
                TestUtil::random_realistic_unicode_string(&mut random);
            spare.copy_chars_with_string(&random_realistic_unicode_string);
            assert_eq!(i + init_size, list.append(spare.get()));
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
        for i in 0..entries {
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
