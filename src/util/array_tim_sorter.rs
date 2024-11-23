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
use crate::util::tim_sorter_base::TimSorterBase;
use crate::util::{Comparator, Sorter};

struct ArrayTimSorter<'a, T, C: Comparator<T>>
where
    T: Default + Clone,
{
    arr: &'a mut Vec<T>,
    tmp: Vec<T>,
    comparator: C,
    max_temp_slots: usize,
    pivot_index: usize,
}
impl<'a, T, C: Comparator<T>> ArrayTimSorter<'a, T, C>
where
    T: Default + Clone,
{
    fn new(arr: &'a mut Vec<T>, comparator: C, max_temp_slots: usize) -> ArrayTimSorter<T, C> {
        let tmp = if max_temp_slots > 0 {
            vec![T::default(); max_temp_slots]
        } else {
            vec![]
        };
        ArrayTimSorter {
            arr,
            tmp,
            comparator,
            max_temp_slots,
            pivot_index: 0,
        }
    }
}
impl<'a, T, C: Comparator<T>> Sorter for ArrayTimSorter<'a, T, C>
where
    T: Default + Clone,
{
    fn compare(&self, i: usize, j: usize) -> i32 {
        self.comparator.compare(&self.arr[i], &self.arr[j])
    }

    fn swap(&mut self, i: usize, j: usize) {
        self.arr.swap(i, j);
    }

    fn set_pivot(&mut self, i: usize) {
        unimplemented!()
    }

    fn compare_pivot(&self, i: usize) -> i32 {
        unimplemented!()
    }

    fn sort(&mut self, from: usize, to: usize) -> Result<(), String> {
        unimplemented!()
    }
}
impl<'a, T, C: Comparator<T>> TimSorterBase for ArrayTimSorter<'a, T, C>
where
    T: Default + Clone,
{
    fn copy(&mut self, src: usize, dest: usize) {
        self.tmp[dest] = self.arr[src].clone();
    }

    fn save(&mut self, start: usize, len: usize) {
        let tmp_len = self.tmp.len();
        if len > tmp_len {
            self.tmp.resize(len, T::default());
        }
        self.tmp[..len].clone_from_slice(&self.arr[start..start + len]);
    }

    fn restore(&mut self, src: usize, dest: usize) {
        self.arr[src] = self.tmp[dest].clone();
    }

    fn compare_saved(&self, i: usize, j: usize) -> i32 {
        self.comparator.compare(&self.tmp[i], &self.tmp[j])
    }
}
