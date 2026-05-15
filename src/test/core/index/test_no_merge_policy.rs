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
use rand::Rng;
use rand::prelude::StdRng;
use std::collections::HashMap;
use std::sync::Arc;

struct TestNoMergePolicy;

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestNoMergePolicy, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestNoMergePolicy;
  f(&case, &mut random)
}

impl BaseMergePolicyTestCase for TestNoMergePolicy {
  type MergePolicy = NoMergePolicy;

  fn merge_policy<R>(&self, _random: &mut R) -> Self::MergePolicy
  where
    R: Rng + ?Sized,
  {
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
  let mp = case.merge_policy(&mut random);
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
  test_not_required_in_rust_lucene!();
}
#[test]
fn test_methods_overridden() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
mod base_merge_policy_test_case_tests {
  use super::*;
  #[test]
  fn test_force_merge_not_needed() -> Result<()> {
    run_case(|case, random| case.test_force_merge_not_needed(random))
  }

  #[test]
  fn test_find_forced_deletes_merges() -> Result<()> {
    run_case(|case, random| case.test_find_forced_deletes_merges(random))
  }
  #[test]
  fn test_simulate_append_only() -> Result<()> {
    run_case(|case, random| {
      let mp = case.merge_policy(random);
      case.do_test_simulate_append_only(
        random,
        &mp,
        Arc::new(FakeDirectory::new()),
        1_000_000,
        10_000,
      )
    })
  }
  #[test]
  fn test_simulate_updates() -> Result<()> {
    run_case(|case, random| {
      let mp = case.merge_policy(random);
      case.do_test_simulate_updates(random, &mp, Arc::new(FakeDirectory::new()), 100_000, 10_000)
    })
  }

  #[test]
  fn test_no_pathological_merges() -> Result<()> {
    run_case(|case, random| {
      let mp = case.merge_policy(random);
      case.test_no_pathological_merges(random, &mp, Arc::new(FakeDirectory::new()))
    })
  }
}
