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
use crate::index::{BytesRef, BytesRefBuilder};
use crate::util::error::runtime_error::RuntimeError;
use crate::util::{
    default_build_histogram, default_get_get_bucket, default_reorder, default_should_fallback,
    BytesRefComparator, Comparator, MSBRadixSorter, MSBRadixSorterBase, MergeSorter, Sorter,
    StableMSBRadixSorter, StableMSBRadixSorterBase, StringSorterBase,
};

pub struct StableStringSorter<T>
where
    T: Sorter + StableStringSorterBase,
{
    delegate_sorter: T,
}
impl<T> StableStringSorter<T>
where
    T: Sorter + StableStringSorterBase,
{
    pub fn new(delegate_sorter: T) -> StableStringSorter<T> {
        StableStringSorter { delegate_sorter }
    }
}

impl<T> Sorter for StableStringSorter<T>
where
    T: Sorter + StableStringSorterBase,
{
    fn compare(&mut self, _i: i32, _j: i32) -> i32 {
        unreachable!("Implemented polymorphism through its delegate_sorter")
    }

    fn swap(&mut self, i: i32, j: i32) {
        self.delegate_sorter.swap(i, j)
    }

    fn set_pivot(&mut self, _i: i32) {
        unreachable!("Implemented polymorphism through its delegate_sorter")
    }

    fn compare_pivot(&mut self, _i: i32) -> i32 {
        unreachable!("Implemented polymorphism through its delegate_sorter")
    }

    fn sort(&mut self, _from: i32, _to: i32) -> Result<(), RuntimeError> {
        unreachable!("Implemented polymorphism through its delegate_sorter")
    }
}

impl<T> StringSorterBase for StableStringSorter<T>
where
    T: Sorter + StableStringSorterBase,
{
    fn get(&mut self, builder: &mut BytesRefBuilder, result: &mut BytesRef, i: i32) {
        self.delegate_sorter.get(builder, result, i)
    }

    fn fall_back_sorter<'a, T1, C1>(
        &'a mut self,
        cmp: &'a mut C1,
        k: Option<i32>,
    ) -> impl Sorter + 'a
    where
        T1: Sorter + StringSorterBase,
        C1: BytesRefComparator + Comparator<BytesRef>,
    {
        self.delegate_sorter.fall_back_sorter::<T, C1>(cmp, k)
    }

    fn radix_sorter<'a, C1>(&'a mut self, cmp: &'a mut C1) -> impl Sorter + 'a
    where
        C1: BytesRefComparator + Comparator<BytesRef>,
    {
        self.delegate_sorter.radix_sorter(cmp)
    }
}
pub struct StableMSBRadixSorterImpl<'a, T, C>
where
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    delegate_sorter: &'a mut T,
    cmp: &'a mut C,
    scratch1: BytesRefBuilder,
    scratch_bytes1: BytesRef,
}
impl<T, C> Sorter for StableMSBRadixSorterImpl<'_, T, C>
where
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    fn compare(&mut self, i: i32, j: i32) -> i32 {
        self.delegate_sorter.compare(i, j)
    }

    fn swap(&mut self, i: i32, j: i32) {
        self.delegate_sorter.swap(i, j)
    }

    fn set_pivot(&mut self, i: i32) {
        self.delegate_sorter.set_pivot(i)
    }

    fn compare_pivot(&mut self, i: i32) -> i32 {
        self.delegate_sorter.compare_pivot(i)
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<(), RuntimeError>
    where
        T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
        C: BytesRefComparator + Comparator<BytesRef>,
    {
        self.delegate_sorter.sort(from, to)
    }
}

impl<'a, T, C> MSBRadixSorterBase for StableMSBRadixSorterImpl<'a, T, C>
where
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    fn byte_at(&mut self, i: i32, k: i32) -> i32 {
        self.delegate_sorter
            .get(&mut self.scratch1, &mut self.scratch_bytes1, i);
        self.cmp.byte_at(&self.scratch_bytes1, k as u32)
    }

    fn get_fallback_sorter(&mut self, k: i32) -> impl Sorter {
        self.delegate_sorter
            .fall_back_sorter::<T, C>(self.cmp, Some(k))
    }

    fn reorder(
        &mut self,
        from: i32,
        to: i32,
        start_offsets: &mut [i32],
        end_offsets: &mut [i32],
        k: i32,
    ) {
        default_reorder(self, from, to, start_offsets, end_offsets, k)
    }

    fn get_bucket(&mut self, i: i32, k: i32) -> i32 {
        default_get_get_bucket(self, i, k)
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
        default_build_histogram(
            self,
            prefix_common_bucket,
            prefix_common_len,
            from,
            to,
            k,
            histogram,
        )
    }

    fn should_fallback(&self, from: i32, to: i32, l: i32) -> bool {
        default_should_fallback(from, to, l)
    }
}

impl<T, C> StableMSBRadixSorterBase for StableMSBRadixSorterImpl<'_, T, C>
where
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    fn save(&mut self, i: i32, j: i32) {
        self.delegate_sorter.save(i, j)
    }

    fn restore(&mut self, i: i32, j: i32) {
        self.delegate_sorter.restore(i, j)
    }
}

pub struct MergeSorterStableImpl<'a, T, C>
where
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    scratch1: BytesRefBuilder,
    scratch2: BytesRefBuilder,
    scratch_bytes1: BytesRef,
    scratch_bytes2: BytesRef,
    cmp: &'a mut C,
    delegate_sorter: &'a mut T,
    k: Option<i32>,
}
impl<T, C> Sorter for MergeSorterStableImpl<'_, T, C>
where
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    fn compare(&mut self, i: i32, j: i32) -> i32 {
        self.delegate_sorter
            .get(&mut self.scratch1, &mut self.scratch_bytes1, i);
        self.delegate_sorter
            .get(&mut self.scratch2, &mut self.scratch_bytes2, j);
        if self.k.is_some() {
            self.cmp.compare_with_offset(
                &self.scratch_bytes1,
                &self.scratch_bytes2,
                self.k.unwrap() as u32,
            )
        } else {
            self.cmp.compare(&self.scratch_bytes1, &self.scratch_bytes2)
        }
    }

    fn swap(&mut self, i: i32, j: i32) {
        self.delegate_sorter.swap(i, j)
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

impl<'a, T, C> StringSorterBase for MergeSorterStableImpl<'a, T, C>
where
    C: BytesRefComparator + Comparator<BytesRef>,
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
{
    fn get(&mut self, builder: &mut BytesRefBuilder, result: &mut BytesRef, i: i32) {
        self.delegate_sorter.get(builder, result, i)
    }

    fn fall_back_sorter<'b, T1, C1>(
        &'b mut self,
        cmp: &'b mut C1,
        k: Option<i32>,
    ) -> impl Sorter + 'b
    where
        T1: Sorter + StringSorterBase,
        C1: BytesRefComparator + Comparator<BytesRef>,
    {
        self.delegate_sorter.fall_back_sorter::<T, C1>(cmp, k)
    }

    fn radix_sorter<'b, C1>(&'b mut self, cmp: &'b mut C1) -> impl Sorter + 'b
    where
        C1: BytesRefComparator + Comparator<BytesRef>,
    {
        self.delegate_sorter.radix_sorter(cmp)
    }
}

impl<T, C> StableStringSorterBase for MergeSorterStableImpl<'_, T, C>
where
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    fn save(&mut self, i: i32, j: i32) {
        self.delegate_sorter.save(i, j)
    }

    fn restore(&mut self, i: i32, j: i32) {
        self.delegate_sorter.restore(i, j)
    }
}

impl<'a, T, C> MSBRadixSorterBase for MergeSorterStableImpl<'a, T, C>
where
    C: BytesRefComparator + Comparator<BytesRef>,
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
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
            .reorder(from, to, start_offsets, end_offsets, k)
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

impl<T, C> StableMSBRadixSorterBase for MergeSorterStableImpl<'_, T, C>
where
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    fn save(&mut self, i: i32, j: i32) {
        self.delegate_sorter.save(i, j)
    }

    fn restore(&mut self, i: i32, j: i32) {
        self.delegate_sorter.restore(i, j)
    }
}

pub trait StableStringSorterBase: StringSorterBase {
    /// Save the i-th value into the j-th position in temporary storage.
    fn save(&mut self, i: i32, j: i32);
    /// Restore values between i-th and j-th(excluding) in temporary storage into original storage.
    fn restore(&mut self, i: i32, j: i32);
}

pub fn default_fall_back_sorter_stable<'a, T, C>(
    cmp: &'a mut C,
    sorter: &'a mut T,
    k: Option<i32>,
) -> impl Sorter + use<'a, T, C>
where
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    let delegate_sorter = MergeSorterStableImpl {
        scratch1: BytesRefBuilder::new(),
        scratch2: BytesRefBuilder::new(),
        scratch_bytes1: BytesRef::default(),
        scratch_bytes2: BytesRef::default(),
        cmp,
        delegate_sorter: sorter,
        k,
    };
    MergeSorter {
        delegate_sorter,
        pivot_index: 0,
    }
}
pub fn default_radix_sorter_stable<'a, C, T>(cmp: &'a mut C, sorter: &'a mut T) -> impl Sorter + 'a
where
    C: BytesRefComparator + Comparator<BytesRef>,
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
{
    let length = cmp.compared_bytes_count();
    let delegate_sorter = StableMSBRadixSorterImpl {
        delegate_sorter: sorter,
        cmp,
        scratch1: BytesRefBuilder::new(),
        scratch_bytes1: BytesRef::default(),
    };
    let stable_msb_radix_sorter = StableMSBRadixSorter::new(delegate_sorter);
    MSBRadixSorter::new(length, stable_msb_radix_sorter)
}
