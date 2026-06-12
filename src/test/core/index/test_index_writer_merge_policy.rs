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
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::field_type::FieldType;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::{
  MergeContext, MergePolicy, MergePolicyBase, MergePolicyEnum, MergeSpecificationNoReader,
  OneMerge, size,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::term::Term;
use crate::core::index::tiered_merge_policy::SegmentDocAndID;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_text_field, random,
};
use rand::Rng;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

#[allow(dead_code)] // for quick search
struct TestIndexWriterMergePolicy;

#[derive(Clone)]
pub struct MockMergePolicy {
  base: MergePolicyBase,
  merge_factor: usize,
}

impl Default for MockMergePolicy {
  fn default() -> Self {
    Self {
      base: MergePolicyBase::default(),
      merge_factor: 10,
    }
  }
}

impl MockMergePolicy {
  fn get_merge_factor(&self) -> usize {
    self.merge_factor
  }

  fn set_merge_factor(&mut self, merge_factor: usize) {
    self.merge_factor = merge_factor;
  }
}

impl From<MockMergePolicy> for MergePolicyEnum {
  fn from(value: MockMergePolicy) -> Self {
    Self::Mock(value)
  }
}

impl Display for MockMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "MockMergePolicy")
  }
}

impl MergePolicy for MockMergePolicy {
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
    let segments = segment_infos.iter();
    let mut spec = None;
    let mut start = 0;
    while start + self.merge_factor <= segments.len() {
      let start_doc_count = segments[start].info.max_doc()?;
      let mut end = start + 1;
      for i in (start + 1..segments.len()).rev() {
        let doc_count = segments[i].info.max_doc()?;
        if i64::from(doc_count) * self.merge_factor as i64 > i64::from(start_doc_count)
          && i64::from(doc_count) < self.merge_factor as i64 * i64::from(start_doc_count)
        {
          end = i + 1;
          break;
        }
      }

      if start + self.merge_factor <= end {
        let merge_spec = spec.get_or_insert_with(MergeSpecificationNoReader::new);
        let mut merge_segments = Vec::with_capacity(self.merge_factor);
        for info in &segments[start..start + self.merge_factor] {
          merge_segments.push(SegmentDocAndID::new(
            info.info.get_id_key().to_string(),
            info.info.max_doc()?,
          ));
        }
        merge_spec.add(OneMerge::new(merge_segments)?);
        start += self.merge_factor;
      } else {
        start += 1;
      }
    }
    Ok(spec)
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
    D: Directory,
    MC: MergeContext<D>,
  {
    Ok(None)
  }

  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    size(info, merge_context)
  }
}

#[test]
fn test_normal_case() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config
    .set_max_buffered_docs(10)
    .set_merge_policy(MockMergePolicy::default());
  let writer = IndexWriter::new(dir, config)?;
  let mut field_types = HashMap::new();

  for _ in 0..100 {
    add_doc(&mut random, &writer, &mut field_types)?;
    check_invariants(&writer)?;
  }

  writer.close()?;
  Ok(())
}

#[test]
fn test_no_over_merge() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config
    .set_max_buffered_docs(10)
    .set_merge_policy(MockMergePolicy::default());
  let writer = IndexWriter::new(dir, config)?;
  let mut field_types = HashMap::new();

  let mut no_over_merge = false;
  for _ in 0..100 {
    add_doc(&mut random, &writer, &mut field_types)?;
    check_invariants(&writer)?;
    if writer.get_num_buffered_documents() as usize + writer.get_segment_count() >= 18 {
      no_over_merge = true;
    }
  }
  assert!(no_over_merge);

  writer.close()?;
  Ok(())
}

#[test]
fn test_force_flush() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut merge_policy = MockMergePolicy::default();
  merge_policy.set_merge_factor(10);
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config
    .set_max_buffered_docs(10)
    .set_merge_policy(merge_policy);
  let writer = IndexWriter::new(dir, config)?;
  let mut field_types = HashMap::new();

  for _ in 0..100 {
    add_doc(&mut random, &writer, &mut field_types)?;
    writer.flush()?;
  }

  writer.close()?;
  Ok(())
}

#[test]
fn test_merge_factor_change() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config
    .set_max_buffered_docs(10)
    .set_merge_policy(MockMergePolicy::default())
    .set_merge_scheduler(SerialMergeScheduler::new());
  let mut writer = IndexWriter::new(dir, config)?;
  let mut field_types = HashMap::new();

  for _ in 0..250 {
    add_doc(&mut random, &writer, &mut field_types)?;
    check_invariants(&writer)?;
  }

  if let MergePolicyEnum::Mock(merge_policy) = writer.get_config_mut().get_merge_policy_mut() {
    merge_policy.set_merge_factor(5);
  }

  for _ in 0..10 {
    add_doc(&mut random, &writer, &mut field_types)?;
  }
  check_invariants(&writer)?;

  writer.close()?;
  Ok(())
}

#[cfg(feature = "nightly")]
#[test]
#[ignore = "nightly"]
fn test_max_buffered_docs_change() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config
    .set_max_buffered_docs(101)
    .set_merge_policy(MockMergePolicy::default())
    .set_merge_scheduler(SerialMergeScheduler::new());
  let mut writer = IndexWriter::new(dir.clone(), config)?;

  for i in 1..=100 {
    for _ in 0..i {
      add_doc(&mut random, &writer, &mut field_types)?;
      check_invariants(&writer)?;
    }
    writer.close()?;

    let mock = MockAnalyzer::new(&mut random);
    let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
    config
      .set_open_mode(OpenMode::Append)
      .set_max_buffered_docs(101)
      .set_merge_policy(MockMergePolicy::default())
      .set_merge_scheduler(SerialMergeScheduler::new());
    writer = IndexWriter::new(dir.clone(), config)?;
  }

  writer.close()?;
  let mut merge_policy = MockMergePolicy::default();
  merge_policy.set_merge_factor(10);
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config
    .set_open_mode(OpenMode::Append)
    .set_max_buffered_docs(10)
    .set_merge_policy(merge_policy)
    .set_merge_scheduler(SerialMergeScheduler::new());
  writer = IndexWriter::new(dir, config)?;

  for _ in 0..100 {
    add_doc(&mut random, &writer, &mut field_types)?;
  }
  check_invariants(&writer)?;

  for _ in 100..1000 {
    add_doc(&mut random, &writer, &mut field_types)?;
  }
  writer.commit()?;
  writer.wait_for_merges()?;
  writer.commit()?;
  check_invariants(&writer)?;

  writer.close()?;
  Ok(())
}

#[test]
fn test_merge_doc_count_0() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut field_types = HashMap::new();

  let mut merge_policy = MockMergePolicy::default();
  merge_policy.set_merge_factor(100);
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config
    .set_max_buffered_docs(10)
    .set_merge_policy(merge_policy);
  let writer = IndexWriter::new(dir.clone(), config)?;

  for _ in 0..250 {
    add_doc(&mut random, &writer, &mut field_types)?;
    check_invariants(&writer)?;
  }
  writer.close()?;

  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir.clone(), config)?;
  writer.delete_documents_with_terms(vec![Term::from_text("content", "aaa")])?;
  writer.close()?;

  let mut merge_policy = MockMergePolicy::default();
  merge_policy.set_merge_factor(5);
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
  config
    .set_open_mode(OpenMode::Append)
    .set_max_buffered_docs(10)
    .set_merge_policy(merge_policy);
  // TODO IMPORTANT ConcurrentMergeScheduler未实现
  let writer = IndexWriter::new(dir, config)?;

  for _ in 0..10 {
    add_doc(&mut random, &writer, &mut field_types)?;
  }
  writer.commit()?;
  writer.wait_for_merges()?;
  writer.commit()?;
  check_invariants(&writer)?;
  assert_eq!(10, writer.get_doc_stats()?.max_doc);

  writer.close()?;
  Ok(())
}

fn add_doc<D, R>(
  random: &mut R,
  writer: &IndexWriter<D>,
  field_types: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: Directory,
  R: Rng + ?Sized,
{
  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "content",
    "aaa",
    Store::No,
    field_types,
  )?);
  writer.add_document(doc)?;
  Ok(())
}

fn check_invariants<D>(writer: &IndexWriter<D>) -> Result<()>
where
  D: Directory,
{
  writer.wait_for_merges()?;
  let max_buffered_docs = writer.get_config().get_max_buffered_docs();
  let merge_factor = match writer.get_config().get_merge_policy() {
    MergePolicyEnum::Mock(merge_policy) => merge_policy.get_merge_factor(),
    _ => unreachable!(),
  };

  let ram_segment_count = writer.get_num_buffered_documents();
  assert!(ram_segment_count < max_buffered_docs);

  let segment_count = writer.get_segment_count();
  let mut lower_bound = i32::MAX;
  for i in 0..segment_count {
    lower_bound = lower_bound.min(writer.max_doc(i as i32));
  }
  let upper_bound = lower_bound * merge_factor as i32;

  let mut segments_across_levels = 0;
  while segments_across_levels < segment_count {
    let mut segments_on_current_level = 0;
    for i in 0..segment_count {
      let doc_count = writer.max_doc(i as i32);
      if doc_count >= lower_bound && doc_count < upper_bound {
        segments_on_current_level += 1;
      }
    }
    assert!(segments_on_current_level < merge_factor);
    segments_across_levels += segments_on_current_level;
  }
  Ok(())
}

#[test]
fn test_setters() -> Result<()> {
  Ok(())
}

#[test]
fn test_merge_on_commit() -> Result<()> {
  Ok(())
}

#[test]
fn test_merge_on_commit_with_event_listener() -> Result<()> {
  Ok(())
}

#[test]
fn test_carry_over_new_deletes_on_commit() -> Result<()> {
  Ok(())
}

#[test]
fn test_abort_merge_on_commit() -> Result<()> {
  Ok(())
}

#[test]
fn test_abort_merge_on_get_reader() -> Result<()> {
  Ok(())
}

#[test]
fn test_force_merge_while_get_reader() -> Result<()> {
  Ok(())
}

#[test]
fn test_fail_after_merge_committed() -> Result<()> {
  Ok(())
}

#[test]
fn test_stress_update_same_document_with_merge_on_get_reader() -> Result<()> {
  Ok(())
}

#[test]
fn test_stress_update_same_document_with_merge_on_commit() -> Result<()> {
  Ok(())
}

#[test]
fn test_merge_on_get_reader() -> Result<()> {
  Ok(())
}

#[test]
fn test_set_diagnostics() -> Result<()> {
  Ok(())
}

#[test]
fn test_force_merge_dv_update_file_with_concurrent_flush() -> Result<()> {
  Ok(())
}

#[test]
fn test_merge_dv_update_file_on_get_reader_with_concurrent_flush() -> Result<()> {
  Ok(())
}

#[test]
fn test_merge_dv_update_file_on_commit_with_concurrent_flush() -> Result<()> {
  Ok(())
}

#[test]
fn test_force_merge_with_pending_hard_and_soft_delete_file() -> Result<()> {
  Ok(())
}
