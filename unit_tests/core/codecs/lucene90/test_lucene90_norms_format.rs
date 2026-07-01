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
use crate::test::support::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::support::core::index::base_norms_format_test_case::BaseNormsFormatTestCase;
use crate::test::support::core::util::lucene_test_case::random;
use rand::Rng;
use rand::prelude::StdRng;

/// Tests Lucene90NormsFormat
#[allow(dead_code)] // for quick search
pub struct TestLucene90NormsFormat;

impl BaseIndexFileFormatTestCase for TestLucene90NormsFormat {
  fn add_random_fields<R>(_random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    todo!()
  }
}

impl BaseNormsFormatTestCase for TestLucene90NormsFormat {}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestLucene90NormsFormat, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestLucene90NormsFormat;
  f(&case, &mut random)
}

mod base_norms_format_test_case_test {
  use crate::codecs_tests::lucene90::test_lucene90_norms_format::run_case;
  use crate::core::util::error::lucene_error::Result;
  use crate::test::support::core::index::base_norms_format_test_case::BaseNormsFormatTestCase;

  #[test]
  fn test_byte_range() -> Result<()> {
    run_case(|case, random| case.test_byte_range(random))
  }

  #[test]
  fn test_sparse_byte_range() -> Result<()> {
    run_case(|case, random| case.test_sparse_byte_range(random))
  }

  #[test]
  fn test_short_range() -> Result<()> {
    run_case(|case, random| case.test_short_range(random))
  }

  #[test]
  fn test_sparse_short_range() -> Result<()> {
    run_case(|case, random| case.test_sparse_short_range(random))
  }

  #[test]
  fn test_long_range() -> Result<()> {
    run_case(|case, random| case.test_long_range(random))
  }

  #[test]
  fn test_sparse_long_range() -> Result<()> {
    run_case(|case, random| case.test_sparse_long_range(random))
  }

  #[test]
  fn test_full_long_range() -> Result<()> {
    run_case(|case, random| case.test_full_long_range(random))
  }

  #[test]
  fn test_sparse_full_long_range() -> Result<()> {
    run_case(|case, random| case.test_sparse_full_long_range(random))
  }

  #[test]
  fn test_few_values() -> Result<()> {
    run_case(|case, random| case.test_few_values(random))
  }

  #[test]
  fn test_few_sparse_values() -> Result<()> {
    run_case(|case, random| case.test_few_sparse_values(random))
  }

  #[test]
  fn test_few_large_values() -> Result<()> {
    run_case(|case, random| case.test_few_large_values(random))
  }

  #[test]
  fn test_few_sparse_large_values() -> Result<()> {
    run_case(|case, random| case.test_few_sparse_large_values(random))
  }

  #[test]
  fn test_all_zeros() -> Result<()> {
    run_case(|case, random| case.test_all_zeros(random))
  }

  #[test]
  fn test_sparse_all_zeros() -> Result<()> {
    run_case(|case, random| case.test_sparse_all_zeros(random))
  }

  #[test]
  fn test_most_zeros() -> Result<()> {
    run_case(|case, random| case.test_most_zeros(random))
  }

  #[test]
  fn test_outliers() -> Result<()> {
    run_case(|case, random| case.test_outliers(random))
  }

  #[test]
  fn test_sparse_outliers() -> Result<()> {
    run_case(|case, random| case.test_sparse_outliers(random))
  }

  #[test]
  fn test_outliers2() -> Result<()> {
    run_case(|case, random| case.test_outliers2(random))
  }

  #[test]
  fn test_sparse_outliers2() -> Result<()> {
    run_case(|case, random| case.test_sparse_outliers2(random))
  }

  #[test]
  fn test_n_common() -> Result<()> {
    run_case(|case, random| case.test_n_common(random))
  }

  #[test]
  fn test_sparse_n_common() -> Result<()> {
    run_case(|case, random| case.test_sparse_n_common(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_n_common_big() -> Result<()> {
    run_case(|case, random| case.test_n_common_big(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_sparse_n_common_big() -> Result<()> {
    run_case(|case, random| case.test_sparse_n_common_big(random))
  }

  #[test]
  fn test_undead_norms() -> Result<()> {
    run_case(|case, random| case.test_undead_norms(random))
  }
  #[test]
  fn test_threads() -> Result<()> {
    run_case(|case, random| case.test_threads(random))
  }

  #[test]
  fn test_independant_iterators() -> Result<()> {
    run_case(|case, random| case.test_independant_iterators(random))
  }

  #[test]
  fn test_independant_sparse_iterators() -> Result<()> {
    run_case(|case, random| case.test_independant_sparse_iterators(random))
  }
}
