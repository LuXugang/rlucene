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
use crate::core::index::merge_policy::MergeContext;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::store::directory::Directory;
use crate::core::util::info_stream::InfoStreamMT;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

/// A wrapper around the IndexWriter `MergeContext`.
///
/// Attempts to cache the result of [`MergeContext::num_deletes_to_merge`], in order to avoid
/// duplicate calculations during the merge phase.
///
/// This helps prevent repeated computation of delete counts for the same
/// `SegmentCommitInfo`.
pub(crate) struct CachingMergeContext<'a, T, D>
where
  T: MergeContext<D>,
  D: Directory,
{
  merge_context: &'a T,
  cached_num_deletes_to_merge: Mutex<HashMap<String, i32>>,
  _mark: PhantomData<D>,
}
impl<'a, T, D> CachingMergeContext<'a, T, D>
where
  T: MergeContext<D>,
  D: Directory,
{
  pub fn new(merge_context: &'a T) -> Self {
    CachingMergeContext {
      merge_context,
      cached_num_deletes_to_merge: Mutex::new(HashMap::new()),
      _mark: PhantomData,
    }
  }
}
impl<T, D> MergeContext<D> for CachingMergeContext<'_, T, D>
where
  T: MergeContext<D>,
  D: Directory,
{
  fn num_deletes_to_merge(
    &self,
    info: &SegmentCommitInfo<D>,
  ) -> crate::core::util::error::lucene_error::Result<i32> {
    let key = info.info.get_id_key();
    if let Some(v) = self.cached_num_deletes_to_merge.lock().get(key) {
      return Ok(*v);
    }
    let v = self.merge_context.num_deletes_to_merge(info)?;
    self
      .cached_num_deletes_to_merge
      .lock()
      .insert(key.to_string(), v);
    Ok(v)
  }

  fn num_deleted_docs(&self, info: &SegmentCommitInfo<D>) -> i32 {
    self.merge_context.num_deleted_docs(info)
  }

  fn get_info_stream(&self) -> InfoStreamMT {
    self.merge_context.get_info_stream()
  }

  fn get_merging_segments(&self, inner: Option<&Inner<D>>) -> HashSet<String> {
    self.merge_context.get_merging_segments(inner)
  }
}

#[cfg(test)]
mod tests {
  use crate::core::index::caching_merge_context::CachingMergeContext;
  use crate::core::index::index_writer::Inner;
  use crate::core::index::merge_policy::MergeContext;
  use crate::core::index::segment_commit_info::SegmentCommitInfo;
  use crate::core::index::segment_info::SegmentInfo;
  use crate::core::store::directory::Directory;
  use crate::core::util::StringHelper;
  use crate::core::util::error::lucene_error::Result;
  use crate::core::util::info_stream::InfoStreamMT;
  use std::collections::HashSet;
  use std::sync::atomic::{AtomicI32, Ordering};

  #[allow(dead_code)]
  struct TestCachingMergeContext;

  #[test]
  fn test_num_deletes_to_merge() -> Result<()> {
    // mock merge context
    let merge_context = MockMergeContext::new();
    let caching_merge_context = CachingMergeContext::new(&merge_context);
    let dummy_commit_info = SegmentCommitInfo::new(
      SegmentInfo::default(),
      0,
      0,
      -1,
      -1,
      -1,
      Some(StringHelper::random_id()),
    )?;
    let id = dummy_commit_info.info.get_id_key().to_string();
    let v = caching_merge_context.num_deletes_to_merge(&dummy_commit_info)?;
    assert_eq!(v, 1);
    {
      let cache = caching_merge_context.cached_num_deletes_to_merge.lock();
      assert_eq!(cache.len(), 1);
      let key = id;
      assert_eq!(cache.get(&key), Some(&1));
    }

    assert_eq!(
      merge_context
        .count
        .load(std::sync::atomic::Ordering::SeqCst),
      1
    );

    merge_context.num_deletes_to_merge(&dummy_commit_info)?;
    assert_eq!(
      merge_context
        .count
        .load(std::sync::atomic::Ordering::SeqCst),
      2
    );

    let v2 = caching_merge_context.num_deletes_to_merge(&dummy_commit_info)?;
    assert_eq!(v2, 1);

    {
      let cache = caching_merge_context.cached_num_deletes_to_merge.lock();
      assert_eq!(cache.len(), 1);
      let key = dummy_commit_info.info.get_id_key();
      assert_eq!(cache.get(key), Some(&1));
    }

    Ok(())
  }

  struct MockMergeContext {
    count: AtomicI32,
  }
  impl MockMergeContext {
    fn new() -> Self {
      Self {
        count: AtomicI32::new(0),
      }
    }
  }
  impl<D> MergeContext<D> for MockMergeContext
  where
    D: Directory,
  {
    fn num_deletes_to_merge(&self, _info: &SegmentCommitInfo<D>) -> Result<i32> {
      let v = self.count.fetch_add(1, Ordering::SeqCst) + 1;
      Ok(v)
    }

    fn num_deleted_docs(&self, _info: &SegmentCommitInfo<D>) -> i32 {
      0
    }

    fn get_info_stream(&self) -> InfoStreamMT {
      unreachable!()
    }

    fn get_merging_segments(&self, _inner: Option<&Inner<D>>) -> HashSet<String> {
      unreachable!()
    }
  }
}
