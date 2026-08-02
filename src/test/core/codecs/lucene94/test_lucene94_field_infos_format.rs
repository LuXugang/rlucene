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
use crate::core::codecs::lucene94::lucene94_field_infos_format::SIMILARITY_FUNCTIONS;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::base_field_info_format_test_case::BaseFieldInfoFormatTestCase;
use crate::test_framework::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test_framework::core::util::lucene_test_case::random;
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;
#[allow(dead_code)] // for quick search
pub struct TestLucene94FieldInfosFormat;

// Ensures that all expected vector similarity functions are translatable
// in the format.
#[test]
fn test_vector_similarity_funcs() {
  // This does not necessarily have to be all similarity functions, but
  // differences should be considered carefully.
  let expected_values = [
    VectorSimilarityFunction::Euclidean,
    VectorSimilarityFunction::DotProduct,
    VectorSimilarityFunction::Cosine,
    VectorSimilarityFunction::MaximumInnerProduct,
  ];
  assert_eq!(SIMILARITY_FUNCTIONS, expected_values);
}

impl BaseIndexFileFormatTestCase for TestLucene94FieldInfosFormat {
  type Defaults = crate::test_framework::core::index::base_field_info_format_test_case::BaseFieldInfoFormatTestCaseDefaults;

  fn get_codec(&self) -> Result<Codecs> {
    Ok(TestUtil::get_default_codec().into())
  }
}

impl BaseFieldInfoFormatTestCase for TestLucene94FieldInfosFormat {}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestLucene94FieldInfosFormat, &mut rand::prelude::StdRng) -> Result<()>,
{
  let mut random = random();
  let test = TestLucene94FieldInfosFormat;
  f(&test, &mut random)
}

#[test]
fn test_one_field() -> Result<()> {
  run_case(|test, random| test.test_one_field(random))
}

#[test]
fn test_immutable_attributes() -> Result<()> {
  run_case(|test, random| test.test_immutable_attributes(random))
}

#[test]
fn test_exception_on_create_output() -> Result<()> {
  run_case(|test, random| test.test_exception_on_create_output(random))
}

#[test]
fn test_exception_on_close_output() -> Result<()> {
  run_case(|test, random| test.test_exception_on_close_output(random))
}

#[test]
fn test_exception_on_open_input() -> Result<()> {
  run_case(|test, random| test.test_exception_on_open_input(random))
}

#[test]
fn test_exception_on_close_input() -> Result<()> {
  run_case(|test, random| test.test_exception_on_close_input(random))
}

#[test]
fn test_random() -> Result<()> {
  run_case(|test, random| test.test_random(random))
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
