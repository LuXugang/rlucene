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
use crate::util::error::lucene_error::Result;
use crate::util::intro_sorter::SINGLE_MEDIAN_THRESHOLD;
use crate::util::selector::Selector;
use rand::rngs::ThreadRng;
use rand::Rng;
/// Adaptive selection algorithm based on the introspective quick select algorithm.
/// The quick select algorithm uses an interpolation variant of Tukey's ninther median-of-medians for pivot,
/// and Bentley-McIlroy 3-way partitioning. For the introspective protection, it shuffles the sub-range
/// if the max recursive depth is exceeded.
///
/// This selection algorithm is fast on most data shapes, especially on nearly sorted data, or
/// when `k` is close to the boundaries. It runs in linear time on average.
///
/// # Internal
/// This method is intended for internal use in the library.
pub struct IntroSelector<T>
where
    T: IntroSelectorBase,
{
    random: Option<ThreadRng>,
    sub_selector: T,
}
impl<T> IntroSelector<T>
where
    T: IntroSelectorBase,
{
    pub fn new(sub_selector: T) -> IntroSelector<T> {
        IntroSelector {
            random: None,
            sub_selector,
        }
    }
    pub fn select(&mut self, mut from: i32, mut to: i32, k: i32, mut max_depth: i32) -> Result<()> {
        // This code is inspired from IntroSorter#sort, adapted to loop on a single partition.

        // For efficiency, we must enter the loop with at least 4 entries to be able to skip
        // some boundary tests during the 3-way partitioning.
        let mut size;

        // Ensure the loop enters with at least 4 entries to skip boundary checks.
        while {
            size = to - from;
            size > 3
        } {
            max_depth -= 1;
            if max_depth == -1 {
                // Max recursion depth exceeded: shuffle (only once) and continue.
                self.shuffle(from, to)?;
            }

            // Pivot selection based on medians.
            let last = to - 1;
            let mid = (from + last) >> 1;
            let pivot;

            if size <= SINGLE_MEDIAN_THRESHOLD {
                // Select the pivot with a single median around the middle element.
                // Do not take the median between [from, mid, last] because it hurts performance
                // if the order is descending in conjunction with the 3-way partitioning.
                let range = size >> 2;
                pivot = self.median(mid - range, mid, mid + range);
            } else {
                // Select the pivot with a variant of the Tukey's ninther median of medians.
                // If k is close to the boundaries, select either the lowest or highest median (this variant
                // is inspired from the interpolation search).
                let range = size >> 3;
                let double_range = range << 1;
                let median_first = self.median(from, from + range, from + double_range);
                let median_middle = self.median(mid - range, mid, mid + range);
                let median_last = self.median(last - double_range, last - range, last);
                if k - from < range {
                    // k is close to 'from': select the lowest median.
                    pivot = self.min(median_first, median_middle, median_last);
                } else if to - k <= range {
                    pivot = self.max(median_first, median_middle, median_last);
                } else {
                    pivot = self.median(median_first, median_middle, median_last);
                }
            }
            // Bentley-McIlroy 3-way partitioning
            self.sub_selector.set_pivot(pivot);
            self.sub_selector.swap(from, pivot)?;

            let mut i = from;
            let mut j = to;
            let mut p = from + 1;
            let mut q = last;

            loop {
                let mut left_cmp;
                let mut right_cmp;

                while {
                    left_cmp = self.sub_selector.compare_pivot(i + 1);
                    i += 1;
                    left_cmp > 0
                } {}

                while {
                    right_cmp = self.sub_selector.compare_pivot(j - 1);
                    j -= 1;
                    right_cmp < 0
                } {}

                if i >= j {
                    if i == j && right_cmp == 0 {
                        self.sub_selector.swap(i, p)?;
                    }
                    break;
                }

                self.sub_selector.swap(i, j)?;
                if right_cmp == 0 {
                    self.sub_selector.swap(i, p)?;
                    p += 1;
                }

                if left_cmp == 0 {
                    self.sub_selector.swap(j, q)?;
                    q -= 1;
                }
            }
            i = j + 1;
            for l in from..p {
                self.sub_selector.swap(l, j)?;
                j -= 1;
            }
            for l in last..q {
                self.sub_selector.swap(l, i)?;
                i += 1;
            }

            if k <= j {
                to = j + 1;
            } else if k >= i {
                from = i;
            } else {
                return Ok(());
            }
        }
        // Sort the final tiny range (3 entries or less) with a very specialized sort.
        match size {
            2 => {
                if IntroSelectorBase::compare(&mut self.sub_selector, from, from + 1) > 0 {
                    self.sub_selector.swap(from, from + 1)?;
                }
            }
            3 => {
                self.sort3(from)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Returns the index of the min element among three elements at provided indices.
    pub fn min(&mut self, i: i32, j: i32, k: i32) -> i32 {
        if IntroSelectorBase::compare(&mut self.sub_selector, i, j) <= 0 {
            if IntroSelectorBase::compare(&mut self.sub_selector, i, k) <= 0 {
                i
            } else {
                k
            }
        } else if IntroSelectorBase::compare(&mut self.sub_selector, j, k) <= 0 {
            j
        } else {
            k
        }
    }

    /// Returns the index of the max element among three elements at provided indices.
    pub fn max(&mut self, i: i32, j: i32, k: i32) -> i32 {
        if IntroSelectorBase::compare(&mut self.sub_selector, i, j) <= 0 {
            if IntroSelectorBase::compare(&mut self.sub_selector, j, k) < 0 {
                k
            } else {
                j
            }
        } else if IntroSelectorBase::compare(&mut self.sub_selector, i, k) < 0 {
            k
        } else {
            i
        }
    }

    pub fn median(&mut self, i: i32, j: i32, k: i32) -> i32 {
        if IntroSelectorBase::compare(&mut self.sub_selector, i, j) < 0 {
            if IntroSelectorBase::compare(&mut self.sub_selector, j, k) <= 0 {
                return j;
            }
            return if IntroSelectorBase::compare(&mut self.sub_selector, i, k) < 0 {
                k
            } else {
                i
            };
        }
        if IntroSelectorBase::compare(&mut self.sub_selector, j, k) >= 0 {
            return j;
        }
        if IntroSelectorBase::compare(&mut self.sub_selector, i, k) < 0 {
            i
        } else {
            k
        }
    }
    /// Sorts 3 entries starting at from (inclusive). This specialized method is more efficient than
    /// calling `insertionSort(int, int)`.
    pub fn sort3(&mut self, from: i32) -> Result<()> {
        let mid = from + 1;
        let last = from + 2;

        if IntroSelectorBase::compare(&mut self.sub_selector, from, mid) <= 0 {
            if IntroSelectorBase::compare(&mut self.sub_selector, mid, last) > 0 {
                self.sub_selector.swap(mid, last)?;
                if IntroSelectorBase::compare(&mut self.sub_selector, from, mid) > 0 {
                    self.sub_selector.swap(from, mid)?;
                }
            }
        } else if IntroSelectorBase::compare(&mut self.sub_selector, mid, last) >= 0 {
            self.sub_selector.swap(from, last)?;
        } else {
            self.sub_selector.swap(from, mid)?;
            if IntroSelectorBase::compare(&mut self.sub_selector, mid, last) > 0 {
                self.sub_selector.swap(mid, last)?;
            }
        }
        Ok(())
    }
    /// Shuffles the entries between from (inclusive) and to (exclusive) with Durstenfeld's algorithm.
    pub fn shuffle(&mut self, from: i32, to: i32) -> Result<()> {
        if self.random.is_none() {
            self.random = Some(rand::rng());
        }

        let random = self.random.as_mut().unwrap();
        for i in (from..to).rev() {
            let j = random.random_range(from..=i);
            self.sub_selector.swap(i, j)?;
        }
        Ok(())
    }
}

impl<T> Selector for IntroSelector<T>
where
    T: IntroSelectorBase,
{
    fn select(&mut self, from: i32, to: i32, k: i32) -> Result<()> {
        self.check_args(from, to, k)?;
        let max_depth = 2 * (f64::log2((to - from) as f64) as i32);
        self.select(from, to, k, max_depth)?;
        Ok(())
    }
}

pub trait IntroSelectorBase: IntroSelectorBaseDefault + Selector {
    /// Compare entries found in slots `i` and `j`.
    fn compare(&mut self, i: i32, j: i32) -> i32 {
        IntroSelectorBaseDefault::compare(self, i, j)
    }
}
pub trait IntroSelectorBaseDefault {
    /// Save the value at slot `i` so that it can later be used as a pivot.
    fn set_pivot(&mut self, i: i32);
    /// Compare the pivot with the slot at `j`, similarly to `compare(i, j)`.
    fn compare_pivot(&mut self, j: i32) -> i32;
    fn compare(&mut self, i: i32, j: i32) -> i32 {
        self.set_pivot(i);
        self.compare_pivot(j)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::util::lucene_test_case::random;
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::Result;
    use crate::util::selector::Selector;
    use crate::util::{IntroSelector, IntroSelectorBase, IntroSelectorBaseDefault, ToInt};
    use rand::rngs::StdRng;
    use rand::Rng;

    #[allow(dead_code)] // for quick search
    pub struct TestIntroSelector;

    #[test]
    pub fn test_select() -> Result<()> {
        let mut random = random();
        for _ in 0..100 {
            do_test_select(&mut random)?;
        }
        Ok(())
    }

    pub fn do_test_select(random: &mut StdRng) -> Result<()> {
        let from: i32 = random.random_range(0..5);
        let to: i32 = from + TestUtil::next_int(random, 1, 10000);
        let max: i32 = if random.random_bool(0.5) {
            random.random_range(0..100)
        } else {
            random.random_range(0..100000)
        };

        let arr: Vec<i32> = if max == 0 {
            vec![0; to as usize + random.random_range(0..5)]
        } else {
            (0..(to + random.random_range(0..5)))
                .map(|_| TestUtil::next_int(random, 0, max))
                .collect()
        };

        let k = TestUtil::next_int(random, from, to - 1);
        let mut expected = arr.clone();
        let mut actual = arr.clone();
        expected[from as usize..to as usize].sort();
        let sub_selector = IntroSelectorMock::new(&mut actual);
        let mut selector = IntroSelector::new(sub_selector);
        if random.random_bool(0.5) {
            Selector::select(&mut selector, from, to, k)?;
        } else {
            IntroSelector::select(&mut selector, from, to, k, random.random_range(0..3))?;
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

    pub struct IntroSelectorMock<'a> {
        pivot: i32,
        actual: &'a mut Vec<i32>,
    }
    impl<'a> IntroSelectorMock<'a> {
        fn new(actual: &'a mut Vec<i32>) -> IntroSelectorMock<'a> {
            IntroSelectorMock { pivot: 0, actual }
        }
    }
    impl Selector for IntroSelectorMock<'_> {
        fn swap(&mut self, i: i32, j: i32) -> Result<()> {
            self.actual.swap(i as usize, j as usize);
            Ok(())
        }
    }

    impl IntroSelectorBaseDefault for IntroSelectorMock<'_> {
        fn set_pivot(&mut self, i: i32) {
            self.pivot = self.actual[i as usize];
        }

        fn compare_pivot(&mut self, j: i32) -> i32 {
            self.pivot.cmp(&self.actual[j as usize]).to_int()
        }
    }

    impl IntroSelectorBase for IntroSelectorMock<'_> {}
}
