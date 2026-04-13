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
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::core::index::base_points_format_test_case::BasePointsFormatTestCase;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use rand::Rng;
use rand::prelude::StdRng;

#[allow(dead_code)] // for quick search
pub struct TestLucene90PointsFormat;

mod base_points_format_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::codecs::lucene90::test_lucene90_points_format::run_case;
  use crate::test::core::index::base_points_format_test_case::BasePointsFormatTestCase;

  #[test]
  fn test_basic() -> Result<()> {
    run_case(|case, random| case.test_basic(random))
  }

  #[test]
  fn test_merge() -> Result<()> {
    run_case(|case, random| case.test_merge(random))
  }

  #[test]
  fn test_all_point_docs_deleted_in_segment() -> Result<()> {
    run_case(|case, random| case.test_all_point_docs_deleted_in_segment(random))
  }
  #[test]
  fn test_with_exceptions() -> Result<()> {
    run_case(|case, _random| case.test_with_exceptions())
  }
  // TODO
  fn test_multi_valued() -> Result<()> {
    run_case(|case, random| case.test_multi_valued(random))
  }

  #[test]
  fn test_all_equal() -> Result<()> {
    run_case(|case, random| case.test_all_equal(random))
  }

  // TODO
  fn test_one_dim_equal() -> Result<()> {
    run_case(|case, random| case.test_one_dim_equal(random))
  }
  // TODO
  fn test_one_dim_two_values() -> Result<()> {
    run_case(|case, random| case.test_one_dim_two_values(random))
  }
  #[test]
  fn test_big_int_n_dims() -> Result<()> {
    run_case(|case, random| case.test_big_int_n_dims(random))
  }

  // TODO
  fn test_random_binary_tiny() -> Result<()> {
    run_case(|case, random| case.test_random_binary_tiny(random))
  }

  // TODO
  fn test_random_binary_medium() -> Result<()> {
    run_case(|case, random| case.test_random_binary_medium(random))
  }
  #[ignore]
  #[test]
  fn test_random_binary_big() -> Result<()> {
    run_case(|case, random| case.test_random_binary_big(random))
  }
  #[test]
  fn test_add_indexes() -> Result<()> {
    run_case(|case, random| case.test_add_indexes(random))
  }
  #[test]
  fn test_merge_missing() -> Result<()> {
    run_case(|case, random| case.test_merge_missing(random))
  }

  #[test]
  fn test_doc_count_edge_cases() -> Result<()> {
    run_case(|case, _random| case.test_doc_count_edge_cases())
  }

  #[test]
  fn test_random_doc_count() -> Result<()> {
    run_case(|case, random| case.test_random_doc_count(random))
  }
  #[test]
  fn test_mismatched_fields() -> Result<()> {
    run_case(|case, random| case.test_mismatched_fields(random))
  }
}

impl BaseIndexFileFormatTestCase for TestLucene90PointsFormat {
  fn add_random_fields<R>(_random: &mut R) -> crate::core::util::error::lucene_error::Result<()>
  where
    R: Rng + ?Sized,
  {
    todo!()
  }
}

impl BasePointsFormatTestCase for TestLucene90PointsFormat {}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestLucene90PointsFormat, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestLucene90PointsFormat;
  f(&case, &mut random)
}
