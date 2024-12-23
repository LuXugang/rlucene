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
use crate::util::{Comparator, Sorter, TimSorterBase};

/// A [`TimSorter`](crate::util::TimSorter) for object arrays.
///
/// # Note
/// This is an internal API.
pub struct ArrayTimSorter<'a, T, C: Comparator<T>>
where
    T: Default + Clone,
{
    arr: &'a mut Vec<T>,
    tmp: Vec<T>,
    comparator: C,
    pivot_index: i32,
}
impl<'a, T, C: Comparator<T>> ArrayTimSorter<'a, T, C>
where
    T: Default + Clone,
{
    pub fn new(
        arr: &'a mut Vec<T>,
        comparator: C,
        max_temp_slots: i32,
    ) -> ArrayTimSorter<'a, T, C> {
        let tmp = if max_temp_slots > 0 {
            vec![T::default(); max_temp_slots as usize]
        } else {
            vec![]
        };
        ArrayTimSorter {
            arr,
            tmp,
            comparator,
            pivot_index: 0,
        }
    }
}
impl<T, C: Comparator<T>> Sorter for ArrayTimSorter<'_, T, C>
where
    T: Default + Clone,
{
    fn compare(&mut self, i: i32, j: i32) -> Result<i32, RuntimeError> {
        Ok(self
            .comparator
            .compare(&self.arr[i as usize], &self.arr[j as usize]))
    }

    fn swap(&mut self, i: i32, j: i32) {
        self.arr.swap(i as usize, j as usize);
    }

    fn set_pivot(&mut self, i: i32) -> Result<(), RuntimeError> {
        self.pivot_index = i;
        Ok(())
    }

    fn compare_pivot(&mut self, j: i32) -> Result<i32, RuntimeError> {
        self.compare(self.pivot_index, j)
    }
}
impl<T, C: Comparator<T>> TimSorterBase for ArrayTimSorter<'_, T, C>
where
    T: Default + Clone + PartialEq,
{
    fn copy(&mut self, src: i32, dest: i32) {
        self.arr[dest as usize] = self.arr[src as usize].clone();
    }

    fn save(&mut self, start: i32, len: i32) {
        let tmp_len = self.tmp.len();
        if len > tmp_len as i32 {
            self.tmp.resize(len as usize, T::default());
        }
        // TODO: avoid using clone
        self.tmp[0..len as usize]
            .clone_from_slice(&self.arr[start as usize..start as usize + len as usize]);
    }

    fn restore(&mut self, src: i32, dest: i32) {
        // TODO: avoid using clone
        self.arr[dest as usize] = self.tmp[src as usize].clone();
    }

    fn compare_saved(&self, i: i32, j: i32) -> i32 {
        self.comparator
            .compare(&self.tmp[i as usize], &self.arr[j as usize])
    }
}
