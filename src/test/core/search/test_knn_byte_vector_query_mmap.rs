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
use crate::test::core::search::test_knn_byte_vector_query::TestKnnByteVectorQuery;
use crate::test_framework::core::util::lucene_test_case::random;
use rand::rngs::StdRng;

#[allow(dead_code)] // for quick search
pub(crate) struct TestKnnByteVectorQueryMMap;

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestKnnByteVectorQuery, &mut StdRng) -> Result<()>,
{
  let case = TestKnnByteVectorQuery::new_mmap();
  let mut random = random();
  f(&case, &mut random)
}

mod base_knn_vector_query_test_case_tests {
  use super::run_case;
  use crate::core::util::error::lucene_error::Result;
  use crate::test_framework::core::search::base_knn_vector_query_test_case::BaseKnnVectorQueryTestCase;

  #[test]
  fn test_equals() -> Result<()> {
    run_case(|case, _random| case.test_equals())
  }

  #[test]
  fn test_get_field() -> Result<()> {
    run_case(|case, _random| case.test_get_field())
  }

  #[test]
  fn test_get_k() -> Result<()> {
    run_case(|case, _random| case.test_get_k())
  }

  #[test]
  fn test_get_filter() -> Result<()> {
    run_case(|case, _random| case.test_get_filter())
  }

  #[test]
  fn test_empty_index() -> Result<()> {
    run_case(|case, _random| case.test_empty_index())
  }

  #[test]
  fn test_find_all() -> Result<()> {
    run_case(|case, random| case.test_find_all(random))
  }

  #[test]
  fn test_find_fewer() -> Result<()> {
    run_case(|case, random| case.test_find_fewer(random))
  }

  #[test]
  fn test_search_boost() -> Result<()> {
    run_case(|case, random| case.test_search_boost(random))
  }

  #[test]
  fn test_simple_filter() -> Result<()> {
    run_case(|case, random| case.test_simple_filter(random))
  }

  #[test]
  fn test_filter_with_no_vector_matches() -> Result<()> {
    run_case(|case, random| case.test_filter_with_no_vector_matches(random))
  }

  #[test]
  fn test_dimension_mismatch() -> Result<()> {
    run_case(|case, random| case.test_dimension_mismatch(random))
  }

  #[test]
  fn test_non_vector_field() -> Result<()> {
    run_case(|case, random| case.test_non_vector_field(random))
  }

  #[test]
  fn test_illegal_arguments() -> Result<()> {
    run_case(|case, _random| case.test_illegal_arguments())
  }

  #[test]
  fn test_different_reader() -> Result<()> {
    run_case(|case, random| case.test_different_reader(random))
  }

  #[test]
  fn test_score_euclidean() -> Result<()> {
    run_case(|case, random| case.test_score_euclidean(random))
  }

  #[test]
  fn test_score_cosine() -> Result<()> {
    run_case(|case, random| case.test_score_cosine(random))
  }

  #[test]
  fn test_score_mip() -> Result<()> {
    run_case(|case, random| case.test_score_mip(random))
  }

  #[test]
  fn test_explain() -> Result<()> {
    run_case(|case, random| case.test_explain(random))
  }

  #[test]
  fn test_explain_multiple_segments() -> Result<()> {
    run_case(|case, random| case.test_explain_multiple_segments(random))
  }

  #[test]
  fn test_skewed_index() -> Result<()> {
    run_case(|case, random| case.test_skewed_index(random))
  }

  #[test]
  fn test_random() -> Result<()> {
    run_case(|case, random| case.test_random(random))
  }

  #[test]
  fn test_filter_with_same_score() -> Result<()> {
    run_case(|case, random| case.test_filter_with_same_score(random))
  }

  #[test]
  fn test_random_with_filter() -> Result<()> {
    run_case(|case, random| case.test_random_with_filter(random))
  }

  #[test]
  fn test_deletes() -> Result<()> {
    run_case(|case, random| case.test_deletes(random))
  }

  #[test]
  fn test_all_deletes() -> Result<()> {
    run_case(|case, random| case.test_all_deletes(random))
  }

  #[test]
  fn test_merge_away_all_values() -> Result<()> {
    run_case(|case, random| case.test_merge_away_all_values(random))
  }

  #[test]
  fn test_no_live_docs_reader() -> Result<()> {
    run_case(|case, random| case.test_no_live_docs_reader(random))
  }

  #[test]
  #[ignore = "BitSet filter reuse is not implemented"]
  fn test_bit_set_query() -> Result<()> {
    run_case(|case, random| case.test_bit_set_query(random))
  }

  #[test]
  fn test_time_limiting_knn_collector_manager() -> Result<()> {
    run_case(|case, random| case.test_time_limiting_knn_collector_manager(random))
  }

  #[test]
  fn test_timeout() -> Result<()> {
    run_case(|case, random| case.test_timeout(random))
  }

  #[test]
  fn test_same_field_different_formats() -> Result<()> {
    run_case(|case, random| case.test_same_field_different_formats(random))
  }
}

#[test]
fn test_to_string() -> Result<()> {
  run_case(|case, random| case.test_to_string(random))
}

#[test]
fn test_get_target() -> Result<()> {
  run_case(|case, _random| case.test_get_target())
}

#[test]
fn test_vector_encoding_mismatch() -> Result<()> {
  run_case(|case, random| case.test_vector_encoding_mismatch(random))
}
