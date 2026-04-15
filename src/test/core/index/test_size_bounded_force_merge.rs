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
use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
use crate::core::index::index_writer_config::{
  DEFAULT_RAM_BUFFER_SIZE_MB, DISABLE_AUTO_FLUSH, IndexWriterConfig,
};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::log_merge_policy::LogMergePolicy;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::term::Term;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config, random,
};
use rand::Rng;

#[allow(dead_code)] // for quick search
pub struct TestSizeBoundedForceMerge;

fn add_docs<D, L, B>(writer: &mut IndexWriter<D, L, B>, num_docs: i32) -> Result<()>
where
  D: Directory,
  L: LiveIndexWriterConfig,
  B: IndexWriterBase,
{
  add_docs_with_id(writer, num_docs, false)
}

fn add_docs_with_id<D, L, B>(
  writer: &mut IndexWriter<D, L, B>,
  num_docs: i32,
  with_id: bool,
) -> Result<()>
where
  D: Directory,
  L: LiveIndexWriterConfig,
  B: IndexWriterBase,
{
  for i in 0..num_docs {
    let mut doc = Document::new();
    if with_id {
      doc.add(StringField::from_string("id", i.to_string(), Store::No)?);
    }
    writer.add_document(doc)?;
  }
  writer.commit()?;
  Ok(())
}

fn new_writer_config<R>(random: &mut R) -> IndexWriterConfig
where
  R: Rng + ?Sized,
{
  let mut conf = new_index_writer_config(random);
  conf.set_max_buffered_docs(DISABLE_AUTO_FLUSH);
  conf.set_ram_buffer_size_mb(DEFAULT_RAM_BUFFER_SIZE_MB);
  conf.set_use_compound_file(false);
  conf.set_merge_policy(NoMergePolicy::default());
  conf
}
#[test]
fn test_byte_size_limit() -> Result<()> {
  let mut random = random();
  // TODO ByteBuffersDirectory未实现
  let dir = new_directory_shared(&mut random)?;

  let conf = new_writer_config(&mut random);
  let num_segments = 15;
  {
    let mut writer = IndexWriter::new(dir.clone(), conf)?;

    for i in 0..num_segments {
      let num_docs = if i == 7 { 30 } else { 1 };
      add_docs(&mut writer, num_docs)?;
    }
    writer.close()?;
  }

  let mut sis = SegmentInfos::read_latest_commit(dir.clone())?;
  let mut number_of_segments_of_minimum_size = 1;
  for i in 1..sis.size() {
    if sis.info(i).unwrap().size_in_bytes()? == sis.info(0).unwrap().size_in_bytes()? {
      number_of_segments_of_minimum_size += 1;
    }
  }
  assert_eq!(num_segments - 1, number_of_segments_of_minimum_size);

  let min = sis.info(0).unwrap().size_in_bytes()? as f64;

  let mut conf = new_writer_config(&mut random);
  let mut lmp = LogMergePolicy::log_bytes_size();
  lmp.set_max_merge_mb_for_forced_merge(min / ((1 << 20) as f64));
  conf.set_merge_policy(lmp);

  let writer = IndexWriter::new(dir.clone(), conf)?;
  writer.force_merge(1)?;
  writer.close()?;

  sis = SegmentInfos::read_latest_commit(dir)?;
  assert_eq!(3, sis.size());

  Ok(())
}

#[test]
fn test_num_docs_limit() -> Result<()> {
  let mut random = random();
  // TODO ByteBuffersDirectory未实现
  let dir = new_directory_shared(&mut random)?;

  {
    let conf = new_writer_config(&mut random);
    let mut writer = IndexWriter::new(dir.clone(), conf)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 5)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    writer.close()?;
  }

  let mut conf = new_writer_config(&mut random);
  let mut lmp = LogMergePolicy::log_doc();
  lmp.set_max_merge_docs(3);
  conf.set_merge_policy(lmp);

  let writer = IndexWriter::new(dir.clone(), conf)?;
  writer.force_merge(1)?;
  writer.close()?;

  let sis = SegmentInfos::read_latest_commit(dir)?;
  assert_eq!(3, sis.size());
  Ok(())
}

#[test]
fn test_last_segment_too_large() -> Result<()> {
  let mut random = random();
  // TODO ByteBuffersDirectory未实现
  let dir = new_directory_shared(&mut random)?;

  {
    let conf = new_writer_config(&mut random);
    let mut writer = IndexWriter::new(dir.clone(), conf)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 5)?;
    writer.close()?;
  }

  let mut conf = new_writer_config(&mut random);
  let mut lmp = LogMergePolicy::log_doc();
  lmp.set_max_merge_docs(3);
  conf.set_merge_policy(lmp);

  let writer = IndexWriter::new(dir.clone(), conf)?;
  writer.force_merge(1)?;
  writer.close()?;

  let sis = SegmentInfos::read_latest_commit(dir)?;
  assert_eq!(2, sis.size());
  Ok(())
}

#[test]
fn test_first_segment_too_large() -> Result<()> {
  let mut random = random();
  // TODO ByteBuffersDirectory未实现
  let dir = new_directory_shared(&mut random)?;

  {
    let conf = new_writer_config(&mut random);
    let mut writer = IndexWriter::new(dir.clone(), conf)?;
    add_docs(&mut writer, 5)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    writer.close()?;
  }

  let mut conf = new_writer_config(&mut random);
  let mut lmp = LogMergePolicy::log_doc();
  lmp.set_max_merge_docs(3);
  conf.set_merge_policy(lmp);

  let writer = IndexWriter::new(dir.clone(), conf)?;
  writer.force_merge(1)?;
  writer.close()?;

  let sis = SegmentInfos::read_latest_commit(dir)?;
  assert_eq!(2, sis.size());
  Ok(())
}

#[test]
fn test_all_segments_small() -> Result<()> {
  let mut random = random();
  // TODO ByteBuffersDirectory未实现
  let dir = new_directory_shared(&mut random)?;

  {
    let conf = new_writer_config(&mut random);
    let mut writer = IndexWriter::new(dir.clone(), conf)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    writer.close()?;
  }

  let mut conf = new_writer_config(&mut random);
  let mut lmp = LogMergePolicy::log_doc();
  lmp.set_max_merge_docs(3);
  conf.set_merge_policy(lmp);

  let writer = IndexWriter::new(dir.clone(), conf)?;
  writer.force_merge(1)?;
  writer.close()?;

  let sis = SegmentInfos::read_latest_commit(dir)?;
  assert_eq!(1, sis.size());
  Ok(())
}

#[test]
fn test_all_segments_large() -> Result<()> {
  let mut random = random();
  // TODO ByteBuffersDirectory未实现
  let dir = new_directory_shared(&mut random)?;

  {
    let conf = new_writer_config(&mut random);
    let mut writer = IndexWriter::new(dir.clone(), conf)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    writer.close()?;
  }

  let mut conf = new_writer_config(&mut random);
  let mut lmp = LogMergePolicy::log_doc();
  lmp.set_max_merge_docs(2);
  conf.set_merge_policy(lmp);

  let writer = IndexWriter::new(dir.clone(), conf)?;
  writer.force_merge(1)?;
  writer.close()?;

  let sis = SegmentInfos::read_latest_commit(dir)?;
  assert_eq!(3, sis.size());
  Ok(())
}

#[test]
fn test_one_large_one_small() -> Result<()> {
  let mut random = random();
  // TODO ByteBuffersDirectory未实现
  let dir = new_directory_shared(&mut random)?;

  {
    let conf = new_writer_config(&mut random);
    let mut writer = IndexWriter::new(dir.clone(), conf)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 5)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 5)?;
    writer.close()?;
  }

  let mut conf = new_writer_config(&mut random);
  let mut lmp = LogMergePolicy::log_doc();
  lmp.set_max_merge_docs(3);
  conf.set_merge_policy(lmp);

  let writer = IndexWriter::new(dir.clone(), conf)?;
  writer.force_merge(1)?;
  writer.close()?;

  let sis = SegmentInfos::read_latest_commit(dir)?;
  assert_eq!(4, sis.size());
  Ok(())
}

#[test]
fn test_merge_factor() -> Result<()> {
  let mut random = random();
  // TODO ByteBuffersDirectory未实现
  let dir = new_directory_shared(&mut random)?;

  {
    let conf = new_writer_config(&mut random);
    let mut writer = IndexWriter::new(dir.clone(), conf)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 5)?;
    add_docs(&mut writer, 3)?;
    add_docs(&mut writer, 3)?;
    writer.close()?;
  }

  let mut conf = new_writer_config(&mut random);
  let mut lmp = LogMergePolicy::log_doc();
  lmp.set_max_merge_docs(3);
  lmp.set_merge_factor(2)?;
  conf.set_merge_policy(lmp);

  let writer = IndexWriter::new(dir.clone(), conf)?;
  writer.force_merge(1)?;
  writer.close()?;

  let sis = SegmentInfos::read_latest_commit(dir)?;
  assert_eq!(4, sis.size());
  Ok(())
}

#[test]
fn test_single_mergeable_segment() -> Result<()> {
  let mut random = random();
  // TODO ByteBuffersDirectory未实现
  let dir = new_directory_shared(&mut random)?;

  {
    let conf = new_writer_config(&mut random);
    let mut writer = IndexWriter::new(dir.clone(), conf)?;
    add_docs_with_id(&mut writer, 3, true)?;
    add_docs_with_id(&mut writer, 5, true)?;
    add_docs_with_id(&mut writer, 3, true)?;
    writer.delete_documents_with_terms(vec![Term::from_text("id", "10")])?;
    writer.close()?;
  }

  let mut conf = new_writer_config(&mut random);
  let mut lmp = LogMergePolicy::log_doc();
  lmp.set_max_merge_docs(3);
  conf.set_merge_policy(lmp);

  let writer = IndexWriter::new(dir.clone(), conf)?;
  writer.force_merge(1)?;
  writer.close()?;

  let sis = SegmentInfos::read_latest_commit(dir)?;
  assert_eq!(3, sis.size());
  assert!(!sis.info(2).unwrap().has_deletions());
  Ok(())
}

#[test]
fn test_single_non_mergeable_segment() -> Result<()> {
  let mut random = random();
  // TODO ByteBuffersDirectory未实现
  let dir = new_directory_shared(&mut random)?;

  {
    let conf = new_writer_config(&mut random);
    let mut writer = IndexWriter::new(dir.clone(), conf)?;
    add_docs_with_id(&mut writer, 3, true)?;
    writer.close()?;
  }

  let mut conf = new_writer_config(&mut random);
  let mut lmp = LogMergePolicy::log_doc();
  lmp.set_max_merge_docs(3);
  conf.set_merge_policy(lmp);

  let writer = IndexWriter::new(dir.clone(), conf)?;
  writer.force_merge(1)?;
  writer.close()?;

  let sis = SegmentInfos::read_latest_commit(dir)?;
  assert_eq!(1, sis.size());
  Ok(())
}

#[test]
fn test_single_mergeable_too_large_segment() -> Result<()> {
  let mut random = random();
  // TODO ByteBuffersDirectory未实现
  let dir = new_directory_shared(&mut random)?;

  {
    let conf = new_writer_config(&mut random);
    let mut writer = IndexWriter::new(dir.clone(), conf)?;
    add_docs_with_id(&mut writer, 5, true)?;
    writer.delete_documents_with_terms(vec![Term::from_text("id", "4")])?;
    writer.close()?;
  }

  let mut conf = new_writer_config(&mut random);
  let mut lmp = LogMergePolicy::log_doc();
  lmp.set_max_merge_docs(2);
  conf.set_merge_policy(lmp);

  let writer = IndexWriter::new(dir.clone(), conf)?;
  writer.force_merge(1)?;
  writer.close()?;

  let sis = SegmentInfos::read_latest_commit(dir)?;
  assert_eq!(1, sis.size());
  assert!(sis.info(0).unwrap().has_deletions());
  Ok(())
}
