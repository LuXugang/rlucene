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
use crate::core::document::text_field;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::parallel_composite_reader::ParallelCompositeReader;
use crate::core::index::parallel_leaf_reader::ParallelLeafReader;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::index::slow_codec_reader_wrapper::SlowCodecReaderWrapper;
use crate::core::index::term::Term;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::util::lucene_test_case::{
  get_only_leaf_reader, new_directory_from, new_directory_shared, new_field,
  new_index_writer_config_with_analyzer, new_text_field, random,
};
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestParallelReaderEmptyIndex;

/// Creates two empty indexes and wraps a ParallelReader around. Adding this reader to a new index
/// should not return an error.
#[test]
fn test_empty_index() -> Result<()> {
  let mut random = random();
  let rd1 = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let iw = IndexWriter::new(
    rd1.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer)?,
  )?;
  iw.close()?;
  // Create a copy.
  let rd2 = Arc::new(new_directory_from(&mut random, rd1.as_ref())?);

  let rd_out = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let iw_out = IndexWriter::new(
    rd_out.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer)?,
  )?;

  // Add a readerless parallel reader.
  let empty_reader = ParallelLeafReader::<DefaultLeafReader<DirEnum>>::new(Vec::new())?;
  iw_out
    .add_indexes_from_codec_readers(vec![SlowCodecReaderWrapper::wrap_leaf_reader(empty_reader)])?;
  iw_out.force_merge(1)?;

  let cpr = ParallelCompositeReader::new(vec![
    directory_reader::open(rd1.clone())?,
    directory_reader::open(rd2.clone())?,
  ])?;

  // When unpatched, Lucene crashes here with a NoSuchElementException (caused by
  // ParallelTermEnum).
  let context = (&cpr).get_context()?;
  let mut leaves = Vec::new();
  for leaf in context.leaves()? {
    leaves.push(SlowCodecReaderWrapper::wrap_leaf_reader(
      leaf.reader().clone(),
    ));
  }
  iw_out.add_indexes_from_codec_readers(leaves)?;
  iw_out.force_merge(1)?;

  iw_out.close()?;
  rd_out.close()?;
  rd1.close()?;
  rd2.close()
}

/// This method creates an empty index (numFields=0, numDocs=0) but is marked to have TermVectors.
/// Adding this index to another index should not return an error.
#[test]
fn test_empty_index_with_vectors() -> Result<()> {
  let mut random = random();
  let rd1 = new_directory_shared(&mut random)?;
  {
    let analyzer = MockAnalyzer::new(&mut random);
    let iw = IndexWriter::new(
      rd1.clone(),
      new_index_writer_config_with_analyzer(&mut random, analyzer)?,
    )?;
    let mut field_types = HashMap::<String, FieldType>::new();
    let mut custom_type = FieldType::from_ref(&*text_field::TYPE_NOT_STORED)?;
    custom_type.set_store_term_vectors(true)?;

    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "id",
      "1",
      Store::No,
      &mut field_types,
    )?);
    doc.add(new_field(
      &mut random,
      "test",
      "",
      &custom_type,
      &mut field_types,
    )?);
    iw.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "id",
      "2",
      Store::No,
      &mut field_types,
    )?);
    doc.add(new_field(
      &mut random,
      "test",
      "",
      &custom_type,
      &mut field_types,
    )?);
    doc.add(new_field(
      &mut random,
      "test",
      "",
      &custom_type,
      &mut field_types,
    )?);
    iw.add_document(doc)?;
    iw.close()?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut dont_merge_config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
    dont_merge_config.set_merge_policy(NoMergePolicy::default());
    let writer = IndexWriter::new(rd1.clone(), dont_merge_config)?;
    writer.delete_documents_with_terms(vec![Term::from_text("id", "1")])?;
    writer.close()?;

    let ir = directory_reader::open(rd1.clone())?;
    assert_eq!(2, ir.max_doc()?);
    assert_eq!(1, ir.num_docs()?);
    ir.close()?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
    config.set_open_mode(OpenMode::Append);
    let iw = IndexWriter::new(rd1.clone(), config)?;
    iw.force_merge(1)?;
    iw.close()?;
  }

  let rd2 = new_directory_shared(&mut random)?;
  {
    let analyzer = MockAnalyzer::new(&mut random);
    let iw = IndexWriter::new(
      rd2.clone(),
      new_index_writer_config_with_analyzer(&mut random, analyzer)?,
    )?;
    iw.add_document(Document::new())?;
    iw.close()?;
  }

  let rd_out = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let iw_out = IndexWriter::new(
    rd_out.clone(),
    new_index_writer_config_with_analyzer(&mut random, analyzer)?,
  )?;
  let reader1 = directory_reader::open(rd1.clone())?;
  let reader2 = directory_reader::open(rd2.clone())?;
  let pr = ParallelLeafReader::new_with_close_sub_readers(
    false,
    vec![
      get_only_leaf_reader(&reader1)?,
      get_only_leaf_reader(&reader2)?,
    ],
  )?;

  // When unpatched, Lucene crashes here with an ArrayIndexOutOfBoundsException (caused by
  // TermVectorsWriter).
  iw_out
    .add_indexes_from_codec_readers(vec![SlowCodecReaderWrapper::wrap_leaf_reader(pr.clone())])?;

  pr.close()?;
  reader1.close()?;
  reader2.close()?;

  // Assert subreaders were closed.
  assert_eq!(0, reader1.get_ref_count());
  assert_eq!(0, reader2.get_ref_count());

  rd1.close()?;
  rd2.close()?;

  iw_out.force_merge(1)?;
  iw_out.close()?;

  rd_out.close()
}
