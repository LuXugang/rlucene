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
use crate::core::document::knn_byte_vector_field::KnnByteVectorField;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::knn_byte_vector_query::KnnByteVectorQuery;
use crate::core::search::query::Query;
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::vector_util::tests::random_vector_bytes_dim;
use crate::test::core::search::base_knn_vector_query_test_case::BaseKnnVectorQueryTestCase;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use rand::Rng;
use rand::rngs::StdRng;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub struct TestKnnByteVectorQuery;
fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestKnnByteVectorQuery, &mut StdRng) -> Result<()>,
{
  let case = TestKnnByteVectorQuery;
  let mut random = random();
  f(&case, &mut random)
}

mod base_knn_vector_query_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::search::base_knn_vector_query_test_case::BaseKnnVectorQueryTestCase;
  use crate::test::core::search::test_knn_byte_vector_query::run_case;

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
    run_case(|case, _random| case.test_find_all())
  }

  #[test]
  fn test_find_fewer() -> Result<()> {
    run_case(|case, _random| case.test_find_fewer())
  }

  #[test]
  fn test_search_boost() -> Result<()> {
    run_case(|case, _random| case.test_search_boost())
  }

  #[test]
  fn test_simple_filter() -> Result<()> {
    run_case(|case, _random| case.test_simple_filter())
  }

  #[test]
  fn test_filter_with_no_vector_matches() -> Result<()> {
    run_case(|case, _random| case.test_filter_with_no_vector_matches())
  }

  #[test]
  fn test_dimension_mismatch() -> Result<()> {
    run_case(|case, _random| case.test_dimension_mismatch())
  }

  #[test]
  fn test_non_vector_field() -> Result<()> {
    run_case(|case, _random| case.test_non_vector_field())
  }

  #[test]
  fn test_illegal_arguments() -> Result<()> {
    run_case(|case, _random| case.test_illegal_arguments())
  }

  #[test]

  fn test_score_euclidean() -> Result<()> {
    run_case(|case, _random| case.test_score_euclidean())
  }

  #[test]

  fn test_score_cosine() -> Result<()> {
    run_case(|case, _random| case.test_score_cosine())
  }

  #[test]
  fn test_score_mip() -> Result<()> {
    run_case(|case, _random| case.test_score_mip())
  }

  #[test]
  fn test_explain() -> Result<()> {
    run_case(|case, _random| case.test_explain())
  }

  #[test]
  fn test_explain_multiple_segments() -> Result<()> {
    run_case(|case, _random| case.test_explain_multiple_segments())
  }

  #[test]
  fn test_skewed_index() -> Result<()> {
    run_case(|case, _random| case.test_skewed_index())
  }
}
impl BaseKnnVectorQueryTestCase for TestKnnByteVectorQuery {
  type KnnVectorQuery = KnnByteVectorQuery;

  fn get_knn_vector_query(
    &self,
    field: &str,
    query: Vec<f32>,
    k: usize,
    query_filter: Option<Query>,
  ) -> Result<Self::KnnVectorQuery> {
    KnnByteVectorQuery::with_filter(field, float_to_bytes(query), k, query_filter)
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
    let v = random_vector_bytes_dim(random, dim);
    v.into_iter().map(|value| (value as i8) as f32).collect()
  }

  fn get_knn_vector_field_with_similarity(
    &self,
    name: &str,
    vector: Vec<f32>,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<Fields> {
    Ok(
      KnnByteVectorField::with_similarity_function(
        name,
        float_to_bytes(vector),
        similarity_function,
      )?
      .into(),
    )
  }

  fn get_knn_vector_field(&self, name: &str, vector: Vec<f32>) -> Result<Fields> {
    Ok(KnnByteVectorField::new(name, float_to_bytes(vector))?.into())
  }

  type Directory = Arc<DirEnum>;

  fn new_directory_for_test<R>(&self, random: &mut R) -> Result<Self::Directory>
  where
    R: Rng + ?Sized,
  {
    self.default_new_directory_for_test(random)
  }
}

fn float_to_bytes(query: Vec<f32>) -> Vec<u8> {
  query
    .into_iter()
    .map(|value| {
      assert!(
        value <= i8::MAX as f32 && value >= i8::MIN as f32 && value.fract() == 0.0,
        "float value cannot be converted to byte; provided: {value}"
      );
      (value as i8) as u8
    })
    .collect()
}
