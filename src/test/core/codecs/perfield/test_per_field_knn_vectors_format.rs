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
use crate::core::codecs::KnnVectorsFormats;
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_format::Lucene99HnswVectorsFormat;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::knn_float_vector_field::KnnFloatVectorField;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::knn_float_vector_query::KnnFloatVectorQuery;
use crate::core::search::top_knn_collector::TopKnnCollector;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::codecs::asserting_codec::{AssertingCodec, AssertingCodecHook};
use crate::test_framework::core::codecs::perfield::test_per_field_knn_vectors_format::{
  KnnVectorsFormatMaxDims32, MaxDimensionsPerFieldFormatAssertingCodec,
  MergeUsesNewFormatAssertingCodec, TwoFieldsTwoFormatsAssertingCodec,
  WriteRecordingKnnVectorsFormat,
};
use crate::test_framework::core::util::lucene_test_case::{
  get_only_leaf_reader, new_directory_shared, new_index_writer_config, new_searcher_with_reader,
  new_text_field, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use std::collections::{HashMap, HashSet};

/// Basic tests of PerFieldKnnVectorsFormat.
#[allow(dead_code)] // for quick search
struct TestPerFieldKnnVectorsFormat;

#[test]
fn test_missing_field_returns_no_results() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut config = new_index_writer_config(&mut random)?;
  config.set_codec(TestUtil::always_knn_vectors_format(
    TestUtil::get_default_knn_vectors_format()?,
  ));
  let writer = IndexWriter::new(dir.clone(), config)?;
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut HashMap::new(),
  )?);
  writer.add_document(doc)?;
  writer.close()?;

  {
    let reader = directory_reader::open(dir.clone())?;
    let leaf = get_only_leaf_reader(reader)?;
    let mut collector = TopKnnCollector::new(10, i32::MAX as usize)?;
    LeafReader::search_nearest_vectors_f32(
      &leaf,
      "missing_field",
      vec![1.0, 2.0, 3.0],
      &mut collector,
      leaf.get_live_docs()?,
    )?;
    assert_eq!(0, collector.top_docs()?.score_docs.len());

    let mut collector = TopKnnCollector::new(10, i32::MAX as usize)?;
    LeafReader::search_nearest_vectors_f32(
      &leaf,
      "id",
      vec![1.0, 2.0, 3.0],
      &mut collector,
      leaf.get_live_docs()?,
    )?;
    assert_eq!(0, collector.top_docs()?.score_docs.len());
  }
  dir.close()
}

#[test]
fn test_two_fields_two_formats() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  // we don't use RandomIndexWriter because it might add more values than we expect !!!!1
  let mut config = new_index_writer_config(&mut random)?;
  let format1 = WriteRecordingKnnVectorsFormat::new(TestUtil::get_default_knn_vectors_format()?);
  let format2 = WriteRecordingKnnVectorsFormat::new(TestUtil::get_default_knn_vectors_format()?);
  config.set_codec(AssertingCodec::with_hook(
    AssertingCodecHook::TwoFieldsTwoFormats(TwoFieldsTwoFormatsAssertingCodec::new(
      format1.clone().into(),
      format2.clone().into(),
    )),
  ));

  let writer = IndexWriter::new(dir.clone(), config)?;
  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "id",
    "1",
    Store::Yes,
    &mut field_to_type,
  )?);
  doc.add(KnnFloatVectorField::new("field1", vec![1.0, 2.0, 3.0])?);
  writer.add_document(doc)?;

  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "id",
    "2",
    Store::Yes,
    &mut field_to_type,
  )?);
  doc.add(KnnFloatVectorField::new("field2", vec![4.0, 5.0, 6.0])?);
  writer.add_document(doc)?;
  writer.close()?;

  // Check that each format was used to write the expected field
  assert_eq!(
    HashSet::from(["field1".to_string()]),
    format1.fields_written()
  );
  assert_eq!(
    HashSet::from(["field2".to_string()]),
    format2.fields_written()
  );

  // Double-check the vectors were written
  {
    let reader = directory_reader::open(dir.clone())?;
    let leaf = get_only_leaf_reader(reader)?;
    let mut collector = TopKnnCollector::new(10, i32::MAX as usize)?;
    LeafReader::search_nearest_vectors_f32(
      &leaf,
      "field1",
      vec![1.0, 2.0, 3.0],
      &mut collector,
      leaf.get_live_docs()?,
    )?;
    assert_eq!(1, collector.top_docs()?.score_docs.len());

    let mut collector = TopKnnCollector::new(10, i32::MAX as usize)?;
    LeafReader::search_nearest_vectors_f32(
      &leaf,
      "field2",
      vec![1.0, 2.0, 3.0],
      &mut collector,
      leaf.get_live_docs()?,
    )?;
    assert_eq!(1, collector.top_docs()?.score_docs.len());
  }
  dir.close()
}

#[test]
fn test_merge_uses_new_format() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut initial_config = new_index_writer_config(&mut random)?;
  initial_config.set_merge_policy(NoMergePolicy::default());

  let writer = IndexWriter::new(dir.clone(), initial_config)?;
  let mut field_to_type = HashMap::new();
  for _ in 0..3 {
    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "id",
      "1",
      Store::Yes,
      &mut field_to_type,
    )?);
    doc.add(KnnFloatVectorField::new("field1", vec![1.0, 2.0, 3.0])?);
    doc.add(KnnFloatVectorField::new("field2", vec![1.0, 2.0, 3.0])?);
    writer.add_document(doc)?;
    writer.commit()?;
  }
  writer.close()?;

  let mut new_config = new_index_writer_config(&mut random)?;
  let format1 = WriteRecordingKnnVectorsFormat::new(TestUtil::get_default_knn_vectors_format()?);
  let format2 = WriteRecordingKnnVectorsFormat::new(TestUtil::get_default_knn_vectors_format()?);
  new_config.set_codec(AssertingCodec::with_hook(
    AssertingCodecHook::MergeUsesNewFormat(MergeUsesNewFormatAssertingCodec::new(
      format1.clone().into(),
      format2.clone().into(),
    )),
  ));

  let writer = IndexWriter::new(dir.clone(), new_config)?;
  writer.force_merge(1)?;
  writer.close()?;

  // Check that the new format was used while merging
  assert_eq!(
    HashSet::from(["field1".to_string()]),
    format1.fields_written()
  );
  assert_eq!(
    HashSet::from(["field2".to_string()]),
    format2.fields_written()
  );
  dir.close()
}

#[test]
fn test_max_dimensions_per_field_format() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut config = new_index_writer_config(&mut random)?;
  let format1 =
    KnnVectorsFormatMaxDims32::new(Lucene99HnswVectorsFormat::with_graph_para(16, 100)?);
  let format2: KnnVectorsFormats = Lucene99HnswVectorsFormat::with_graph_para(16, 100)?.into();
  config.set_codec(AssertingCodec::with_hook(
    AssertingCodecHook::MaxDimensionsPerFieldFormat(
      MaxDimensionsPerFieldFormatAssertingCodec::new(format1.into(), format2.into()),
    ),
  ));

  let writer = IndexWriter::new(dir.clone(), config)?;
  let mut doc1 = Document::new();
  doc1.add(KnnFloatVectorField::new("field1", vec![0.0; 33])?);
  let error = writer.add_document(doc1).unwrap_err();
  assert!(matches!(error, LuceneError::IllegalArgument(_)));
  assert!(
    error
      .to_string()
      .contains("vector's dimensions must be <= [32]")
  );

  let mut doc2 = Document::new();
  doc2.add(KnnFloatVectorField::new("field1", vec![0.0; 32])?);
  doc2.add(KnnFloatVectorField::new("field2", vec![0.0; 33])?);
  writer.add_document(doc2)?;
  writer.close()?;

  // Check that the vectors were written
  {
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    let query1 = KnnFloatVectorQuery::new("field1", vec![0.0; 32], 10)?;
    let top_docs1 = searcher.search(query1, 1)?;
    assert_eq!(1, top_docs1.score_docs.len());

    let query2 = KnnFloatVectorQuery::new("field2", vec![0.0; 33], 10)?;
    let top_docs2 = searcher.search(query2, 1)?;
    assert_eq!(1, top_docs2.score_docs.len());
    searcher.get_index_reader().close()?;
  }
  dir.close()
}
