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
use crate::util::comparator::Comparator;
use crate::util::error::runtime_error::RuntimeError;
use crate::util::intro_sorter::IntroSorter;
use crate::util::sorter::Sorter;

/// An [`IntroSorter`] for object arrays.
///
/// # Note
/// This is an internal API.
pub struct ArrayIntroSorter<'a, T, C: Comparator<T>> {
    pub arr: &'a mut Vec<T>,
    comparator: C,
    pivot: i32,
}

impl<'a, T, C: Comparator<T>> ArrayIntroSorter<'a, T, C> {
    pub fn new(arr: &'a mut Vec<T>, comparator: C) -> ArrayIntroSorter<'a, T, C> {
        ArrayIntroSorter {
            arr,
            comparator,
            pivot: 0,
        }
    }
}

impl<'a, T, C: Comparator<T>> Sorter for ArrayIntroSorter<'a, T, C>
where
    T: Ord,
{
    fn compare(&self, i: i32, j: i32) -> i32 {
        self.comparator
            .compare(&self.arr[i as usize], &self.arr[j as usize])
    }

    fn swap(&mut self, i: i32, j: i32) {
        // The data pointed to by the pivot has been swapped.
        // We need to adjust the pivot value to ensure that
        // the value corresponding to the pivot remains unchanged.
        if self.pivot == j {
            self.pivot = i;
        }
        self.arr.swap(i as usize, j as usize);
    }

    fn set_pivot(&mut self, i: i32) {
        self.pivot = i;
    }

    fn compare_pivot(&self, i: i32) -> i32 {
        self.comparator
            .compare(&self.arr[self.pivot as usize], &self.arr[i as usize])
    }

    fn sort(&mut self, from: usize, to: usize) -> Result<(), RuntimeError> {
        IntroSorter::sort_range(self, from, to)?;
        Ok(())
    }
}

impl<'a, T, C: Comparator<T>> IntroSorter for ArrayIntroSorter<'a, T, C> where T: Ord {}
