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
use std::fmt::Display;

use crate::codecs::doc_values_consumer::DocValuesConsumer;
use crate::codecs::doc_values_producer::DocValuesProducer;
use crate::index::segment_read_state::SegmentReadState;
use crate::index::segment_write_state::SegmentWriteState;
use crate::store::directory::Directory;
use crate::store::{IndexInput, IndexOutput};
use crate::util::error::lucene_error::Result;

/// Encodes/decodes per-document values.
pub trait DocValuesFormat: Display {
    type DocValuesConsumer<T: IndexOutput>: DocValuesConsumer;
    /// Returns a [`DocValuesConsumer`] to write docvalues to the index.
    fn fields_consumer<D>(
        &self,
        state: &SegmentWriteState<D>,
    ) -> Result<Self::DocValuesConsumer<D::IndexOutputType>>
    where
        D: Directory;

    type DocValuesProducer<T: IndexInput>: DocValuesProducer;
    /// Returns a [`DocValuesProducer`] to read docvalues from the index.
    ///
    /// NOTE: By the time this call returns, it must hold open any files it will
    /// need to use; otherwise, those files may be deleted. Additionally,
    /// required files may be deleted during the execution of this call
    /// before there is a chance to open them. Under these circumstances, an
    /// io error should be returned by the implementation. IOExceptions are
    /// expected and will automatically cause a retry of the segment opening
    /// logic with the newly revised segments.
    fn fields_producer<D>(
        &self,
        state: &SegmentReadState<D>,
    ) -> Result<Self::DocValuesProducer<D::IndexInputType>>
    where
        D: Directory;
}
