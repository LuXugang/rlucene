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
use crate::core::index::merge_policy::{DefaultMergeSpecification, MergePolicy};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::util::LATEST;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::base_merge_policy_test_case::{
  FakeDirectory, MockMergeContext,
};
use crate::test_framework::core::index::force_merge_policy::ForceMergePolicy;

#[allow(dead_code)] // for quick search
struct TestForceMergePolicy;

#[test]
fn test_force_merge_policy() -> Result<()> {
  let policy = ForceMergePolicy::new(NoMergePolicy::default());
  let segment_infos = SegmentInfos::<FakeDirectory>::new(LATEST.major)?;
  let merge_context =
    MockMergeContext::new(|_: &SegmentCommitInfo<FakeDirectory>| -> Result<i32> { Ok(0) });
  let merges: Option<DefaultMergeSpecification<FakeDirectory>> =
    policy.find_merges(MergeTrigger::Explicit, &segment_infos, None, &merge_context)?;
  assert!(merges.is_none());
  Ok(())
}
