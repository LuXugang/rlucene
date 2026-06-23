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
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::{
  MergeContext, MergePolicy, MergePolicyBase, MergePolicyEnum, MergeSpecificationNoReader,
  OneMerge, size,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::multi_terms::get_term_postings_enum_with_flag;
use crate::core::index::postings_enum::{NONE, PostingsEnum};
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::term::Term;
use crate::core::index::tiered_merge_policy::SegmentDocAndID;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::doc_helper::DocHelper;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, random,
};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Mutex;

#[allow(dead_code)] // for quick search
struct TestPerSegmentDeletes;

#[test]
fn test_deletes1() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwc.set_max_buffered_docs(5000);
  iwc.set_ram_buffer_size_mb(100.0);
  iwc.set_merge_policy(MergePolicyEnum::Range(RangeMergePolicy::new(false)));
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  for x in 0..5 {
    writer.add_document(DocHelper::create_document(x, "1", 2))?;
  }
  writer.commit()?;
  assert_eq!(1, writer.clone_segment_infos()?.size());

  for x in 5..10 {
    writer.add_document(DocHelper::create_document(x, "2", 2))?;
  }
  writer.commit()?;
  assert_eq!(2, writer.clone_segment_infos()?.size());

  for x in 10..15 {
    writer.add_document(DocHelper::create_document(x, "3", 2))?;
  }

  writer.delete_documents_with_terms(vec![Term::from_text("id", "1")])?;
  writer.delete_documents_with_terms(vec![Term::from_text("id", "11")])?;
  writer.flush_with_apply_merge_deletes(false, false)?;

  // deletes are now resolved on flush, so there shouldn't be any deletes after flush
  assert!(!writer.has_changes_in_ram()?);

  // get reader flushes pending deletes so there should not be anymore
  let r1 = writer.get_reader(true, true)?;
  assert!(!writer.has_changes_in_ram()?);
  drop(r1);

  // delete id:2 from the first segment
  // merge segments 0 and 1
  // which should apply the delete id:2
  writer.delete_documents_with_terms(vec![Term::from_text("id", "2")])?;
  writer.flush_with_apply_merge_deletes(false, false)?;
  match writer.get_config_mut().get_merge_policy_mut() {
    MergePolicyEnum::Range(fsmp) => fsmp.set_merge(0, 2),
    _ => panic!("expected RangeMergePolicy"),
  }
  writer.maybe_merge()?;

  assert_eq!(2, writer.clone_segment_infos()?.size());

  // id:2 shouldn't exist anymore because
  // it's been applied in the merge and now it's gone
  let r2 = writer.get_reader(true, true)?;
  let id2docs = to_docs_array(Term::from_text("id", "2"), &r2)?;
  assert!(id2docs.is_none());
  drop(r2);

  writer.close()?;
  Ok(())
}

fn to_docs_array<CR>(term: Term, reader: &CR) -> Result<Option<Vec<i32>>>
where
  CR: crate::core::index::composite_reader::CompositeReader,
{
  if let Some(postings_enum) =
    get_term_postings_enum_with_flag(reader, &term.field, &term.bytes, NONE as i32)?
  {
    return Ok(Some(to_array(postings_enum)?));
  }
  Ok(None)
}

fn to_array<P>(mut postings_enum: P) -> Result<Vec<i32>>
where
  P: PostingsEnum,
{
  let mut docs = Vec::new();
  while postings_enum.next_doc()? != NO_MORE_DOCS {
    docs.push(postings_enum.doc_id());
  }
  Ok(docs)
}

pub struct RangeMergePolicy {
  base: MergePolicyBase,
  state: Mutex<RangeMergePolicyState>,
  use_compound_file: bool,
}

#[derive(Clone, Copy)]
struct RangeMergePolicyState {
  do_merge: bool,
  start: usize,
  length: usize,
}

impl RangeMergePolicy {
  fn new(use_compound_file: bool) -> Self {
    Self {
      base: MergePolicyBase::default(),
      state: Mutex::new(RangeMergePolicyState {
        do_merge: false,
        start: 0,
        length: 0,
      }),
      use_compound_file,
    }
  }

  fn set_merge(&self, start: usize, length: usize) {
    let mut state = self.state.lock().unwrap();
    state.start = start;
    state.length = length;
    state.do_merge = true;
  }
}

impl Clone for RangeMergePolicy {
  fn clone(&self) -> Self {
    Self {
      base: self.base.clone(),
      state: Mutex::new(*self.state.lock().unwrap()),
      use_compound_file: self.use_compound_file,
    }
  }
}

impl Display for RangeMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "RangeMergePolicy")
  }
}

impl MergePolicy for RangeMergePolicy {
  fn get_base(&self) -> &MergePolicyBase {
    &self.base
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    &mut self.base
  }

  fn find_merges<D, MC>(
    &self,
    _merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    _inner: Option<&crate::core::index::index_writer::Inner<D>>,
    _merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    let mut state = self.state.lock().unwrap();
    if state.do_merge {
      state.do_merge = false;
      let start = state.start;
      let length = state.length;
      drop(state);

      let mut merge_segments = Vec::with_capacity(length);
      for info in &segment_infos.iter()[start..start + length] {
        merge_segments.push(SegmentDocAndID::new(
          info.info.get_id_key().to_string(),
          info.info.max_doc()?,
        ));
      }
      let mut ms = MergeSpecificationNoReader::new();
      ms.add(OneMerge::new(merge_segments)?);
      return Ok(Some(ms));
    }
    Ok(None)
  }

  fn find_forced_merges<D, MC>(
    &self,
    _segment_infos: &SegmentInfos<D>,
    _max_segment_count: usize,
    _segments_to_merge: &HashMap<String, Option<bool>>,
    _inner: Option<&crate::core::index::index_writer::Inner<D>>,
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
    _inner: Option<&crate::core::index::index_writer::Inner<D>>,
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
    _merged_info: &SegmentCommitInfo<D>,
    _merge_context: &MC,
  ) -> Result<bool>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    Ok(self.use_compound_file)
  }

  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    size(info, merge_context)
  }
}
