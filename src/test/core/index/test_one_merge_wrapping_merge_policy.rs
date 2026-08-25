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
use crate::core::index::index_reader::Identity;
use crate::core::index::merge_policy::{DefaultMergeSpecification, MergePolicy, OneMerge};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::one_merge_wrapping_merge_policy::{
  OneMergeUnaryOperator, OneMergeWrappingMergePolicy,
};
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::tiered_merge_policy::SegmentDocAndID;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::io_utils::IOUtils;
use crate::core::util::{LATEST, StringHelper};
use crate::test_framework::core::index::base_merge_policy_test_case::MockMergeContext;
use crate::test_framework::core::index::test_one_merge_wrapping_merge_policy::{
  PredeterminedMergePolicy, WrappedOneMerge, WrappedOneMergeUnaryOperator,
};
use crate::test_framework::core::util::lucene_test_case::{new_directory_shared, random};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestOneMergeWrappingMergePolicy;

#[test]
fn test_segments_are_wrapped() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let body_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
    // First create random merge specs.
    let ms_m = create_random_merge_specification(&mut random, dir.clone())?;
    let ms_f = create_random_merge_specification(&mut random, dir.clone())?;
    let ms_d = create_random_merge_specification(&mut random, dir.clone())?;
    let original_m = ms_m.as_ref().map(|merge_specification| {
      merge_specification
        .merges
        .iter()
        .map(|merge| merge.stat.id.clone())
        .collect::<Vec<_>>()
    });
    let original_f = ms_f.as_ref().map(|merge_specification| {
      merge_specification
        .merges
        .iter()
        .map(|merge| merge.stat.id.clone())
        .collect::<Vec<_>>()
    });
    let original_d = ms_d.as_ref().map(|merge_specification| {
      merge_specification
        .merges
        .iter()
        .map(|merge| merge.stat.id.clone())
        .collect::<Vec<_>>()
    });
    // Secondly, pass them to the predetermined merge policy constructor.
    let original_mp = PredeterminedMergePolicy::new(ms_m, ms_f, ms_d);
    // Thirdly wrap the predetermined merge policy.
    let operator = WrappedOneMergeUnaryOperator::new();
    let one_merge_wrapping_mp = OneMergeWrappingMergePolicy::new(
      original_mp,
      OneMergeUnaryOperator::custom(operator.clone()),
    );
    // Finally, ask for merges and check what we got.
    let segment_infos = SegmentInfos::new(LATEST.major)?;
    let merge_context =
      MockMergeContext::new(|_: &SegmentCommitInfo<DirEnum>| -> Result<i32> { Ok(0) });
    let test_m = one_merge_wrapping_mp.find_merges(
      MergeTrigger::Explicit,
      &segment_infos,
      None,
      &merge_context,
    )?;
    impl_test_segments_are_wrapped(
      original_m.as_deref(),
      test_m,
      operator.take_wrapped_merges(),
    );
    let test_f = one_merge_wrapping_mp.find_forced_merges(
      &segment_infos,
      0,
      &HashMap::new(),
      None,
      &merge_context,
    )?;
    impl_test_segments_are_wrapped(
      original_f.as_deref(),
      test_f,
      operator.take_wrapped_merges(),
    );
    let test_d =
      one_merge_wrapping_mp.find_forced_deletes_merges(&segment_infos, None, &merge_context)?;
    impl_test_segments_are_wrapped(
      original_d.as_deref(),
      test_d,
      operator.take_wrapped_merges(),
    );
    Ok(())
  }));
  let close_result = catch_unwind(AssertUnwindSafe(|| dir.as_ref().close()));
  IOUtils::use_or_suppress_caught_result(body_result, close_result)
}

fn impl_test_segments_are_wrapped<D>(
  original_ms: Option<&[Identity]>,
  test_ms: Option<DefaultMergeSpecification<D>>,
  wrapped_merges: Vec<WrappedOneMerge>,
) where
  D: Directory,
{
  // Wrapping does not add or remove merge specs.
  assert_eq!(original_ms.is_none(), test_ms.is_none());
  let Some(original_ms) = original_ms else {
    assert!(wrapped_merges.is_empty());
    return;
  };
  let test_ms = test_ms.unwrap();
  assert_eq!(original_ms.len(), test_ms.merges.len());
  assert_eq!(original_ms.len(), wrapped_merges.len());
  // Wrapping does not re-order merge specs.
  for ii in 0..original_ms.len() {
    let test_om = &test_ms.merges[ii];
    let wrapped_om = &wrapped_merges[ii];
    // Wrapping wraps.
    assert_eq!(test_om.stat.id, wrapped_om.wrapped);
    assert_ne!(wrapped_om.original, wrapped_om.wrapped);
    // And what is wrapped is what was originally passed in.
    assert_eq!(original_ms[ii], wrapped_om.original);
  }
}

fn create_random_merge_specification<R, D>(
  random: &mut R,
  dir: Arc<D>,
) -> Result<Option<DefaultMergeSpecification<D>>>
where
  R: Rng + ?Sized,
  D: Directory,
{
  if random.random_range(0..10) == 0 {
    // About 1 in 10 times return None.
    return Ok(None);
  }
  let mut ms = DefaultMergeSpecification::new();
  // Append up to 10 random non-sensical one merge objects.
  for _ii in 0..random.random_range(0..10) {
    let mut max_doc = random.random();
    if max_doc == -1 {
      max_doc = 0;
    }
    let id: [u8; StringHelper::ID_LENGTH] = TestUtil::random_simple_string_range(
      random,
      StringHelper::ID_LENGTH,
      StringHelper::ID_LENGTH,
    )
    .into_bytes()
    .try_into()
    .unwrap();
    let si = SegmentInfo::new(
      dir.clone(),
      Some((*LATEST).clone()),
      Some((*LATEST).clone()),
      &TestUtil::random_simple_string(random),
      max_doc,
      random.random_bool(0.5),
      false,
      None,
      HashMap::new(),
      id,
      HashMap::new(),
      None,
    )?;
    let segments = vec![SegmentCommitInfo::new(
      si,
      0,
      0,
      0,
      0,
      0,
      Some(StringHelper::random_id()),
    )];
    let mut merge_segments = Vec::new();
    for info in segments {
      merge_segments.push(SegmentDocAndID::new(
        info.info.get_id_key().to_string(),
        info.info.max_doc()?,
      ));
    }
    ms.add(OneMerge::new(merge_segments)?);
  }
  Ok(Some(ms))
}
