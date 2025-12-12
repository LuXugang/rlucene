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
use crate::core::index::merge_policy::MergeContext;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::store::directory::Directory;
use crate::core::util::info_stream::InfoStreamMT;
use std::collections::{HashMap, HashSet};
/// A wrapper around the IndexWriter `MergeContext`.
///
/// Attempts to cache the result of [`MergeContext::num_deletes_to_merge`], in order to avoid
/// duplicate calculations during the merge phase.
///
/// This helps prevent repeated computation of delete counts for the same
/// `SegmentCommitInfo`.
pub(crate) struct CachingMergeContext<T>
where
    T: MergeContext,
{
    merge_context: T,
    cached_num_deletes_to_merge: HashMap<String, i32>,
}
impl<T> CachingMergeContext<T>
where
    T: MergeContext,
{
    pub fn new(merge_context: T) -> Self {
        CachingMergeContext {
            merge_context,
            cached_num_deletes_to_merge: HashMap::new(),
        }
    }
}
impl<T> MergeContext for CachingMergeContext<T>
where
    T: MergeContext,
{
    fn num_deletes_to_merge<D>(
        &mut self,
        info: &SegmentCommitInfo<D>,
    ) -> crate::core::util::error::lucene_error::Result<i32>
    where
        D: Directory,
    {
        let key = info.info.get_id_str();
        if let Some(v) = self.cached_num_deletes_to_merge.get(&key) {
            return Ok(*v);
        }
        let v = self.merge_context.num_deletes_to_merge(info)?;
        self.cached_num_deletes_to_merge.insert(key, v);
        Ok(v)
    }

    fn num_deleted_docs<D>(&self, info: &SegmentCommitInfo<D>) -> i32
    where
        D: Directory,
    {
        self.merge_context.num_deleted_docs(info)
    }

    fn get_info_stream(&self) -> InfoStreamMT {
        self.merge_context.get_info_stream()
    }

    fn get_merging_segments(&self) -> &HashSet<String> {
        self.merge_context.get_merging_segments()
    }
}
