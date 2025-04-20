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
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::selector::Selector;
use crate::util::{IntroSelector, IntroSelectorBase, IntroSelectorBaseDefault};

pub struct RadixSelector<T>
where
    T: RadixSelectorBase,
{
    max_length: i32,
    common_prefix: Vec<i32>,
    histogram: Vec<i32>,
    pub(crate) sub_selector: T,
}

impl<T> RadixSelector<T>
where
    T: RadixSelectorBase,
{
    // after that many levels of recursion we fall back to introselect anyway
    // this is used as a protection against the fact that radix sort performs
    // worse when there are long common prefixes (probably because of cache
    // locality)
    const LEVEL_THRESHOLD: i32 = 8;
    // size of histograms: 256 + 1 to indicate that the string is finished
    const HISTOGRAM_SIZE: usize = 257;
    // buckets below this size will be sorted with introselect
    const LENGTH_THRESHOLD: i32 = 100;

    pub fn new(max_length: i32, sub_selector: T) -> Self {
        RadixSelector {
            max_length,
            common_prefix: vec![0; std::cmp::max(24, max_length) as usize],
            histogram: vec![0; Self::HISTOGRAM_SIZE],
            sub_selector,
        }
    }

    fn select(&mut self, from: i32, to: i32, k: i32, d: i32, l: i32) -> Result<()> {
        if to - from <= Self::LENGTH_THRESHOLD || l > Self::LEVEL_THRESHOLD {
            self.sub_selector
                .get_fallback_selector(d, self.max_length)
                .select(from, to, k)?;
        } else {
            self.radix_select(from, to, k, d, l)?;
        }
        Ok(())
    }

    /// `d` the character number to compare
    ///
    /// `l` the level of recursion
    pub fn radix_select(&mut self, from: i32, to: i32, k: i32, d: i32, l: i32) -> Result<()> {
        self.histogram.fill(0);

        let common_prefix_length =
            self.compute_common_prefix_length_and_build_histogram(from, to, d);
        if common_prefix_length > 0 {
            // if there are no more chars to compare or if all entries fell into the
            // first bucket (which means strings are shorter than d) then we are done
            // otherwise recurse
            if d + common_prefix_length < self.max_length && self.histogram[0] < to - from {
                self.radix_select(from, to, k, d + common_prefix_length, l)?;
            }
            return Ok(());
        }
        debug_assert!(self.assert_histogram(common_prefix_length, &self.histogram));

        let mut bucket_from = from;
        for bucket in 0..Self::HISTOGRAM_SIZE as i32 {
            let bucket_to = bucket_from + self.histogram[bucket as usize];
            if bucket_to > k {
                self.partition(from, to, bucket, bucket_from, bucket_to, d)?;
                if bucket != 0 && d + 1 < self.max_length {
                    // all elements in bucket 0 are equal so we only need to recurse if bucket != 0
                    self.select(bucket_from, bucket_to, k, d + 1, l + 1)?;
                }
                return Ok(());
            }
            bucket_from = bucket_to;
        }
        Err(LuceneError::unreachable("Unreachable code"))
    }

    // only used from assert
    fn assert_histogram(&self, common_prefix_length: i32, histogram: &[i32]) -> bool {
        let mut number_of_unique_bytes = 0;
        for &freq in histogram.iter() {
            if freq > 0 {
                number_of_unique_bytes += 1;
            }
        }
        if number_of_unique_bytes == 1 {
            debug_assert!(common_prefix_length >= 1);
        } else {
            debug_assert!(common_prefix_length == 0);
        }
        true
    }

    /** Return a number for the k-th character between 0 and {@link #HISTOGRAM_SIZE}. */
    fn get_bucket(&self, i: i32, k: i32) -> i32 {
        self.sub_selector.byte_at(i, k) + 1
    }

    /// Build a histogram of the number of values per `get_bucket(int, int)` and return a
    /// common prefix length for all visited values.
    fn compute_common_prefix_length_and_build_histogram(
        &mut self,
        from: i32,
        to: i32,
        k: i32,
    ) -> i32 {
        let common_prefix_length = self.compute_initial_common_prefix_length(from, k);
        self.compute_common_prefix_length_and_build_histogram_part1(
            from,
            to,
            k,
            common_prefix_length,
        )
    }

    fn compute_initial_common_prefix_length(&mut self, from: i32, k: i32) -> i32 {
        let common_prefix = &mut self.common_prefix;
        let mut common_prefix_length =
            std::cmp::min(common_prefix.len() as i32, self.max_length - k);
        for j in 0..common_prefix_length {
            let b = self.sub_selector.byte_at(from, k + j);
            common_prefix[j as usize] = b;
            if b == -1 {
                common_prefix_length = j + 1;
                break;
            }
        }
        common_prefix_length
    }

    #[allow(clippy::mut_range_bound)]
    fn compute_common_prefix_length_and_build_histogram_part1(
        &mut self,
        from: i32,
        to: i32,
        k: i32,
        mut common_prefix_length: i32,
    ) -> i32 {
        let common_prefix = &mut self.common_prefix;
        let mut i = from + 1;
        'outer: for current in (from + 1)..=to {
            i = current;
            if i == to {
                break;
            }
            for j in 0..common_prefix_length {
                let b = self.sub_selector.byte_at(current, k + j);
                if b != common_prefix[j as usize] {
                    common_prefix_length = j;
                    if common_prefix_length == 0 {
                        self.histogram[(common_prefix[0] + 1) as usize] = i - from;
                        self.histogram[(b + 1) as usize] = 1;
                        break 'outer;
                    }
                    break;
                }
            }
        }
        self.compute_common_prefix_length_and_build_histogram_part2(
            from,
            to,
            k,
            common_prefix_length,
            i,
        )
    }

    fn compute_common_prefix_length_and_build_histogram_part2(
        &mut self,
        from: i32,
        to: i32,
        k: i32,
        common_prefix_length: i32,
        i: i32,
    ) -> i32 {
        if i < to {
            // the loop got broken because there is no common prefix
            debug_assert!(common_prefix_length == 0);
            self.build_histogram(i + 1, to, k);
        } else {
            debug_assert!(common_prefix_length > 0);
            self.histogram[(self.common_prefix[0] + 1) as usize] = to - from;
        }
        common_prefix_length
    }

    /// Build an histogram of the k-th characters of values occurring between offsets `from` and
    /// `to`, using `get_bucket`.
    fn build_histogram(&mut self, from: i32, to: i32, k: i32) {
        for i in from..to {
            let index = self.get_bucket(i, k) as usize;
            self.histogram[index] += 1;
        }
    }

    /// Reorder elements so that all of them that fall into `bucket` are
    /// between offsets `bucketFrom` and `bucketTo`.
    fn partition(
        &mut self,
        from: i32,
        to: i32,
        bucket: i32,
        bucket_from: i32,
        bucket_to: i32,
        d: i32,
    ) -> Result<()> {
        let mut left = from;
        let mut right = to - 1;
        let mut slot = bucket_from;
        loop {
            let mut left_bucket = self.get_bucket(left, d);
            let mut right_bucket = self.get_bucket(right, d);
            while left_bucket <= bucket && left < bucket_from {
                if left_bucket == bucket {
                    self.swap(left, slot)?;
                    slot += 1;
                } else {
                    left += 1;
                }
                left_bucket = self.get_bucket(left, d);
            }
            while right_bucket >= bucket && right >= bucket_to {
                if right_bucket == bucket {
                    self.swap(right, slot)?;
                    slot += 1;
                } else {
                    right -= 1;
                }
                right_bucket = self.get_bucket(right, d);
            }
            if left < bucket_from && right >= bucket_to {
                self.swap(left, right)?;
                left += 1;
                right -= 1;
            } else {
                debug_assert!(left == bucket_from);
                debug_assert!(right == bucket_to - 1);
                break;
            }
        }
        Ok(())
    }
}

impl<T> Selector for RadixSelector<T>
where
    T: RadixSelectorBase,
{
    fn select(&mut self, from: i32, to: i32, k: i32) -> Result<()> {
        self.check_args(from, to, k)?;
        self.select(from, to, k, 0, 0)
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        self.sub_selector.swap(i, j)
    }
}

pub trait RadixSelectorBase: Selector {
    /// Return the k-th byte of the entry at index `i`, or `-1\ if its length is less than
    /// or equal to `k`. This may only be called with a value of `k` between `0`
    /// included and `maxLength` excluded.
    fn byte_at(&self, i: i32, k: i32) -> i32;
    /// Get a fall-back selector which may assume that the first `d` bytes of all compared
    /// strings are equal. This fallback selector is used when the range becomes narrow or when the
    /// maximum level of recursion has been exceeded.
    fn get_fallback_selector(&mut self, d: i32, max_length: i32) -> impl Selector
    where
        Self: Sized,
    {
        let delegate_sorter = IntroSelectorImpl {
            d,
            max_length,
            pivot: BytesRefBuilder::new(),
            delegate_sorter: self,
        };
        IntroSelector::new(delegate_sorter)
    }
}

pub struct IntroSelectorImpl<'a, T>
where
    T: RadixSelectorBase,
{
    d: i32,
    max_length: i32,
    pivot: BytesRefBuilder<Vec<u8>>,
    delegate_sorter: &'a mut T,
}
impl<T> IntroSelectorBaseDefault for IntroSelectorImpl<'_, T>
where
    T: RadixSelectorBase,
{
    fn set_pivot(&mut self, i: i32) {
        self.pivot.set_length(0);
        for o in self.d..self.max_length {
            let b = self.delegate_sorter.byte_at(i, o);
            if b == -1 {
                break;
            }
            self.pivot.append_byte(b as u8);
        }
    }

    fn compare_pivot(&mut self, j: i32) -> i32 {
        for o in 0..self.pivot.length() {
            let b1 = self.pivot.byte_at(o) as i32;
            let b2 = self.delegate_sorter.byte_at(j, self.d + o as i32);
            if b1 != b2 {
                return b1 - b2;
            }
        }
        if self.d + self.pivot.length() as i32 == self.max_length {
            0
        } else {
            -1 - self
                .delegate_sorter
                .byte_at(j, self.d + self.pivot.length() as i32)
        }
    }
}

impl<T> IntroSelectorBase for IntroSelectorImpl<'_, T>
where
    T: RadixSelectorBase,
{
    fn compare(&mut self, i: i32, j: i32) -> i32 {
        for o in self.d..self.max_length {
            let b1 = self.delegate_sorter.byte_at(i, o);
            let b2 = self.delegate_sorter.byte_at(j, o);
            if b1 != b2 {
                return b1 - b2;
            } else if b1 == -1 {
                break;
            }
        }
        0
    }
}
impl<T> Selector for IntroSelectorImpl<'_, T>
where
    T: RadixSelectorBase,
{
    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        self.delegate_sorter.swap(i, j)
    }
}

#[cfg(test)]
mod tests {
    use crate::index::BytesRef;
    use crate::test::util::lucene_test_case::random;
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::Result;
    use crate::util::radix_selector::{RadixSelector, RadixSelectorBase};
    use crate::util::selector::Selector;
    use rand::rngs::StdRng;
    use rand::{Rng, RngCore};
    use std::cmp::{min, Ordering};

    #[allow(dead_code)] // for quick search
    struct TestRadixSelector;
    #[test]
    pub fn test_select() -> Result<()> {
        let mut random = random();
        for _ in 0..100 {
            do_test_select(&mut random)?;
        }
        Ok(())
    }

    fn do_test_select(random: &mut StdRng) -> Result<()> {
        let from = random.random_range(0..5);
        let to = from + TestUtil::next_int(random, 1, 10000);
        let max_len = TestUtil::next_int(random, 1, 12);
        let arr_len = (from + to + random.random_range(0..5)) as usize;
        let mut arr: Vec<BytesRef<Vec<u8>>> = Vec::with_capacity(arr_len);
        for _ in 0..arr_len {
            let byte_len = TestUtil::next_int(random, 0, max_len);
            let mut bytes = vec![0u8; byte_len as usize];
            random.fill_bytes(&mut bytes);
            arr.push(BytesRef::from_bytes(bytes));
        }
        do_test(random, &arr, from, to, max_len)
    }

    #[test]
    pub fn test_shared_prefixes() -> Result<()> {
        let mut random = random();
        for _ in 0..100 {
            do_test_shared_prefixes(&mut random)?;
        }
        Ok(())
    }

    pub fn do_test_shared_prefixes(random: &mut StdRng) -> Result<()> {
        let from = random.random_range(0..5);
        let to = from + TestUtil::next_int(random, 1, 10000);
        let max_len = TestUtil::next_int(random, 1, 12);
        let arr_len = (from + to + random.random_range(0..5)) as usize;
        let mut arr: Vec<BytesRef<Vec<u8>>> = Vec::with_capacity(arr_len);
        for _ in 0..arr_len {
            let byte_len = TestUtil::next_int(random, 0, max_len);
            let mut bytes = vec![0u8; byte_len as usize];
            random.fill_bytes(&mut bytes);
            arr.push(BytesRef::from_bytes(bytes));
        }
        let shared_prefix_length =
            min(arr[0].length as i32, TestUtil::next_int(random, 1, max_len));
        for i in 1..arr.len() {
            let copy_len = min(shared_prefix_length, arr[i].length as i32) as usize;
            let offset_1 = arr[i].offset;
            let offset_2 = arr[0].offset;
            arr[i]
                .bytes
                .copy_within(offset_2..offset_2 + copy_len, offset_1);
        }
        do_test(random, &arr, from, to, max_len)
    }

    pub fn do_test(
        random: &mut StdRng,
        arr: &[BytesRef<Vec<u8>>],
        from: i32,
        to: i32,
        max_len: i32,
    ) -> Result<()> {
        let k = TestUtil::next_int(random, from, to - 1) as usize;

        let mut expected = arr.to_vec();
        expected[from as usize..to as usize].sort();

        let mut actual = arr.to_vec();
        let enforced_max_len = if random.random_bool(0.5) {
            max_len
        } else {
            i32::MAX
        };

        let selector_impl = RadixSelectorMock {
            actual,
            enforced_max_len,
        };

        let mut selector = RadixSelector::new(enforced_max_len, selector_impl);
        Selector::select(&mut selector, from, to, k as i32)?;
        actual = selector.sub_selector.actual.clone();

        assert_eq!(expected[k], actual[k]);
        for i in 0..actual.len() {
            if i < from as usize || i >= to as usize {
                assert_eq!(&arr[i], &actual[i]);
            } else if i <= k {
                assert_ne!(actual[i].cmp(&actual[k]), Ordering::Greater);
            } else {
                assert_ne!(actual[i].cmp(&actual[k]), Ordering::Less);
            }
        }
        Ok(())
    }

    struct RadixSelectorMock {
        enforced_max_len: i32,
        actual: Vec<BytesRef<Vec<u8>>>,
    }

    impl Selector for RadixSelectorMock {
        fn swap(&mut self, i: i32, j: i32) -> Result<()> {
            self.actual.swap(i as usize, j as usize);
            Ok(())
        }
    }

    impl RadixSelectorBase for RadixSelectorMock {
        fn byte_at(&self, i: i32, k: i32) -> i32 {
            assert!(k < self.enforced_max_len);
            let b = self.actual[i as usize].clone();
            if k < b.length as i32 {
                b.bytes[k as usize] as i32
            } else {
                -1
            }
        }
    }
}
