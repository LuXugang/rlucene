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
use crate::core::store::directory::Directory;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::info_stream::InfoStreamMT;
use std::sync::Arc;

/// Holder struct for common parameters used during write.
///
/// @lucene.experimental
pub struct SegmentWriteState<'a, D>
where
    D: Directory,
{
    /// InfoStream used for debugging messages.
    pub info_stream: Option<InfoStreamMT>,

    /// Directory where this segment will be written to.
    pub directory: &'a D,

    /// FieldInfos describing all fields in this segment.
    pub field_infos: Arc<FieldInfos>,

    /// Number of deleted documents set while flushing the segment.
    pub del_count_on_flush: i32,

    /// Number of only soft deleted documents set while flushing the segment.
    pub soft_del_count_on_flush: i32,

    /// FixedBitSet recording live documents; this is only set if there
    /// is one or more deleted documents.
    pub live_docs: Option<FixedBitSet>,

    /// Unique suffix for any postings files written for this segment.
    /// PerFieldPostingsFormat sets this for each of the postings formats it
    /// wraps. If you create a new PostingsFormat, then any files you
    /// write/read must be derived using this suffix (use
    /// IndexFileNames::segment_file_name).
    ///
    /// Note: the suffix must be either empty, or be a textual suffix
    /// containing exactly two parts (separated by underscore), or be a
    /// base36 generation.
    pub segment_suffix: String,

    /// IOContext for all writes; you should pass this to
    /// Directory::create_output.
    pub context: &'a IOContext,
}

impl<'a, D> SegmentWriteState<'a, D>
where
    D: Directory,
{
    /// Constructor without suffix.
    pub(crate) fn new(
        info_stream: Option<InfoStreamMT>,
        directory: &'a D,
        field_infos: Arc<FieldInfos>,
        context: &'a IOContext,
    ) -> Self {
        Self::with_suffix(info_stream, directory, field_infos, context, "")
    }

    /// Constructor with segment suffix.
    pub(crate) fn with_suffix(
        info_stream: Option<InfoStreamMT>,
        directory: &'a D,
        field_infos: Arc<FieldInfos>,
        context: &'a IOContext,
        segment_suffix: &str,
    ) -> Self {
        debug_assert!(Self::assert_segment_suffix(segment_suffix));
        Self {
            info_stream,
            directory,
            field_infos,
            context,
            segment_suffix: segment_suffix.to_string(),
            del_count_on_flush: 0,
            soft_del_count_on_flush: 0,
            live_docs: None,
        }
    }

    /// Create a shallow copy of SegmentWriteState with a new segment suffix.
    pub fn copy_with_suffix(
        state: &'a mut SegmentWriteState<'a, D>,
        segment_suffix: String,
    ) -> Self {
        Self {
            info_stream: state.info_stream.clone(),
            directory: state.directory,
            field_infos: Arc::clone(&state.field_infos),
            context: state.context,
            segment_suffix,
            del_count_on_flush: state.del_count_on_flush,
            soft_del_count_on_flush: state.soft_del_count_on_flush,
            live_docs: state.live_docs.clone(),
        }
    }
    // currently only used by assert? clean up and make real check?
    // either it's a segment suffix (_X_Y) or it's a parsable generation
    // TODO: this is very confusing how ReadersAndUpdates passes generations via
    // this mechanism, maybe add 'generation' explicitly to ctor create the
    // 'actual suffix' here?
    fn assert_segment_suffix(suffix: &str) -> bool {
        if suffix.is_empty() {
            return true;
        }

        let parts: Vec<&str> = suffix.split('_').collect();
        if parts.len() == 2 {
            return true;
        } else if parts.len() == 1 {
            return i64::from_str_radix(parts[0], 36).is_ok();
        }
        false
    }
}
