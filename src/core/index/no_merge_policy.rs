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
use crate::core::index::index_writer::Inner;
use crate::core::index::merge_policy::{
    MergeContext, MergePolicy, MergePolicyBase, MergeSpecificationNoReader,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

/// A [`MergePolicy`] which never returns any merges to execute.
/// Use this policy if you want to prevent segment merges entirely.
pub struct NoMergePolicy {
    base: MergePolicyBase,
}
impl NoMergePolicy {
    fn new() -> NoMergePolicy {
        Self {
            base: MergePolicyBase::default(),
        }
    }
}

impl Display for NoMergePolicy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoMergePolicy")
    }
}

impl MergePolicy for NoMergePolicy {
    fn get_base(&self) -> &MergePolicyBase {
        &self.base
    }

    fn get_base_mut(&mut self) -> &mut MergePolicyBase {
        &mut self.base
    }

    fn find_merges<D, MC>(
        &self,
        _merge_trigger: MergeTrigger,
        _segment_infos: &SegmentInfos<D>,
        _inner: Option<&Inner<D>>,
        _merge_context: &MC,
    ) -> Result<Option<MergeSpecificationNoReader<D>>>
    where
        D: Directory,
        MC: MergeContext<D>,
    {
        Ok(None)
    }

    fn find_forced_merges<D, MC>(
        &self,
        _segment_infos: &SegmentInfos<D>,
        _max_segment_count: i32,
        _segments_to_merge: &HashMap<String, Option<bool>>,
        _inner: Option<&Inner<D>>,
        _merge_context: &MC,
    ) -> Result<Option<MergeSpecificationNoReader<D>>>
    where
        D: Directory,
        MC: MergeContext<D>,
    {
        Ok(None)
    }

    fn find_forced_deletes_merges<D, MC>(
        &self,
        _segment_infos: &SegmentInfos<D>,
        _inner: Option<&Inner<D>>,
        _merge_context: &MC,
    ) -> Result<Option<MergeSpecificationNoReader<D>>>
    where
        MC: MergeContext<D>,
        D: Directory,
    {
        Ok(None)
    }

    fn use_compound_file<D, MC>(
        &self,
        _infos: &SegmentInfos<D>,
        new_segment: &SegmentCommitInfo<D>,
        _merge_context: &MC,
    ) -> Result<bool>
    where
        D: Directory,
        MC: MergeContext<D>,
    {
        Ok(new_segment.info.get_use_compound_file())
    }

    fn size<D, MC>(&self, _info: &SegmentCommitInfo<D>, _merge_context: &MC) -> Result<i64>
    where
        D: Directory,
        MC: MergeContext<D>,
    {
        Ok(i64::MAX)
    }
}
