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
use crate::core::index::merge_policy::{MergePolicyEnum, MergeSpecification};
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::upgrade_index_merge_policy::UpgradeIndexMergePolicy;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::base_merge_policy_test_case::{
  BaseMergePolicyTestCase, FakeDirectory,
};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_tiered_merge_policy, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use rand::prelude::StdRng;
use std::sync::Arc;

struct TestUpgradeIndexMergePolicy;

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestUpgradeIndexMergePolicy, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestUpgradeIndexMergePolicy;
  f(&case, &mut random)
}

impl BaseMergePolicyTestCase for TestUpgradeIndexMergePolicy {
  type MergePolicy = MergePolicyEnum;

  fn merge_policy<R>(&self, random: &mut R) -> Self::MergePolicy
  where
    R: Rng + ?Sized,
  {
    let mut inner = new_tiered_merge_policy(random);
    let size = TestUtil::next_int(random, 1024, 10 * 1024);
    inner.set_max_merged_segment_mb(size as f64).expect("");
    UpgradeIndexMergePolicy::new(inner.into()).into()
  }

  fn assert_segment_infos<D>(_policy: &Self::MergePolicy, _infos: &SegmentInfos<D>) -> Result<()>
  where
    D: Directory,
  {
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
    Ok(())
  }
}
mod base_merge_policy_test_case_tests {
  use super::*;
  use std::sync::Arc;
  use crate::test::core::index::base_merge_policy_test_case::FakeDirectory;
  use crate::test::core::index::test_upgrade_index_merge_policy::run_case;

  #[test]
  fn test_force_merge_not_needed() -> crate::core::util::error::lucene_error::Result<()> {
    run_case(|case, random| case.test_force_merge_not_needed(random))
  }

  #[test]
  fn test_find_forced_deletes_merges() -> crate::core::util::error::lucene_error::Result<()> {
    run_case(|case, random| case.test_find_forced_deletes_merges(random))
  }

  #[test]
  fn test_simulate_append_only() -> crate::core::util::error::lucene_error::Result<()> {
    run_case(|case, random| {
      let mp = case.merge_policy(random);
      case.test_simulate_append_only(random, &mp, Arc::new(FakeDirectory::new()))
    })
  }

  #[test]
  fn test_simulate_updates() -> crate::core::util::error::lucene_error::Result<()> {
    run_case(|case, random| {
      let mp = case.merge_policy(random);
      case.test_simulate_updates(random, &mp, Arc::new(FakeDirectory::new()))
    })
  }

  #[test]
  fn test_no_pathological_merges() -> crate::core::util::error::lucene_error::Result<()> {
    run_case(|case, random| {
      let mp = case.merge_policy(random);
      case.test_no_pathological_merges(random, &mp, Arc::new(FakeDirectory::new()))
    })
  }

}
