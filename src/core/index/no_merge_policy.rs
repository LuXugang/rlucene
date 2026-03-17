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
impl Default for NoMergePolicy {
  fn default() -> Self {
    Self::new()
  }
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
    _max_segment_count: usize,
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

#[cfg(test)]
mod tests {
  use crate::core::index::codec_reader::CodecReader;
  use crate::core::index::dummy::dummy_merge_context::DummyMergeContext;
  use crate::core::index::index_writer::{SOURCE, SOURCE_FLUSH};
  use crate::core::index::merge_policy::{MergePolicy, MergeSpecification};
  use crate::core::index::merge_trigger::MergeTrigger;
  use crate::core::index::no_merge_policy::NoMergePolicy;
  use crate::core::index::segment_infos::SegmentInfos;
  use crate::core::store::directory::Directory;
  use crate::core::store::dummy::dummy_directory::DummyDirectory;
  use crate::core::util::LATEST;
  use crate::core::util::error::lucene_error::{LuceneError, Result};
  use crate::test::core::index::base_merge_policy_test_case::{
    BaseMergePolicyTestCase, FakeDirectory,
  };
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
  use std::collections::HashMap;
  use std::sync::Arc;

  struct TestNoMergePolicy;
  impl BaseMergePolicyTestCase for TestNoMergePolicy {
    type MergePolicy = NoMergePolicy;

    fn merge_policy(&self) -> Self::MergePolicy {
      NoMergePolicy::default()
    }

    fn assert_segment_infos<D>(_policy: &Self::MergePolicy, infos: &SegmentInfos<D>) -> Result<()>
    where
      D: Directory,
    {
      for info in infos.segments.iter() {
        assert_eq!(SOURCE_FLUSH, info.info.get_attribute(SOURCE).unwrap());
      }
      Ok(())
    }

    fn assert_merge<D, CR>(
      _policy: &Self::MergePolicy,
      _merge: &MergeSpecification<D, CR>,
    ) -> Result<()>
    where
      D: Directory,
      CR: CodecReader,
    {
      Err(LuceneError::unreachable("should never happen"))
    }
  }

  #[test]
  fn test_no_merge_policy() -> Result<()> {
    let mut random = random();
    let case = TestNoMergePolicy;
    let mp = case.merge_policy();
    assert!(
      mp.find_merges(
        MergeTrigger::random_trigger(&mut random),
        &SegmentInfos::<DummyDirectory>::new(LATEST.major - 1)?,
        None,
        &DummyMergeContext,
      )?
      .is_none()
    );
    assert!(
      mp.find_forced_merges(
        &SegmentInfos::<DummyDirectory>::new(LATEST.major - 1)?,
        0,
        &HashMap::new(),
        None,
        &DummyMergeContext,
      )?
      .is_none()
    );
    assert!(
      mp.find_forced_deletes_merges(
        &SegmentInfos::<DummyDirectory>::new(LATEST.major - 1)?,
        None,
        &DummyMergeContext
      )?
      .is_none()
    );
    Ok(())
  }
  #[test]
  fn test_final_singleton() -> Result<()> {
    // this test is not required in Rust Lucene
    Ok(())
  }
  #[test]
  fn test_methods_overridden() -> Result<()> {
    // this test is not required in Rust Lucene
    Ok(())
  }

  #[test]
  fn test_force_merge_not_needed() -> Result<()> {
    let mut random = random();
    let case = TestNoMergePolicy;
    case.test_force_merge_not_needed(&mut random)?;
    Ok(())
  }

  #[test]
  fn test_find_forced_deletes_merges() -> Result<()> {
    let mut random = random();
    let case = TestNoMergePolicy;
    case.test_find_forced_deletes_merges(&mut random)?;
    Ok(())
  }
  #[test]
  fn test_simulate_append_only() -> Result<()> {
    let mut random = random();
    let case = TestNoMergePolicy;
    let mp = case.merge_policy();
    let fake_dir = Arc::new(FakeDirectory::new());
    case.do_test_simulate_append_only(&mut random, &mp, fake_dir, 1_000_000, 10_000)?;
    Ok(())
  }
  #[test]
  fn test_simulate_updates() -> Result<()> {
    let mut random = random();
    let case = TestNoMergePolicy;
    let mp = case.merge_policy();
    let fake_dir = Arc::new(FakeDirectory::new());
    case.do_test_simulate_updates(&mut random, &mp, fake_dir, 100_000, 10_000)?;
    Ok(())
  }

  #[test]
  fn test_no_pathological_merges() -> Result<()> {
    let mut random = random();
    let case = TestNoMergePolicy;
    let mp = case.merge_policy();
    let fake_dir = Arc::new(FakeDirectory::new());
    case.test_no_pathological_merges(&mut random, &mp, fake_dir)?;
    Ok(())
  }
}
