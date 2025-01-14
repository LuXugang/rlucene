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
use crate::util::bytes_ref_comparator::{BytesRefComparator, BYTES_REF_COMPARATOR_TYPE};

use crate::util::error::lucene_error::LuceneError;
use crate::util::intro_sorter::IntroSorter;
use crate::util::{Comparator, MSBRadixSorter, MSBRadixSorterBase, Sorter};

/// A [`BytesRef`] sorter that attempts to use an efficient radix sorter if [`StringSorter::compare`]
/// is a [`BytesRefComparator`]. Otherwise, it falls back to [`StringSorterBase::fall_back_sorter`].
///
/// # Note
/// - This is an internal API and is not intended for external use.
pub struct StringSorter<T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    delegate_sorter: T,
    scratch1: BytesRefBuilder,
    scratch2: BytesRefBuilder,
    scratch_bytes1: BytesRef,
    scratch_bytes2: BytesRef,
    cmp: C,
}

impl<T, C> StringSorter<T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    pub fn new(delegate_sorter: T, cmp: C) -> StringSorter<T, C> {
        StringSorter {
            delegate_sorter,
            scratch1: BytesRefBuilder::default(),
            scratch2: BytesRefBuilder::default(),
            scratch_bytes1: BytesRef::default(),
            scratch_bytes2: BytesRef::default(),
            cmp,
        }
    }
    #[cfg(feature = "test_only")]
    pub fn get_delegate_sorter(&self) -> &T {
        &self.delegate_sorter
    }
}

impl<T, C> Sorter for StringSorter<T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    fn compare(&mut self, i: i32, j: i32) -> Result<i32, LuceneError> {
        self.delegate_sorter
            .get(&mut self.scratch1, &mut self.scratch_bytes1, i)?;
        self.delegate_sorter
            .get(&mut self.scratch2, &mut self.scratch_bytes2, j)?;
        Ok(self.cmp.compare(&self.scratch_bytes1, &self.scratch_bytes2))
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        self.delegate_sorter.swap(i, j)
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<(), LuceneError> {
        // In fact, it is necessary to provide an instance that implements BytesRefComparator to simplify the code.
        // However, the TYPE of this instance cannot be specified as "BytesRefComparator".
        if C::TYPE.eq(BYTES_REF_COMPARATOR_TYPE) {
            self.delegate_sorter
                .radix_sorter(&mut self.cmp)
                .sort(from, to)
        } else {
            self.delegate_sorter
                .fall_back_sorter::<T, C>(&mut self.cmp, None)
                .sort(from, to)
        }
    }
}

pub struct MSBStringRadixSorter<'a, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    scratch1: BytesRefBuilder,
    scratch_bytes1: BytesRef,
    cmp: &'a mut C,
    delegate_sorter: &'a mut T,
}
impl<'a, T, C> MSBStringRadixSorter<'a, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    pub fn new(cmp: &'a mut C, delegate_sorter: &'a mut T) -> MSBStringRadixSorter<'a, T, C> {
        MSBStringRadixSorter {
            scratch1: BytesRefBuilder::default(),
            scratch_bytes1: BytesRef::default(),
            cmp,
            delegate_sorter,
        }
    }
}

impl<T, C> Sorter for MSBStringRadixSorter<'_, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        self.delegate_sorter.swap(i, j)
    }
}

impl<T, C> MSBRadixSorterBase for MSBStringRadixSorter<'_, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    fn byte_at(&mut self, i: i32, k: i32) -> Result<i32, LuceneError> {
        self.delegate_sorter
            .get(&mut self.scratch1, &mut self.scratch_bytes1, i)?;
        Ok(self.cmp.byte_at(&self.scratch_bytes1, k as u32))
    }

    fn get_fallback_sorter(&mut self, k: i32, _length: i32) -> impl Sorter {
        self.delegate_sorter
            .fall_back_sorter::<T, C>(self.cmp, Some(k))
    }
}

pub struct StringIntroSorter<'a, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    pivot: BytesRef,
    pivot_builder: BytesRefBuilder,
    scratch1: BytesRefBuilder,
    scratch2: BytesRefBuilder,
    scratch_bytes1: BytesRef,
    scratch_bytes2: BytesRef,
    cmp: &'a mut C,
    delegate_sorter: &'a mut T,
    k: Option<i32>,
}
impl<'a, T, C> StringIntroSorter<'a, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    pub fn new(
        cmp: &'a mut C,
        delegate_sorter: &'a mut T,
        k: Option<i32>,
    ) -> StringIntroSorter<'a, T, C> {
        StringIntroSorter {
            pivot: BytesRef::default(),
            pivot_builder: BytesRefBuilder::default(),
            scratch1: BytesRefBuilder::default(),
            scratch2: BytesRefBuilder::default(),
            scratch_bytes1: BytesRef::default(),
            scratch_bytes2: BytesRef::default(),
            cmp,
            delegate_sorter,
            k,
        }
    }
}
impl<T, C> Sorter for StringIntroSorter<'_, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
    fn compare(&mut self, i: i32, j: i32) -> Result<i32, LuceneError> {
        self.delegate_sorter
            .get(&mut self.scratch1, &mut self.scratch_bytes1, i)?;
        self.delegate_sorter
            .get(&mut self.scratch2, &mut self.scratch_bytes2, j)?;
        if self.k.is_some() {
            Ok(self.cmp.compare_with_offset(
                &self.scratch_bytes1,
                &self.scratch_bytes2,
                self.k.unwrap() as u32,
            ))
        } else {
            Ok(self.cmp.compare(&self.scratch_bytes1, &self.scratch_bytes2))
        }
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        self.delegate_sorter.swap(i, j)
    }

    fn set_pivot(&mut self, i: i32) -> Result<(), LuceneError> {
        self.delegate_sorter
            .get(&mut self.pivot_builder, &mut self.pivot, i)?;
        Ok(())
    }

    fn compare_pivot(&mut self, j: i32) -> Result<i32, LuceneError> {
        self.delegate_sorter
            .get(&mut self.scratch1, &mut self.scratch_bytes1, j)?;
        if self.k.is_some() {
            Ok(self.cmp.compare_with_offset(
                &self.pivot,
                &self.scratch_bytes1,
                self.k.unwrap() as u32,
            ))
        } else {
            Ok(self.cmp.compare(&self.pivot, &self.scratch_bytes1))
        }
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<(), LuceneError> {
        IntroSorter::sort_range(self, from, to)?;
        Ok(())
    }
}

impl<T, C> IntroSorter for StringIntroSorter<'_, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef>,
{
}

pub trait StringSorterBase {
    fn get(
        &mut self,
        builder: &mut BytesRefBuilder,
        result: &mut BytesRef,
        i: i32,
    ) -> Result<(), LuceneError>;
    fn fall_back_sorter<'a, T, C>(&'a mut self, cmp: &'a mut C, k: Option<i32>) -> impl Sorter + 'a
    where
        T: Sorter + StringSorterBase,
        C: BytesRefComparator + Comparator<BytesRef>,
        Self: Sorter + Sized,
    {
        StringIntroSorter::new(cmp, self, k)
    }
    fn radix_sorter<'a, C>(&'a mut self, cmp: &'a mut C) -> impl Sorter + 'a
    where
        C: BytesRefComparator + Comparator<BytesRef>,
        Self: Sorter + Sized,
    {
        let length = cmp.compared_bytes_count();
        let msb_radix_sorter_delegate_sorter = MSBStringRadixSorter::new(cmp, self);
        MSBRadixSorter::new(length, msb_radix_sorter_delegate_sorter)
    }
}
