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
use crate::core::codecs::lucene90_doc_values_format::Lucene90DocValuesFormat;
use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::document::stored_field::StoredField;
use crate::core::index::BytesRef;
use crate::core::index::binary_doc_values::BinaryDocValues;
use crate::core::index::bytes_ref_builder::BytesRefBuilder;
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader;
use crate::core::index::doc_values::DocValues;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::store::{ByteBuffersDataOutput, DataInput, DataOutput};
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::long_supplier::LongSupplier;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::base_compressing_doc_values_format_test_case::BaseCompressingDocValuesFormatTestCase;
use crate::test::core::index::base_doc_values_format_test_case::BaseDocValuesFormatTestCase;
use crate::test::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::core::index::legacy_base_doc_values_format_test_case::LegacyBaseDocValuesFormatTestCase;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  at_least, get_only_leaf_reader, new_bytes_ref_from_string, new_directory_shared,
  new_index_writer_config_with_analyzer, new_log_merge_policy, new_string_field, random, rarely,
};
use crate::test::core::util::test_util::TestUtil;
use rand::SeedableRng;
use rand::prelude::StdRng;
use rand::{Rng, RngExt};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};

#[allow(dead_code)] // for quick search
pub struct TestLucene90DocValuesFormat;
impl BaseIndexFileFormatTestCase for TestLucene90DocValuesFormat {
  fn add_random_fields<R>(_random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    todo!()
  }
}
impl LegacyBaseDocValuesFormatTestCase for TestLucene90DocValuesFormat {}
impl BaseDocValuesFormatTestCase for TestLucene90DocValuesFormat {}
impl BaseCompressingDocValuesFormatTestCase for TestLucene90DocValuesFormat {}
impl TestLucene90DocValuesFormatTests for TestLucene90DocValuesFormat {}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestLucene90DocValuesFormat, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestLucene90DocValuesFormat;
  f(&case, &mut random)
}

mod lucene90_doc_values_format_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::codecs::lucene90::test_lucene90_doc_values_format::{
    TestLucene90DocValuesFormatTests, run_case,
  };

  #[test]
  fn test_sorted_set_variable_length_big_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_variable_length_big_vs_stored_fields(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_sorted_set_variable_length_many_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_variable_length_many_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_variable_length_big_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_variable_length_big_vs_stored_fields(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_sorted_variable_length_many_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_variable_length_many_vs_stored_fields(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_terms_enum_fixed_width() -> Result<()> {
    run_case(|case, random| case.test_terms_enum_fixed_width(random))
  }
  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_terms_enum_variable_width() -> Result<()> {
    run_case(|case, random| case.test_terms_enum_variable_width(random))
  }
  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_terms_enum_random_many() -> Result<()> {
    run_case(|case, random| case.test_terms_enum_random_many(random))
  }
  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_terms_enum_long_shared_prefixes() -> Result<()> {
    run_case(|case, random| case.test_terms_enum_long_shared_prefixes(random))
  }

  #[test]
  fn test_sparse_doc_values_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_doc_values_vs_stored_fields(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_sorted_set_around_block_size() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_around_block_size(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_sorted_numeric_around_block_size() -> Result<()> {
    run_case(|case, random| case.test_sorted_numeric_around_block_size(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_sorted_numeric_blocks_of_various_bits_per_value() -> Result<()> {
    run_case(|case, random| case.test_sorted_numeric_blocks_of_various_bits_per_value(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_sparse_sorted_numeric_blocks_of_various_bits_per_value() -> Result<()> {
    run_case(|case, random| {
      case.test_sparse_sorted_numeric_blocks_of_various_bits_per_value(random)
    })
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_numeric_blocks_of_various_bits_per_value() -> Result<()> {
    run_case(|case, random| case.test_numeric_blocks_of_various_bits_per_value(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_sparse_numeric_blocks_of_various_bits_per_value() -> Result<()> {
    run_case(|case, random| case.test_sparse_numeric_blocks_of_various_bits_per_value(random))
  }
  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_numeric_field_jump_tables() -> Result<()> {
    run_case(|case, random| case.test_numeric_field_jump_tables(random))
  }
  #[test]
  fn test_reseek_after_skip_decompression() -> Result<()> {
    run_case(|case, random| case.test_reseek_after_skip_decompression(random))
  }

  #[test]
  fn test_large_terms_compression() -> Result<()> {
    run_case(|case, random| case.test_large_terms_compression(random))
  }

  #[test]
  fn test_sorted_terms_dict_lookup_ord() -> Result<()> {
    run_case(|case, random| case.test_sorted_terms_dict_lookup_ord(random))
  }

  #[test]
  fn test_sorted_set_terms_dict_lookup_ord() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_terms_dict_lookup_ord(random))
  }

  #[test]
  fn test_terms_enum_dictionary() -> Result<()> {
    run_case(|case, random| case.test_terms_enum_dictionary(random))
  }

  #[test]
  fn test_terms_enum_consistency() -> Result<()> {
    run_case(|case, random| case.test_terms_enum_consistency(random))
  }
}

mod base_compressing_doc_values_format_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::codecs::lucene90::test_lucene90_doc_values_format::run_case;
  use crate::test::core::index::base_compressing_doc_values_format_test_case::BaseCompressingDocValuesFormatTestCase;

  #[test]
  fn test_unique_values_compression() -> Result<()> {
    run_case(|case, random| case.test_unique_values_compression(random))
  }

  #[test]
  fn test_date_compression() -> Result<()> {
    run_case(|case, random| case.test_date_compression(random))
  }

  #[test]
  fn test_single_big_value_compression() -> Result<()> {
    run_case(|case, random| case.test_single_big_value_compression(random))
  }
}

mod base_doc_values_format_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::codecs::lucene90::test_lucene90_doc_values_format::run_case;
  use crate::test::core::index::base_doc_values_format_test_case::BaseDocValuesFormatTestCase;

  #[test]
  fn test_sorted_merge_away_all_values_with_skipper() -> Result<()> {
    run_case(|case, random| case.test_sorted_merge_away_all_values_with_skipper(random))
  }

  #[test]
  fn test_sorted_set_merge_away_all_values_with_skipper() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_merge_away_all_values_with_skipper(random))
  }

  #[test]
  fn test_number_merge_away_all_values_with_skipper() -> Result<()> {
    run_case(|case, random| case.test_number_merge_away_all_values_with_skipper(random))
  }

  #[test]
  fn test_sorted_number_merge_away_all_values_with_skipper() -> Result<()> {
    run_case(|case, random| case.test_sorted_number_merge_away_all_values_with_skipper(random))
  }

  #[test]
  fn test_sorted_merge_away_all_values_large_segment_with_skipper() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_merge_away_all_values_large_segment_with_skipper(random)
    })
  }

  #[test]
  fn test_sorted_set_merge_away_all_values_large_segment_with_skipper() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_set_merge_away_all_values_large_segment_with_skipper(random)
    })
  }

  #[test]
  fn test_numeric_merge_away_all_values_large_segment_with_skipper() -> Result<()> {
    run_case(|case, random| {
      case.test_numeric_merge_away_all_values_large_segment_with_skipper(random)
    })
  }

  #[test]
  fn test_sorted_numeric_merge_away_all_values_large_segment_with_skipper() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_numeric_merge_away_all_values_large_segment_with_skipper(random)
    })
  }

  #[test]
  fn test_numeric_doc_values_with_skipper_small() -> Result<()> {
    run_case(|case, random| case.test_numeric_doc_values_with_skipper_small(random))
  }

  #[test]
  fn test_numeric_doc_values_with_skipper_medium() -> Result<()> {
    run_case(|case, random| case.test_numeric_doc_values_with_skipper_medium(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_numeric_doc_values_with_skipper_big() -> Result<()> {
    run_case(|case, random| case.test_numeric_doc_values_with_skipper_big(random))
  }

  #[test]
  fn test_sorted_numeric_doc_values_with_skipper_small() -> Result<()> {
    run_case(|case, random| case.test_sorted_numeric_doc_values_with_skipper_small(random))
  }

  #[test]
  fn test_sorted_numeric_doc_values_with_skipper_medium() -> Result<()> {
    run_case(|case, random| case.test_sorted_numeric_doc_values_with_skipper_medium(random))
  }
  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_sorted_numeric_doc_values_with_skipper_big() -> Result<()> {
    run_case(|case, random| case.test_sorted_numeric_doc_values_with_skipper_big(random))
  }
  #[test]
  fn test_sorted_doc_values_with_skipper_small() -> Result<()> {
    run_case(|case, random| case.test_sorted_doc_values_with_skipper_small(random))
  }

  #[test]
  fn test_sorted_doc_values_with_skipper_medium() -> Result<()> {
    run_case(|case, random| case.test_sorted_doc_values_with_skipper_medium(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_sorted_doc_values_with_skipper_big() -> Result<()> {
    run_case(|case, random| case.test_sorted_doc_values_with_skipper_big(random))
  }

  #[test]
  fn test_sorted_set_doc_values_with_skipper_small() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_doc_values_with_skipper_small(random))
  }

  #[test]
  fn test_sorted_set_doc_values_with_skipper_medium() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_doc_values_with_skipper_medium(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_sorted_set_doc_values_with_skipper_big() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_doc_values_with_skipper_big(random))
  }
  #[test]
  fn test_mismatched_fields() -> Result<()> {
    run_case(|case, random| case.test_mismatched_fields(random))
  }
}

mod legacy_base_doc_values_format_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::codecs::lucene90::test_lucene90_doc_values_format::run_case;
  use crate::test::core::index::legacy_base_doc_values_format_test_case::LegacyBaseDocValuesFormatTestCase;

  #[test]
  fn test_one_number() -> Result<()> {
    run_case(|case, random| case.test_one_number(random))
  }

  #[test]
  fn test_one_float() -> Result<()> {
    run_case(|case, random| case.test_one_float(random))
  }

  #[test]
  fn test_two_numbers() -> Result<()> {
    run_case(|case, random| case.test_two_numbers(random))
  }

  #[test]
  fn test_two_binary_values() -> Result<()> {
    run_case(|case, random| case.test_two_binary_values(random))
  }

  #[test]
  fn test_variously_compressible_binary_values() -> Result<()> {
    run_case(|case, random| case.test_variously_compressible_binary_values(random))
  }

  #[test]
  fn test_two_fields_mixed() -> Result<()> {
    run_case(|case, random| case.test_two_fields_mixed(random))
  }

  #[test]
  fn test_three_fields_mixed() -> Result<()> {
    run_case(|case, random| case.test_three_fields_mixed(random))
  }

  #[test]
  fn test_three_fields_mixed2() -> Result<()> {
    run_case(|case, random| case.test_three_fields_mixed2(random))
  }

  #[test]
  fn test_two_documents_numeric() -> Result<()> {
    run_case(|case, random| case.test_two_documents_numeric(random))
  }

  #[test]
  fn test_two_documents_merged() -> Result<()> {
    run_case(|case, random| case.test_two_documents_merged(random))
  }

  #[test]
  fn test_big_numeric_range() -> Result<()> {
    run_case(|case, random| case.test_big_numeric_range(random))
  }

  #[test]
  fn test_big_numeric_range2() -> Result<()> {
    run_case(|case, random| case.test_big_numeric_range2(random))
  }

  #[test]
  fn test_bytes() -> Result<()> {
    run_case(|case, random| case.test_bytes(random))
  }

  #[test]
  fn test_bytes_two_documents_merged() -> Result<()> {
    run_case(|case, random| case.test_bytes_two_documents_merged(random))
  }

  #[test]
  fn test_bytes_merge_away_all_values() -> Result<()> {
    run_case(|case, random| case.test_bytes_merge_away_all_values(random))
  }

  #[test]
  fn test_sorted_bytes() -> Result<()> {
    run_case(|case, random| case.test_sorted_bytes(random))
  }

  #[test]
  fn test_sorted_bytes_two_documents() -> Result<()> {
    run_case(|case, random| case.test_sorted_bytes_two_documents(random))
  }

  #[test]
  fn test_sorted_bytes_three_documents() -> Result<()> {
    run_case(|case, random| case.test_sorted_bytes_three_documents(random))
  }

  #[test]
  fn test_sorted_bytes_two_documents_merged() -> Result<()> {
    run_case(|case, random| case.test_sorted_bytes_two_documents_merged(random))
  }

  #[test]
  fn test_sorted_merge_away_all_values() -> Result<()> {
    run_case(|case, random| case.test_sorted_merge_away_all_values(random))
  }

  #[test]
  fn test_bytes_with_newline() -> Result<()> {
    run_case(|case, random| case.test_bytes_with_newline(random))
  }

  #[test]
  fn test_missing_sorted_bytes() -> Result<()> {
    run_case(|case, random| case.test_missing_sorted_bytes(random))
  }

  #[test]
  fn test_sorted_terms_enum() -> Result<()> {
    run_case(|case, random| case.test_sorted_terms_enum(random))
  }

  #[test]
  fn test_empty_sorted_bytes() -> Result<()> {
    run_case(|case, random| case.test_empty_sorted_bytes(random))
  }

  #[test]
  fn test_empty_bytes() -> Result<()> {
    run_case(|case, random| case.test_empty_bytes(random))
  }

  #[test]
  fn test_very_large_but_legal_bytes() -> Result<()> {
    run_case(|case, random| case.test_very_large_but_legal_bytes(random))
  }

  #[test]
  fn test_very_large_but_legal_sorted_bytes() -> Result<()> {
    run_case(|case, random| case.test_very_large_but_legal_sorted_bytes(random))
  }

  #[test]
  fn test_codec_uses_own_bytes() -> Result<()> {
    run_case(|case, random| case.test_codec_uses_own_bytes(random))
  }

  #[test]
  fn test_codec_uses_own_sorted_bytes() -> Result<()> {
    run_case(|case, random| case.test_codec_uses_own_sorted_bytes(random))
  }

  #[test]
  fn test_doc_values_simple() -> Result<()> {
    run_case(|case, random| case.test_doc_values_simple(random))
  }
  #[test]
  fn test_random_sorted_bytes() -> Result<()> {
    run_case(|case, random| case.test_random_sorted_bytes(random))
  }
  #[test]
  fn test_boolean_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_boolean_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_boolean_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_boolean_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_byte_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_byte_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_byte_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_byte_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_short_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_short_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_short_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_short_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_int_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_int_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_int_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_int_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_long_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_long_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_long_numerics_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_long_numerics_vs_stored_fields(random))
  }

  #[test]
  fn test_binary_fixed_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_binary_fixed_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_binary_fixed_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_binary_fixed_length_vs_stored_fields(random))
  }

  #[test]
  fn test_binary_variable_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_binary_variable_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_binary_variable_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_binary_variable_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_fixed_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_fixed_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_sorted_fixed_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_sorted_fixed_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_variable_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_variable_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sparse_sorted_variable_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sparse_sorted_variable_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_set_one_value() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_one_value(random))
  }

  #[test]
  fn test_sorted_set_two_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_fields(random))
  }

  #[test]
  fn test_sorted_set_two_documents_merged() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_documents_merged(random))
  }

  #[test]
  fn test_sorted_set_two_values() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_values(random))
  }

  #[test]
  fn test_sorted_set_two_values_unordered() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_values_unordered(random))
  }

  #[test]
  fn test_sorted_set_three_values_two_docs() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_three_values_two_docs(random))
  }

  #[test]
  fn test_sorted_set_two_documents_last_missing() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_documents_last_missing(random))
  }

  #[test]
  fn test_sorted_set_two_documents_last_missing_merge() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_documents_last_missing_merge(random))
  }

  #[test]
  fn test_sorted_set_two_documents_first_missing() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_documents_first_missing(random))
  }

  #[test]
  fn test_sorted_set_two_documents_first_missing_merge() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_two_documents_first_missing_merge(random))
  }

  #[test]
  fn test_sorted_set_merge_away_all_values() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_merge_away_all_values(random))
  }

  #[test]
  fn test_sorted_set_terms_enum() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_terms_enum(random))
  }

  #[test]
  fn test_sorted_set_fixed_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_fixed_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_numerics_single_valued_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_numerics_single_valued_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_numerics_single_valued_missing_vs_stored_fields() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_numerics_single_valued_missing_vs_stored_fields(random)
    })
  }

  #[test]
  fn test_sorted_numerics_multiple_values_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_numerics_multiple_values_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_numerics_few_unique_sets_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_numerics_few_unique_sets_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_set_variable_length_vs_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_variable_length_vs_stored_fields(random))
  }

  #[test]
  fn test_sorted_set_fixed_length_single_valued_vs_stored_fields() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_set_fixed_length_single_valued_vs_stored_fields(random)
    })
  }

  #[test]
  fn test_sorted_set_variable_length_single_valued_vs_stored_fields() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_set_variable_length_single_valued_vs_stored_fields(random)
    })
  }

  #[test]
  fn test_sorted_set_fixed_length_few_unique_sets_vs_stored_fields() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_set_fixed_length_few_unique_sets_vs_stored_fields(random)
    })
  }

  #[test]
  fn test_sorted_set_variable_length_few_unique_sets_vs_stored_fields() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_set_variable_length_few_unique_sets_vs_stored_fields(random)
    })
  }

  #[test]
  fn test_sorted_set_variable_length_many_values_per_doc_vs_stored_fields() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_set_variable_length_many_values_per_doc_vs_stored_fields(random)
    })
  }

  #[test]
  fn test_sorted_set_fixed_length_many_values_per_doc_vs_stored_fields() -> Result<()> {
    run_case(|case, random| {
      case.test_sorted_set_fixed_length_many_values_per_doc_vs_stored_fields(random)
    })
  }

  #[test]
  fn test_gcd_compression() -> Result<()> {
    run_case(|case, random| case.test_gcd_compression(random))
  }

  #[test]
  fn test_sparse_gcd_compression() -> Result<()> {
    run_case(|case, random| case.test_sparse_gcd_compression(random))
  }

  #[test]
  fn test_zeros() -> Result<()> {
    run_case(|case, random| case.test_zeros(random))
  }

  #[test]
  fn test_sparse_zeros() -> Result<()> {
    run_case(|case, random| case.test_sparse_zeros(random))
  }

  #[test]
  fn test_zero_or_min() -> Result<()> {
    run_case(|case, random| case.test_zero_or_min(random))
  }

  #[test]
  fn test_two_numbers_one_missing() -> Result<()> {
    run_case(|case, random| case.test_two_numbers_one_missing(random))
  }

  #[test]
  fn test_two_numbers_one_missing_with_merging() -> Result<()> {
    run_case(|case, random| case.test_two_numbers_one_missing_with_merging(random))
  }

  #[test]
  fn test_three_numbers_one_missing_with_merging() -> Result<()> {
    run_case(|case, random| case.test_three_numbers_one_missing_with_merging(random))
  }

  #[test]
  fn test_two_bytes_one_missing() -> Result<()> {
    run_case(|case, random| case.test_two_bytes_one_missing(random))
  }

  #[test]
  fn test_two_bytes_one_missing_with_merging() -> Result<()> {
    run_case(|case, random| case.test_two_bytes_one_missing_with_merging(random))
  }

  #[test]
  fn test_three_bytes_one_missing_with_merging() -> Result<()> {
    run_case(|case, random| case.test_three_bytes_one_missing_with_merging(random))
  }
  #[test]
  fn test_threads() -> Result<()> {
    run_case(|case, random| case.test_threads(random))
  }
  #[test]
  fn test_threads2() -> Result<()> {
    run_case(|case, random| case.test_threads2(random))
  }
  #[test]
  fn test_threads3() -> Result<()> {
    run_case(|case, random| case.test_threads3(random))
  }
  #[test]
  fn test_empty_binary_value_on_page_sizes() -> Result<()> {
    run_case(|case, random| case.test_empty_binary_value_on_page_sizes(random))
  }

  #[test]
  fn test_one_sorted_number() -> Result<()> {
    run_case(|case, random| case.test_one_sorted_number(random))
  }

  #[test]
  fn test_one_sorted_number_one_missing() -> Result<()> {
    run_case(|case, random| case.test_one_sorted_number_one_missing(random))
  }

  #[test]
  fn test_number_merge_away_all_values() -> Result<()> {
    run_case(|case, random| case.test_number_merge_away_all_values(random))
  }

  #[test]
  fn test_two_sorted_number() -> Result<()> {
    run_case(|case, random| case.test_two_sorted_number(random))
  }

  #[test]
  fn test_two_sorted_number_same_value() -> Result<()> {
    run_case(|case, random| case.test_two_sorted_number_same_value(random))
  }

  #[test]
  fn test_two_sorted_number_one_missing() -> Result<()> {
    run_case(|case, random| case.test_two_sorted_number_one_missing(random))
  }

  #[test]
  fn test_sorted_number_merge() -> Result<()> {
    run_case(|case, random| case.test_sorted_number_merge(random))
  }

  #[test]
  fn test_sorted_number_merge_away_all_values() -> Result<()> {
    run_case(|case, random| case.test_sorted_number_merge_away_all_values(random))
  }

  #[test]
  fn test_sorted_enum_advance_independently() -> Result<()> {
    run_case(|case, random| case.test_sorted_enum_advance_independently(random))
  }

  #[test]
  fn test_sorted_set_enum_advance_independently() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_enum_advance_independently(random))
  }

  #[test]
  fn test_sorted_merge_away_all_values_large_segment() -> Result<()> {
    run_case(|case, random| case.test_sorted_merge_away_all_values_large_segment(random))
  }

  #[test]
  fn test_sorted_set_merge_away_all_values_large_segment() -> Result<()> {
    run_case(|case, random| case.test_sorted_set_merge_away_all_values_large_segment(random))
  }

  #[test]
  fn test_numeric_merge_away_all_values_large_segment() -> Result<()> {
    run_case(|case, random| case.test_numeric_merge_away_all_values_large_segment(random))
  }

  #[test]
  fn test_sorted_numeric_merge_away_all_values_large_segment() -> Result<()> {
    run_case(|case, random| case.test_sorted_numeric_merge_away_all_values_large_segment(random))
  }

  #[test]
  fn test_binary_merge_away_all_values_large_segment() -> Result<()> {
    run_case(|case, random| case.test_binary_merge_away_all_values_large_segment(random))
  }

  #[test]
  fn test_random_advance_numeric() -> Result<()> {
    run_case(|case, random| case.test_random_advance_numeric(random))
  }

  #[test]
  fn test_random_advance_binary() -> Result<()> {
    run_case(|case, random| case.test_random_advance_binary(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_high_ords_sorted_set_dv() -> Result<()> {
    run_case(|case, random| case.test_high_ords_sorted_set_dv(random))
  }
}

trait TestLucene90DocValuesFormatTests: BaseCompressingDocValuesFormatTestCase {
  fn test_sorted_set_variable_length_big_vs_stored_fields<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_iterations = at_least(random, 1);
    for _ in 0..num_iterations {
      let num_docs = at_least(random, 10);
      self.do_test_sorted_set_vs_stored_fields(random, num_docs, 1, 32_766, 16, 100)?;
    }
    Ok(())
  }

  fn test_sorted_set_variable_length_many_vs_stored_fields<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_iterations = at_least(random, 1);
    for _ in 0..num_iterations {
      let num_docs = TestUtil::next_int(random, 1024, 2049);
      self.do_test_sorted_set_vs_stored_fields(random, num_docs, 1, 500, 16, 100)?;
    }
    Ok(())
  }

  fn test_sorted_variable_length_big_vs_stored_fields<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_iterations = at_least(random, 1);
    for _ in 0..num_iterations {
      let num_docs = at_least(random, 100);
      self.do_test_sorted_vs_stored_fields(random, num_docs, 1.0, 1, 32_766)?;
    }
    Ok(())
  }

  fn test_sorted_variable_length_many_vs_stored_fields<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_iterations = at_least(random, 1);
    for _ in 0..num_iterations {
      let num_docs = TestUtil::next_int(random, 1024, 2049);
      self.do_test_sorted_vs_stored_fields(random, num_docs, 1.0, 1, 500)?;
    }
    Ok(())
  }

  fn test_terms_enum_fixed_width<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_iterations = at_least(random, 1);
    for _ in 0..num_iterations {
      let num_docs = TestUtil::next_int(random, 1025, 5121);
      self.do_test_terms_enum_random(random, num_docs, |r| {
        TestUtil::random_simple_string_range(r, 10, 10)
      })?;
    }
    Ok(())
  }

  fn test_terms_enum_variable_width<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_iterations = at_least(random, 1);
    for _ in 0..num_iterations {
      let num_docs = TestUtil::next_int(random, 1025, 5121);
      self.do_test_terms_enum_random(random, num_docs, |r| {
        TestUtil::random_simple_string_range(r, 1, 500)
      })?;
    }
    Ok(())
  }

  fn test_terms_enum_random_many<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_iterations = at_least(random, 1);
    for _ in 0..num_iterations {
      let num_docs = TestUtil::next_int(random, 1025, 8121);
      self.do_test_terms_enum_random(random, num_docs, |r| {
        TestUtil::random_simple_string_range(r, 1, 500)
      })?;
    }
    Ok(())
  }

  fn test_terms_enum_long_shared_prefixes<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_iterations = at_least(random, 1);
    for _ in 0..num_iterations {
      let num_docs = TestUtil::next_int(random, 1025, 5121);
      self.do_test_terms_enum_random(random, num_docs, |r| {
        let len = r.random_range(0..500);
        let mut bytes = vec![b'a'; len];
        if !bytes.is_empty() {
          let idx = r.random_range(0..bytes.len());
          bytes[idx] = b'b';
        }
        String::from_utf8(bytes).expect("ASCII test bytes should be valid UTF-8")
      })?;
    }
    Ok(())
  }

  fn test_sparse_doc_values_vs_stored_fields<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_iterations = at_least(random, 1);
    for _ in 0..num_iterations {
      self.do_test_sparse_doc_values_vs_stored_fields(random)?;
    }
    Ok(())
  }

  fn do_test_sparse_doc_values_vs_stored_fields<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let values_len = TestUtil::next_int(random, 1, 500) as usize;
    let mut values = vec![0_i64; values_len];
    for value in &mut values {
      *value = random.random::<i64>();
    }

    let dir = new_directory_shared(random)?;
    let analyzer = crate::test::core::analysis::mock_analyzer::MockAnalyzer::new(random);
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_merge_scheduler(SerialMergeScheduler::new());
    let writer = RandomIndexWriter::with_config(random, dir.clone(), conf);

    let avg_gap = 100;
    let num_docs = at_least(random, 200);
    for _ in (0..=random.random_range(0..(avg_gap * 2 + 1))).rev() {
      writer.add_document(random, Document::new())?;
    }

    let max_num_values_per_doc = if random.random_bool(0.5) {
      1
    } else {
      TestUtil::next_int(random, 2, 5)
    };

    for _ in 0..num_docs {
      let mut doc = Document::new();

      let mut doc_value = values[random.random_range(0..values.len())];
      doc.add(NumericDocValuesField::new("numeric", doc_value));
      doc.add(SortedDocValuesField::new(
        "sorted",
        new_bytes_ref_from_string(random, &doc_value.to_string())?,
      ));
      doc.add(BinaryDocValuesField::new(
        "binary",
        new_bytes_ref_from_string(random, &doc_value.to_string())?,
      ));
      doc.add(StoredField::from_i64("value", doc_value)?);

      let num_values = TestUtil::next_int(random, 1, max_num_values_per_doc);
      for _ in 0..num_values {
        doc_value = values[random.random_range(0..values.len())];
        doc.add(SortedNumericDocValuesField::new(
          "sorted_numeric",
          doc_value,
        ));
        doc.add(SortedSetDocValuesField::new(
          "sorted_set",
          new_bytes_ref_from_string(random, &doc_value.to_string())?,
        ));
        doc.add(StoredField::from_i64("values", doc_value)?);
      }

      writer.add_document(random, doc)?;

      for _ in (0..=TestUtil::next_int(random, 0, avg_gap * 2)).rev() {
        writer.add_document(random, Document::new())?;
      }
    }

    if random.random_bool(0.5) {
      writer.force_merge(random, 1)?;
    }

    let index_reader = writer.get_reader(random)?;
    writer.close(random)?;

    let context = get_context(&index_reader)?;
    for leaf in context.leaves()? {
      let reader = leaf.reader();
      let mut numeric = DocValues::get_numeric(reader, "numeric")?;
      let mut sorted = DocValues::get_sorted(reader, "sorted")?;
      let mut binary = DocValues::get_binary(reader, "binary")?;
      let mut sorted_numeric = DocValues::get_sorted_numeric(reader, "sorted_numeric")?;
      let mut sorted_set = DocValues::get_sorted_set(reader, "sorted_set")?;

      let mut stored_fields = reader.stored_fields()?;
      for doc_id in 0..reader.max_doc()? {
        let doc = stored_fields.document(doc_id)?;

        let value = match doc.get_field("value") {
          Some(field) => match field.numeric_value()? {
            Some(number) => number.to_i64(),
            None => None,
          },
          None => None,
        };

        if let Some(value) = value {
          assert_eq!(doc_id, numeric.next_doc()?);
          assert_eq!(doc_id, binary.next_doc()?);
          assert_eq!(doc_id, sorted.next_doc()?);
          assert_eq!(value, numeric.long_value()?);
          let ord = sorted.ord_value()?;
          assert!(ord >= 0);
          assert_eq!(
            new_bytes_ref_from_string(random, &value.to_string())?,
            sorted.lookup_ord(ord)?.into_owned()
          );
          assert_eq!(
            new_bytes_ref_from_string(random, &value.to_string())?,
            binary.binary_value()?.into_owned()
          );
        } else {
          assert!(numeric.doc_id() < doc_id);
        }

        let value_fields = doc.get_fields_with_name("values");
        if value_fields.is_empty() {
          assert!(sorted_numeric.doc_id() < doc_id);
        } else {
          let value_set = value_fields
            .iter()
            .map(|field| {
              let numeric_value = field
                .numeric_value()
                .expect("stored numeric value should be readable");
              numeric_value
                .and_then(|number| number.to_i64())
                .expect("stored numeric value should fit into i64")
            })
            .collect::<HashSet<_>>();

          assert_eq!(doc_id, sorted_numeric.next_doc()?);
          assert_eq!(value_fields.len() as i32, sorted_numeric.doc_value_count()?);
          for _ in 0..sorted_numeric.doc_value_count()? {
            assert!(value_set.contains(&sorted_numeric.next_value()?));
          }

          assert_eq!(doc_id, sorted_set.next_doc()?);
          assert_eq!(value_set.len() as i32, sorted_set.doc_value_count()?);
          for _ in 0..sorted_set.doc_value_count()? {
            let ord = sorted_set.next_ord()?;
            let value = sorted_set
              .lookup_ord(ord)?
              .utf8_to_string()?
              .parse::<i64>()
              .expect("sorted-set ord should decode to i64");
            assert!(value_set.contains(&value));
          }
        }
      }
    }

    Ok(())
  }

  fn do_test_terms_enum_random<R, F>(
    &self,
    _random: &mut R,
    _num_docs: i32,
    _values_producer: F,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    F: FnMut(&mut R) -> String,
  {
    // 自定义 Codec 未实现
    Ok(())
  }
  fn test_sorted_set_around_block_size<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let frontier = 1 << Lucene90DocValuesFormat::DIRECT_MONOTONIC_BLOCK_SHIFT;
    for max_doc in (frontier - 1)..=(frontier + 1) {
      let dir = new_directory_shared(random)?;
      let analyzer = crate::test::core::analysis::mock_analyzer::MockAnalyzer::new(random);
      let mut iwc = new_index_writer_config_with_analyzer(random, analyzer);
      iwc.set_merge_policy(new_log_merge_policy(random)?);
      let writer = IndexWriter::new(dir, iwc)?;
      let mut out = ByteBuffersDataOutput::new();

      for _ in 0..max_doc {
        let s1 = TestUtil::random_simple_string_range(random, 2, 2);
        let s2 = TestUtil::random_simple_string_range(random, 2, 2);
        let mut doc = Document::new();
        doc.add(SortedSetDocValuesField::new(
          "sset",
          new_bytes_ref_from_string(random, &s1)?,
        ));
        doc.add(SortedSetDocValuesField::new(
          "sset",
          new_bytes_ref_from_string(random, &s2)?,
        ));
        writer.add_document(doc)?;

        let mut set = BTreeSet::new();
        set.insert(s1);
        set.insert(s2);
        out.write_vint(set.len() as i32)?;
        for value in set {
          out.write_vint(value.len() as i32)?;
          out.write_bytes_with_len(value.as_bytes(), value.len())?;
        }
      }

      writer.force_merge(1)?;
      let reader = writer.get_reader(false, false)?;
      writer.close()?;

      let leaf = get_only_leaf_reader(&reader)?;
      assert_eq!(max_doc, leaf.max_doc()?);
      let mut values = leaf
        .get_sorted_set_doc_values("sset")?
        .expect("sorted-set doc values should exist");
      let mut input = out.get_data_input_owner(false)?;
      let mut builder = BytesRefBuilder::<Vec<u8>>::new();
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, values.next_doc()?);
        let num_values = input.read_vint()?;
        assert_eq!(num_values, values.doc_value_count()?);
        for _ in 0..num_values {
          let len = input.read_vint()? as usize;
          builder.set_length(len);
          builder.grow(len);
          input.read_bytes(builder.bytes_mut().bytes.as_mut(), 0, len)?;
          let ord = values.next_ord()?;
          assert_eq!(
            builder.bytes().clone(),
            values.lookup_ord(ord)?.into_owned()
          );
        }
      }
    }
    Ok(())
  }

  fn test_sorted_numeric_around_block_size<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let frontier = 1 << Lucene90DocValuesFormat::DIRECT_MONOTONIC_BLOCK_SHIFT;
    for max_doc in (frontier - 1)..=(frontier + 1) {
      let dir = new_directory_shared(random)?;
      let analyzer = crate::test::core::analysis::mock_analyzer::MockAnalyzer::new(random);
      let mut iwc = new_index_writer_config_with_analyzer(random, analyzer);
      iwc.set_merge_policy(new_log_merge_policy(random)?);
      let writer = IndexWriter::new(dir, iwc)?;
      let mut out = ByteBuffersDataOutput::new();

      for _ in 0..max_doc {
        let s1 = random.random_range(0..100) as i64;
        let s2 = random.random_range(0..100) as i64;
        let mut doc = Document::new();
        doc.add(SortedNumericDocValuesField::new("snum", s1));
        doc.add(SortedNumericDocValuesField::new("snum", s2));
        writer.add_document(doc)?;
        out.write_vlong(std::cmp::min(s1, s2))?;
        out.write_vlong(std::cmp::max(s1, s2))?;
      }

      writer.force_merge(1)?;
      let reader = writer.get_reader(false, false)?;
      writer.close()?;

      let leaf = get_only_leaf_reader(&reader)?;
      assert_eq!(max_doc, leaf.max_doc()?);
      let mut values = leaf
        .get_sorted_numeric_doc_values("snum")?
        .expect("sorted-numeric doc values should exist");
      let mut input = out.get_data_input_owner(false)?;
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, values.next_doc()?);
        assert_eq!(2, values.doc_value_count()?);
        assert_eq!(input.read_vlong()?, values.next_value()?);
        assert_eq!(input.read_vlong()?, values.next_value()?);
      }
    }
    Ok(())
  }

  fn test_sorted_numeric_blocks_of_various_bits_per_value<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.do_test_sorted_numeric_blocks_of_various_bits_per_value(random, |r| {
      TestUtil::next_int(r, 1, 3) as i64
    })
  }

  fn test_sparse_sorted_numeric_blocks_of_various_bits_per_value<R>(
    &self,
    random: &mut R,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.do_test_sorted_numeric_blocks_of_various_bits_per_value(random, |r| {
      TestUtil::next_int(r, 0, 2) as i64
    })
  }

  fn test_numeric_blocks_of_various_bits_per_value<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.do_test_sparse_numeric_blocks_of_various_bits_per_value(random, 1.0)
  }

  fn test_sparse_numeric_blocks_of_various_bits_per_value<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let density = random.random::<f64>();
    self.do_test_sparse_numeric_blocks_of_various_bits_per_value(random, density)
  }
  fn test_numeric_field_jump_tables<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // IndexedDISI block skipping only activates if target >= current + 2, so
    // we need at least 5 blocks to trigger consecutive block skips.
    let max_doc = at_least(random, 5 * 65_536);

    let dir = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_max_buffered_docs(max_doc);
    conf.set_ram_buffer_size_mb(-1.0);
    conf.set_use_compound_file(false);
    let iw = IndexWriter::new(dir.clone(), conf)?;

    let mut field_to_type = HashMap::new();
    for i in 0..max_doc {
      let mut doc = Document::new();
      doc.add(new_string_field(
        random,
        "id",
        format!("{i:b}"),
        Store::No,
        &mut field_to_type,
      )?);
      if random.random_range(0..100) > 10 {
        let value = random.random_range(0..100_000) as i64;
        doc.add(new_string_field(
          random,
          "stored",
          value.to_string(),
          Store::Yes,
          &mut field_to_type,
        )?);
        doc.add(NumericDocValuesField::new("dv", value));
      }
      iw.add_document(doc)?;
    }

    iw.flush()?;
    iw.force_merge(1)?;
    iw.commit()?;
    iw.close()?;

    self.assert_dv_iterate(dir.clone())?;
    self.assert_dv_advance(dir, if rarely(random) { 1 } else { 7 })?;
    Ok(())
  }
  fn do_test_sorted_numeric_blocks_of_various_bits_per_value<R, FC>(
    &self,
    random: &mut R,
    mut counts: FC,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    FC: FnMut(&mut R) -> i64,
  {
    let dir = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_max_buffered_docs(at_least(
      random,
      Lucene90DocValuesFormat::NUMERIC_BLOCK_SIZE,
    ));
    conf.set_ram_buffer_size_mb(-1.0);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let writer = IndexWriter::new(dir.clone(), conf)?;

    let num_docs = at_least(random, Lucene90DocValuesFormat::NUMERIC_BLOCK_SIZE * 3);
    let mut write_doc_values = Vec::with_capacity(num_docs as usize);
    let values = BlocksOfVariousBPV::new(random);

    for _i in 0..num_docs {
      let mut doc = Document::new();

      let value_count = counts(random).max(0) as usize;
      let mut value_array = vec![0_i64; value_count];
      for slot in &mut value_array {
        let value = values.get_as_long();
        *slot = value;
        doc.add(SortedNumericDocValuesField::new("dv", value));
      }
      value_array.sort();
      write_doc_values.push(value_array.clone());
      for value in value_array {
        doc.add(StoredField::from_string("stored", value.to_string())?);
      }
      writer.add_document(doc)?;
      if random.random_range(0..31) == 0 {
        writer.commit()?;
      }
    }

    writer.force_merge(1)?;
    writer.close()?;

    let reader = directory_reader::open(dir.clone())?;
    let context = get_context(&reader)?;
    for leaf in context.leaves()? {
      let r = leaf.reader();
      let mut doc_values = DocValues::get_sorted_numeric(r, "dv")?;
      let mut stored_fields = r.stored_fields()?;
      for i in 0..r.max_doc()? {
        if i > doc_values.doc_id() {
          doc_values.next_doc()?;
        }
        let stored_doc = stored_fields.document(i)?;
        let expected_stored = stored_doc.get_values("stored")?;
        if i < doc_values.doc_id() {
          assert_eq!(0, expected_stored.len());
        } else {
          let count = doc_values.doc_value_count()? as usize;
          let mut read_value_array = vec![0_i64; count];
          let mut actual_doc_value = vec![String::new(); count];
          for j in 0..count {
            let actual_dv = doc_values.next_value()?;
            read_value_array[j] = actual_dv;
            actual_doc_value[j] = actual_dv.to_string();
          }
          let write_value_array = &write_doc_values[i as usize];
          assert_eq!(read_value_array, *write_value_array);
          let expected_stored = expected_stored
            .into_iter()
            .map(|value| value.into_owned())
            .collect::<Vec<_>>();
          assert_eq!(expected_stored, actual_doc_value);
        }
      }
    }
    Ok(())
  }

  fn do_test_sparse_numeric_blocks_of_various_bits_per_value<R>(
    &self,
    random: &mut R,
    density: f64,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer);
    conf.set_max_buffered_docs(at_least(
      random,
      Lucene90DocValuesFormat::NUMERIC_BLOCK_SIZE,
    ));
    conf.set_ram_buffer_size_mb(-1.0);
    conf.set_merge_policy(new_log_merge_policy(random)?);
    let writer = IndexWriter::new(dir.clone(), conf)?;

    let num_docs = at_least(random, Lucene90DocValuesFormat::NUMERIC_BLOCK_SIZE * 3);
    let longs = BlocksOfVariousBPV::new(random);
    for _ in 0..num_docs {
      if random.random::<f64>() > density {
        writer.add_document(Document::new())?;
        continue;
      }
      let value = longs.get_as_long();
      let mut doc = Document::new();
      doc.add(StoredField::from_string("stored", value.to_string())?);
      doc.add(NumericDocValuesField::new("dv", value));
      writer.add_document(doc)?;
    }

    writer.force_merge(1)?;
    writer.close()?;

    self.assert_dv_iterate(dir.clone())?;
    self.assert_dv_advance(dir, 1)
  }

  fn assert_dv_advance<D>(&self, dir: std::sync::Arc<D>, jump_step: i32) -> Result<()>
  where
    D: crate::core::store::directory::Directory + 'static,
  {
    let reader = directory_reader::open(dir)?;
    let context = get_context(&reader)?;
    for leaf in context.leaves()? {
      let r = leaf.reader();
      let mut stored_fields = r.stored_fields()?;

      let max_doc = r.max_doc()?;
      let mut jump = jump_step;
      while jump < max_doc {
        let mut doc_values = DocValues::get_numeric(r, "dv")?;
        let mut doc_id = 0;
        while doc_id < max_doc {
          let base = format!(
            "document #{}/{}, jumping {} from #{}",
            doc_id,
            max_doc,
            jump,
            doc_id - jump
          );
          let stored_doc = stored_fields.document(doc_id)?;
          let stored_value = stored_doc.get("stored")?;
          if let Some(stored_value) = stored_value {
            assert!(
              doc_values.advance_exact(doc_id)?,
              "There should be a DocValue for {}",
              base
            );
            assert_eq!(
              stored_value
                .parse::<i64>()
                .expect("stored value should parse"),
              doc_values.long_value()?,
              "The doc value should be correct for {}",
              base
            );
          } else {
            assert!(
              !doc_values.advance_exact(doc_id)?,
              "There should be no DocValue for {}",
              base
            );
          }
          doc_id += jump;
        }
        jump += jump_step;
      }
    }
    Ok(())
  }

  fn test_reseek_after_skip_decompression<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let cardinality = (Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SIZE << 1) + 11;
    let mut value_set = HashSet::with_capacity(cardinality as usize);
    while value_set.len() < cardinality as usize {
      value_set.insert(TestUtil::random_simple_string_with_len(random, 64));
    }

    let mut values = value_set.into_iter().collect::<Vec<_>>();
    values.sort();

    let nonexistent_value = format!(
      "{}{}",
      values[(Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SIZE - 1) as usize],
      TestUtil::random_simple_string_range(random, 64, 128)
    );
    let doc_values = values.len();

    let directory = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let mut config = new_index_writer_config_with_analyzer(random, analyzer);
    config.set_use_compound_file(false);
    let writer = RandomIndexWriter::with_config(random, directory.clone(), config);

    let mut field_to_type = HashMap::new();
    for i in 0..280 {
      let mut doc = Document::new();
      doc.add(new_string_field(
        random,
        "id",
        format!("Doc{i}"),
        Store::No,
        &mut field_to_type,
      )?);
      doc.add(SortedDocValuesField::new(
        "sdv",
        new_bytes_ref_from_string(random, &values[i as usize % doc_values])?,
      ));
      writer.add_document(random, doc)?;
    }
    writer.commit(random)?;
    writer.force_merge(random, 1)?;

    let reader = writer.get_reader(random)?;
    writer.close(random)?;

    let leaf = get_only_leaf_reader(&reader)?;
    let mut sorted = leaf
      .get_sorted_doc_values("sdv")?
      .expect("sorted doc values should exist");
    assert_eq!(doc_values as i32, sorted.get_value_count()?);

    let ord1 = sorted.lookup_term(&new_bytes_ref_from_string(random, &values[0])?)?;
    assert!(ord1 >= 0);
    let ord2 = sorted.lookup_term(&new_bytes_ref_from_string(random, &values[1])?)?;
    assert!(ord2 >= ord1);

    let nonexistent_ord =
      sorted.lookup_term(&new_bytes_ref_from_string(random, &nonexistent_value)?)?;
    assert!(nonexistent_ord < 0);
    reader.close()?;
    Ok(())
  }

  fn test_large_terms_compression<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let cardinality = 64;
    let mut values_set = HashSet::with_capacity(cardinality);
    while values_set.len() < cardinality {
      let length = TestUtil::next_int(random, 512, 1024) as usize;
      values_set.insert(TestUtil::random_simple_string_range(random, length, length));
    }

    let values = values_set.into_iter().collect::<Vec<_>>();
    let values_count = values.len();

    let directory = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let mut config = new_index_writer_config_with_analyzer(random, analyzer);
    config.set_use_compound_file(false);
    let writer = RandomIndexWriter::with_config(random, directory.clone(), config);

    let mut field_to_type = HashMap::new();
    for i in 0..256 {
      let mut doc = Document::new();
      doc.add(new_string_field(
        random,
        "id",
        format!("Doc{i}"),
        Store::No,
        &mut field_to_type,
      )?);
      doc.add(SortedDocValuesField::new(
        "sdv",
        new_bytes_ref_from_string(random, &values[i as usize % values_count])?,
      ));
      writer.add_document(random, doc)?;
    }
    writer.commit(random)?;
    writer.force_merge(random, 1)?;

    let reader = writer.get_reader(random)?;
    writer.close(random)?;

    let leaf = get_only_leaf_reader(&reader)?;
    let sorted = leaf
      .get_sorted_doc_values("sdv")?
      .expect("sorted doc values should exist");
    assert_eq!(values_count as i32, sorted.get_value_count()?);
    reader.close()?;
    Ok(())
  }

  fn test_sorted_terms_dict_lookup_ord<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let directory = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let config = new_index_writer_config_with_analyzer(random, analyzer);
    let writer = RandomIndexWriter::with_config(random, directory.clone(), config);
    let num_docs = at_least(
      random,
      Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SIZE + 1,
    );

    for i in 0..num_docs {
      let mut doc = Document::new();
      doc.add(SortedDocValuesField::new(
        "foo",
        new_bytes_ref_from_string(random, &i.to_string())?,
      ));
      writer.add_document(random, doc)?;
    }

    writer.force_merge(random, 1)?;
    let reader = writer.get_reader(random)?;
    writer.close(random)?;

    let leaf = get_only_leaf_reader(&reader)?;
    let mut doc_values = leaf
      .get_sorted_doc_values("foo")?
      .expect("sorted doc values should exist");
    let mut terms_enum = doc_values.terms_enum()?;
    self.do_test_terms_dict_lookup_ord(random, &mut terms_enum)?;
    reader.close()?;
    Ok(())
  }

  fn test_sorted_set_terms_dict_lookup_ord<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let directory = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let config = new_index_writer_config_with_analyzer(random, analyzer);
    let writer = RandomIndexWriter::with_config(random, directory.clone(), config);
    let num_docs = at_least(
      random,
      2 * Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SIZE + 1,
    );

    for i in 0..num_docs {
      let mut doc = Document::new();
      doc.add(SortedSetDocValuesField::new(
        "foo",
        new_bytes_ref_from_string(random, &i.to_string())?,
      ));
      writer.add_document(random, doc)?;
    }

    writer.force_merge(random, 1)?;
    let reader = writer.get_reader(random)?;
    writer.close(random)?;

    let leaf = get_only_leaf_reader(&reader)?;
    let mut doc_values = leaf
      .get_sorted_set_doc_values("foo")?
      .expect("sorted-set doc values should exist");
    let mut terms_enum = doc_values.terms_enum()?;
    self.do_test_terms_dict_lookup_ord(random, &mut terms_enum)?;
    reader.close()?;
    Ok(())
  }

  fn do_test_terms_dict_lookup_ord<R, T>(&self, random: &mut R, terms_enum: &mut T) -> Result<()>
  where
    R: Rng + ?Sized,
    T: TermsEnum,
  {
    let mut terms = Vec::new();
    while let Some(term) = terms_enum.next()? {
      terms.push(BytesRef::deep_copy_of(term.as_ref()));
    }

    for (i, expected) in terms.iter().enumerate() {
      terms_enum.seek_exact_with_ord(i as i64)?;
      assert_eq!(expected, terms_enum.term()?.as_ref());
    }

    for (i, expected) in terms.iter().enumerate().rev() {
      terms_enum.seek_exact_with_ord(i as i64)?;
      assert_eq!(expected, terms_enum.term()?.as_ref());
    }

    let mut i = random.random_range(0..5) as usize;
    while i < terms.len() {
      terms_enum.seek_exact_with_ord(i as i64)?;
      assert_eq!(&terms[i], terms_enum.term()?.as_ref());
      i += 1 + random.random_range(0..5) as usize;
    }

    Ok(())
  }

  fn test_terms_enum_dictionary<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let directory = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let conf = new_index_writer_config_with_analyzer(random, analyzer);
    let writer = RandomIndexWriter::with_config(random, directory.clone(), conf);

    for value in ["abc0defghijkl", "abc1defghijkl", "abc2defghijkl"] {
      let mut doc = Document::new();
      doc.add(SortedDocValuesField::new(
        "field",
        new_bytes_ref_from_string(random, value)?,
      ));
      writer.add_document(random, doc)?;
    }
    writer.force_merge(random, 1)?;
    writer.close(random)?;

    let reader = directory_reader::open(directory)?;
    let leaf = get_only_leaf_reader(&reader)?;
    let mut values = leaf
      .get_sorted_doc_values("field")?
      .expect("sorted doc values should exist");
    let mut terms_enum = values.terms_enum()?;
    assert_eq!(
      BytesRef::from_string("abc0defghijkl"),
      terms_enum
        .next()?
        .expect("first term should exist")
        .into_owned()
    );
    assert_eq!(
      BytesRef::from_string("abc1defghijkl"),
      terms_enum
        .next()?
        .expect("second term should exist")
        .into_owned()
    );
    assert_eq!(
      BytesRef::from_string("abc2defghijkl"),
      terms_enum
        .next()?
        .expect("third term should exist")
        .into_owned()
    );
    assert!(terms_enum.next()?.is_none());
    reader.close()?;
    Ok(())
  }

  fn test_terms_enum_consistency<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_terms = Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SIZE + 10;
    let directory = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let conf = new_index_writer_config_with_analyzer(random, analyzer);
    let writer = RandomIndexWriter::with_config(random, directory.clone(), conf);

    let term_a = b'A';
    let string_supplier = |n: i32| -> String {
      assert!(n < 25 * 25);
      let chars = [
        (term_a + 1 + (n / 25) as u8) as char,
        (term_a + 1 + (n % 25) as u8) as char,
      ];
      chars.into_iter().collect()
    };

    for i in 0..num_terms {
      let mut doc = Document::new();
      doc.add(SortedDocValuesField::new(
        "field",
        new_bytes_ref_from_string(random, &string_supplier(i))?,
      ));
      writer.add_document(random, doc)?;
    }
    writer.force_merge(random, 1)?;
    writer.close(random)?;

    let reader = directory_reader::open(directory)?;
    let leaf = get_only_leaf_reader(&reader)?;
    let mut values = leaf
      .get_sorted_doc_values("field")?
      .expect("sorted doc values should exist");
    let mut terms_enum = values.terms_enum()?;

    terms_enum.seek_exact_with_ord(0)?;
    assert_eq!(0, terms_enum.ord()?);
    assert_eq!(
      SeekStatus::NotFound,
      terms_enum.seek_ceil(&BytesRef::from_string("A"))?
    );
    assert_eq!(0, terms_enum.ord()?);
    assert_eq!(
      BytesRef::from_string(&string_supplier(0)),
      terms_enum.term()?.into_owned()
    );

    for i in 1..num_terms {
      assert_eq!(
        BytesRef::from_string(&string_supplier(i)),
        terms_enum
          .next()?
          .expect("next term should exist while iterating blocks")
          .into_owned()
      );
    }
    assert!(terms_enum.next()?.is_none());
    reader.close()?;
    Ok(())
  }
}
struct BlocksOfVariousBPV {
  rng: RefCell<StdRng>,
  mul: i64,
  min: i64,
  i: RefCell<i32>,
  max_delta: RefCell<i32>,
}

impl BlocksOfVariousBPV {
  fn new<R>(random: &mut R) -> Self
  where
    R: Rng + ?Sized,
  {
    Self {
      rng: RefCell::new(StdRng::seed_from_u64(random.random::<u64>())),
      mul: TestUtil::next_int(random, 1, 100) as i64,
      min: random.random::<i32>() as i64,
      i: RefCell::new(Lucene90DocValuesFormat::NUMERIC_BLOCK_SIZE),
      max_delta: RefCell::new(0),
    }
  }
}

impl LongSupplier for BlocksOfVariousBPV {
  fn get_as_long(&self) -> i64 {
    let mut rng = self.rng.borrow_mut();
    let mut i = self.i.borrow_mut();
    let mut max_delta = self.max_delta.borrow_mut();
    if *i == Lucene90DocValuesFormat::NUMERIC_BLOCK_SIZE {
      *max_delta = 1 << rng.random_range(0..5);
      *i = 0;
    }
    *i += 1;
    self.min + self.mul * rng.random_range(0..std::cmp::max(*max_delta, 1)) as i64
  }
}
