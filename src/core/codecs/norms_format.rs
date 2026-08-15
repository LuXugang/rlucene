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
use crate::core::codecs::norms_consumer::NormsConsumer;
use crate::core::codecs::norms_producer::NormsProducer;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::error::lucene_error::Result;

/// Encodes/decodes per-document score normalization values.
pub trait NormsFormat {
  type NormsConsumer<T: IndexOutput>: NormsConsumer;
  /// Returns a [`NormsConsumer`] to write norms to the index.
  ///
  /// # Arguments
  /// * `state` - The write state containing segment info, directory, etc.
  fn norms_consumer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::NormsConsumer<D1::IndexOutput>>
  where
    D1: Directory;

  type NormsProducer<T: IndexInput>: NormsProducer;
  /// Returns a [`NormsProducer`] to read norms from the index.
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
  fn norms_producer<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::NormsProducer<D1::IndexInput>>
  where
    D1: Directory;
}
