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
use crate::test::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::core::index::base_knn_vectors_format_test_case::BaseKnnVectorsFormatTestCase;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use rand::Rng;
use rand::prelude::StdRng;

#[allow(dead_code)] // for quick search
pub struct TestLucene99HnswVectorsFormat;

mod base_knn_vectors_format_test_case_test {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::codecs::lucene99::test_lucene99_hnsw_vectors_format::run_case;
  use crate::test::core::index::base_knn_vectors_format_test_case::BaseKnnVectorsFormatTestCase;

  #[test]
  fn test_field_constructor() -> Result<()> {
    run_case(|case, random| case.test_field_constructor(random))
  }

  #[test]
  fn test_field_constructor_exceptions() -> Result<()> {
    run_case(|case, random| case.test_field_constructor_exceptions(random))
  }

  #[test]
  fn test_field_set_value() -> Result<()> {
    run_case(|case, random| case.test_field_set_value(random))
  }

  #[test]
  fn test_illegal_dim_change_two_docs() -> Result<()> {
    run_case(|case, random| case.test_illegal_dim_change_two_docs(random))
  }

  #[test]
  fn test_illegal_similarity_function_change() -> Result<()> {
    run_case(|case, random| case.test_illegal_similarity_function_change(random))
  }

  #[test]
  fn test_illegal_dim_change_two_writers() -> Result<()> {
    run_case(|case, random| case.test_illegal_dim_change_two_writers(random))
  }
}

impl BaseIndexFileFormatTestCase for TestLucene99HnswVectorsFormat {
  fn add_random_fields<R>(_random: &mut R) -> crate::core::util::error::lucene_error::Result<()>
  where
    R: Rng + ?Sized,
  {
    todo!()
  }
}

impl BaseKnnVectorsFormatTestCase for TestLucene99HnswVectorsFormat {}
impl TestLucene99HnswVectorsFormatTests for TestLucene99HnswVectorsFormat {}

fn run_case<F>(f: F) -> crate::core::util::error::lucene_error::Result<()>
where
  F: FnOnce(
    &TestLucene99HnswVectorsFormat,
    &mut StdRng,
  ) -> crate::core::util::error::lucene_error::Result<()>,
{
  let mut random = random();
  let case = TestLucene99HnswVectorsFormat;
  f(&case, &mut random)
}

trait TestLucene99HnswVectorsFormatTests: BaseKnnVectorsFormatTestCase {}
