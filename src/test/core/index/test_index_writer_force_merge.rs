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
use crate::core::document::string_field::StringField;
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::log_merge_policy::LogMergePolicy;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::{
  is_night_mode, new_directory_shared, new_index_writer_config_with_analyzer,
  new_log_merge_policy_with_merge_factor, random,
};
use crate::test::core::util::test_util::TestUtil;

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
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
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
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
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
  // TODO IMPORTANT ConcurrentMergeScheduler未实现
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  iwc.set_max_buffered_docs(2);
  iwc.set_merge_policy(ldmp);
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
  // TODO IMPORTANT MockDirectoryWrapper未实现
  Ok(())
}

#[test]
fn test_background_force_merge() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  for pass in 0..2 {
    let mock = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock);
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
