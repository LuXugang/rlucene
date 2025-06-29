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
use crate::codecs::norms_consumer::NormsConsumerEnum;
use crate::codecs::norms_producer::NormsProducerEnum;
use crate::index::segment_read_state::SegmentReadState;
use crate::index::segment_write_state::SegmentWriteState;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;

/// Encodes/decodes per-document score normalization values.
pub trait NormsFormat {
    /// Returns a [`NormsConsumer`](crate::codecs::norms_consumer::NormsConsumer) to write norms to the index.
    ///
    /// # Arguments
    /// * `state` - The write state containing segment info, directory, etc.
    fn norms_consumer<D>(
        &self,
        state: &SegmentWriteState<D>,
    ) -> Result<NormsConsumerEnum<D::IndexOutputType>>
    where
        D: Directory;

    /// Returns a [`NormsProducer`](crate::codecs::norms_producer::NormsProducer) to read norms from the index.
    ///
    /// # Notes
    /// - By the time this call returns, it **must hold open** any files it will
    ///   need to use. Otherwise, those files may be deleted by the time they
    ///   are accessed.
    ///
    /// - Additionally, required files might be deleted **during the execution**
    ///   of this call, before there's a chance to open them. In such cases,
    ///   implementations **must return an error**.
    ///
    /// - I/O errors are expected and will automatically trigger a retry of
    ///   segment opening logic using the newly revised segments.
    fn norms_producer<D>(
        &self,
        state: &SegmentReadState<D>,
    ) -> Result<NormsProducerEnum<D::IndexInputType>>
    where
        D: Directory;
}
