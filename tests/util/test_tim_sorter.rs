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
use crate::util::base_sort_test_case::{BaseSortTestCase, Entry};
use crate::util::lucene_test_case::random;
use rand::rngs::StdRng;
use rand::Rng;
use rlucene::util::{ArrayTimSorter, Comparator, NaturalOrder, Sorter, TimSorter};

struct TestTimSorter<T, C> {
    _marker: std::marker::PhantomData<(T, C)>,
}

impl TestTimSorter<Entry, NaturalOrder<Entry>> {
    fn default() -> Self {
        TestTimSorter {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: Default + Clone, C: Comparator<T>> BaseSortTestCase for TestTimSorter<T, C> {
    fn new_sorter(&self, random: &mut StdRng, arr: &mut Vec<Entry>) -> impl Sorter {
        let arr_len = arr.len();
        let max_temp_slots = random.gen_range(0..=arr_len);
        let array_tim_sorter = ArrayTimSorter::new(arr, NaturalOrder::new(), arr_len as i32);
        TimSorter::new(max_temp_slots as i32, array_tim_sorter)
    }

    fn get_stable(&self) -> bool {
        true
    }
}

#[test]
fn test_empty() {
    let mut random = random();
    let case = TestTimSorter::default();
    case.test_empty(&mut random);
}
#[test]
fn test_one() {
    let mut random = random();
    let case = TestTimSorter::default();
    case.test_one(&mut random);
}
#[test]
fn test_two() {
    let mut random = random();
    let case = TestTimSorter::default();
    case.test_two(&mut random);
}
#[test]
fn test_random() {
    let mut random = random();
    let case = TestTimSorter::default();
    case.test_random(&mut random);
}
#[test]
fn test_random_low_cardinality() {
    let mut random = random();
    let case = TestTimSorter::default();
    case.test_random_low_cardinality(&mut random);
}
#[test]
fn test_ascending() {
    let mut random = random();
    let case = TestTimSorter::default();
    case.test_ascending(&mut random);
}
#[test]
fn test_ascending_sequences() {
    let mut random = random();
    let case = TestTimSorter::default();
    case.test_ascending_sequences(&mut random);
}
#[test]
fn test_descending() {
    let mut random = random();
    let case = TestTimSorter::default();
    case.test_descending(&mut random);
}
#[test]
fn test_strictly_descending() {
    let mut random = random();
    let case = TestTimSorter::default();
    case.test_strictly_descending(&mut random);
}
