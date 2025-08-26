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
use std::fmt::Display;

use crate::codecs::doc_values_consumer::DocValuesConsumer;
use crate::codecs::doc_values_producer::DocValuesProducer;
use crate::index::segment_info::SegmentInfo;
use crate::index::segment_read_state::SegmentReadState;
use crate::index::segment_write_state::SegmentWriteState;
use crate::store::directory::Directory;
use crate::store::{IndexInput, IndexOutput};
use crate::util::error::lucene_error::Result;

/// Encodes/decodes per-document values.
pub trait DocValuesFormat: Display {
    type DocValuesConsumer<T: IndexOutput>: DocValuesConsumer;
    /// Returns a [`DocValuesConsumer`] to write docvalues to the index.
    fn fields_consumer<D1, D2>(
        &self,
        state: &SegmentWriteState<D1>,
        segment_info: &SegmentInfo<D2>,
    ) -> Result<Self::DocValuesConsumer<D1::IndexOutput>>
    where
        D1: Directory,
        D2: Directory;

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
    fn fields_producer<D1, D2>(
        &self,
        state: &SegmentReadState<D1>,
        segment_info: &SegmentInfo<D2>,
    ) -> Result<Self::DocValuesProducer<D1::IndexInput>>
    where
        D1: Directory,
        D2: Directory;
}
