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
use crate::util::error::runtime_error::RuntimeError;
use crate::util::{check_range, MSBRadixSorterBase, Sorter, BINARY_SORT_THRESHOLD, HISTOGRAM_SIZE};

pub struct StableMSBRadixSorter<T>
where
    T: StableMSBRadixSorterBase,
{
    delegate_sorter: T,
    fixed_start_offsets: Vec<i32>,
}

impl<T> StableMSBRadixSorter<T>
where
    T: StableMSBRadixSorterBase,
{
    pub fn new(delegate_sorter: T) -> StableMSBRadixSorter<T> {
        StableMSBRadixSorter {
            delegate_sorter,
            fixed_start_offsets: vec![0; HISTOGRAM_SIZE],
        }
    }
}

impl<T> Sorter for StableMSBRadixSorter<T>
where
    T: StableMSBRadixSorterBase,
{
    fn compare(&mut self, _i: i32, _j: i32) -> i32 {
        unreachable!()
    }

    fn swap(&mut self, _i: i32, _j: i32) {
        unreachable!()
    }

    fn set_pivot(&mut self, _i: i32) {
        unreachable!()
    }

    fn compare_pivot(&mut self, _i: i32) -> i32 {
        unreachable!()
    }

    fn sort(&mut self, _from: i32, _to: i32) -> Result<(), RuntimeError> {
        unreachable!()
    }
}

impl<T> MSBRadixSorterBase for StableMSBRadixSorter<T>
where
    T: StableMSBRadixSorterBase,
{
    fn byte_at(&mut self, i: i32, k: i32) -> i32 {
        self.delegate_sorter.byte_at(i, k)
    }

    fn get_fallback_sorter(&mut self, k: i32) -> impl Sorter {
        self.delegate_sorter.get_fallback_sorter(k)
    }

    fn reorder(
        &mut self,
        from: i32,
        to: i32,
        start_offsets: &mut [i32],
        end_offsets: &mut [i32],
        k: i32,
    ) {
        // Copy start_offsets to fixed_start_offsets
        self.fixed_start_offsets[..start_offsets.len()].copy_from_slice(start_offsets);

        for (i, &limit) in end_offsets.iter().enumerate().take(HISTOGRAM_SIZE) {
            let mut h1 = self.fixed_start_offsets[i];
            while h1 < limit {
                let b = self.get_bucket(from + h1, k);
                let h2 = start_offsets[b as usize];
                start_offsets[b as usize] += 1;
                self.delegate_sorter.save(from + h1, from + h2);
                h1 += 1;
            }
        }

        self.delegate_sorter.restore(from, to);
    }

    fn get_bucket(&mut self, i: i32, k: i32) -> i32 {
        self.delegate_sorter.get_bucket(i, k)
    }

    fn build_histogram(
        &mut self,
        prefix_common_bucket: i32,
        prefix_common_len: i32,
        from: i32,
        to: i32,
        k: i32,
        histogram: &mut [i32],
    ) {
        self.delegate_sorter.build_histogram(
            prefix_common_bucket,
            prefix_common_len,
            from,
            to,
            k,
            histogram,
        )
    }

    fn should_fallback(&self, from: i32, to: i32, l: i32) -> bool {
        self.delegate_sorter.should_fallback(from, to, l)
    }
}

pub trait StableMSBRadixSorterBase: MSBRadixSorterBase {
    /// Save the i-th value into the j-th position in temporary storage.
    fn save(&mut self, i: i32, j: i32);
    /// Restore values between i-th and j-th(excluding) in temporary storage into original storage.
    fn restore(&mut self, i: i32, j: i32);
}

pub struct MergeSorter<T>
where
    T: Sorter + StableMSBRadixSorterBase,
{
    delegate_sorter: T,
    pivot_index: i32,
}

impl<T> MergeSorter<T>
where
    T: Sorter + StableMSBRadixSorterBase,
{
    fn merge_sort(&mut self, from: i32, to: i32) {
        if to - from < BINARY_SORT_THRESHOLD {
            self.binary_sort(from, to);
        } else {
            let mid = (from + to) / 2;
            self.merge_sort(from, mid);
            self.merge_sort(mid, to);
            self.merge(from, to, mid);
        }
    }
    /// We tried to expose this to implementations to get a bulk copy optimization.
    /// However, it did not bring a noticeable improvement in benchmarks as `len` is usually small.
    fn bulk_save(&mut self, from: i32, tmp_from: i32, len: i32) {
        for i in 0..len {
            self.delegate_sorter.save(from + i, tmp_from + i);
        }
    }
    fn merge(&mut self, from: i32, to: i32, mid: i32) {
        debug_assert!(
            to > mid && mid > from,
            "Invalid indices: to={}, mid={}, from={}",
            to,
            mid,
            from
        );
        // If already sorted, return early
        if self.delegate_sorter.compare(mid - 1, mid) <= 0 {
            return;
        }
        let mut left = from;
        let mut right = mid;
        let mut index = from;
        loop {
            let cmp = self.delegate_sorter.compare(left, right);

            if cmp <= 0 {
                self.delegate_sorter.save(left, index);
                left += 1;
                index += 1;

                if left == mid {
                    assert_eq!(
                        index, right,
                        "Index mismatch: index={}, right={}",
                        index, right
                    );
                    self.bulk_save(right, index, to - right);
                    break;
                }
            } else {
                self.delegate_sorter.save(right, index);
                right += 1;
                index += 1;

                if right == to {
                    assert_eq!(
                        to - index,
                        mid - left,
                        "Range mismatch: to-index={}, mid-left={}",
                        to - index,
                        mid - left
                    );
                    self.bulk_save(left, index, mid - left);
                    break;
                }
            }
        }
        self.delegate_sorter.restore(from, to);
    }
}
impl<T> Sorter for MergeSorter<T>
where
    T: Sorter + StableMSBRadixSorterBase,
{
    fn compare(&mut self, i: i32, j: i32) -> i32 {
        self.delegate_sorter.compare(i, j)
    }

    fn swap(&mut self, i: i32, j: i32) {
        self.delegate_sorter.swap(i, j)
    }

    fn set_pivot(&mut self, i: i32) {
        self.pivot_index = i;
    }

    fn compare_pivot(&mut self, i: i32) -> i32 {
        self.compare(self.pivot_index, i)
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<(), RuntimeError> {
        check_range(from, to)?;
        self.merge_sort(from, to);
        Ok(())
    }
}

pub struct MergeSorterImpl<'a, T>
where
    T: Sorter + MSBRadixSorterBase + StableMSBRadixSorterBase,
{
    k: i32,
    max_length: i32,
    delegate_sorter: &'a mut T,
}
impl<'a, T> MergeSorterImpl<'a, T>
where
    T: Sorter + MSBRadixSorterBase + StableMSBRadixSorterBase,
{
    pub fn new(k: i32, max_length: i32, delegate_sorter: &'a mut T) -> MergeSorterImpl<'a, T>
    where
        T: Sorter + StableMSBRadixSorterBase,
    {
        MergeSorterImpl {
            k,
            max_length,
            delegate_sorter,
        }
    }
}
impl<T> Sorter for MergeSorterImpl<'_, T>
where
    T: Sorter + MSBRadixSorterBase + StableMSBRadixSorterBase,
{
    fn compare(&mut self, i: i32, j: i32) -> i32 {
        for o in self.k..self.max_length {
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

    fn swap(&mut self, i: i32, j: i32) {
        self.delegate_sorter.swap(i, j);
    }

    fn set_pivot(&mut self, _i: i32) {
        unreachable!(
            "use MergeSorter to wrap MergeSorterImpl in order to enable `set_pivot` functionality.\
        MergeSorterImpl is only used for MergeSorterBase's methods."
        )
    }

    fn compare_pivot(&mut self, _i: i32) -> i32 {
        unreachable!("use MergeSorter to wrap MergeSorterImpl in order to enable `compare_pivot` functionality.\
        MergeSorterImpl is only used for MergeSorterBase's methods.")
    }

    fn sort(&mut self, _from: i32, _to: i32) -> Result<(), RuntimeError> {
        unreachable!("You need to use MergeSorter to wrap MergeSorterImpl in order to enable sorting functionality.")
    }
}

impl<T> MSBRadixSorterBase for MergeSorterImpl<'_, T>
where
    T: MSBRadixSorterBase + Sorter + StableMSBRadixSorterBase,
{
    fn byte_at(&mut self, i: i32, k: i32) -> i32 {
        self.delegate_sorter.byte_at(i, k)
    }

    fn get_fallback_sorter(&mut self, k: i32) -> impl Sorter {
        self.delegate_sorter.get_fallback_sorter(k)
    }

    fn reorder(
        &mut self,
        from: i32,
        to: i32,
        start_offsets: &mut [i32],
        end_offsets: &mut [i32],
        k: i32,
    ) {
        self.delegate_sorter
            .reorder(from, to, start_offsets, end_offsets, k);
    }

    fn get_bucket(&mut self, i: i32, k: i32) -> i32 {
        self.delegate_sorter.get_bucket(i, k)
    }

    fn build_histogram(
        &mut self,
        prefix_common_bucket: i32,
        prefix_common_len: i32,
        from: i32,
        to: i32,
        k: i32,
        histogram: &mut [i32],
    ) {
        self.delegate_sorter.build_histogram(
            prefix_common_bucket,
            prefix_common_len,
            from,
            to,
            k,
            histogram,
        );
    }

    fn should_fallback(&self, from: i32, to: i32, l: i32) -> bool {
        self.delegate_sorter.should_fallback(from, to, l)
    }
}

impl<T> StableMSBRadixSorterBase for MergeSorterImpl<'_, T>
where
    T: Sorter + MSBRadixSorterBase + StableMSBRadixSorterBase,
{
    fn save(&mut self, i: i32, j: i32) {
        self.delegate_sorter.save(i, j);
    }

    fn restore(&mut self, i: i32, j: i32) {
        self.delegate_sorter.restore(i, j);
    }
}

pub fn default_get_fallback_sorter_stable<T>(
    final_max_length: i32,
    sorter: &mut T,
    k: i32,
) -> impl Sorter + '_
where
    T: Sorter + MSBRadixSorterBase + StableMSBRadixSorterBase,
{
    let delegate_sorter = MergeSorterImpl::new(k, final_max_length, sorter);
    MergeSorter {
        delegate_sorter,
        pivot_index: 0,
    }
}
