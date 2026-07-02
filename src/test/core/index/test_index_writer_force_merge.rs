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
use crate::core::analysis::analyzer::{Analyzer, AnalyzerStoredValue, TokenStreamComponents};
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::string_field::StringField;
use crate::core::index::composite_reader::get_context;
use crate::core::index::concurrent_merge_scheduler::ConcurrentMergeScheduler;
use crate::core::index::directory_reader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::log_merge_policy::LogMergePolicy;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::analysis::mock_tokenizer::{MockTokenizer, WHITESPACE};
use crate::test_framework::core::index::test_index_writer::add_doc_with_index;
use crate::test_framework::core::util::lucene_test_case::{
  is_night_mode, new_directory_shared, new_index_writer_config_with_analyzer, new_log_merge_policy,
  new_log_merge_policy_with_merge_factor, new_mock_directory, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::prelude::StdRng;
use rand::{Rng, RngExt, SeedableRng};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[allow(dead_code)] // for quick search
struct TestIndexWriterForceMerge;

#[test]
fn test_partial_merge() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("content", "aaa", Store::No)?);

  let incr_min = if is_night_mode() { 15 } else { 40 };
  let mut num_docs = 10;
  while num_docs < 500 {
    let mut ldmp = LogMergePolicy::log_doc();
    ldmp.set_min_merge_docs(1);
    ldmp.set_merge_factor(5)?;

    let mock = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
    iwc.set_open_mode(OpenMode::Create);
    iwc.set_max_buffered_docs(2);
    iwc.set_merge_policy(ldmp);
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    for _ in 0..num_docs {
      writer.add_document(doc.clone())?;
    }
    writer.close()?;
    drop(writer);

    let sis = SegmentInfos::read_latest_commit(dir.clone())?;
    let seg_count = sis.size();

    let mut ldmp = LogMergePolicy::log_doc();
    ldmp.set_merge_factor(5)?;

    let mock = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
    iwc.set_merge_policy(ldmp);
    let writer = IndexWriter::new(dir.clone(), iwc)?;
    writer.force_merge(3)?;
    writer.close()?;

    let sis = SegmentInfos::read_latest_commit(dir.clone())?;
    let opt_seg_count = sis.size();

    if seg_count < 3 {
      assert_eq!(seg_count, opt_seg_count);
    } else {
      assert_eq!(3, opt_seg_count);
    }

    num_docs += TestUtil::next_int(&mut random, incr_min, 5 * incr_min) as usize;
  }

  Ok(())
}

#[test]
fn test_max_num_segments2() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("content", "aaa", Store::No)?);

  let mut ldmp = LogMergePolicy::log_doc();
  ldmp.set_min_merge_docs(1);
  ldmp.set_merge_factor(4)?;

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  iwc.set_max_buffered_docs(2);
  iwc.set_merge_policy(ldmp);
  iwc.set_merge_scheduler(ConcurrentMergeScheduler::new());
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  for _ in 0..10 {
    for _ in 0..19 {
      writer.add_document(doc.clone())?;
    }

    writer.commit()?;
    writer.wait_for_merges()?;
    writer.commit()?;

    let sis = SegmentInfos::read_latest_commit(dir.clone())?;
    let seg_count = sis.size();

    writer.force_merge(7)?;
    writer.commit()?;
    writer.wait_for_merges()?;

    let sis = SegmentInfos::read_latest_commit(dir.clone())?;
    let opt_seg_count = sis.size();

    if seg_count < 7 {
      assert_eq!(seg_count, opt_seg_count);
    } else {
      assert_eq!(7, opt_seg_count, "seg: {seg_count}");
    }
  }

  writer.close()?;

  Ok(())
}
#[test]
fn test_force_merge_temp_space_usage() -> Result<()> {
  let mut random = random();
  let dir = new_mock_directory(&mut random)?;
  // don't use MockAnalyzer, variable length payloads can cause merge to make things bigger,
  // since things are optimized for fixed length case. this is a problem for MemoryPF's encoding.
  // (it might have other problems too)
  let analyzer = ForceMergeTempSpaceUsageAnalyzer::new(&mut random);
  let mut iwc =
    new_index_writer_config_with_analyzer(&mut random, Box::new(analyzer) as Box<dyn Analyzer>)?;
  iwc.set_max_buffered_docs(10);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let writer = IndexWriter::new(Arc::new(dir.clone()), iwc)?;
  let mut field_types = HashMap::new();

  for j in 0..500 {
    add_doc_with_index(&mut random, &writer, j, &mut field_types)?;
  }
  // force one extra segment w/ different doc store so
  // we see the doc stores get merged
  writer.commit()?;
  add_doc_with_index(&mut random, &writer, 500, &mut field_types)?;
  writer.close()?;
  drop(writer);
  let mut start_disk_usage = 0;
  for file in dir.list_all()? {
    start_disk_usage += dir.file_length(&file)?;
  }
  let start_listing = dir.list_all()?;

  dir.reset_max_used_size_in_bytes()?;
  dir.set_track_disk_usage(true);

  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  iwc.set_open_mode(OpenMode::Append);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let writer = IndexWriter::new(Arc::new(dir.clone()), iwc)?;

  writer.force_merge(1)?;
  writer.close()?;

  let mut final_disk_usage = 0;
  for file in dir.list_all()? {
    final_disk_usage += dir.file_length(&file)?;
  }

  // The result of the merged index is often smaller, but sometimes it could
  // be bigger (compression slightly changes, Codec changes etc.). Therefore
  // we compare the temp space used to the max of the initial and final index
  // size
  let max_start_final_disk_usage = start_disk_usage.max(final_disk_usage);
  let max_disk_usage = dir.get_max_used_size_in_bytes() as usize;
  assert!(
    max_disk_usage <= 4 * max_start_final_disk_usage,
    "forceMerge used too much temporary space: starting usage was {start_disk_usage} bytes; final usage was {final_disk_usage} bytes; max temp usage was {max_disk_usage} but should have been at most {} (= 4X starting usage), BEFORE={start_listing:?}AFTER={:?}",
    4 * max_start_final_disk_usage,
    dir.list_all()?
  );

  Ok(())
}

struct ForceMergeTempSpaceUsageAnalyzer {
  random: Mutex<StdRng>,
  stored_value: AnalyzerStoredValue,
}

impl ForceMergeTempSpaceUsageAnalyzer {
  fn new<R>(random: &mut R) -> Self
  where
    R: Rng + ?Sized,
  {
    Self {
      random: Mutex::new(StdRng::seed_from_u64(random.random())),
      stored_value: AnalyzerStoredValue::new(),
    }
  }

  fn next_random(&self) -> StdRng {
    StdRng::seed_from_u64(self.random.lock().expect("random mutex poisoned").random())
  }
}

impl Analyzer for ForceMergeTempSpaceUsageAnalyzer {
  fn create_components(&self, _field_name: &str) -> Result<TokenStreamComponents> {
    Ok(TokenStreamComponents::new(
      Box::new(MockTokenizer::with_default_max_token_length(
        self.next_random(),
        WHITESPACE.clone(),
        true,
      )) as Box<dyn TokenStream + Send + Sync>,
      None,
    ))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

crate::impl_analyzer_close!(ForceMergeTempSpaceUsageAnalyzer);

#[test]
fn test_background_force_merge() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  for pass in 0..2 {
    let mock = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
    iwc.set_open_mode(OpenMode::Create);
    iwc.set_max_buffered_docs(2);
    iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 51)?);
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(StringField::from_string("field", "aaa", Store::No)?);

    for _ in 0..100 {
      writer.add_document(doc.clone())?;
    }

    // TODO IMPORTANT: forceMerge(maxNumSegments, doWait=false) 未实现
    writer.force_merge(1)?;

    if pass == 0 {
      writer.close()?;
      let reader = get_context(directory_reader::open(dir.clone())?)?;
      assert_eq!(1, reader.leaves()?.len());
    } else {
      // Get another segment to flush so we can verify it is NOT included in the merging.
      writer.add_document(doc.clone())?;
      writer.add_document(doc.clone())?;
      writer.close()?;

      let reader = get_context(directory_reader::open(dir.clone())?)?;
      assert!(reader.leaves()?.len() > 1);

      let infos = SegmentInfos::read_latest_commit(dir.clone())?;
      assert_eq!(2, infos.size());
    }
  }

  Ok(())
}
#[test]
fn test_merge_per_field() -> Result<()> {
  // TODO set_codec 未实现
  Ok(())
}
