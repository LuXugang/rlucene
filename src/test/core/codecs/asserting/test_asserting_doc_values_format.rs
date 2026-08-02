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
use crate::core::codecs::Codecs;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::codecs::asserting_codec::AssertingCodec;
use crate::test_framework::core::index::base_doc_values_format_test_case::BaseDocValuesFormatTestCase;
use crate::test_framework::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test_framework::core::index::legacy_base_doc_values_format_test_case::LegacyBaseDocValuesFormatTestCase;
use crate::test_framework::core::util::lucene_test_case::random;
use rand::Rng;
use rand::prelude::StdRng;

/// Test `AssertingDocValuesFormat` directly.
#[allow(dead_code)] // for quick search
struct TestAssertingDocValuesFormat {
  codec: AssertingCodec,
}

impl TestAssertingDocValuesFormat {
  fn new() -> Self {
    Self {
      codec: AssertingCodec::new(),
    }
  }
}

impl BaseIndexFileFormatTestCase for TestAssertingDocValuesFormat {
  type Defaults = crate::test_framework::core::index::legacy_base_doc_values_format_test_case::LegacyBaseDocValuesFormatTestCaseDefaults;

  fn get_codec(&self) -> Result<Codecs> {
    Ok(self.codec.clone().into())
  }
}

impl LegacyBaseDocValuesFormatTestCase for TestAssertingDocValuesFormat {}
impl BaseDocValuesFormatTestCase for TestAssertingDocValuesFormat {}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestAssertingDocValuesFormat, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestAssertingDocValuesFormat::new();
  let codec_guard = case.set_up()?;
  let result = f(&case, &mut random);
  case.tear_down(codec_guard);
  result
}

mod base_doc_values_format_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::codecs::asserting::test_asserting_doc_values_format::run_case;
  use crate::test_framework::core::index::base_doc_values_format_test_case::BaseDocValuesFormatTestCase;

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
  use crate::test::core::codecs::asserting::test_asserting_doc_values_format::run_case;
  use crate::test_framework::core::index::legacy_base_doc_values_format_test_case::LegacyBaseDocValuesFormatTestCase;

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
  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_threads2() -> Result<()> {
    run_case(|case, random| case.test_threads2(random))
  }
  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
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

mod base_index_file_format_test_case_test {
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
