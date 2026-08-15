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
use std::sync::Arc;

use crate::core::codecs::doc_values_consumer::DocValuesConsumer;
use crate::core::codecs::doc_values_producer::DocValuesProducer;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::Result;

/// Encodes/decodes per-document values.
pub trait DocValuesFormat: Display + HasIdentity {
  /// Returns this doc values format's name.
  fn get_name(&self) -> &str;

  type DocValuesConsumer<T: IndexOutput>: DocValuesConsumer<IndexOutput = T>;
  /// Returns a [`DocValuesConsumer`] to write docvalues to the index.
  fn fields_consumer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::DocValuesConsumer<D1::IndexOutput>>
  where
    D1: Directory;

  type DocValuesProducer<T: IndexInput>: DocValuesProducer;
  /// Returns a [`DocValuesProducer`] to read docvalues from the index.
  ///
  /// NOTE: By the time this call returns, it must hold open any files it will
  /// need to use; otherwise, those files may be deleted. Additionally,
  /// required files may be deleted during the execution of this call
  /// before there is a chance to open them. Under these circumstances, an
  /// I/O error should be returned by the implementation. I/O errors are
  /// expected and will automatically cause a retry of the segment opening
  /// logic with the newly revised segments.
  fn fields_producer<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::DocValuesProducer<D1::IndexInput>>
  where
    D1: Directory;

  /// Looks up a format by name.
  fn for_name(name: &str) -> Result<Arc<Self>>
  where
    Self: Sized;
}
