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
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::composite_reader::get_context;
use crate::core::index::concurrent_merge_scheduler::ConcurrentMergeScheduler;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::{Identity, IndexReader};
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer::Inner;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::log_byte_size_merge_policy::LogByteSizeMergePolicy;
use crate::core::index::log_merge_policy::LogMergePolicy;
use crate::core::index::merge_policy::{
  MergeContext, MergePolicy, MergePolicyBase, MergePolicyEnum, MergeSpecification,
  MergeSpecificationNoReader, OneMerge, size,
};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::one_merge_wrapping_merge_policy::{
  NewOneMergeUnaryOperator, OneMergeWrappingMergePolicy,
};
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::term::Term;
use crate::core::index::tiered_merge_policy::{SegmentDocAndID, TieredMergePolicy};
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::store::data_input::DataInput;
use crate::test::core::util::lucene_test_case::{
  create_temp_dir_with_prefix, new_directory_shared, new_fs_directory, new_index_writer_config,
  new_index_writer_config_with_analyzer, new_merge_policy, new_string_field, new_text_field,
  random,
};

use crate::core::store::directory::Directory;
use crate::core::store::index_input::IndexInput;
use crate::core::store::io_context::IOContext;
use crate::core::store::random_access_input::RandomAccessInputWrapper;
use crate::core::util::HasIdentity;
use crate::core::util::clone::TryClone;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestIndexWriterMergePolicy;

#[derive(Clone)]
pub struct MockMergePolicy {
  base: MergePolicyBase,
  merge_factor: i32,
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
  fn get_merge_factor(&self) -> i32 {
    self.merge_factor
  }

  fn set_merge_factor(&mut self, merge_factor: i32) {
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
    let merge_factor = self.merge_factor as usize;
    let mut spec = None;
    let mut start = 0;
    while start + merge_factor <= segments.len() {
      let start_doc_count = segments[start].info.max_doc()?;
      let mut end = start + 1;
      for i in (start + 1..segments.len()).rev() {
        let doc_count = segments[i].info.max_doc()?;
        if i64::from(doc_count) * i64::from(self.merge_factor) > i64::from(start_doc_count)
          && i64::from(doc_count) < i64::from(self.merge_factor) * i64::from(start_doc_count)
        {
          end = i + 1;
          break;
        }
      }

      if start + merge_factor <= end {
        let merge_spec = spec.get_or_insert_with(MergeSpecificationNoReader::new);
        let mut merge_segments = Vec::with_capacity(merge_factor);
        for info in &segments[start..start + merge_factor] {
          merge_segments.push(SegmentDocAndID::new(
            info.info.get_id_key().to_string(),
            info.info.max_doc()?,
          ));
        }
        merge_spec.add(OneMerge::new(merge_segments)?);
        start += merge_factor;
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
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
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
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
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
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
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
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
  config
    .set_max_buffered_docs(10)
    .set_merge_policy(MockMergePolicy::default())
    .set_merge_scheduler(SerialMergeScheduler::new());
  let writer = IndexWriter::new(dir, config)?;
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
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
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
    let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
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
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
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
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
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
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
  config.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir.clone(), config)?;
  writer.delete_documents_with_terms(vec![Term::from_text("content", "aaa")])?;
  writer.close()?;

  let mut merge_policy = MockMergePolicy::default();
  merge_policy.set_merge_factor(5);
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
  config
    .set_open_mode(OpenMode::Append)
    .set_max_buffered_docs(10)
    .set_merge_policy(merge_policy)
    .set_merge_scheduler(ConcurrentMergeScheduler::new());
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
  D: Directory + 'static,
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
  D: Directory + 'static,
{
  writer.wait_for_merges()?;
  let max_buffered_docs = writer.get_config().get_max_buffered_docs();
  let merge_factor = match writer.get_config().get_merge_policy() {
    MergePolicyEnum::Mock(merge_policy) => merge_policy.get_merge_factor(),
    _ => unreachable!(),
  };

  let ram_segment_count = writer.get_num_buffered_documents();
  assert!(ram_segment_count < max_buffered_docs);

  let segment_count = writer.get_segment_count() as i32;
  let mut lower_bound = i32::MAX;
  for i in 0..segment_count {
    lower_bound = lower_bound.min(writer.max_doc(i));
  }
  let upper_bound = lower_bound.wrapping_mul(merge_factor);

  let mut segments_across_levels = 0;
  while segments_across_levels < segment_count {
    let mut segments_on_current_level = 0;
    for i in 0..segment_count {
      let doc_count = writer.max_doc(i);
      if doc_count >= lower_bound && doc_count < upper_bound {
        segments_on_current_level += 1;
      }
    }
    assert!(segments_on_current_level < merge_factor);
    segments_across_levels += segments_on_current_level;
  }
  Ok(())
}

/// Port of Java TestIndexWriterMergePolicy.assertSetters(MergePolicy)
const EPSILON: f64 = 1e-14;

fn assert_setters<P>(lmp: &mut P) -> Result<()>
where
  P: MergePolicy,
{
  let base = lmp.get_base_mut();
  base.set_max_cfs_segment_size_mb(2.0)?;
  assert!((base.get_max_cfs_segment_size_mb() - 2.0).abs() < EPSILON);

  base.set_max_cfs_segment_size_mb(f64::INFINITY)?;
  assert!(
    (base.get_max_cfs_segment_size_mb() - (i64::MAX as f64 / 1024.0 / 1024.0)).abs()
      < EPSILON * i64::MAX as f64
  );

  base.set_max_cfs_segment_size_mb(i64::MAX as f64 / 1024.0 / 1024.0)?;
  assert!(
    (base.get_max_cfs_segment_size_mb() - (i64::MAX as f64 / 1024.0 / 1024.0)).abs()
      < EPSILON * i64::MAX as f64
  );

  assert!(base.set_max_cfs_segment_size_mb(-2.0).is_err());

  Ok(())
}

#[derive(Clone)]
pub struct MergeOnXMergePolicy {
  pub(crate) in_: Box<MergePolicyEnum>,
  pub(crate) trigger: MergeTrigger,
}

impl MergeOnXMergePolicy {
  pub(crate) fn new(in_: MergePolicyEnum, trigger: MergeTrigger) -> Self {
    Self {
      in_: Box::new(in_),
      trigger,
    }
  }
}

impl Display for MergeOnXMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "MergeOnCommit({})", self.in_)
  }
}

impl MergePolicy for MergeOnXMergePolicy {
  fn get_base(&self) -> &MergePolicyBase {
    self.in_.get_base()
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    self.in_.get_base_mut()
  }

  fn find_merges<D, MC>(
    &self,
    merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self
      .in_
      .find_merges(merge_trigger, segment_infos, inner, merge_context)
  }

  fn find_merges_readers<CR, D>(
    &self,
    readers: Vec<CR>,
  ) -> Result<Option<crate::core::index::merge_policy::MergeSpecification<D, CR>>>
  where
    CR: CodecReader,
    D: Directory,
  {
    self.in_.find_merges_readers(readers)
  }

  fn find_forced_merges<D, MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    max_segment_count: usize,
    segments_to_merge: &HashMap<String, Option<bool>>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self.in_.find_forced_merges(
      segment_infos,
      max_segment_count,
      segments_to_merge,
      inner,
      merge_context,
    )
  }

  fn find_forced_deletes_merges<D, MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self
      .in_
      .find_forced_deletes_merges(segment_infos, inner, merge_context)
  }

  fn find_full_flush_merges<D, MC>(
    &self,
    merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    if merge_trigger == self.trigger && segment_infos.iter().len() > 1 {
      let merging = merge_context.get_merging_segments(inner);
      let mut non_merging_segments = Vec::new();
      for sci in segment_infos.iter() {
        if !merging.contains(sci.info.get_id_key()) {
          non_merging_segments.push(SegmentDocAndID::new(
            sci.info.get_id_key().to_string(),
            sci.info.max_doc()?,
          ));
        }
      }
      if non_merging_segments.len() > 1 {
        let mut spec = MergeSpecificationNoReader::new();
        spec.add(OneMerge::new(non_merging_segments)?);
        return Ok(Some(spec));
      }
    }
    Ok(None)
  }

  fn use_compound_file<D, MC>(
    &self,
    infos: &SegmentInfos<D>,
    merged_info: &SegmentCommitInfo<D>,
    merge_context: &MC,
  ) -> Result<bool>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self
      .in_
      .use_compound_file(infos, merged_info, merge_context)
  }

  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self.in_.size(info, merge_context)
  }
}

#[test]
fn test_setters() -> Result<()> {
  let mut lmp = LogMergePolicy::<LogByteSizeMergePolicy>::log_bytes_size();
  assert_setters(&mut lmp)?;

  let mut mock = MockMergePolicy::default();
  assert_setters(&mut mock)?;

  Ok(())
}

#[test]
fn test_merge_on_commit() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  // First writer: no merge policy, add 5 docs (each flushed)
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
  config.set_merge_policy(NoMergePolicy::default());
  let first_writer = IndexWriter::new(dir.clone(), config)?;

  let mut field_types = HashMap::new();
  for _ in 0..5 {
    add_doc(&mut random, &first_writer, &mut field_types)?;
    first_writer.flush()?;
  }

  // Check 5 leaf segments
  {
    let first_reader = directory_reader::open_from_writer(&first_writer)?;
    let first_ctx = get_context(first_reader)?;
    assert_eq!(5, first_ctx.leaves()?.len());
  }
  first_writer.close()?;

  // Second writer: MergeOnX with COMMIT trigger
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
  config
    .set_merge_policy(MergeOnXMergePolicy::new(
      new_merge_policy(&mut random)?,
      MergeTrigger::Commit,
    ))
    .set_max_full_flush_merge_wait_millis(i64::MAX);
  let writer_with_merge_policy = IndexWriter::new(dir.clone(), config)?;

  {
    let unmerged_reader = directory_reader::open_from_writer(&writer_with_merge_policy)?;
    let unmerged_ctx = get_context(unmerged_reader)?;
    let leaf_count = unmerged_ctx.leaves()?.len();
    assert_eq!(5, leaf_count);
  }

  // Commit triggers merge
  writer_with_merge_policy.commit()?;
  // TODO IMPORTANT commitOnMerge 未实现
  // assert_eq!(1, writer_with_merge_policy.get_segment_count());
  assert_eq!(5, writer_with_merge_policy.get_segment_count());

  {
    let merged_reader = directory_reader::open_from_writer(&writer_with_merge_policy)?;
    let merged_ctx = get_context(merged_reader)?;
    // TODO IMPORTANT commitOnMerge 未实现
    // assert_eq!(1, merged_ctx.leaves()?.len());
    assert_eq!(5, merged_ctx.leaves()?.len());
  }

  let reader = Arc::new(directory_reader::open_from_writer(
    &writer_with_merge_policy,
  )?);
  let searcher = IndexSearcher::from_cr(reader.clone())?;
  assert_eq!(5, reader.num_docs()?);
  assert_eq!(5, searcher.count(MatchAllDocsQuery::new())?);

  writer_with_merge_policy.close()?;
  Ok(())
}

#[test]
fn test_merge_on_commit_with_event_listener() -> Result<()> {
  // TODO IMPORTANT IndexWriterEventListener未实现
  Ok(())
}

#[test]
fn test_carry_over_new_deletes_on_commit() -> Result<()> {
  // TODO: SoftDeletesDirectoryReaderWrapper未实现
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
  // TODO SoftDeletesDirectoryReaderWrapper未实现
  Ok(())
}

#[test]
fn test_stress_update_same_document_with_merge_on_commit() -> Result<()> {
  // TODO SoftDeletesDirectoryReaderWrapper未实现
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
  let mut random = random();
  let temp_dir = create_temp_dir_with_prefix("testForceMergeWithPendingHardAndSoftDeleteFile")?;
  let path = temp_dir.path().to_path_buf();
  let fs_directory = new_fs_directory(&mut random, temp_dir)?;
  let mock_directory = Arc::new(MockAssertFileExistDirectory::new(fs_directory, path));

  let mock_merge_policy = OneMergeWrappingMergePolicy::new(
    OnlyForceMergeMergePolicy::new(TieredMergePolicy::new()),
    NewOneMergeUnaryOperator,
  );
  let mut config = new_index_writer_config(&mut random)?;
  config.set_merge_policy(mock_merge_policy);

  let writer = IndexWriter::new(mock_directory, config)?;
  let mut field_types = HashMap::new();

  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_types,
  )?);
  doc.add(new_string_field(
    &mut random,
    "version",
    "1",
    Store::Yes,
    &mut field_types,
  )?);
  writer.add_document(doc)?;
  writer.commit()?;

  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_types,
  )?);
  doc.add(new_string_field(
    &mut random,
    "version",
    "1",
    Store::Yes,
    &mut field_types,
  )?);
  writer.add_document(doc)?;

  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "id",
    "3",
    Store::Yes,
    &mut field_types,
  )?);
  doc.add(new_string_field(
    &mut random,
    "version",
    "1",
    Store::Yes,
    &mut field_types,
  )?);
  writer.add_document(doc)?;

  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "id",
    "4",
    Store::Yes,
    &mut field_types,
  )?);
  doc.add(new_string_field(
    &mut random,
    "version",
    "1",
    Store::Yes,
    &mut field_types,
  )?);
  writer.add_document(doc)?;

  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "id",
    "5",
    Store::Yes,
    &mut field_types,
  )?);
  doc.add(new_string_field(
    &mut random,
    "version",
    "1",
    Store::Yes,
    &mut field_types,
  )?);
  writer.add_document(doc)?;
  writer.commit()?;

  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_types,
  )?);
  doc.add(new_string_field(
    &mut random,
    "version",
    "2",
    Store::Yes,
    &mut field_types,
  )?);
  writer.update_document_with_term(Term::from_text("id", "2"), doc)?;
  writer.commit()?;

  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "id",
    "3",
    Store::Yes,
    &mut field_types,
  )?);
  doc.add(new_string_field(
    &mut random,
    "version",
    "2",
    Store::Yes,
    &mut field_types,
  )?);
  writer.update_document_with_term(Term::from_text("id", "3"), doc)?;

  let mut doc = Document::new();
  doc.add(new_string_field(
    &mut random,
    "id",
    "4",
    Store::Yes,
    &mut field_types,
  )?);
  doc.add(new_string_field(
    &mut random,
    "version",
    "2",
    Store::Yes,
    &mut field_types,
  )?);
  let field = NumericDocValuesField::new("soft_delete", 1);
  writer.soft_update_document(Term::from_text("id", "4"), doc, vec![field.into()])?;

  let reader = writer.get_reader(true, false)?;
  reader.close()?;
  writer.commit()?;

  writer.force_merge(1)?;
  writer.close()?;
  Ok(())
}

struct MockAssertFileExistDirectory<D>
where
  D: Directory,
{
  in_: D,
  path: PathBuf,
  id: Identity,
}

impl<D> MockAssertFileExistDirectory<D>
where
  D: Directory,
{
  fn new(in_: D, path: PathBuf) -> Self {
    Self {
      in_,
      path,
      id: Identity::new(),
    }
  }
}

impl<D> Display for MockAssertFileExistDirectory<D>
where
  D: Directory,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "MockAssertFileExistDirectory({})", self.in_)
  }
}

impl<D> Closeable for MockAssertFileExistDirectory<D>
where
  D: Directory,
{
  fn close(&mut self) -> Result<()> {
    self.in_.close()
  }
}

impl<D> HasIdentity for MockAssertFileExistDirectory<D>
where
  D: Directory,
{
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl<D> Directory for MockAssertFileExistDirectory<D>
where
  D: Directory,
  D::IndexInput: IndexInput<IndexInput = D::IndexInput>,
{
  fn list_all(&self) -> Result<Vec<String>> {
    self.in_.list_all()
  }

  fn delete_file(&self, name: &str) -> Result<()> {
    self.in_.delete_file(name)
  }

  fn file_length(&self, name: &str) -> Result<usize> {
    self.in_.file_length(name)
  }

  type IndexOutput = D::IndexOutput;

  fn create_output(&self, name: &str, context: &IOContext) -> Result<Self::IndexOutput> {
    self.in_.create_output(name, context)
  }

  fn create_temp_output(
    &self,
    prefix: &str,
    suffix: &str,
    context: &IOContext,
  ) -> Result<Self::IndexOutput> {
    self.in_.create_temp_output(prefix, suffix, context)
  }

  fn sync(&self, names: &[String]) -> Result<()> {
    self.in_.sync(names)
  }

  fn sync_metadata(&self) -> Result<()> {
    self.in_.sync_metadata()
  }

  fn rename(&self, source: &str, dest: &str) -> Result<()> {
    self.in_.rename(source, dest)
  }

  type IndexInput = MockAssertFileExistIndexInput<D::IndexInput>;

  fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInput> {
    let index_input = self.in_.open_input(name, context)?;
    Ok(MockAssertFileExistIndexInput::new(
      name.to_string(),
      index_input,
      self.path.join(name),
    ))
  }

  type Lock = D::Lock;

  fn obtain_lock(&self, name: &str) -> Result<Self::Lock> {
    self.in_.obtain_lock(name)
  }

  fn copy_from<F>(&self, from: &F, src: &str, dest: &str, context: &IOContext) -> Result<()>
  where
    F: Directory + ?Sized,
  {
    self.in_.copy_from(from, src, dest, context)
  }

  fn get_pending_deletions(&self) -> Result<HashSet<String>> {
    self.in_.get_pending_deletions()
  }

  #[cfg(debug_assertions)]
  fn is_fs_directory(&self) -> bool {
    self.in_.is_fs_directory()
  }

  fn ensure_open(&self) -> Result<()> {
    self.in_.ensure_open()
  }
}

struct MockAssertFileExistIndexInput<I>
where
  I: IndexInput,
{
  name: String,
  delegate: I,
  file_path: PathBuf,
}

impl<I> MockAssertFileExistIndexInput<I>
where
  I: IndexInput,
{
  fn new(name: String, in_: I, file_path: PathBuf) -> Self {
    Self {
      name,
      delegate: in_,
      file_path,
    }
  }

  fn check_file_exists(&self) -> Result<()> {
    if !self.file_path.exists() {
      return Err(LuceneError::io_with_path(
        self.file_path.to_string_lossy().to_string(),
        Error::new(
          ErrorKind::NotFound,
          self.file_path.to_string_lossy().to_string(),
        ),
      ));
    }
    Ok(())
  }
}

impl<I> Closeable for MockAssertFileExistIndexInput<I>
where
  I: IndexInput,
{
  fn close(&mut self) -> Result<()> {
    self.delegate.close()
  }
}

impl<I> DataInput for MockAssertFileExistIndexInput<I>
where
  I: IndexInput,
{
  fn read_byte(&mut self) -> Result<u8> {
    self.check_file_exists()?;
    self.delegate.read_byte()
  }

  fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    self.check_file_exists()?;
    self.delegate.read_bytes(b, offset, len)
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    self.check_file_exists()?;
    self.delegate.read_group_vint(dst, offset)
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    self.check_file_exists()?;
    IndexInput::skip_bytes(&mut self.delegate, num_bytes)
  }
}

impl<I> Display for MockAssertFileExistIndexInput<I>
where
  I: IndexInput,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "MockAssertFileExistIndexInput(name={} delegate={})",
      self.name, self.delegate
    )
  }
}

impl<I> TryClone for MockAssertFileExistIndexInput<I>
where
  I: IndexInput,
{
  fn try_clone(&self) -> Result<Self>
  where
    Self: Sized,
  {
    Ok(Self::new(
      self.name.clone(),
      self.delegate.try_clone()?,
      self.file_path.clone(),
    ))
  }
}

impl<I> IndexInput for MockAssertFileExistIndexInput<I>
where
  I: IndexInput<IndexInput = I>,
{
  type IndexInput = MockAssertFileExistIndexInput<I>;

  fn get_file_pointer(&self) -> Result<usize> {
    self.delegate.get_file_pointer()
  }

  fn seek(&mut self, pos: usize) -> Result<()> {
    self.check_file_exists()?;
    self.delegate.seek(pos)
  }

  fn length(&self) -> Result<usize> {
    self.delegate.length()
  }

  fn slice(
    &self,
    slice_description: &str,
    offset: usize,
    length: usize,
  ) -> Result<Self::IndexInput> {
    self.check_file_exists()?;
    let slice = self.delegate.slice(slice_description, offset, length)?;
    Ok(Self::new(
      slice_description.to_string(),
      slice,
      self.file_path.clone(),
    ))
  }

  type RandomAccessSlice = RandomAccessInputWrapper<MockAssertFileExistIndexInput<I>>;

  fn random_access_slice(&self, offset: usize, length: usize) -> Result<Self::RandomAccessSlice> {
    Ok(RandomAccessInputWrapper::new(self.slice(
      "randomaccess",
      offset,
      length,
    )?))
  }
}

#[derive(Clone)]
pub struct OnlyForceMergeMergePolicy {
  base: TieredMergePolicy,
}

impl OnlyForceMergeMergePolicy {
  fn new(base: TieredMergePolicy) -> Self {
    Self { base }
  }
}

impl From<OnlyForceMergeMergePolicy> for MergePolicyEnum {
  fn from(value: OnlyForceMergeMergePolicy) -> Self {
    Self::OnlyForceMerge(value)
  }
}

impl Display for OnlyForceMergeMergePolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.base)
  }
}

impl MergePolicy for OnlyForceMergeMergePolicy {
  fn get_base(&self) -> &MergePolicyBase {
    self.base.get_base()
  }

  fn get_base_mut(&mut self) -> &mut MergePolicyBase {
    self.base.get_base_mut()
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
    // only allow force merge
    Ok(None)
  }

  fn find_merges_readers<CR, D>(
    &self,
    readers: Vec<CR>,
  ) -> Result<Option<MergeSpecification<D, CR>>>
  where
    CR: CodecReader,
    D: Directory,
  {
    self.base.find_merges_readers(readers)
  }

  fn find_forced_merges<D, MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    max_segment_count: usize,
    segments_to_merge: &HashMap<String, Option<bool>>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self.base.find_forced_merges(
      segment_infos,
      max_segment_count,
      segments_to_merge,
      inner,
      merge_context,
    )
  }

  fn find_forced_deletes_merges<D, MC>(
    &self,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self
      .base
      .find_forced_deletes_merges(segment_infos, inner, merge_context)
  }

  fn find_full_flush_merges<D, MC>(
    &self,
    merge_trigger: MergeTrigger,
    segment_infos: &SegmentInfos<D>,
    inner: Option<&Inner<D>>,
    merge_context: &MC,
  ) -> Result<Option<MergeSpecificationNoReader<D>>>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self
      .base
      .find_full_flush_merges(merge_trigger, segment_infos, inner, merge_context)
  }

  fn use_compound_file<D, MC>(
    &self,
    infos: &SegmentInfos<D>,
    merged_info: &SegmentCommitInfo<D>,
    merge_context: &MC,
  ) -> Result<bool>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self
      .base
      .use_compound_file(infos, merged_info, merge_context)
  }

  fn size<D, MC>(&self, info: &SegmentCommitInfo<D>, merge_context: &MC) -> Result<i64>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self.base.size(info, merge_context)
  }

  fn max_full_flush_merge_size(&self) -> i64 {
    self.base.max_full_flush_merge_size()
  }

  fn has_merged<D, MC>(
    &self,
    infos: &SegmentInfos<D>,
    info: &SegmentCommitInfo<D>,
    merge_context: &MC,
  ) -> Result<bool>
  where
    D: Directory,
    MC: MergeContext<D>,
  {
    self.base.has_merged(infos, info, merge_context)
  }

  fn keep_fully_deleted_segment<D, F>(&self, reader_supplier: F) -> Result<bool>
  where
    D: Directory,
    F: Fn() -> Result<DefaultLeafReader<D>>,
  {
    self.base.keep_fully_deleted_segment(reader_supplier)
  }

  fn num_deletes_to_merge<D, F>(
    &self,
    info: &SegmentCommitInfo<D>,
    del_count: i32,
    reader_supplier: F,
  ) -> Result<i32>
  where
    D: Directory,
    F: Fn() -> Result<DefaultLeafReader<D>>,
  {
    self
      .base
      .num_deletes_to_merge(info, del_count, reader_supplier)
  }

  fn seg_string<MC, D>(&self, merge_context: &MC, infos: &[SegmentCommitInfo<D>]) -> String
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self.base.seg_string(merge_context, infos)
  }

  fn message<MC, D>(&self, message: &str, merge_context: &MC) -> Result<()>
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self.base.message(message, merge_context)
  }

  fn verbose<MC, D>(&self, merge_context: &MC) -> bool
  where
    MC: MergeContext<D>,
    D: Directory,
  {
    self.base.verbose(merge_context)
  }
}
