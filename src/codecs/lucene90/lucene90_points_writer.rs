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
use crate::codecs::points_reader::PointsReader;
use crate::codecs::points_writer::PointsWriter;
use crate::index::field_info::FieldInfo;
use crate::index::segment_write_state::SegmentWriteState;
use crate::store::directory::Directory;
use std::rc::Rc;

pub struct Lucene90PointWriter;

impl Lucene90PointWriter {
    pub fn new<D>(state: &SegmentWriteState<D>) -> Self
    where
        D: Directory,
    {
        todo!()
    }
}

impl PointsWriter for Lucene90PointWriter {
    fn write_field<PR>(
        &mut self,
        _field_info: &Rc<FieldInfo>,
        _values: &mut PR,
    ) -> crate::util::error::lucene_error::Result<()>
    where
        PR: PointsReader,
    {
        todo!()
    }

    fn finish(&mut self) -> crate::util::error::lucene_error::Result<()> {
        todo!()
    }
}
