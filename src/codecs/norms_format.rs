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
use crate::codecs::norms_consumer::NormsConsumerEnum;
use crate::codecs::norms_producer::NormsProducerEnum;
use crate::index::segment_info::SegmentInfo;
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
    fn norms_consumer<D, D1>(
        &self,
        state: &SegmentWriteState<D>,
        segment_info: &SegmentInfo<D1>,
    ) -> Result<NormsConsumerEnum<D::IndexOutput>>
    where
        D: Directory,
        D1: Directory;

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
    fn norms_producer<D, D1>(
        &self,
        state: &SegmentReadState<D>,
        segment_info: &SegmentInfo<D1>,
    ) -> Result<NormsProducerEnum<D::IndexInput>>
    where
        D: Directory,
        D1: Directory;
}
