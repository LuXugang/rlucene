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

use crate::util::error::lucene_error::Result;
use crate::util::{
    BytesRefComparator, Comparator, MSBRadixSorter, MSBRadixSorterBase, MergeSorter, Sorter,
    StableMSBRadixSorter, StableMSBRadixSorterBase, StringSorterBase,
};

pub(crate) struct StableStringSorter<T>
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

impl<T> Sorter for StableStringSorter<T> where T: Sorter + StableStringSorterBase {}

impl<T> StringSorterBase for StableStringSorter<T>
where
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
{
    fn get(&mut self, builder: &mut BytesRefBuilder, result: &mut BytesRef, i: i32) -> Result<()> {
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
        fall_back_sorter_stable(cmp, &mut self.delegate_sorter, k)
    }

    fn radix_sorter<'a, C1>(&'a mut self, cmp: &'a mut C1) -> impl Sorter + 'a
    where
        C1: BytesRefComparator + Comparator<BytesRef>,
    {
        let length = cmp.compared_bytes_count();
        let delegate_sorter = StableMSBRadixSorterImpl {
            delegate_sorter: &mut self.delegate_sorter,
            cmp,
            scratch1: BytesRefBuilder::new(),
            scratch_bytes1: BytesRef::default(),
        };
        let stable_msb_radix_sorter = StableMSBRadixSorter::new(delegate_sorter, length);
        MSBRadixSorter::new(length, stable_msb_radix_sorter)
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
    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        self.delegate_sorter.swap(i, j)
    }
}

impl<T, C> MSBRadixSorterBase for StableMSBRadixSorterImpl<'_, T, C>
where
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    fn byte_at(&mut self, i: i32, k: i32) -> Result<i32> {
        self.delegate_sorter
            .get(&mut self.scratch1, &mut self.scratch_bytes1, i)?;
        Ok(self.cmp.byte_at(&self.scratch_bytes1, k))
    }

    fn get_fallback_sorter(&mut self, k: i32, _length: i32) -> impl Sorter {
        fall_back_sorter_stable(self.cmp, self.delegate_sorter, Some(k))
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
    fn compare(&mut self, i: i32, j: i32) -> Result<i32> {
        self.delegate_sorter
            .get(&mut self.scratch1, &mut self.scratch_bytes1, i)?;
        self.delegate_sorter
            .get(&mut self.scratch2, &mut self.scratch_bytes2, j)?;
        if self.k.is_some() {
            Ok(self.cmp.compare_with_offset(
                &self.scratch_bytes1,
                &self.scratch_bytes2,
                self.k.unwrap(),
            ))
        } else {
            self.cmp.compare(&self.scratch_bytes1, &self.scratch_bytes2)
        }
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        self.delegate_sorter.swap(i, j)
    }
}

impl<T, C> StringSorterBase for MergeSorterStableImpl<'_, T, C>
where
    C: BytesRefComparator + Comparator<BytesRef>,
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
{
    fn get(&mut self, builder: &mut BytesRefBuilder, result: &mut BytesRef, i: i32) -> Result<()> {
        self.delegate_sorter.get(builder, result, i)
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

impl<T, C> MSBRadixSorterBase for MergeSorterStableImpl<'_, T, C>
where
    C: BytesRefComparator + Comparator<BytesRef>,
    T: Sorter + StableStringSorterBase + MSBRadixSorterBase,
{
    fn byte_at(&mut self, i: i32, k: i32) -> Result<i32> {
        self.delegate_sorter.byte_at(i, k)
    }

    fn get_fallback_sorter(&mut self, k: i32, length: i32) -> impl Sorter {
        self.delegate_sorter.get_fallback_sorter(k, length)
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

fn fall_back_sorter_stable<'a, T, C>(
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
