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
use crate::core::codecs::lucene99::lucene99_scalar_quantized_vectors_format::Lucene99ScalarQuantizedVectorsFormat;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test_framework::core::index::base_knn_vectors_format_test_case::BaseKnnVectorsFormatTestCase;
use rand::Rng;

#[allow(dead_code)] // for quick search
pub struct TestLucene99ScalarQuantizedVectorsFormat;

#[test]
fn test_search() -> Result<()> {
  // TODO: Custom codec injection is not implemented, so this test cannot yet open a
  // Lucene99ScalarQuantizedVectorsReader through an IndexWriter-configured codec.
  Ok(())
}

#[test]
fn test_quantized_vectors_write_and_read() -> Result<()> {
  // TODO: Custom codec injection is not implemented, so this test cannot yet write and reopen an
  // index with Lucene99ScalarQuantizedVectorsFormat as required by the Java test.
  Ok(())
}

#[test]
fn test_to_string() -> Result<()> {
  let format = Lucene99ScalarQuantizedVectorsFormat::with_confidence_interval_bits_compress(
    Some(0.9),
    4,
    false,
  )?;
  assert_eq!(
    "Lucene99ScalarQuantizedVectorsFormat(name=Lucene99ScalarQuantizedVectorsFormat, confidenceInterval=0.9, bits=4, compress=false, flatVectorScorer=ScalarQuantizedVectorScorer(nonQuantizedDelegate=DefaultFlatVectorScorer()), rawVectorFormat=Lucene99FlatVectorsFormat(vectorsScorer=DefaultFlatVectorScorer()))",
    format.to_string()
  );
  Ok(())
}

#[test]
fn test_limits() -> Result<()> {
  assert!(matches!(
    Lucene99ScalarQuantizedVectorsFormat::with_confidence_interval_bits_compress(
      Some(1.1),
      7,
      false
    ),
    Err(LuceneError::IllegalArgument(_))
  ));
  assert!(matches!(
    Lucene99ScalarQuantizedVectorsFormat::with_confidence_interval_bits_compress(None, -1, false),
    Err(LuceneError::IllegalArgument(_))
  ));
  assert!(matches!(
    Lucene99ScalarQuantizedVectorsFormat::with_confidence_interval_bits_compress(None, 5, false),
    Err(LuceneError::IllegalArgument(_))
  ));
  assert!(matches!(
    Lucene99ScalarQuantizedVectorsFormat::with_confidence_interval_bits_compress(None, 9, false),
    Err(LuceneError::IllegalArgument(_))
  ));
  Ok(())
}

#[test]
fn test_random_with_updates_and_graph() -> Result<()> {
  // graph not supported
  Ok(())
}

#[test]
fn test_search_with_visited_limit() -> Result<()> {
  // search not supported
  Ok(())
}

mod base_knn_vectors_format_test_case_tests {
  use crate::core::util::error::lucene_error::Result;

  #[test]
  fn test_field_constructor() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_field_constructor_exceptions() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_field_set_value() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_illegal_dim_change_two_docs() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_illegal_similarity_function_change() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_illegal_dim_change_two_writers() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_merging_with_different_knn_fields() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_merging_with_different_byte_knn_fields() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_writer_ram_estimate() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_illegal_similarity_function_change_two_writers() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_add_indexes_directory0() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_add_indexes_directory1() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_add_indexes_directory01() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_illegal_dim_change_via_add_indexes_directory() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_illegal_similarity_function_change_via_add_indexes_directory() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_illegal_dim_change_via_add_indexes_codec_reader() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_illegal_similarity_function_change_via_add_indexes_codec_reader() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_illegal_dim_change_via_add_indexes_slow_codec_reader() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_illegal_similarity_function_change_via_add_indexes_slow_codec_reader() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_illegal_multiple_values() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_illegal_dimension_too_large() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_illegal_empty_vector() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_different_codecs1() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_different_codecs2() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_invalid_knn_vector_field_usage() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_delete_all_vector_docs() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_knn_vector_field_missing_from_one_segment() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_sparse_vectors() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_float_vector_scorer_iteration() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_byte_vector_scorer_iteration() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_empty_float_vector_data() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_empty_byte_vector_data() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_indexed_value_not_aliased() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_sorted_index() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_sorted_index_bytes() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_index_multiple_knn_vector_fields() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_random() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_random_bytes() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_check_index_includes_vectors() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_similarity_function_identifiers() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_vector_encoding_ordinals() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_advance() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_vector_values_report_correct_docs() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }

  #[test]
  fn test_mismatched_fields() -> Result<()> {
    // TODO: Custom codec injection is not implemented, so the inherited test cannot exercise the
    // scalar-quantized format yet.
    Ok(())
  }
}

impl BaseIndexFileFormatTestCase for TestLucene99ScalarQuantizedVectorsFormat {
  fn add_random_fields<R>(_random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO: Port the Java base-test random field hook when custom codec injection is available.
    Ok(())
  }
}

impl BaseKnnVectorsFormatTestCase for TestLucene99ScalarQuantizedVectorsFormat {}
