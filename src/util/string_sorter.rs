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
use crate::util::bytes_ref_comparator::{BYTES_REF_COMPARATOR_TYPE, BytesRefComparator};
use crate::util::error::lucene_error::Result;
use crate::util::intro_sorter::IntroSorter;
use crate::util::{Comparator, MSBRadixSorter, MSBRadixSorterBase, Sorter};

/// A [`BytesRef`] sorter that attempts to use an efficient radix sorter if
/// [`StringSorter::compare`] is a [`BytesRefComparator`]. Otherwise, it falls
/// back to [`StringSorterBase::fall_back_sorter`].
///
/// # Note
/// - This is an internal API and is not intended for external use.
pub(crate) struct StringSorter<T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
{
    delegate_sorter: T,
    scratch1: BytesRefBuilder<Vec<u8>>,
    scratch2: BytesRefBuilder<Vec<u8>>,
    scratch_bytes1: BytesRef<Vec<u8>>,
    scratch_bytes2: BytesRef<Vec<u8>>,
    cmp: C,
}

impl<T, C> StringSorter<T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
{
    pub(crate) fn new(delegate_sorter: T, cmp: C) -> StringSorter<T, C> {
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
    #[allow(unused)]
    pub(crate) fn get_delegate_sorter(&self) -> &T {
        &self.delegate_sorter
    }
}

impl<T, C> Sorter for StringSorter<T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
{
    fn compare(&mut self, i: i32, j: i32) -> Result<i32> {
        self.delegate_sorter
            .get(&mut self.scratch1, &mut self.scratch_bytes1, i)?;
        self.delegate_sorter
            .get(&mut self.scratch2, &mut self.scratch_bytes2, j)?;
        self.cmp.compare(&self.scratch_bytes1, &self.scratch_bytes2)
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        self.delegate_sorter.swap(i, j)
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<()> {
        // In fact, it is necessary to provide an instance that implements
        // BytesRefComparator to simplify the code. However, the TYPE of
        // this instance cannot be specified as "BytesRefComparator".
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
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
{
    scratch1: BytesRefBuilder<Vec<u8>>,
    scratch_bytes1: BytesRef<Vec<u8>>,
    cmp: &'a mut C,
    delegate_sorter: &'a mut T,
}
impl<'a, T, C> MSBStringRadixSorter<'a, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
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
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
{
    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        self.delegate_sorter.swap(i, j)
    }
}

impl<T, C> MSBRadixSorterBase for MSBStringRadixSorter<'_, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
{
    fn byte_at(&mut self, i: i32, k: i32) -> Result<i32> {
        self.delegate_sorter
            .get(&mut self.scratch1, &mut self.scratch_bytes1, i)?;
        Ok(self.cmp.byte_at(&self.scratch_bytes1, k))
    }

    fn get_fallback_sorter(&mut self, k: i32, _length: i32) -> impl Sorter {
        self.delegate_sorter
            .fall_back_sorter::<T, C>(self.cmp, Some(k))
    }
}

pub struct IntroSorterImpl<'a, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
{
    pivot: BytesRef<Vec<u8>>,
    pivot_builder: BytesRefBuilder<Vec<u8>>,
    scratch1: BytesRefBuilder<Vec<u8>>,
    scratch2: BytesRefBuilder<Vec<u8>>,
    scratch_bytes1: BytesRef<Vec<u8>>,
    scratch_bytes2: BytesRef<Vec<u8>>,
    cmp: &'a mut C,
    delegate_sorter: &'a mut T,
    k: Option<i32>,
}
impl<'a, T, C> IntroSorterImpl<'a, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
{
    pub fn new(
        cmp: &'a mut C,
        delegate_sorter: &'a mut T,
        k: Option<i32>,
    ) -> IntroSorterImpl<'a, T, C> {
        IntroSorterImpl {
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
impl<T, C> Sorter for IntroSorterImpl<'_, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
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

    fn set_pivot(&mut self, i: i32) -> Result<()> {
        self.delegate_sorter
            .get(&mut self.pivot_builder, &mut self.pivot, i)?;
        Ok(())
    }

    fn compare_pivot(&mut self, j: i32) -> Result<i32> {
        self.delegate_sorter
            .get(&mut self.scratch1, &mut self.scratch_bytes1, j)?;
        if self.k.is_some() {
            Ok(self
                .cmp
                .compare_with_offset(&self.pivot, &self.scratch_bytes1, self.k.unwrap()))
        } else {
            self.cmp.compare(&self.pivot, &self.scratch_bytes1)
        }
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<()> {
        IntroSorter::sort_range(self, from, to)?;
        Ok(())
    }
}

impl<T, C> IntroSorter for IntroSorterImpl<'_, T, C>
where
    T: Sorter + StringSorterBase,
    C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
{
}

pub trait StringSorterBase {
    fn get(
        &mut self,
        builder: &mut BytesRefBuilder<Vec<u8>>,
        result: &mut BytesRef<Vec<u8>>,
        i: i32,
    ) -> Result<()>;
    fn fall_back_sorter<'a, T, C>(&'a mut self, cmp: &'a mut C, k: Option<i32>) -> impl Sorter + 'a
    where
        T: Sorter + StringSorterBase,
        C: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
        Self: Sorter + Sized,
    {
        IntroSorterImpl::new(cmp, self, k)
    }
    fn radix_sorter<'a, C1>(&'a mut self, cmp: &'a mut C1) -> impl Sorter + 'a
    where
        C1: BytesRefComparator + Comparator<BytesRef<Vec<u8>>>,
        Self: Sorter + Sized,
    {
        let length = cmp.compared_bytes_count();
        let msb_radix_sorter_delegate_sorter = MSBStringRadixSorter::new(cmp, self);
        MSBRadixSorter::new(length, msb_radix_sorter_delegate_sorter)
    }
}
