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
use crate::core::document::fields::Fields;
use crate::core::document::knn_float_vector_field::KnnFloatVectorField;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::knn_float_vector_query::KnnFloatVectorQuery;
use crate::core::search::query::Query;
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::vector_util::tests::random_vector_dim;
use crate::test::core::search::base_knn_vector_query_test_case::BaseKnnVectorQueryTestCase;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use rand::Rng;
use rand::rngs::StdRng;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub struct TestKnnFloatVectorQuery;

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestKnnFloatVectorQuery, &mut StdRng) -> Result<()>,
{
  let case = TestKnnFloatVectorQuery;
  let mut random = random();
  f(&case, &mut random)
}

mod base_knn_vector_query_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::search::base_knn_vector_query_test_case::BaseKnnVectorQueryTestCase;
  use crate::test::core::search::test_knn_float_vector_query::run_case;

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
  fn test_random_with_filter() -> Result<()> {
    run_case(|case, random| case.test_random_with_filter(random))
  }
  #[test]
  fn test_filter_with_same_score() -> Result<()> {
    run_case(|case, random| case.test_filter_with_same_score(random))
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
  fn test_bot_set_query() -> Result<()> {
    run_case(|case, random| case.test_bot_set_query(random))
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

impl BaseKnnVectorQueryTestCase for TestKnnFloatVectorQuery {
  type KnnVectorQuery = KnnFloatVectorQuery;

  fn get_knn_vector_query(
    &self,
    field: &str,
    query: Vec<f32>,
    k: usize,
    query_filter: Option<Query>,
  ) -> Result<Self::KnnVectorQuery> {
    KnnFloatVectorQuery::with_filter(field, query, k, query_filter)
  }

  fn get_throwing_knn_vector_query(
    &self,
    field: &str,
    query: Vec<f32>,
    k: usize,
    query_filter: Option<Query>,
  ) -> Result<Self::KnnVectorQuery> {
    let _ = (field, query, k, query_filter);
    todo!()
  }

  fn get_knn_vector_query_no_filter(
    &self,
    field: &str,
    query: Vec<f32>,
    k: usize,
  ) -> Result<Self::KnnVectorQuery> {
    self.get_knn_vector_query(field, query, k, None)
  }

  fn random_vector<R>(&self, random: &mut R, dim: usize) -> Vec<f32>
  where
    R: Rng + ?Sized,
  {
    random_vector_dim(random, dim)
  }

  fn get_knn_vector_field_with_similarity(
    &self,
    name: &str,
    vector: Vec<f32>,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<Fields> {
    Ok(KnnFloatVectorField::with_similarity_function(name, vector, similarity_function)?.into())
  }

  fn get_knn_vector_field(&self, name: &str, vector: Vec<f32>) -> Result<Fields> {
    Ok(KnnFloatVectorField::new(name, vector)?.into())
  }

  type Directory = Arc<DirEnum>;

  fn new_directory_for_test<R>(&self, random: &mut R) -> Result<Self::Directory>
  where
    R: Rng + ?Sized,
  {
    self.default_new_directory_for_test(random)
  }
}
