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
use rand::rngs::StdRng;
use rand::Rng;
use rlucene::util::error::lucene_error::LuceneError;
use rlucene::util::selector::Selector;
use rlucene::util::{IntroSelector, IntroSelectorBase, IntroSelectorBaseDefault};

#[allow(dead_code)] // for quick search
pub struct TestIntroSelector;

#[test]
pub fn test_select() -> Result<(), LuceneError> {
    let mut random = my_random("test_select".to_string());
    for _ in 0..100 {
        do_test_select(&mut random)?;
    }
    Ok(())
}

pub fn do_test_select(random: &mut StdRng) -> Result<(), LuceneError> {
    let from: i32 = random.gen_range(0..5);
    let to: i32 = from + random.gen_range(1..=10000);
    let max: i32 = if random.gen_bool(0.5) {
        random.gen_range(0..100)
    } else {
        random.gen_range(0..100000)
    };

    let arr: Vec<i32> = if max == 0 {
        vec![0; to as usize + random.gen_range(0..5)]
    } else {
        (0..(to + random.gen_range(0..5)))
            .map(|_| random.gen_range(0..max))
            .collect()
    };

    let k = random.gen_range(from..=to - 1);
    let mut expected = arr.clone();
    let mut actual = arr.clone();
    expected[from as usize..to as usize].sort();
    let sub_selector = IntroSelectorImpl::new(&mut actual);
    let mut selector = IntroSelector::new(sub_selector);
    if random.gen_bool(0.5) {
        Selector::select(&mut selector, from, to, k)?;
    } else {
        IntroSelector::select(&mut selector, from, to, k, random.gen_range(0..3))?;
    }
    assert_eq!(expected[k as usize], actual[k as usize]);
    for i in 0..actual.len() {
        if i < from as usize || i >= to as usize {
            assert_eq!(arr[i], actual[i]);
        } else if i <= k as usize {
            assert!(actual[i] <= actual[k as usize]);
        } else {
            assert!(actual[i] >= actual[k as usize]);
        }
    }
    Ok(())
}

pub struct IntroSelectorImpl<'a> {
    pivot: i32,
    actual: &'a mut Vec<i32>,
}
impl<'a> IntroSelectorImpl<'a> {
    fn new(actual: &'a mut Vec<i32>) -> IntroSelectorImpl<'a> {
        IntroSelectorImpl { pivot: 0, actual }
    }
}
impl Selector for IntroSelectorImpl<'_> {
    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        self.actual.swap(i as usize, j as usize);
        Ok(())
    }
}

impl IntroSelectorBaseDefault for IntroSelectorImpl<'_> {
    fn set_pivot(&mut self, i: i32) {
        self.pivot = self.actual[i as usize];
    }

    fn compare_pivot(&self, j: i32) -> i32 {
        let result = self.pivot.cmp(&self.actual[j as usize]);
        match result {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }
}

impl IntroSelectorBase for IntroSelectorImpl<'_> {}
