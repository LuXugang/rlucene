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
use crate::test::core::codecs::lucene90::test_lucene90_stored_fields_format::TestLucene90StoredFieldsFormatTests;
use crate::test_framework::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test_framework::core::index::base_stored_fields_format_test_case::BaseStoredFieldsFormatTestCase;
use crate::test_framework::core::util::lucene_test_case::random;
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;
use rand::prelude::StdRng;

/// Test the merge instance of the Lucene90 stored fields format.
#[allow(dead_code)] // for quick search
pub struct TestLucene90StoredFieldsFormatMergeInstance;

impl BaseIndexFileFormatTestCase for TestLucene90StoredFieldsFormatMergeInstance {
  type Defaults = crate::test_framework::core::index::base_stored_fields_format_test_case::BaseStoredFieldsFormatTestCaseDefaults;

  fn get_codec(&self) -> Result<Codecs> {
    Ok(TestUtil::get_default_codec().into())
  }

  fn should_test_merge_instance(&self) -> bool {
    true
  }
}

impl BaseStoredFieldsFormatTestCase for TestLucene90StoredFieldsFormatMergeInstance {}
impl TestLucene90StoredFieldsFormatTests for TestLucene90StoredFieldsFormatMergeInstance {}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestLucene90StoredFieldsFormatMergeInstance, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestLucene90StoredFieldsFormatMergeInstance;
  f(&case, &mut random)
}

mod base_stored_fields_format_test_case_test {
  use super::*;

  #[test]
  fn test_random_stored_fields() -> Result<()> {
    run_case(|case, random| case.test_random_stored_fields(random))
  }

  #[test]
  fn test_stored_fields_order() -> Result<()> {
    run_case(|case, random| case.test_stored_fields_order(random))
  }

  #[test]
  fn test_binary_field_offset_length() -> Result<()> {
    run_case(|case, random| case.test_binary_field_offset_length(random))
  }

  #[test]
  fn test_numeric_field() -> Result<()> {
    run_case(|case, random| case.test_numeric_field(random))
  }

  #[test]
  fn test_indexed_bit() -> Result<()> {
    run_case(|case, random| case.test_indexed_bit(random))
  }

  #[test]
  fn test_read_skip() -> Result<()> {
    run_case(|case, random| case.test_read_skip(random))
  }

  #[test]
  fn test_empty_docs() -> Result<()> {
    run_case(|case, random| case.test_empty_docs(random))
  }

  #[test]
  fn test_concurrent_reads() -> Result<()> {
    run_case(|case, random| case.test_concurrent_reads(random))
  }

  #[test]
  fn test_write_read_merge() -> Result<()> {
    run_case(|case, random| case.test_write_read_merge(random))
  }

  #[test]
  fn test_merge_filter_reader() -> Result<()> {
    run_case(|case, random| case.test_merge_filter_reader(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_big_documents() -> Result<()> {
    run_case(|case, random| case.test_big_documents(random))
  }

  #[test]
  fn test_bulk_merge_with_deletes() -> Result<()> {
    run_case(|case, random| case.test_bulk_merge_with_deletes(random))
  }

  #[test]
  fn test_mismatched_fields() -> Result<()> {
    run_case(|case, random| case.test_mismatched_fields(random))
  }

  #[test]
  fn test_random_stored_fields_with_index_sort() -> Result<()> {
    run_case(|case, random| case.test_random_stored_fields_with_index_sort(random))
  }

  #[test]
  fn test_line_file_docs() -> Result<()> {
    run_case(|case, random| case.test_line_file_docs(random))
  }
}

mod lucene90_stored_fields_format_tests {
  use super::*;

  #[test]
  fn test_skip_redundant_prefetches() -> Result<()> {
    run_case(|case, random| case.test_skip_redundant_prefetches(random))
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
