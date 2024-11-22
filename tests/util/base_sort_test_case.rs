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
use crate::common::rarely;
use crate::util::base_sort_test_case::Strategy::{RandomLowCardinality, RANDOM};
use rand::prelude::StdRng;
use rand::Rng;
use rlucene::util::{Comparator, Sorter};
use std::cmp::Ordering;

trait BaseSortTestCase {
    fn new_sorter(&self) -> impl Sorter;
    fn get_stable(&self) -> bool;
    fn assert_sorted(&self, original: &Vec<Entry>, sorted: &Vec<Entry>) {
        assert_eq!(original.len(), sorted.len());
        let mut actually_sorted = original.clone();
        actually_sorted.sort();
        for i in 0..original.len() {
            assert_eq!(actually_sorted[i].value, sorted[i].value);
            if self.get_stable() {
                assert_eq!(actually_sorted[i].ord, sorted[i].ord);
            }
        }
    }
    fn test_impl(&self, random: &mut StdRng, arr: Vec<Entry>) {
        let o = random.gen_range(0..1000);
        let value = random.gen_range(0..3);
        let mut to_sort = vec![Entry::default(); o + arr.len() + value];
        to_sort[o..o + arr.len()].clone_from_slice(&arr[0..arr.len()]);
        let mut sorter = self.new_sorter();
        let result = sorter.sort(o as i32, (o + arr.len()) as i32);
        assert!(result.is_ok());
        self.assert_sorted(&arr, &to_sort[o..o + arr.len()].to_vec());
    }
    fn test(&self, random: &mut StdRng, strategy: Strategy, length: i32) {
        let mut arr = vec![Entry::default(); length as usize];
        let arr_length = arr.len();
        for i in 0..arr_length {
            strategy.set(&mut arr, i as i32, random);
        }
        self.test_impl(random, arr);
    }
    fn test_with_strategy(&self, random: &mut StdRng, strategy: Strategy) {
        let value = random.gen_range(0..20000);
        self.test(random, strategy, value);
    }
    fn test_empty(&self, random: &mut StdRng) {
        let arr = vec![];
        self.test_impl(random, arr);
    }

    fn test_one(&self, random: &mut StdRng) {
        self.test(random, RANDOM(), 1);
    }
    fn test_two(&self, random: &mut StdRng) {
        self.test(random, RandomLowCardinality(), 2);
    }
    fn test_random(&self, random: &mut StdRng) {
        self.test_with_strategy(random, RANDOM());
    }
    fn test_random_low_cardinality(&self, random: &mut StdRng) {
        self.test_with_strategy(random, RandomLowCardinality());
    }
    fn test_ascending(&self, random: &mut StdRng) {
        self.test_with_strategy(random, Strategy::ASCENDING());
    }
    fn test_ascending_sequences(&self, random: &mut StdRng) {
        self.test_with_strategy(random, Strategy::AscendingSequences());
    }
    fn test_descending(&self, random: &mut StdRng) {
        self.test_with_strategy(random, Strategy::DESCENDING());
    }
    fn test_strictly_descending(&self, random: &mut StdRng) {
        self.test_with_strategy(random, Strategy::StrictlyDescending());
    }
}

enum Strategy {
    RANDOM(),
    RandomLowCardinality(),
    RandomMediumCardinality(),
    ASCENDING(),
    DESCENDING(),
    StrictlyDescending(),
    AscendingSequences(),
    MostlyAscending(),
}
impl Strategy {
    fn set(&self, arr: &mut Vec<Entry>, i: i32, random: &mut StdRng) {
        match self {
            Strategy::RANDOM() => {
                arr[i as usize] = Entry::new(random.gen_range(0..=i32::MAX), i);
            }
            Strategy::RandomLowCardinality() => {
                arr[i as usize] = Entry::new(random.gen_range(0..6), i);
            }
            Strategy::RandomMediumCardinality() => {
                let length = arr.len() >> 1;
                arr[i as usize] = Entry::new(random.gen_range(0..length) as i32, i);
            }
            Strategy::ASCENDING() => {
                arr[i as usize] = if i == 0 {
                    Entry::new(random.gen_range(0..6), 0)
                } else {
                    Entry::new(arr[(i - 1) as usize].value + random.gen_range(0..6), i)
                }
            }
            Strategy::DESCENDING() => {
                arr[i as usize] = if i == 0 {
                    Entry::new(random.gen_range(0..6), 0)
                } else {
                    Entry::new(arr[(i - 1) as usize].value - random.gen_range(0..6), i)
                }
            }
            Strategy::StrictlyDescending() => {
                arr[i as usize] = if i == 0 {
                    Entry::new(random.gen_range(0..6), 0)
                } else {
                    Entry::new(arr[(i - 1) as usize].value - random.gen_range(1..5), i)
                }
            }
            Strategy::AscendingSequences() => {
                arr[i as usize] = if i == 0 {
                    Entry::new(random.gen_range(0..6), 0)
                } else {
                    let value = if rarely(random.gen()) {
                        random.gen_range(0..1000)
                    } else {
                        arr[(i - 1) as usize].value + random.gen_range(0..6)
                    };
                    Entry::new(value, i)
                }
            }
            Strategy::MostlyAscending() => {
                arr[i as usize] = if i == 0 {
                    Entry::new(random.gen_range(0..6), 0)
                } else {
                    Entry::new(arr[(i - 1) as usize].value + random.gen_range(-8..=10), i)
                }
            }
        }
    }
}

#[derive(Clone, Eq)]
struct Entry {
    value: i32,
    ord: i32,
}
impl Default for Entry {
    fn default() -> Entry {
        Entry { value: 0, ord: 0 }
    }
}
impl Entry {
    fn new(value: i32, ord: i32) -> Entry {
        Entry { value, ord }
    }
}
impl Comparator<Entry> for Entry {
    fn compare(&self, a: &Entry, b: &Entry) -> i32 {
        match a.value.cmp(&b.value) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }
}

impl PartialEq<Self> for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.ord == other.ord
    }
}

impl PartialOrd<Self> for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}
