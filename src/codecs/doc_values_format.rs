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
use crate::codecs::doc_values_consumer::DocValuesConsumerEnum;
use crate::codecs::doc_values_producer::DocValuesProducerEnum;
use crate::index::segment_read_state::SegmentReadState;
use crate::index::segment_write_state::SegmentWriteState;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
use std::fmt::Display;
/// Encodes/decodes per-document values.
pub trait DocValuesFormat: Display {
    /// Returns a [`DocValuesConsumer`](crate::codecs::doc_values_consumer::DocValuesConsumer) to write docvalues to the index.
    fn fields_consumer<D>(
        &self,
        state: &SegmentWriteState<D>,
    ) -> Result<DocValuesConsumerEnum<D::IndexOutputType>>
    where
        D: Directory;
    /// Returns a [`DocValuesProducer`](crate::codecs::doc_values_producer::DocValuesProducer) to read docvalues from the index.
    ///
    /// NOTE: By the time this call returns, it must hold open any files it will need to use;
    /// otherwise, those files may be deleted. Additionally, required files may be deleted during
    /// the execution of this call before there is a chance to open them. Under these circumstances,
    /// an [`IOException`] should be returned by the implementation. IOExceptions are expected and
    /// will automatically cause a retry of the segment opening logic with the newly revised segments.
    fn fields_producer<D>(
        &self,
        state: &SegmentReadState<D>,
    ) -> Result<DocValuesProducerEnum<D::IndexInputType>>
    where
        D: Directory;
}
