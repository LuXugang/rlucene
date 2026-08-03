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
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use parking_lot::Mutex;
use rand::prelude::{SliceRandom, StdRng};
use rand::{Rng, RngExt, SeedableRng};

use crate::core::index::codec_reader::CodecReader;
use crate::core::index::doc_values::DocValues;
use crate::core::index::index_writer::Inner;
use crate::core::index::merge_policy::{
  DefaultMergeSpecification, MergeContext, MergePolicy, MergePolicyBase, OneMerge, OneMergeHook,
  size,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::index::sorter::DocMap;
use crate::core::index::tiered_merge_policy::SegmentDocAndID;
use crate::core::store::directory::Directory;
use crate::core::util::TryIntoInt;
use crate::core::util::bit_set::{BitSet, SparseFixedBitSetBitSet, of};
use crate::core::util::error::lucene_error::Result;
pub(crate) use crate::test_framework::core::index::mock_random_wrapped_reader::MockRandomWrappedReader;
use crate::test_framework::core::util::test_util::TestUtil;

/// Merge policy that makes random decisions for testing.
#[derive(Clone)]
pub struct MockRandomMergePolicy {
  base: MergePolicyBase,
  random: Arc<Mutex<StdRng>>,
  do_non_bulk_merges: bool,
}

impl MockRandomMergePolicy {
  pub fn new<R>(random: &mut R) -> Self
  where
    R: Rng + ?Sized,
  {
    // Fork a private random, since we are called unpredictably from threads.
    Self {
      base: MergePolicyBase::default(),
      random: Arc::new(Mutex::new(StdRng::seed_from_u64(random.random()))),
      do_non_bulk_merges: true,
    }
  }

  /// Set to true if sometimes readers to be merged should be wrapped in a filter reader to mix up
  /// bulk merging.
  pub fn set_do_non_bulk_merges(&mut self, value: bool) {
    self.do_non_bulk_merges = value;
  }
}

impl Display for MockRandomMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "MockRandomMergePolicy")
  }
}

impl<D> MergePolicy<D> for MockRandomMergePolicy
where
  D: Directory,
{
  fn get_base(&self) -> &MergePolicyBase {
    &self.base
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    &mut self.base
  }

  fn find_merges<MC>(
    &self,
    _merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
  {
    let merging = merge_context.get_merging_segments(inner);
    let mut segments: Vec<_> = segment_infos
      .iter()
      .iter()
      .filter(|info| !merging.contains(info.info.get_id_key()))
      .collect();
    let num_segments = segments.len();
    let mut random = self.random.lock();

    if num_segments > 1 && (num_segments > 30 || random.random_range(0..5) == 3) {
      segments.shuffle(&mut *random);

      // TODO IMPORTANT: sometimes make more than 1 merge?
      let mut merge_spec = DefaultMergeSpecification::new();
      let segs_to_merge = TestUtil::next_usize(&mut *random, 1, num_segments);
      let mut merge_segments = Vec::with_capacity(segs_to_merge);
      for info in &segments[..segs_to_merge] {
        merge_segments.push(SegmentDocAndID::new(
          info.info.get_id_key().to_string(),
          info.info.max_doc()?,
        ));
      }
      let one_merge = OneMerge::new(merge_segments)?;
      if self.do_non_bulk_merges && random.random_bool(0.5) {
        merge_spec.add(one_merge.with_hook(OneMergeHook::MockRandom(Box::new(
          MockRandomOneMerge::new(random.random()),
        ))));
      } else {
        merge_spec.add(one_merge);
      }
      Ok(Some(merge_spec))
    } else {
      Ok(None)
    }
  }

  fn find_forced_merges<MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    _max_segment_count: usize,
    segments_to_merge: &HashMap<String, Option<bool>>,
    _inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
  {
    let mut eligible_segments: Vec<_> = segment_infos
      .iter()
      .iter()
      .filter(|info| segments_to_merge.contains_key(info.info.get_id_key()))
      .collect();

    let needs_merge = eligible_segments.len() > 1
      || (eligible_segments.len() == 1
        && !self.has_merged(segment_infos, eligible_segments[0], merge_context)?);
    if !needs_merge {
      return Ok(None);
    }

    let mut merge_spec = DefaultMergeSpecification::new();
    let mut random = self.random.lock();
    // Already shuffled having come out of a set but shuffle again for good measure.
    eligible_segments.shuffle(&mut *random);
    let mut upto = 0;
    while upto < eligible_segments.len() {
      let max = 10.min(eligible_segments.len() - upto);
      let inc = if max <= 2 {
        max
      } else {
        TestUtil::next_usize(&mut *random, 2, max)
      };
      let mut merge_segments = Vec::with_capacity(inc);
      for info in &eligible_segments[upto..upto + inc] {
        merge_segments.push(SegmentDocAndID::new(
          info.info.get_id_key().to_string(),
          info.info.max_doc()?,
        ));
      }
      let one_merge = OneMerge::new(merge_segments)?;
      if self.do_non_bulk_merges && random.random_bool(0.5) {
        merge_spec.add(one_merge.with_hook(OneMergeHook::MockRandom(Box::new(
          MockRandomOneMerge::new(random.random()),
        ))));
      } else {
        merge_spec.add(one_merge);
      }
      upto += inc;
    }

    debug_assert!(merge_spec.merges.iter().all(|merge| {
      merge
        .stat
        .segments
        .iter()
        .all(|id| segments_to_merge.contains_key(id))
    }));
    Ok(Some(merge_spec))
  }

  fn find_forced_deletes_merges<MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
  {
    self.find_merges(MergeTrigger::Explicit, segment_infos, inner, merge_context)
  }

  fn find_full_flush_merges<MC>(
    &self,
    merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<DefaultMergeSpecification<D>>>
  where
    MC: MergeContext<D>,
  {
    self.find_merges(merge_trigger, segment_infos, inner, merge_context)
  }

  fn use_compound_file<MC>(
    &self,
    _infos: &SegmentInfos<D>,
    _merged_info: &SegmentCommitInfo<D>,
    _merge_context: &MC,
  ) -> Result<bool>
  where
    MC: MergeContext<D>,
  {
    // 80% of the time we create CFS.
    Ok(self.random.lock().random_range(0..5) != 1)
  }

  fn size<MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    MC: MergeContext<D>,
  {
    size(info, merge_context)
  }
}

pub(crate) struct MockRandomOneMerge {
  r: Mutex<StdRng>,
}

impl MockRandomOneMerge {
  pub(crate) fn new(seed: u64) -> Self {
    Self {
      r: Mutex::new(StdRng::seed_from_u64(seed)),
    }
  }

  pub(crate) fn wrap_for_merge<D>(
    &self,
    reader: DefaultLeafReader<D>,
  ) -> Result<MockRandomWrappedReader<D>>
  where
    D: Directory,
  {
    // Wrap it (e.g. prevent bulk merge etc).
    // TODO IMPORTANT: cut this over to FilterCodecReader api, we can explicitly
    // enable/disable bulk merge for portions of the index we want.
    let mut random = self.r.lock();
    let thing_to_do = random.random_range(0..7);
    if thing_to_do == 0 {
      // Simple no-op FilterReader.
      MockRandomWrappedReader::slow(reader)
    } else if thing_to_do == 1 {
      // Renumber fields.
      // NOTE: currently this only "blocks" bulk merges just by
      // being a FilterReader. But it might find bugs elsewhere,
      // and maybe the situation can be improved in the future.
      MockRandomWrappedReader::mismatched(reader, &mut *random)
    } else {
      // Otherwise, reader is unchanged.
      Ok(MockRandomWrappedReader::unchanged(reader))
    }
  }

  pub(crate) fn reorder<CR, D>(
    &self,
    reader: &CR,
    _dir: D,
  ) -> Result<Option<MockRandomOneMergeDocMap>>
  where
    CR: CodecReader,
    D: Directory,
  {
    if self.r.lock().random_bool(0.5) {
      // Reverse the doc ID order.
      return Ok(Some(MockRandomOneMergeDocMap::Reverse(reverse(reader)?)));
    }
    Ok(None)
  }
}

#[derive(Clone)]
pub(crate) enum MockRandomOneMergeDocMap {
  Default(crate::core::index::dummy::dummy_doc_map_sorter::DummyDocMap),
  Reverse(ReverseDocMap),
}

impl DocMap for MockRandomOneMergeDocMap {
  fn old_to_new(&self, doc_id: i32) -> Result<i32> {
    match self {
      Self::Default(doc_map) => doc_map.old_to_new(doc_id),
      Self::Reverse(doc_map) => doc_map.old_to_new(doc_id),
    }
  }

  fn new_to_old(&self, doc_id: i32) -> Result<i32> {
    match self {
      Self::Default(doc_map) => doc_map.new_to_old(doc_id),
      Self::Reverse(doc_map) => doc_map.new_to_old(doc_id),
    }
  }

  fn size(&self) -> i32 {
    match self {
      Self::Default(doc_map) => doc_map.size(),
      Self::Reverse(doc_map) => doc_map.size(),
    }
  }
}

#[derive(Clone)]
pub(crate) struct ReverseDocMap {
  max_doc: i32,
  parents: Option<Arc<SparseFixedBitSetBitSet>>,
}

pub(crate) fn reverse<CR>(reader: &CR) -> Result<ReverseDocMap>
where
  CR: CodecReader,
{
  let max_doc = reader.max_doc()?;
  let parents = match reader.get_field_infos()?.get_parent_field() {
    None => None,
    Some(parent_field) => {
      let mut parent_values = DocValues::get_numeric(reader, parent_field)?;
      Some(Arc::new(of(&mut parent_values, max_doc.try_convert()?)?))
    },
  };
  Ok(ReverseDocMap { max_doc, parents })
}

impl DocMap for ReverseDocMap {
  fn old_to_new(&self, doc_id: i32) -> Result<i32> {
    match &self.parents {
      None => Ok(self.max_doc - 1 - doc_id),
      Some(parents) => {
        let old_block_start = if doc_id == 0 {
          0
        } else {
          parents
            .prev_set_bit((doc_id - 1).try_convert()?)
            .map_or(0, |doc| doc + 1)
            .try_convert()?
        };
        let old_block_end: i32 = parents.next_set_bit(doc_id.try_convert()?).try_convert()?;
        let new_block_end = self.max_doc - 1 - old_block_start;
        Ok(new_block_end - (old_block_end - doc_id))
      },
    }
  }

  fn new_to_old(&self, doc_id: i32) -> Result<i32> {
    match &self.parents {
      None => Ok(self.max_doc - 1 - doc_id),
      Some(parents) => {
        let old_block_end: i32 = parents
          .next_set_bit((self.max_doc - 1 - doc_id).try_convert()?)
          .try_convert()?;
        let new_block_end = self.old_to_new(old_block_end)?;
        Ok(old_block_end - (new_block_end - doc_id))
      },
    }
  }

  fn size(&self) -> i32 {
    self.max_doc
  }
}
