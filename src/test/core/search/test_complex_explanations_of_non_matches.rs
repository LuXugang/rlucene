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
use crate::core::search::query::IntoQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::search::test_complex_explanations::{
  ComplexExplanations, TestComplexExplanations,
};
use crate::test_framework::core::search::base_explanation_test_case::{
  BaseExplanationTestCase, BaseExplanationTestContext, FIELD,
};
use crate::test_framework::core::search::check_hits::CheckHits;
use crate::test_framework::core::util::DefaultIndexSearchCRShared;
use crate::test_framework::core::util::lucene_test_case::random;
use rand::Rng;
use rand::prelude::StdRng;
use std::sync::LazyLock;

/// TestSimpleExplanations that verifies non matches.
#[allow(dead_code)] // for quick search
struct TestComplexExplanationsOfNonMatches {
  base: TestComplexExplanations,
}

static CONTEXT: LazyLock<TestComplexExplanationsOfNonMatches> = LazyLock::new(|| {
  let mut random = random();
  let base = TestComplexExplanations::new(&mut random)
    .expect("failed to initialize TestComplexExplanationsOfNonMatches");
  TestComplexExplanationsOfNonMatches { base }
});

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestComplexExplanationsOfNonMatches, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  f(&CONTEXT, &mut random)
}

impl BaseExplanationTestCase for TestComplexExplanationsOfNonMatches {
  fn q_test<R, Q>(
    &self,
    _random: &mut R,
    searcher: &DefaultIndexSearchCRShared,
    q: Q,
    exp_doc_nrs: &[i32],
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    Q: IntoQuery,
  {
    CheckHits::check_no_match_explanations(q.into_query(), FIELD, searcher, exp_doc_nrs)
  }
}

impl ComplexExplanations for TestComplexExplanationsOfNonMatches {
  fn context(&self) -> &BaseExplanationTestContext {
    self.base.context()
  }
}
mod complex_explanations_tests {
  use super::{ComplexExplanations, run_case};
  use crate::core::util::error::lucene_error::Result;

  #[test]
  fn test_t3() -> Result<()> {
    run_case(|case, random| case.test_t3(random))
  }

  #[test]
  fn test_ma3() -> Result<()> {
    run_case(|case, random| case.test_ma3(random))
  }

  #[test]
  fn test_fq5() -> Result<()> {
    run_case(|case, random| case.test_fq5(random))
  }

  #[test]
  fn test_csq4() -> Result<()> {
    run_case(|case, random| case.test_csq4(random))
  }

  #[test]
  fn test_dmq10() -> Result<()> {
    run_case(|case, random| case.test_dmq10(random))
  }

  #[test]
  fn test_mpq7() -> Result<()> {
    run_case(|case, random| case.test_mpq7(random))
  }

  #[test]
  fn test_bq12() -> Result<()> {
    run_case(|case, random| case.test_bq12(random))
  }

  #[test]
  fn test_bq13() -> Result<()> {
    run_case(|case, random| case.test_bq13(random))
  }

  #[test]
  fn test_bq18() -> Result<()> {
    run_case(|case, random| case.test_bq18(random))
  }

  #[test]
  fn test_bq21() -> Result<()> {
    run_case(|case, random| case.test_bq21(random))
  }

  #[test]
  fn test_bq22() -> Result<()> {
    run_case(|case, random| case.test_bq22(random))
  }
}
