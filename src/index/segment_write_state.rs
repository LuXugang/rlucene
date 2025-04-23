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
use std::sync::Arc;

use parking_lot::Mutex;

use crate::index::buffered_updates::STBufferedUpdates;
use crate::index::field_infos::FieldInfos;
use crate::index::segment_info::SegmentInfo;
use crate::search::dummy::dummy_query::DummyQuery;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::fixed_bit_set::FixedBitSet;
use crate::util::info_stream::InfoStreamLock;

/// Holder struct for common parameters used during write.
///
/// @lucene.experimental
pub struct SegmentWriteState<D>
where
    D: Directory,
{
    /// InfoStream used for debugging messages.
    pub info_stream: InfoStreamLock,

    /// Directory where this segment will be written to.
    pub directory: Arc<Mutex<D>>,

    /// SegmentInfo describing this segment.
    pub segment_info: Rc<SegmentInfo<D>>,

    /// FieldInfos describing all fields in this segment.
    pub field_infos: Rc<FieldInfos>,

    /// Number of deleted documents set while flushing the segment.
    pub del_count_on_flush: i32,

    /// Number of only soft deleted documents set while flushing the segment.
    pub soft_del_count_on_flush: i32,

    /// Deletes and updates to apply while we are flushing the segment.
    /// A Term is enrolled here if it was deleted/updated at one point,
    /// and it's mapped to the docIDUpto, meaning any docID < docIDUpto
    /// containing this term should be deleted/updated.
    pub(crate) seg_updates: Option<Rc<STBufferedUpdates<DummyQuery>>>,

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
    pub context: Rc<IOContext>,
}
#[allow(unused)]
impl<D> SegmentWriteState<D>
where
    D: Directory,
{
    /// Constructor without suffix.
    pub(crate) fn new(
        info_stream: InfoStreamLock,
        directory: Arc<Mutex<D>>,
        segment_info: Rc<SegmentInfo<D>>,
        field_infos: Rc<FieldInfos>,
        seg_updates: Option<STBufferedUpdates<DummyQuery>>,
        context: Rc<IOContext>,
    ) -> Self {
        Self::with_suffix(
            info_stream,
            directory,
            segment_info,
            field_infos,
            seg_updates,
            context,
            "",
        )
    }

    /// Constructor with segment suffix.
    pub(crate) fn with_suffix(
        info_stream: InfoStreamLock,
        directory: Arc<Mutex<D>>,
        segment_info: Rc<SegmentInfo<D>>,
        field_infos: Rc<FieldInfos>,
        seg_updates: Option<STBufferedUpdates<DummyQuery>>,
        context: Rc<IOContext>,
        segment_suffix: &str,
    ) -> Self {
        let seg_updates = seg_updates.map(Rc::new);
        debug_assert!(Self::assert_segment_suffix(segment_suffix));
        Self {
            info_stream,
            directory,
            segment_info,
            field_infos,
            seg_updates,
            context,
            segment_suffix: segment_suffix.to_string(),
            del_count_on_flush: 0,
            soft_del_count_on_flush: 0,
            live_docs: None,
        }
    }

    /// Create a shallow copy of SegmentWriteState with a new segment suffix.
    pub fn copy_with_suffix(state: &SegmentWriteState<D>, segment_suffix: String) -> Self {
        Self {
            info_stream: state.info_stream.clone(),
            directory: Arc::clone(&state.directory),
            segment_info: Rc::clone(&state.segment_info),
            field_infos: Rc::clone(&state.field_infos),
            seg_updates: state.seg_updates.clone(),
            context: Rc::clone(&state.context),
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
