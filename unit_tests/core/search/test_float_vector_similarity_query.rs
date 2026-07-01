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
use crate::core::search::float_vector_similarity_query::FloatVectorSimilarityQuery;
use crate::core::search::query::Query;
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::search::base_vector_similarity_query_test_case::{
  BaseVectorSimilarityQueryBase, BaseVectorSimilarityQueryTestCase,
};
use crate::test::support::core::util::lucene_test_case::{at_least_usize, random};
use crate::test::support::core::util::test_vector_util::random_vector_dim;
use rand::Rng;
use rand::rngs::StdRng;
use std::sync::Arc;

pub struct TestFloatVectorSimilarityQuery {
  base: BaseVectorSimilarityQueryBase,
}

impl TestFloatVectorSimilarityQuery {
  fn new<R>(random: &mut R) -> Self
  where
    R: Rng + ?Sized,
  {
    let name = std::any::type_name::<Self>();
    let vector_field = format!("{}:VectorField", name);
    let id_field = format!("{}:IdField", name);
    let num_docs = at_least_usize(random, 100);
    let dim = at_least_usize(random, 5);
    let base = BaseVectorSimilarityQueryBase::new(
      vector_field,
      id_field,
      VectorSimilarityFunction::Euclidean,
      num_docs,
      dim,
    );
    Self { base }
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestFloatVectorSimilarityQuery, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestFloatVectorSimilarityQuery::new(&mut random);
  f(&case, &mut random)
}

fn run_case_mut<F>(f: F) -> Result<()>
where
  F: FnOnce(&mut TestFloatVectorSimilarityQuery, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let mut case = TestFloatVectorSimilarityQuery::new(&mut random);
  f(&mut case, &mut random)
}

impl BaseVectorSimilarityQueryTestCase for TestFloatVectorSimilarityQuery {
  type Vector = Vec<f32>;
  type VectorQuery = FloatVectorSimilarityQuery;
  type Directory = Arc<DirEnum>;

  fn get_random_vector<R>(&self, random: &mut R, dim: usize) -> Self::Vector
  where
    R: Rng + ?Sized,
  {
    random_vector_dim(random, dim)
  }

  fn compare(&self, vector1: &Self::Vector, vector2: &Self::Vector) -> Result<f32> {
    self.base.function.compare_f32(vector1, vector2)
  }

  fn check_equals(&self, vector1: &Self::Vector, vector2: &Self::Vector) -> bool {
    vector1 == vector2
  }

  fn get_vector_field(
    &self,
    name: &str,
    vector: Self::Vector,
    function: VectorSimilarityFunction,
  ) -> Result<Fields> {
    Ok(KnnFloatVectorField::with_similarity_function(name, vector, function)?.into())
  }

  fn get_vector_query(
    &self,
    field: &str,
    vector: Self::Vector,
    traversal_similarity: f32,
    result_similarity: f32,
    filter: Option<Query>,
  ) -> Result<Self::VectorQuery> {
    FloatVectorSimilarityQuery::with_traversal_similarity_and_filter(
      field,
      vector,
      traversal_similarity,
      result_similarity,
      filter,
    )
  }

  fn get_throwing_vector_query(
    &self,
    field: &str,
    vector: Self::Vector,
    traversal_similarity: f32,
    result_similarity: f32,
    filter: Option<Query>,
  ) -> Result<Self::VectorQuery> {
    let mut v = FloatVectorSimilarityQuery::with_traversal_similarity_and_filter(
      field,
      vector,
      traversal_similarity,
      result_similarity,
      filter,
    )?;
    v.has_vector_scorer = false;
    Ok(v)
  }

  fn new_directory_for_test<R>(&self, random: &mut R) -> Result<Self::Directory>
  where
    R: Rng + ?Sized,
  {
    self.default_new_directory_for_test(random)
  }

  fn get_base(&self) -> &BaseVectorSimilarityQueryBase {
    &self.base
  }

  fn get_base_mut(&mut self) -> &mut BaseVectorSimilarityQueryBase {
    &mut self.base
  }
}

mod base_vector_similarity_query_test_case_test {
  use crate::core::util::error::lucene_error::Result;
  use crate::search_tests::test_float_vector_similarity_query::{run_case, run_case_mut};
  use crate::test::support::core::search::base_vector_similarity_query_test_case::BaseVectorSimilarityQueryTestCase;

  #[test]
  fn test_equals() -> Result<()> {
    run_case(|case, random| case.test_equals(random))
  }

  #[test]
  fn test_empty_index() -> Result<()> {
    run_case_mut(|case, random| case.test_empty_index(random))
  }

  #[test]
  fn test_extremes() -> Result<()> {
    run_case(|case, random| case.test_extremes(random))
  }

  #[test]
  fn test_random_filter() -> Result<()> {
    run_case(|case, random| case.test_random_filter(random))
  }

  #[test]
  fn test_filter_with_no_matches() -> Result<()> {
    run_case(|case, random| case.test_filter_with_no_matches(random))
  }

  #[test]
  fn test_dimension_mismatch() -> Result<()> {
    run_case(|case, random| case.test_dimension_mismatch(random))
  }

  #[test]
  fn test_non_vectors_field() -> Result<()> {
    run_case(|case, random| case.test_non_vectors_field(random))
  }

  #[test]
  fn test_some_deletes() -> Result<()> {
    run_case(|case, random| case.test_some_deletes(random))
  }

  #[test]
  fn test_all_deletes() -> Result<()> {
    run_case(|case, random| case.test_all_deletes(random))
  }

  #[test]
  fn test_boost_query() -> Result<()> {
    run_case(|case, random| case.test_boost_query(random))
  }

  #[test]
  fn test_vectors_above_similarity() -> Result<()> {
    run_case(|case, random| case.test_vectors_above_similarity(random))
  }

  #[test]
  fn test_fallback_to_exact() -> Result<()> {
    run_case(|case, random| case.test_fallback_to_exact(random))
  }

  #[test]
  fn test_approximate() -> Result<()> {
    run_case(|case, random| case.test_approximate(random))
  }

  #[test]
  fn test_timeout() -> Result<()> {
    run_case(|case, random| case.test_timeout(random))
  }
}
