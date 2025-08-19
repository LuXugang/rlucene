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
use std::rc::Rc;

use crate::index::field_infos::FieldInfos;
use crate::store::IOContext;
use crate::store::directory::Directory;

/// Holder struct for common parameters used during read.
///
/// @lucene.experimental
pub struct SegmentReadState<'a, D>
where
    D: Directory,
{
    /// Directory where this segment is read from.
    pub directory: &'a mut D,

    /// FieldInfos describing all fields in this segment.
    pub field_infos: Rc<FieldInfos>,

    /// IOContext to pass to Directory::open_input.
    pub context: Rc<IOContext>,

    /// Unique suffix for any postings files read for this segment.
    pub segment_suffix: String,
}

impl<'a, D> SegmentReadState<'a, D>
where
    D: Directory,
{
    /// Creates a SegmentReadState with an empty segment suffix.
    pub fn new(directory: &'a mut D, field_infos: Rc<FieldInfos>, context: Rc<IOContext>) -> Self {
        Self::with_suffix(directory, field_infos, context, "")
    }

    /// Creates a SegmentReadState with a custom segment suffix.
    pub fn with_suffix(
        directory: &'a mut D,
        field_infos: Rc<FieldInfos>,
        context: Rc<IOContext>,
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
    pub fn copy_with_suffix(other: &'a mut SegmentReadState<'_, D>, segment_suffix: &str) -> Self {
        Self {
            directory: other.directory,
            field_infos: other.field_infos.clone(),
            context: other.context.clone(),
            segment_suffix: segment_suffix.to_string(),
        }
    }
}
