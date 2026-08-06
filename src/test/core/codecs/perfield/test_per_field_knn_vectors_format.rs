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
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_format::Lucene99HnswVectorsFormat;
use crate::core::codecs::{Codecs, KnnVectorsFormats};
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
use crate::test_framework::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test_framework::core::index::base_knn_vectors_format_test_case::{
  BaseKnnVectorsFormatTestCase, BaseKnnVectorsFormatTestCaseState,
};
use crate::test_framework::core::index::random_codec::RandomCodec;
use crate::test_framework::core::util::lucene_test_case::{
  get_only_leaf_reader, new_directory_shared, new_index_writer_config, new_searcher_with_reader,
  new_text_field, random, random_from_seed,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::prelude::StdRng;
use rand::{Rng, RngExt};
use std::collections::{HashMap, HashSet};

/// Basic tests of PerFieldKnnVectorsFormat.
#[allow(dead_code)] // for quick search
struct TestPerFieldKnnVectorsFormat {
  codec: RandomCodec,
  base_knn_vectors_format_test_case_state: BaseKnnVectorsFormatTestCaseState,
}

impl TestPerFieldKnnVectorsFormat {
  fn new<R>(random: &mut R) -> Self
  where
    R: Rng + ?Sized,
  {
    let mut codec_random = random_from_seed(random.random());
    Self {
      codec: RandomCodec::with_avoid_codecs(&mut codec_random, &HashSet::new()),
      base_knn_vectors_format_test_case_state: BaseKnnVectorsFormatTestCaseState::new(random),
    }
  }
}

impl BaseIndexFileFormatTestCase for TestPerFieldKnnVectorsFormat {
  type Defaults = crate::test_framework::core::index::base_knn_vectors_format_test_case::BaseKnnVectorsFormatTestCaseDefaults;

  fn get_codec(&self) -> Result<Codecs> {
    Ok(self.codec.clone().into())
  }
}

impl BaseKnnVectorsFormatTestCase for TestPerFieldKnnVectorsFormat {
  fn base_knn_vectors_format_test_case_state(&self) -> &BaseKnnVectorsFormatTestCaseState {
    &self.base_knn_vectors_format_test_case_state
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestPerFieldKnnVectorsFormat, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestPerFieldKnnVectorsFormat::new(&mut random);
  let codec_guard = case.set_up()?;
  let result = f(&case, &mut random);
  case.tear_down(codec_guard);
  result
}

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

mod base_knn_vectors_format_test_case_tests {
  use super::run_case;
  use crate::core::util::error::lucene_error::Result;
  use crate::test_framework::core::index::base_knn_vectors_format_test_case::BaseKnnVectorsFormatTestCase;

  #[test]
  fn test_field_constructor() -> Result<()> {
    run_case(|case, random| case.test_field_constructor(random))
  }

  #[test]
  fn test_field_constructor_exceptions() -> Result<()> {
    run_case(|case, random| case.test_field_constructor_exceptions(random))
  }

  #[test]
  fn test_field_set_value() -> Result<()> {
    run_case(|case, random| case.test_field_set_value(random))
  }

  #[test]
  fn test_illegal_dim_change_two_docs() -> Result<()> {
    run_case(|case, random| case.test_illegal_dim_change_two_docs(random))
  }

  #[test]
  fn test_illegal_similarity_function_change() -> Result<()> {
    run_case(|case, random| case.test_illegal_similarity_function_change(random))
  }

  #[test]
  fn test_illegal_dim_change_two_writers() -> Result<()> {
    run_case(|case, random| case.test_illegal_dim_change_two_writers(random))
  }

  #[test]
  fn test_merging_with_different_knn_fields() -> Result<()> {
    run_case(|case, random| case.test_merging_with_different_knn_fields(random))
  }

  #[test]
  fn test_merging_with_different_byte_knn_fields() -> Result<()> {
    run_case(|case, random| case.test_merging_with_different_byte_knn_fields(random))
  }

  #[test]
  fn test_writer_ram_estimate() -> Result<()> {
    run_case(|case, random| case.test_writer_ram_estimate(random))
  }

  #[test]
  fn test_illegal_similarity_function_change_two_writers() -> Result<()> {
    run_case(|case, random| case.test_illegal_similarity_function_change_two_writers(random))
  }

  #[test]
  fn test_add_indexes_directory0() -> Result<()> {
    run_case(|case, random| case.test_add_indexes_directory0(random))
  }

  #[test]
  fn test_add_indexes_directory1() -> Result<()> {
    run_case(|case, random| case.test_add_indexes_directory1(random))
  }

  #[test]
  fn test_add_indexes_directory01() -> Result<()> {
    run_case(|case, random| case.test_add_indexes_directory01(random))
  }

  #[test]
  fn test_illegal_dim_change_via_add_indexes_directory() -> Result<()> {
    run_case(|case, random| case.test_illegal_dim_change_via_add_indexes_directory(random))
  }

  #[test]
  fn test_illegal_similarity_function_change_via_add_indexes_directory() -> Result<()> {
    run_case(|case, random| {
      case.test_illegal_similarity_function_change_via_add_indexes_directory(random)
    })
  }

  #[test]
  fn test_illegal_dim_change_via_add_indexes_codec_reader() -> Result<()> {
    run_case(|case, random| case.test_illegal_dim_change_via_add_indexes_codec_reader(random))
  }

  #[test]
  fn test_illegal_similarity_function_change_via_add_indexes_codec_reader() -> Result<()> {
    run_case(|case, random| {
      case.test_illegal_similarity_function_change_via_add_indexes_codec_reader(random)
    })
  }

  #[test]
  fn test_illegal_dim_change_via_add_indexes_slow_codec_reader() -> Result<()> {
    run_case(|case, random| case.test_illegal_dim_change_via_add_indexes_slow_codec_reader(random))
  }

  #[test]
  fn test_illegal_similarity_function_change_via_add_indexes_slow_codec_reader() -> Result<()> {
    run_case(|case, random| {
      case.test_illegal_similarity_function_change_via_add_indexes_slow_codec_reader(random)
    })
  }

  #[test]
  fn test_illegal_multiple_values() -> Result<()> {
    run_case(|case, random| case.test_illegal_multiple_values(random))
  }

  #[test]
  fn test_illegal_dimension_too_large() -> Result<()> {
    run_case(|case, random| case.test_illegal_dimension_too_large(random))
  }

  #[test]
  fn test_illegal_empty_vector() -> Result<()> {
    run_case(|case, random| case.test_illegal_empty_vector(random))
  }

  #[test]
  fn test_different_codecs1() -> Result<()> {
    run_case(|case, random| case.test_different_codecs1(random))
  }

  #[test]
  fn test_different_codecs2() -> Result<()> {
    run_case(|case, random| case.test_different_codecs2(random))
  }

  #[test]
  fn test_invalid_knn_vector_field_usage() -> Result<()> {
    run_case(|case, random| case.test_invalid_knn_vector_field_usage(random))
  }

  #[test]
  fn test_delete_all_vector_docs() -> Result<()> {
    run_case(|case, random| case.test_delete_all_vector_docs(random))
  }

  #[test]
  fn test_knn_vector_field_missing_from_one_segment() -> Result<()> {
    run_case(|case, random| case.test_knn_vector_field_missing_from_one_segment(random))
  }

  #[test]
  fn test_sparse_vectors() -> Result<()> {
    run_case(|case, random| case.test_sparse_vectors(random))
  }

  #[test]
  fn test_float_vector_scorer_iteration() -> Result<()> {
    run_case(|case, random| case.test_float_vector_scorer_iteration(random))
  }

  #[test]
  fn test_byte_vector_scorer_iteration() -> Result<()> {
    run_case(|case, random| case.test_byte_vector_scorer_iteration(random))
  }

  #[test]
  fn test_empty_float_vector_data() -> Result<()> {
    run_case(|case, random| case.test_empty_float_vector_data(random))
  }

  #[test]
  fn test_empty_byte_vector_data() -> Result<()> {
    run_case(|case, random| case.test_empty_byte_vector_data(random))
  }

  #[test]
  fn test_indexed_value_not_aliased() -> Result<()> {
    run_case(|case, random| case.test_indexed_value_not_aliased(random))
  }

  #[test]
  fn test_sorted_index() -> Result<()> {
    run_case(|case, random| case.test_sorted_index(random))
  }

  #[test]
  fn test_sorted_index_bytes() -> Result<()> {
    run_case(|case, random| case.test_sorted_index_bytes(random))
  }

  #[test]
  fn test_index_multiple_knn_vector_fields() -> Result<()> {
    run_case(|case, random| case.test_index_multiple_knn_vector_fields(random))
  }

  #[test]
  fn test_random() -> Result<()> {
    run_case(|case, random| case.test_random(random))
  }

  #[test]
  fn test_random_bytes() -> Result<()> {
    run_case(|case, random| case.test_random_bytes(random))
  }

  #[test]
  fn test_search_with_visited_limit() -> Result<()> {
    run_case(|case, random| case.test_search_with_visited_limit(random))
  }

  #[test]
  fn test_random_with_updates_and_graph() -> Result<()> {
    run_case(|case, random| case.test_random_with_updates_and_graph(random))
  }

  #[test]
  fn test_check_index_includes_vectors() -> Result<()> {
    run_case(|case, random| case.test_check_index_includes_vectors(random))
  }

  #[test]
  fn test_similarity_function_identifiers() -> Result<()> {
    run_case(|case, _random| case.test_similarity_function_identifiers())
  }

  #[test]
  fn test_vector_encoding_ordinals() -> Result<()> {
    run_case(|case, _random| case.test_vector_encoding_ordinals())
  }

  #[test]
  fn test_advance() -> Result<()> {
    run_case(|case, random| case.test_advance(random))
  }

  #[test]
  fn test_vector_values_report_correct_docs() -> Result<()> {
    run_case(|case, random| case.test_vector_values_report_correct_docs(random))
  }

  #[test]
  fn test_mismatched_fields() -> Result<()> {
    run_case(|case, random| case.test_mismatched_fields(random))
  }
}

mod base_index_file_format_test_case_tests {
  use super::run_case;
  use crate::core::util::error::lucene_error::Result;
  use crate::test_framework::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;

  #[test]
  fn test_merge_stability() -> Result<()> {
    run_case(|case, random| case.test_merge_stability(random))
  }

  #[test]
  fn test_multi_close() -> Result<()> {
    run_case(|case, random| case.test_multi_close(random))
  }

  #[test]
  fn test_random_exceptions() -> Result<()> {
    run_case(|case, random| case.test_random_exceptions(random))
  }

  #[test]
  fn test_check_integrity_reads_all_bytes() -> Result<()> {
    run_case(|case, random| case.test_check_integrity_reads_all_bytes(random))
  }
}
