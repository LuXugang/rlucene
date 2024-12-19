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
use crate::util::bytes_ref_comparator::BytesRefComparator;
use crate::util::error::runtime_error::RuntimeError;
use crate::util::intro_sorter::IntroSorter;
use crate::util::{Comparator, MSBRadixSorter, MSBRadixSorterBase, Sorter};

pub struct StringSorter<T, C>
where
    T: Sorter + StringSorterBase,
    C: Comparator<BytesRef>,
{
    sub_sorter: T,
    scratch1: BytesRefBuilder,
    scratch2: BytesRefBuilder,
    pivot_builder: BytesRefBuilder,
    scratch_bytes1: BytesRef,
    scratch_bytes2: BytesRef,
    pivot: BytesRef,
    cmp: C,
}

impl<T, C> StringSorter<T, C>
where
    T: Sorter + StringSorterBase,
    C: Comparator<BytesRef>,
{
    fn new(sub_sorter: T, cmp: C) -> StringSorter<T, C> {
        StringSorter {
            sub_sorter,
            scratch1: BytesRefBuilder::new(),
            scratch2: BytesRefBuilder::new(),
            pivot_builder: BytesRefBuilder::new(),
            scratch_bytes1: BytesRef::new(),
            scratch_bytes2: BytesRef::new(),
            pivot: BytesRef::new(),
            cmp,
        }
    }

    fn radix_sorter(
        &mut self,
        cmp: impl BytesRefComparator,
    ) -> MSBRadixSorter<MSBStringRadixSorter<T, C>> {
        {
            let max_length = cmp.compared_bytes_count();
            let sub_sorter = MSBStringRadixSorter {
                scratch1: &mut self.scratch1,
                scratch_bytes1: &mut self.scratch_bytes1,
                cmp: &mut self.cmp,
                sub_sorter: &mut self.sub_sorter,
            };
            MSBRadixSorter::new(max_length as i32, sub_sorter)
        }
    }

    fn fall_back_sorter<'a>(&'a mut self, cmp: &'a mut C) -> StringIntroSorter<'a, T, C> {
        StringIntroSorter {
            pivot: &mut self.pivot,
            pivot_builder: &mut self.pivot_builder,
            scratch1: &mut self.scratch1,
            scratch2: &mut self.scratch2,
            scratch_bytes1: &mut self.scratch_bytes1,
            scratch_bytes2: &mut self.scratch_bytes2,
            cmp,
            sub_sorter: &mut self.sub_sorter,
        }
    }
}

impl<T, C> Sorter for StringSorter<T, C>
where
    T: Sorter + StringSorterBase,
    C: Comparator<BytesRef>,
{
    fn compare(&mut self, i: i32, j: i32) -> i32 {
        self.sub_sorter
            .get(&mut self.scratch1, &mut self.scratch_bytes1, i);
        self.sub_sorter
            .get(&mut self.scratch2, &mut self.scratch_bytes2, j);
        self.cmp.compare(&self.scratch_bytes1, &self.scratch_bytes2)
    }

    fn swap(&mut self, i: i32, j: i32) {
        todo!()
    }

    fn set_pivot(&mut self, i: i32) {
        todo!()
    }

    fn compare_pivot(&mut self, i: i32) -> i32 {
        todo!()
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<(), RuntimeError> {
        todo!()
    }
}

struct MSBStringRadixSorter<'a, T, C>
where
    T: Sorter + StringSorterBase,
    C: Comparator<BytesRef>,
{
    scratch1: &'a mut BytesRefBuilder,
    scratch_bytes1: &'a mut BytesRef,
    cmp: &'a mut C,
    sub_sorter: &'a mut T,
}
impl<T, C> Sorter for MSBStringRadixSorter<'_, T, C>
where
    T: Sorter + StringSorterBase,
    C: Comparator<BytesRef>,
{
    fn compare(&mut self, _i: i32, _j: i32) -> i32 {
        unreachable!("unused: not a comparison-based sort")
    }

    fn swap(&mut self, i: i32, j: i32) {
        self.sub_sorter.swap(i, j);
    }

    fn set_pivot(&mut self, i: i32) {
        todo!()
    }

    fn compare_pivot(&mut self, j: i32) -> i32 {
        todo!()
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<(), RuntimeError> {
        unreachable!("You need to use MSBRadixSorter to wrap MSBStringRadixSorter in order to enable sorting functionality.")
    }
}

impl<T, C> MSBRadixSorterBase for MSBStringRadixSorter<'_, T, C>
where
    T: Sorter + StringSorterBase,
    C: Comparator<BytesRef>,
{
    fn byte_at(&self, i: i32, k: i32) -> i32 {
        todo!()
    }

    fn get_fallback_sorter(&mut self, k: i32) -> impl Sorter {
        self.sub_sorter.fall_back_sorter(self.cmp)
    }
}

struct StringIntroSorter<'a, T, C>
where
    T: Sorter + StringSorterBase,
    C: Comparator<BytesRef>,
{
    pivot: &'a mut BytesRef,
    pivot_builder: &'a mut BytesRefBuilder,
    scratch1: &'a mut BytesRefBuilder,
    scratch2: &'a mut BytesRefBuilder,
    scratch_bytes1: &'a mut BytesRef,
    scratch_bytes2: &'a mut BytesRef,
    cmp: &'a mut C,
    sub_sorter: &'a mut T,
}
impl<T, C> Sorter for StringIntroSorter<'_, T, C>
where
    T: Sorter + StringSorterBase,
    C: Comparator<BytesRef>,
{
    fn compare(&mut self, i: i32, j: i32) -> i32 {
        self.sub_sorter.get(self.scratch1, self.scratch_bytes1, i);
        self.sub_sorter.get(self.scratch2, self.scratch_bytes2, j);
        self.cmp.compare(self.scratch_bytes1, self.scratch_bytes2)
    }

    fn swap(&mut self, i: i32, j: i32) {
        self.sub_sorter.swap(i, j);
    }

    fn set_pivot(&mut self, i: i32) {
        self.sub_sorter.get(self.pivot_builder, self.pivot, i);
    }

    fn compare_pivot(&mut self, j: i32) -> i32 {
        self.sub_sorter.get(self.scratch1, self.scratch_bytes1, j);
        self.cmp.compare(self.pivot, self.scratch_bytes1)
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<(), RuntimeError> {
        IntroSorter::sort_range(self, from, to)?;
        Ok(())
    }
}

impl<T, C> IntroSorter for StringIntroSorter<'_, T, C>
where
    T: Sorter + StringSorterBase,
    C: Comparator<BytesRef>,
{
}

pub trait StringSorterBase {
    fn get(&self, builder: &mut BytesRefBuilder, result: &mut BytesRef, i: i32);
    fn fall_back_sorter<C>(&self, cmp: &C) -> impl Sorter
    where
        C: Comparator<BytesRef>;
}
