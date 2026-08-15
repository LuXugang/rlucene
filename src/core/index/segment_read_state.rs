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
use crate::core::index::field_infos::FieldInfos;
use crate::core::store::IOContext;
use std::sync::Arc;

/// Holder struct for common parameters used during read.
pub struct SegmentReadState<'a, D> {
  /// Directory where this segment is read from.
  pub directory: &'a D,

  /// FieldInfos describing all fields in this segment.
  pub field_infos: Arc<FieldInfos>,

  /// IOContext to pass to Directory::open_input.
  pub context: &'a IOContext,

  /// Unique suffix for any postings files read for this segment.
  pub segment_suffix: String,
}

impl<'a, D> SegmentReadState<'a, D> {
  /// Creates a SegmentReadState with an empty segment suffix.
  pub fn new(directory: &'a D, field_infos: Arc<FieldInfos>, context: &'a IOContext) -> Self {
    Self::with_suffix(directory, field_infos, context, "")
  }

  /// Creates a SegmentReadState with a custom segment suffix.
  pub fn with_suffix(
    directory: &'a D,
    field_infos: Arc<FieldInfos>,
    context: &'a IOContext,
    segment_suffix: &str,
  ) -> Self {
    Self {
      directory,
      field_infos,
      context,
      segment_suffix: segment_suffix.to_string(),
    }
  }

  /// Creates a copy of an existing SegmentReadState with a different segment
  /// suffix.
  pub fn copy_with_suffix(other: &SegmentReadState<'a, D>, segment_suffix: &str) -> Self {
    Self {
      directory: other.directory,
      field_infos: other.field_infos.clone(),
      context: other.context,
      segment_suffix: segment_suffix.to_string(),
    }
  }
}
