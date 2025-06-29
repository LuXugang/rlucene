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
use crate::codecs::term_vectors_reader::TermVectorsReaderEnum;
use crate::codecs::term_vectors_writer::TermVectorsWriterEnum;
use crate::index::field_infos::FieldInfos;
use crate::index::segment_info::SegmentInfo;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::rc::Rc;
use std::sync::Arc;

/// Controls the format of term vectors
pub trait TermVectorsFormat {
    /// Returns a [`TermVectorsReader`](crate::codecs::term_vectors_reader::TermVectorsReader) to read term vectors.
    fn vectors_reader<D1, D2>(
        &self,
        directory: &mut D1,
        segment_info: Rc<SegmentInfo<D2>>,
        field_infos: Rc<FieldInfos>,
        context: &IOContext,
    ) -> Result<TermVectorsReaderEnum<D1::IndexInputType>>
    where
        D1: Directory,
        D2: Directory;
    /// Returns a [`TermVectorsWriter`](crate::codecs::term_vectors_writer::TermVectorsWriter) to write term vectors.
    fn vectors_writer<D1, D2>(
        &self,
        directory: Arc<Mutex<D1>>,
        segment_info: Rc<SegmentInfo<D2>>,
        context: &IOContext,
    ) -> Result<TermVectorsWriterEnum<D1>>
    where
        D1: Directory,
        D2: Directory;
}
