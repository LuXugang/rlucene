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

use crate::util::bkd::point_reader::PointReader;
use crate::util::bkd::point_value::{PointValue, PointValueEnum};
use crate::util::error::lucene_error::Result;

pub struct HeapPointReader {
    points: Option<PointValueEnum>,
    cur_read: i32,
    end: i32,
    bytes_per_doc: i32,
}

impl HeapPointReader {
    pub fn new(
        get_slice: Option<PointValueEnum>,
        start: i32,
        end: i32,
        bytes_per_doc: i32,
    ) -> Self {
        HeapPointReader {
            points: get_slice,
            cur_read: start - 1,
            end,
            bytes_per_doc,
        }
    }
    pub fn remove_points(&mut self) -> Option<PointValueEnum> {
        self.points.take()
    }
}
impl PointReader for HeapPointReader {
    fn next(&mut self) -> Result<bool> {
        self.cur_read += 1;
        Ok(self.cur_read < self.end)
    }

    fn point_value(&mut self) -> &PointValueEnum {
        debug_assert!(self.points.is_some());
        self.points
            .as_mut()
            .unwrap()
            .set_offset(self.bytes_per_doc * self.cur_read);
        self.points.as_ref().unwrap()
    }
}
