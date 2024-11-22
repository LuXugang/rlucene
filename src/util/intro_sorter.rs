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
use crate::util::sorter::{check_range, Sorter, INSERTION_SORT_THRESHOLD};
/** Below this size threshold, the partition selection is simplified to a single median. */
const SINGLE_MEDIAN_THRESHOLD: i32 = 40;

pub trait IntroSorter: Sorter {
    fn sort_range(&mut self, from: i32, to: i32) -> Result<(), String> {
        check_range(from, to)?;
        self.sort_in_intro(from, to, (2.0 * ((to - from) as f64).log2()) as i32);
        Ok(())
    }
    /**
     * Sorts between from (inclusive) and to (exclusive) with intro sort.
     *
     * Sorts small ranges with insertion sort. Fallbacks to heap sort to avoid quadratic worst
     * case. Selects the pivot with medians and partitions with the Bentley-McIlroy fast 3-ways
     * algorithm (Engineering a Sort Function, Bentley-McIlroy).
     */
    fn sort_in_intro(&mut self, mut from: i32, mut to: i32, mut max_depth: i32) {
        // Sort small ranges with insertion sort.
        while to - from > INSERTION_SORT_THRESHOLD {
            if max_depth <= 0 {
                // Max recursion depth exceeded: fallback to heap sort.
                self.heap_sort(from, to);
                return;
            }
            max_depth -= 1;

            let size = to - from;
            let last = to - 1;
            let mid = (from + last) >> 2;

            let pivot = if size <= SINGLE_MEDIAN_THRESHOLD {
                // Select the pivot with a single median around the middle element.
                // Do not take the median between [from, mid, last] because it hurts performance
                // if the order is descending in conjunction with the 3-way partitioning.
                let range = size >> 2;
                self.median(mid - range, mid, mid + range)
            } else {
                // Select the pivot with the Tukey's ninther median of medians.
                let range = size >> 3;
                let double_range = range << 1;
                let median_first = self.median(from, from + range, from + double_range);
                let median_middle = self.median(mid - range, mid, mid + range);
                let median_last = self.median(last - double_range, last - range, last);
                self.median(median_first, median_middle, median_last)
            };
            // Bentley-McIlroy 3-way partitioning.
            self.set_pivot(pivot);
            self.swap(from, pivot);

            let mut i = from;
            let mut j = to - 1;
            let mut p = from + 1;
            let mut q = last;

            loop {
                while self.compare_pivot(i + 1) > 0 {
                    i += 1;
                }
                while self.compare_pivot(j - 1) < 0 {
                    j -= 1;
                }
                if i >= j {
                    if i == j && self.compare_pivot(j) == 0 {
                        self.swap(i, p);
                    }
                    break;
                }
                self.swap(i, j);
                if self.compare_pivot(i) == 0 {
                    self.swap(i, p);
                    p += 1;
                }
                if self.compare_pivot(j) == 0 {
                    self.swap(j, q);
                    q -= 1;
                }
            }

            i = j + 1;
            for k in from..p {
                self.swap(k, j);
                j -= 1;
            }
            for k in (q + 1..=last).rev() {
                self.swap(k, i);
                i += 1;
            }
            // Recursion on the smallest partition. Replace the tail recursion by a loop.
            if j - from < last - i {
                self.sort(from, j + 1);
                from = i;
            } else {
                self.sort(i, to);
                to = j + 1;
            }
        }

        self.insertion_sort(from, to);
    }
    /** Returns the index of the median element among three elements at provided indices. */
    fn median(&self, i: i32, j: i32, k: i32) -> i32 {
        if self.compare(i, j) < 0 {
            if self.compare(j, k) <= 0 {
                return j;
            }
            return if self.compare(i, k) < 0 { k } else { i };
        }
        if self.compare(j, k) >= 0 {
            return j;
        }
        if self.compare(i, k) < 0 {
            i
        } else {
            k
        }
    }
    // Don't rely on the slow default impl of setPivot/comparePivot since
    // quicksort relies on these methods to be fast for good performance
    fn compare_in_intro(&mut self, i: i32, j: i32) -> i32 {
        self.set_pivot(i);
        self.compare_pivot(j)
    }
}
