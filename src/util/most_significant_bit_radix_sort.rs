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
use crate::index::BytesRefBuilder;
use crate::util::error::runtime_error::RuntimeError;
use crate::util::intro_sorter::IntroSorter;
use crate::util::{check_range, Sorter};
use std::cmp::min;

/// After this many levels of recursion, we fall back to introsort.
/// This protects against poor performance when there are long common prefixes,
/// likely due to cache locality issues.
pub const LEVEL_THRESHOLD: usize = 8;
/// Size of histograms: 256 + 1 to indicate that the string is finished.
pub const HISTOGRAM_SIZE: usize = 257;
/// Buckets below this size will be sorted with the fallback sorter.
pub const LENGTH_THRESHOLD: usize = 100;
pub struct MSBRadixSorter<T>
where
    T: Sorter + MSBRadixSorterBase,
{
    /// One histogram per recursion level.
    histograms: Vec<Vec<i32>>,
    /// End offsets for histograms.
    end_offsets: Vec<i32>,
    /// Array to store common prefixes.
    common_prefix: Vec<i32>,
    /// Maximum length of strings to sort.
    max_length: i32,
    sub_sorter: T,
}
impl<T> MSBRadixSorter<T>
where
    T: Sorter + MSBRadixSorterBase,
{
    /// Sole constructor.
    ///
    /// # Parameters
    /// - `max_length`: The maximum length of keys. Pass `i32::MAX` if unknown.
    pub fn new(max_length: i32, sub_sorter: T) -> Self {
        let histograms: Vec<Vec<i32>> = (0..LEVEL_THRESHOLD).map(|_| Vec::new()).collect();
        Self {
            histograms,
            end_offsets: vec![0; HISTOGRAM_SIZE],
            max_length,
            common_prefix: vec![0; 24.min(max_length as usize)],
            sub_sorter,
        }
    }
    pub fn sort_impl(&mut self, from: i32, to: i32, k: i32, l: i32) -> Result<(), RuntimeError> {
        if self.should_fallback(from, to, l) {
            self.get_fallback_sorter(k).sort(from, to)
        } else {
            self.radix_sort(from, to, k, l)
        }
    }
    fn should_fallback(&self, from: i32, to: i32, l: i32) -> bool {
        (to - from) <= LENGTH_THRESHOLD as i32 || l >= LEVEL_THRESHOLD as i32
    }
    /// Computes the initial common prefix length for the given range.
    ///
    /// This method has been split to avoid platform-specific issues.
    ///
    fn compute_initial_common_prefix_length(&mut self, from: i32, k: i32) -> i32 {
        let common_prefix = &mut self.common_prefix;
        let mut common_prefix_length = min(common_prefix.len(), (self.max_length - k) as usize);

        for (j, slot) in common_prefix
            .iter_mut()
            .enumerate()
            .take(common_prefix_length)
        {
            let b = self.sub_sorter.byte_at(from, k + j as i32);
            *slot = b;
            if b == -1 {
                common_prefix_length = j + 1;
                break;
            }
        }
        common_prefix_length as i32
    }
    fn compute_common_prefix_length_and_build_histogram_part2(
        &mut self,
        from: i32,
        to: i32,
        k: i32,
        l: i32,
        common_prefix_length: i32,
        i: i32,
    ) -> i32 {
        if i < to {
            debug_assert!(common_prefix_length == 0);
            self.build_histogram(self.common_prefix[0] + 1, i - from, i, to, k, l);
        } else {
            debug_assert!(common_prefix_length > 0);
            self.histograms[l as usize][(self.common_prefix[0] + 1) as usize] = to - from;
        }

        common_prefix_length
    }
    /// Build a histogram of the k-th characters of values occurring between offsets `from` and `to`,
    /// using the `get_bucket` method.
    fn build_histogram(
        &mut self,
        prefix_common_bucket: i32,
        prefix_common_len: i32,
        from: i32,
        to: i32,
        k: i32,
        l: i32,
    ) {
        self.histograms[l as usize][prefix_common_bucket as usize] = prefix_common_len;

        for i in from..to {
            let b = self.get_bucket(i, k) as usize;
            self.histograms[l as usize][b] += 1;
        }
    }
    fn get_bucket(&self, i: i32, k: i32) -> i32 {
        self.sub_sorter.byte_at(i, k) + 1
    }
    fn compute_common_prefix_length_and_build_histogram_part1(
        &mut self,
        from: i32,
        to: i32,
        k: i32,
        l: i32,
        mut common_prefix_length: i32,
    ) -> i32 {
        let mut i = from + 1;

        'outer: for idx in from + 1..to {
            let mut j = 0;
            while j < common_prefix_length {
                let b = self.sub_sorter.byte_at(idx, k + j);
                if b != self.common_prefix[j as usize] {
                    common_prefix_length = j;
                    if common_prefix_length == 0 {
                        break 'outer;
                    }
                    break;
                }
                j += 1;
            }
            i = idx + 1;
        }

        self.compute_common_prefix_length_and_build_histogram_part2(
            from,
            to,
            k,
            l,
            common_prefix_length,
            i,
        )
    }
    pub fn compute_common_prefix_length_and_build_histogram(
        &mut self,
        from: i32,
        to: i32,
        k: i32,
        l: i32,
    ) -> i32 {
        let common_prefix_length = self.compute_initial_common_prefix_length(from, k);
        self.compute_common_prefix_length_and_build_histogram_part1(
            from,
            to,
            k,
            l,
            common_prefix_length,
        )
    }
    fn sum_histogram(histogram: &mut [i32], end_offsets: &mut [i32]) {
        let mut accum = 0;
        for (hist, end_offset) in histogram.iter_mut().zip(end_offsets.iter_mut()) {
            let count = *hist;
            *hist = accum;
            accum += count;
            *end_offset = accum;
        }
    }
    /// Reorder based on start/end offsets for each bucket. When this method returns, `start_offsets`
    /// and `end_offsets` are equal.
    ///
    /// # Parameters
    /// - `from`: The starting index (inclusive).
    /// - `to`: The ending index (exclusive).
    /// - `start_offsets`: Start offsets per bucket.
    /// - `end_offsets`: End offsets per bucket.
    /// - `k`: The current position offset.
    fn reorder(&mut self, from: i32, _to: i32, l: i32, k: i32) {
        // Reorder in place, similar to the Dutch national flag problem
        for i in 0..HISTOGRAM_SIZE {
            let limit = self.end_offsets[i];
            while self.histograms[l as usize][i] < limit {
                let h1 = self.histograms[l as usize][i];
                let b = self.get_bucket(from + h1, k);
                let h2 = self.histograms[l as usize][b as usize];
                self.histograms[l as usize][b as usize] += 1;
                self.swap(from + h1, from + h2);
            }
        }
    }
    /// Performs radix sort on the specified range and recursion level.
    ///
    /// # Parameters
    /// - `from`: Start index (inclusive).
    /// - `to`: End index (exclusive).
    /// - `k`: The character number to compare.
    /// - `l`: The level of recursion.
    fn radix_sort(&mut self, from: i32, to: i32, k: i32, l: i32) -> Result<(), RuntimeError> {
        // Access or initialize the histogram for this level
        if self.histograms[l as usize].is_empty() {
            self.histograms[l as usize] = vec![0; HISTOGRAM_SIZE];
        } else {
            self.histograms[l as usize].fill(0);
        }

        // Compute the common prefix length and build the histogram
        let common_prefix_length =
            self.compute_common_prefix_length_and_build_histogram(from, to, k, l);

        if common_prefix_length > 0 {
            // if there are no more chars to compare or if all entries fell into the
            // first bucket (which means strings are shorter than k) then we are done
            // otherwise recurse
            if k + common_prefix_length < self.max_length
                && self.histograms[l as usize][0] < (to - from)
            {
                self.radix_sort(from, to, k + common_prefix_length, l)?;
            }
            return Ok(());
        }

        // Assert histogram correctness (can be implemented as a debug check)
        debug_assert!(Self::assert_histogram(
            common_prefix_length,
            &self.histograms[l as usize]
        ));

        // Prepare start and end offsets
        Self::sum_histogram(&mut self.histograms[l as usize], &mut self.end_offsets);

        // Reorder the range
        self.reorder(from, to, l, k);

        // Update end offsets
        self.histograms[l as usize] = self.end_offsets.clone();

        // Recursively sort buckets if more levels are allowed
        if k + 1 < self.max_length {
            let mut prev = self.histograms[l as usize][0];
            for i in 1..HISTOGRAM_SIZE {
                let h = self.histograms[l as usize][i];
                let bucket_len = h - prev;
                if bucket_len > 1 {
                    self.sort_impl(from + prev, from + h, k + 1, l + 1)?;
                }
                prev = h;
            }
        }
        Ok(())
    }

    fn get_fallback_sorter(&mut self, k: i32) -> IntroSorterImpl<T> {
        IntroSorterImpl::new(self.max_length, k, &mut self.sub_sorter)
    }

    /// Always returns `true` if the assertions pass.
    #[cfg(debug_assertions)]
    fn assert_histogram(common_prefix_length: i32, histogram: &[i32]) -> bool {
        let number_of_unique_bytes = histogram.iter().filter(|&&freq| freq > 0).count();

        if number_of_unique_bytes == 1 {
            debug_assert!(common_prefix_length >= 1);
        } else {
            debug_assert!(
                common_prefix_length == 0,
                "Expected common_prefix_length to be 0, but found {}",
                common_prefix_length
            );
        }
        true
    }
    #[cfg(feature = "test_only")]
    pub fn get_sub_sorter(&self) -> &T {
        &self.sub_sorter
    }
}

impl<T> Sorter for MSBRadixSorter<T>
where
    T: Sorter + MSBRadixSorterBase,
{
    fn compare(&self, _i: i32, _j: i32) -> i32 {
        unreachable!("unused: not a comparison-based sort")
    }

    fn swap(&mut self, i: i32, j: i32) {
        self.sub_sorter.swap(i, j);
    }

    fn set_pivot(&mut self, i: i32) {
        self.sub_sorter.set_pivot(i);
    }

    fn compare_pivot(&self, i: i32) -> i32 {
        self.sub_sorter.compare_pivot(i)
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<(), RuntimeError> {
        check_range(from, to)?;
        self.sort_impl(from, to, 0, 0)
    }
}

struct IntroSorterImpl<'a, T>
where
    T: Sorter + MSBRadixSorterBase,
{
    pivot: BytesRefBuilder,
    max_length: i32,
    k: i32,
    sub_sorter: &'a mut T,
}
impl<'a, T> IntroSorterImpl<'a, T>
where
    T: Sorter + MSBRadixSorterBase,
{
    fn new(max_length: i32, k: i32, sub_sorter: &'a mut T) -> Self {
        Self {
            pivot: BytesRefBuilder::new(),
            max_length,
            k,
            sub_sorter,
        }
    }
}

impl<T> Sorter for IntroSorterImpl<'_, T>
where
    T: Sorter + MSBRadixSorterBase,
{
    fn compare(&self, i: i32, j: i32) -> i32 {
        for o in self.k..self.max_length {
            let b1 = self.sub_sorter.byte_at(i, o);
            let b2 = self.sub_sorter.byte_at(j, o);

            if b1 != b2 {
                return b1 - b2;
            } else if b1 == -1 {
                break;
            }
        }
        0
    }

    fn swap(&mut self, i: i32, j: i32) {
        self.sub_sorter.swap(i, j);
    }

    fn set_pivot(&mut self, i: i32) {
        self.pivot.set_length(0);

        for o in self.k..self.max_length {
            let b = self.sub_sorter.byte_at(i, o);
            if b == -1 {
                break;
            }
            self.pivot.append_byte(b as u8);
        }
    }

    fn compare_pivot(&self, j: i32) -> i32 {
        for o in 0..self.pivot.length() {
            let b1 = self.pivot.byte_at(o) as i32;
            let b2 = self.sub_sorter.byte_at(j, self.k + o as i32);
            if b1 != b2 {
                return b1 - b2;
            }
        }

        if self.k + self.pivot.length() as i32 == self.max_length {
            0
        } else {
            -1 - self
                .sub_sorter
                .byte_at(j, self.k + self.pivot.length() as i32)
        }
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<(), RuntimeError> {
        IntroSorter::sort_range(self, from, to)?;
        Ok(())
    }
}

impl<T> IntroSorter for IntroSorterImpl<'_, T> where T: Sorter + MSBRadixSorterBase {}

pub trait MSBRadixSorterBase {
    /// Returns the k-th byte of the entry at the given index `i`, or `-1` if its length is less than
    /// or equal to `k`.
    ///
    /// # Parameters
    /// - `i`: The index of the entry, which must be between `0` (inclusive) and `max_length` (exclusive).
    /// - `k`: The position of the byte to retrieve within the entry.
    ///
    /// # Returns
    /// The k-th byte of the entry at index `i` as an `i32`, or `-1` if the entry's length is less than or equal to `k`.
    ///
    /// # Note
    /// In Rust, this method might return a signed integer (`i32`) to accommodate the `-1` case, which differs from Java's default integer handling.
    fn byte_at(&self, i: i32, k: i32) -> i32;
}
