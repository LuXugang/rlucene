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
use crate::test_framework::core::index::base_postings_format_test_case::BasePostingsFormatTestCase;
use crate::test_framework::core::index::random_postings_tester::RandomPostingsTester;
use crate::test_framework::core::util::lucene_test_case::random;
use parking_lot::Mutex;
use rand::Rng;
use rand::prelude::StdRng;
use std::sync::LazyLock;

/// Test `AssertingPostingsFormat` directly.
#[allow(dead_code)] // for quick search
struct TestAssertingPostingsFormat {
  codec: AssertingCodec,
}

impl TestAssertingPostingsFormat {
  fn new() -> Self {
    Self {
      codec: AssertingCodec::new(),
    }
  }
}

static POSTINGS_TESTER: LazyLock<Mutex<RandomPostingsTester>> = LazyLock::new(|| {
  let mut random = random();
  Mutex::new(
    RandomPostingsTester::new(&mut random)
      .expect("failed to initialize TestAssertingPostingsFormat"),
  )
});

impl BaseIndexFileFormatTestCase for TestAssertingPostingsFormat {
  type Defaults = crate::test_framework::core::index::base_postings_format_test_case::BasePostingsFormatTestCaseDefaults;

  fn get_codec(&self) -> Result<Codecs> {
    Ok(self.codec.clone().into())
  }
}

impl BasePostingsFormatTestCase for TestAssertingPostingsFormat {
  fn create_postings<R>(&self, _random: &mut R) -> &Mutex<RandomPostingsTester>
  where
    R: Rng + ?Sized,
  {
    &POSTINGS_TESTER
  }

  fn is_postings_enum_reuse_implemented(&self) -> bool {
    false
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestAssertingPostingsFormat, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestAssertingPostingsFormat::new();
  let codec_guard = case.set_up()?;
  let result = f(&case, &mut random);
  case.tear_down(codec_guard);
  result
}

mod base_postings_format_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::codecs::asserting::test_asserting_postings_format::run_case;
  use crate::test_framework::core::index::base_postings_format_test_case::BasePostingsFormatTestCase;

  #[test]
  fn test_docs_only() -> Result<()> {
    run_case(|case, random| case.test_docs_only(random))
  }

  #[test]
  fn test_docs_and_freqs() -> Result<()> {
    run_case(|case, random| case.test_docs_and_freqs(random))
  }

  #[test]
  fn test_docs_and_freqs_and_positions() -> Result<()> {
    run_case(|case, random| case.test_docs_and_freqs_and_positions(random))
  }

  #[test]
  fn test_docs_and_freqs_and_positions_and_payloads() -> Result<()> {
    run_case(|case, random| case.test_docs_and_freqs_and_positions_and_payloads(random))
  }

  #[test]
  fn test_docs_and_freqs_and_positions_and_offsets() -> Result<()> {
    run_case(|case, random| case.test_docs_and_freqs_and_positions_and_offsets(random))
  }

  #[test]
  fn test_docs_and_freqs_and_positions_and_offsets_and_payloads() -> Result<()> {
    run_case(|case, random| case.test_docs_and_freqs_and_positions_and_offsets_and_payloads(random))
  }

  #[test]
  fn test_random() -> Result<()> {
    run_case(|case, random| case.test_random(random))
  }

  #[test]
  fn test_postings_enum_reuse() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_reuse(random))
  }

  #[test]
  fn test_just_empty_field() -> Result<()> {
    run_case(|case, random| case.test_just_empty_field(random))
  }

  #[test]
  fn test_empty_field_and_empty_term() -> Result<()> {
    run_case(|case, random| case.test_empty_field_and_empty_term(random))
  }

  #[test]
  fn test_didnt_want_freqs_but_asked_anyway() -> Result<()> {
    run_case(|case, random| case.test_didnt_want_freqs_but_asked_anyway(random))
  }

  #[test]
  fn test_ask_for_positions_when_not_there() -> Result<()> {
    run_case(|case, random| case.test_ask_for_positions_when_not_there(random))
  }

  #[test]
  fn test_ghosts() -> Result<()> {
    run_case(|case, random| case.test_ghosts(random))
  }

  #[test]
  fn test_disorder() -> Result<()> {
    run_case(|case, random| case.test_disorder(random))
  }

  #[test]
  fn test_binary_search_term_leaf() -> Result<()> {
    run_case(|case, random| case.test_binary_search_term_leaf(random))
  }

  #[test]
  fn test_level2_ghosts() -> Result<()> {
    run_case(|case, random| case.test_level2_ghosts(random))
  }

  #[test]
  fn test_inverted_write() -> Result<()> {
    run_case(|case, random| case.test_inverted_write(random))
  }

  #[test]
  fn test_postings_enum_docs_only() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_docs_only(random))
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
  fn test_postings_enum_payloads() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_payloads(random))
  }

  #[test]
  fn test_postings_enum_all() -> Result<()> {
    run_case(|case, random| case.test_postings_enum_all(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_line_file_docs() -> Result<()> {
    run_case(|case, random| case.test_line_file_docs(random))
  }

  #[test]
  fn test_mismatched_fields() -> Result<()> {
    run_case(|case, random| case.test_mismatched_fields(random))
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
