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
use crate::test_framework::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test_framework::core::index::base_term_vectors_format_test_case::BaseTermVectorsFormatTestCase;
use crate::test_framework::core::util::lucene_test_case::random;
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;
use rand::prelude::StdRng;

#[allow(dead_code)] // for quick search
pub struct TestLucene90TermVectorsFormat;

#[test]
fn test_skip_redundant_prefetches() -> Result<()> {
  // TODO IMPORTANT DummyCompressingCodec 尚未迁移，因此该
  // test cannot yet force two documents per term-vector chunk as the Java test requires.
  Ok(())
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestLucene90TermVectorsFormat, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestLucene90TermVectorsFormat;
  f(&case, &mut random)
}

mod base_term_vectors_format_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::codecs::lucene90::test_lucene90_term_vectors_format::run_case;
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

impl BaseIndexFileFormatTestCase for TestLucene90TermVectorsFormat {
  type Defaults = crate::test_framework::core::index::base_term_vectors_format_test_case::BaseTermVectorsFormatTestCaseDefaults;

  fn get_codec(&self) -> Result<Codecs> {
    Ok(TestUtil::get_default_codec().into())
  }
}

impl BaseTermVectorsFormatTestCase for TestLucene90TermVectorsFormat {}

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
