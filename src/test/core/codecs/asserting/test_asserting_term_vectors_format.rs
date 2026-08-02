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
use crate::test_framework::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test_framework::core::index::base_term_vectors_format_test_case::{
  BaseTermVectorsFormatTestCase, ReadPastLastPositionException,
};
use crate::test_framework::core::util::lucene_test_case::random;
use rand::Rng;
use rand::prelude::StdRng;

/// Test `AssertingTermVectorsFormat` directly.
#[allow(dead_code)] // for quick search
struct TestAssertingTermVectorsFormat {
  codec: AssertingCodec,
}

impl TestAssertingTermVectorsFormat {
  fn new() -> Self {
    Self {
      codec: AssertingCodec::new(),
    }
  }
}

impl BaseIndexFileFormatTestCase for TestAssertingTermVectorsFormat {
  type Defaults = crate::test_framework::core::index::base_term_vectors_format_test_case::BaseTermVectorsFormatTestCaseDefaults;

  fn get_codec(&self) -> Result<Codecs> {
    Ok(self.codec.clone().into())
  }
}

impl BaseTermVectorsFormatTestCase for TestAssertingTermVectorsFormat {
  fn get_read_past_last_position_exception_class(&self) -> ReadPastLastPositionException {
    ReadPastLastPositionException::Assertion
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestAssertingTermVectorsFormat, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestAssertingTermVectorsFormat::new();
  let codec_guard = case.set_up()?;
  let result = f(&case, &mut random);
  case.tear_down(codec_guard);
  result
}

mod base_term_vectors_format_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::codecs::asserting::test_asserting_term_vectors_format::run_case;
  use crate::test_framework::core::index::base_term_vectors_format_test_case::BaseTermVectorsFormatTestCase;

  #[test]
  fn test_rare_vectors() -> Result<()> {
    run_case(|case, random| case.test_rare_vectors(random))
  }

  #[test]
  fn test_high_freqs() -> Result<()> {
    run_case(|case, random| case.test_high_freqs(random))
  }

  #[test]
  fn test_lots_of_fields() -> Result<()> {
    run_case(|case, random| case.test_lots_of_fields(random))
  }

  #[test]
  fn test_mixed_options() -> Result<()> {
    run_case(|case, random| case.test_mixed_options(random))
  }

  #[test]
  fn test_random() -> Result<()> {
    run_case(|case, random| case.test_random(random))
  }

  #[test]
  fn test_merge() -> Result<()> {
    run_case(|case, random| case.test_merge(random))
  }

  #[test]
  fn test_merge_with_deletes() -> Result<()> {
    run_case(|case, random| case.test_merge_with_deletes(random))
  }

  #[test]
  fn test_merge_with_index_sort() -> Result<()> {
    run_case(|case, random| case.test_merge_with_index_sort(random))
  }

  #[test]
  fn test_merge_with_index_sort_and_deletes() -> Result<()> {
    run_case(|case, random| case.test_merge_with_index_sort_and_deletes(random))
  }

  #[test]
  fn test_clone() -> Result<()> {
    run_case(|case, random| case.test_clone(random))
  }

  #[test]
  fn test_postings_enum_freqs() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_freqs(random))
  }

  #[test]
  fn test_postings_enum_positions() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_positions(random))
  }

  #[test]
  fn test_postings_enum_offsets() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_offsets(random))
  }

  #[test]
  fn test_postings_enum_offsets_without_positions() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_offsets_without_positions(random))
  }

  #[test]
  fn test_postings_enum_payloads() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_payloads(random))
  }

  #[test]
  fn test_postings_enum_all() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_all(random))
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
