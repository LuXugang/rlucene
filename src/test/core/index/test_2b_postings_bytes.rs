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
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::document::Document;
use crate::core::document::field::Field;
use crate::core::document::field_type::FieldType;
use crate::core::document::fields::FieldTokenStreamEnum;
use crate::core::document::text_field::TYPE_NOT_STORED;
use crate::core::index::concurrent_merge_scheduler::ConcurrentMergeScheduler;
use crate::core::index::directory_reader;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicyEnum;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::store::mock_directory_wrapper::Throttling;
use crate::test_framework::core::util::lucene_test_case::{
  create_temp_dir_with_prefix, new_fs_directory, new_index_writer_config,
  new_index_writer_config_with_analyzer, new_log_merge_policy_with_merge_factor, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use std::sync::Arc;

/// Tests indexing two billion documents with about 65K frequencies each, producing more than
/// `i32::MAX` bytes of postings data for the term.
#[allow(dead_code)] // for quick search
struct Test2BPostingsBytes;

#[cfg(feature = "monster")]
#[test]
#[ignore = "monster"]
fn test() -> Result<()> {
  let mut random = random();
  let dir = new_fs_directory(
    &mut random,
    create_temp_dir_with_prefix("2BPostingsBytes1")?,
  )?;
  if let crate::core::store::directory::DirEnum::B(dir) = dir.as_ref() {
    dir.set_throttling(Throttling::Never);
  }
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  let mut merge_policy = new_log_merge_policy_with_merge_factor(&mut random, 10)?;
  if let MergePolicyEnum::LogBytesSize(policy) = &mut merge_policy {
    // 1 petabyte:
    policy.set_max_merge_mb(1024.0 * 1024.0 * 1024.0);
  }
  iwc
    .set_max_buffered_docs(-1)
    .set_ram_buffer_size_mb(256.0)
    .set_merge_scheduler(ConcurrentMergeScheduler::new())
    .set_merge_policy(merge_policy)
    .set_open_mode(OpenMode::Create)
    .set_codec(TestUtil::get_default_codec());
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let mut field_type = FieldType::from_ref(&*TYPE_NOT_STORED)?;
  field_type.set_index_options(IndexOptions::DocsAndFreqs)?;
  field_type.set_omit_norms(true)?;

  let num_docs = 1000;
  for i in 0..num_docs {
    let n = if i % 2 == 1 {
      // Trick blockPF's small optimization.
      65536
    } else {
      65537
    };
    let mut doc = Document::new();
    doc.add(Field::from_token_stream(
      "field",
      FieldTokenStreamEnum::custom(MyTokenStream::new(n)),
      field_type.clone(),
    )?);
    writer.add_document(doc)?;
  }
  writer.force_merge(1)?;
  writer.close()?;

  let one_thousand = directory_reader::open(dir.clone())?;
  let sub_readers = vec![&one_thousand; 1000];
  let dir2 = new_fs_directory(
    &mut random,
    create_temp_dir_with_prefix("2BPostingsBytes2")?,
  )?;
  if let crate::core::store::directory::DirEnum::B(dir) = dir2.as_ref() {
    dir.set_throttling(Throttling::Never);
  }
  let writer2 = IndexWriter::new(dir2.clone(), new_index_writer_config(&mut random)?)?;
  TestUtil::add_indexes_slowly(&writer2, &sub_readers)?;
  writer2.force_merge(1)?;
  writer2.close()?;
  one_thousand.close()?;

  let one_million = directory_reader::open(dir2.clone())?;
  let sub_readers = vec![&one_million; 2000];
  let dir3 = new_fs_directory(
    &mut random,
    create_temp_dir_with_prefix("2BPostingsBytes3")?,
  )?;
  if let crate::core::store::directory::DirEnum::B(dir) = dir3.as_ref() {
    dir.set_throttling(Throttling::Never);
  }
  let writer3 = IndexWriter::new(dir3.clone(), new_index_writer_config(&mut random)?)?;
  TestUtil::add_indexes_slowly(&writer3, &sub_readers)?;
  writer3.force_merge(1)?;
  writer3.close()?;
  one_million.close()?;

  dir.as_ref().close()?;
  dir2.as_ref().close()?;
  dir3.as_ref().close()?;
  Ok(())
}

struct MyTokenStream {
  attrs: Attributes,
  index: i32,
  n: i32,
}

impl MyTokenStream {
  fn new(n: i32) -> Self {
    Self {
      attrs: Attributes::default(),
      index: 0,
      n,
    }
  }
}

impl Closeable for MyTokenStream {}

impl TokenStream for MyTokenStream {
  fn increment_token(&mut self) -> Result<bool> {
    if self.index < self.n {
      self.attrs.clear_attributes()?;
      self.attrs.append_str(Some("a"))?;
      self.index += 1;
      return Ok(true);
    }
    Ok(false)
  }

  fn reset(&mut self) -> Result<()> {
    self.index = 0;
    Ok(())
  }

  fn end(&mut self) -> Result<()> {
    self.default_end()
  }

  fn get_attribute_source(&self) -> &Attributes {
    &self.attrs
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    &mut self.attrs
  }
}
