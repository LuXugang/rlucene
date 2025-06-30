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
use crate::util::{check_range, sorter_util, Sorter};

pub struct InPlaceMergeSorter<S>
where
    S: Sorter,
{
    sub: S,
    pivot_index: i32,
}
impl<S> InPlaceMergeSorter<S>
where
    S: Sorter,
{
    pub fn new(sub: S) -> Self {
        InPlaceMergeSorter {
            sub,
            pivot_index: 0,
        }
    }
    fn merge_sort(&mut self, from: i32, to: i32) -> Result<()> {
        if to - from < sorter_util::BINARY_SORT_THRESHOLD {
            self.binary_sort(from, to)
        } else {
            let mid = (from + to) >> 1;
            self.merge_sort(from, mid)?;
            self.merge_sort(mid, to)?;
            self.merge_in_place(from, mid, to)
        }
    }
}
impl<S> Sorter for InPlaceMergeSorter<S>
where
    S: Sorter,
{
    fn compare(&mut self, i: i32, j: i32) -> Result<i32> {
        self.sub.compare(i, j)
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        self.sub.swap(i, j)
    }

    fn set_pivot(&mut self, i: i32) -> Result<()> {
        self.pivot_index = i;
        Ok(())
    }

    fn compare_pivot(&mut self, j: i32) -> Result<i32> {
        self.compare(self.pivot_index, j)
    }

    fn sort(&mut self, from: i32, to: i32) -> Result<()> {
        check_range(from, to)?;
        self.merge_sort(from, to)
    }
}
