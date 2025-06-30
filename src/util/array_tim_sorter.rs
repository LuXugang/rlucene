/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use crate::util::error::lucene_error::Result;
use crate::util::{Comparator, SliceCopyOps, Sorter, TimSorter, TimSorterBase};

/// A [`TimSorter`] for object arrays.
///
/// # Note
/// This is an internal API.
pub struct ArrayTimSorter<'a, T, C: Comparator<T>>
where
    T: Default + Clone + Ord,
{
    arr: &'a mut Vec<T>,
    tmp: Vec<T>,
    comparator: C,
    pivot_index: i32,
}
impl<'a, T, C: Comparator<T>> ArrayTimSorter<'a, T, C>
where
    T: Default + Clone + Ord,
{
    pub fn new(
        arr: &'a mut Vec<T>,
        comparator: C,
        max_temp_slots: i32,
    ) -> TimSorter<ArrayTimSorter<'a, T, C>> {
        let tmp = if max_temp_slots > 0 {
            vec![T::default(); max_temp_slots as usize]
        } else {
            vec![]
        };
        let sub = ArrayTimSorter {
            arr,
            tmp,
            comparator,
            pivot_index: 0,
        };
        TimSorter::new(max_temp_slots, sub)
    }
}
impl<T, C: Comparator<T>> Sorter for ArrayTimSorter<'_, T, C>
where
    T: Default + Clone + Ord,
{
    fn compare(&mut self, i: i32, j: i32) -> Result<i32> {
        self.comparator
            .compare(&self.arr[i as usize], &self.arr[j as usize])
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        self.arr.swap(i as usize, j as usize);
        Ok(())
    }

    fn set_pivot(&mut self, i: i32) -> Result<()> {
        self.pivot_index = i;
        Ok(())
    }

    fn compare_pivot(&mut self, j: i32) -> Result<i32> {
        self.compare(self.pivot_index, j)
    }
}
impl<T, C: Comparator<T>> TimSorterBase for ArrayTimSorter<'_, T, C>
where
    T: Default + Clone + Ord,
{
    fn copy(&mut self, src: i32, dest: i32) {
        self.arr[dest as usize] = self.arr[src as usize].clone();
    }

    fn save(&mut self, start: i32, len: i32) {
        self.tmp
            .copy_from(&self.arr[start as usize..start as usize + len as usize], 0);
    }

    fn restore(&mut self, src: i32, dest: i32) {
        // TODO: avoid clone
        self.arr[dest as usize] = self.tmp[src as usize].clone();
    }

    fn compare_saved(&self, i: i32, j: i32) -> i32 {
        self.comparator
            .compare_unchecked(&self.tmp[i as usize], &self.arr[j as usize])
    }
}
