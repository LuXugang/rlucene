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
use crate::codecs::fields_consumer::FieldsConsumerEnum;
use crate::codecs::fields_producer::FieldsProducerEnum;
use crate::index::segment_read_state::SegmentReadState;
use crate::index::segment_write_state::SegmentWriteState;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
/// Encodes/decodes terms, postings, and proximity data.
pub trait PostingsFormat {
    /// Writes a new segment
    fn fields_consumer<D: Directory>(
        &self,
        state: &SegmentWriteState<D>,
    ) -> Result<FieldsConsumerEnum<D::IndexOutputType>>;
    /// Reads a segment. **NOTE**: by the time this call returns, it must hold open any files it will need
    /// to use; else, those files may be deleted. Additionally, required files may be deleted during
    /// the execution of this call before there is a chance to open them. Under these circumstances an
    /// `IOException` should be returned by the implementation. IO exceptions are expected and will
    /// automatically cause a retry of the segment opening logic with the newly revised segments.
    fn fields_producer<D: Directory>(
        &self,
        state: &SegmentReadState<D>,
    ) -> Result<FieldsProducerEnum<D::IndexInputType>>;
}
