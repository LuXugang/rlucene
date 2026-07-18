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
use crate::test::core::search::test_simple_explanations::{
  SimpleExplanations, TestSimpleExplanations,
};
use crate::test_framework::core::search::base_explanation_test_case::{
  BaseExplanationTestCase, BaseExplanationTestContext, FIELD,
};
use crate::test_framework::core::search::check_hits::CheckHits;
use crate::test_framework::core::util::DefaultIndexSearchCRShared;
use crate::test_framework::core::util::lucene_test_case::random;
use rand::Rng;
use rand::prelude::StdRng;

/// TestSimpleExplanations implementation that verifies non matches.
#[allow(dead_code)] // for quick search
struct TestSimpleExplanationsOfNonMatches {
  base: TestSimpleExplanations,
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestSimpleExplanationsOfNonMatches, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestSimpleExplanations::new(&mut random)?;
  let case = TestSimpleExplanationsOfNonMatches { base: case };
  f(&case, &mut random)
}

impl BaseExplanationTestCase for TestSimpleExplanationsOfNonMatches {
  /// Overrides the super-trait to ignore matches and focus on non-matches.
  ///
  /// See [`CheckHits::check_no_match_explanations`].
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

impl SimpleExplanations for TestSimpleExplanationsOfNonMatches {
  fn context(&self) -> &BaseExplanationTestContext {
    self.base.context()
  }
}

mod simple_explanations_tests {
  use super::{SimpleExplanations, run_case};
  use crate::core::util::error::lucene_error::Result;

  #[test]
  fn test_t1() -> Result<()> {
    run_case(|case, random| case.test_t1(random))
  }

  #[test]
  fn test_t2() -> Result<()> {
    run_case(|case, random| case.test_t2(random))
  }

  #[test]
  fn test_ma1() -> Result<()> {
    run_case(|case, random| case.test_ma1(random))
  }

  #[test]
  fn test_ma2() -> Result<()> {
    run_case(|case, random| case.test_ma2(random))
  }

  #[test]
  fn test_p1() -> Result<()> {
    run_case(|case, random| case.test_p1(random))
  }

  #[test]
  fn test_p2() -> Result<()> {
    run_case(|case, random| case.test_p2(random))
  }

  #[test]
  fn test_p3() -> Result<()> {
    run_case(|case, random| case.test_p3(random))
  }

  #[test]
  fn test_p4() -> Result<()> {
    run_case(|case, random| case.test_p4(random))
  }

  #[test]
  fn test_p5() -> Result<()> {
    run_case(|case, random| case.test_p5(random))
  }

  #[test]
  fn test_p6() -> Result<()> {
    run_case(|case, random| case.test_p6(random))
  }

  #[test]
  fn test_p7() -> Result<()> {
    run_case(|case, random| case.test_p7(random))
  }

  #[test]
  fn test_csq1() -> Result<()> {
    run_case(|case, random| case.test_csq1(random))
  }

  #[test]
  fn test_csq2() -> Result<()> {
    run_case(|case, random| case.test_csq2(random))
  }

  #[test]
  fn test_csq3() -> Result<()> {
    run_case(|case, random| case.test_csq3(random))
  }

  #[test]
  fn test_dmq1() -> Result<()> {
    run_case(|case, random| case.test_dmq1(random))
  }

  #[test]
  fn test_dmq2() -> Result<()> {
    run_case(|case, random| case.test_dmq2(random))
  }

  #[test]
  fn test_dmq3() -> Result<()> {
    run_case(|case, random| case.test_dmq3(random))
  }

  #[test]
  fn test_dmq4() -> Result<()> {
    run_case(|case, random| case.test_dmq4(random))
  }

  #[test]
  fn test_dmq5() -> Result<()> {
    run_case(|case, random| case.test_dmq5(random))
  }

  #[test]
  fn test_dmq6() -> Result<()> {
    run_case(|case, random| case.test_dmq6(random))
  }

  #[test]
  fn test_dmq7() -> Result<()> {
    run_case(|case, random| case.test_dmq7(random))
  }

  #[test]
  fn test_dmq8() -> Result<()> {
    run_case(|case, random| case.test_dmq8(random))
  }

  #[test]
  fn test_dmq9() -> Result<()> {
    run_case(|case, random| case.test_dmq9(random))
  }

  #[test]
  fn test_mpq1() -> Result<()> {
    run_case(|case, random| case.test_mpq1(random))
  }

  #[test]
  fn test_mpq2() -> Result<()> {
    run_case(|case, random| case.test_mpq2(random))
  }

  #[test]
  fn test_mpq3() -> Result<()> {
    run_case(|case, random| case.test_mpq3(random))
  }

  #[test]
  fn test_mpq4() -> Result<()> {
    run_case(|case, random| case.test_mpq4(random))
  }

  #[test]
  fn test_mpq5() -> Result<()> {
    run_case(|case, random| case.test_mpq5(random))
  }

  #[test]
  fn test_mpq6() -> Result<()> {
    run_case(|case, random| case.test_mpq6(random))
  }

  #[test]
  fn test_bq1() -> Result<()> {
    run_case(|case, random| case.test_bq1(random))
  }

  #[test]
  fn test_bq2() -> Result<()> {
    run_case(|case, random| case.test_bq2(random))
  }

  #[test]
  fn test_bq3() -> Result<()> {
    run_case(|case, random| case.test_bq3(random))
  }

  #[test]
  fn test_bq4() -> Result<()> {
    run_case(|case, random| case.test_bq4(random))
  }

  #[test]
  fn test_bq5() -> Result<()> {
    run_case(|case, random| case.test_bq5(random))
  }

  #[test]
  fn test_bq6() -> Result<()> {
    run_case(|case, random| case.test_bq6(random))
  }

  #[test]
  fn test_bq7() -> Result<()> {
    run_case(|case, random| case.test_bq7(random))
  }

  #[test]
  fn test_bq8() -> Result<()> {
    run_case(|case, random| case.test_bq8(random))
  }

  #[test]
  fn test_bq9() -> Result<()> {
    run_case(|case, random| case.test_bq9(random))
  }

  #[test]
  fn test_bq10() -> Result<()> {
    run_case(|case, random| case.test_bq10(random))
  }

  #[test]
  fn test_bq11() -> Result<()> {
    run_case(|case, random| case.test_bq11(random))
  }

  #[test]
  fn test_bq14() -> Result<()> {
    run_case(|case, random| case.test_bq14(random))
  }

  #[test]
  fn test_bq15() -> Result<()> {
    run_case(|case, random| case.test_bq15(random))
  }

  #[test]
  fn test_bq16() -> Result<()> {
    run_case(|case, random| case.test_bq16(random))
  }

  #[test]
  fn test_bq17() -> Result<()> {
    run_case(|case, random| case.test_bq17(random))
  }

  #[test]
  fn test_bq19() -> Result<()> {
    run_case(|case, random| case.test_bq19(random))
  }

  #[test]
  fn test_bq20() -> Result<()> {
    run_case(|case, random| case.test_bq20(random))
  }

  #[test]
  fn test_bq21() -> Result<()> {
    run_case(|case, random| case.test_bq21(random))
  }

  #[test]
  fn test_bq23() -> Result<()> {
    run_case(|case, random| case.test_bq23(random))
  }

  #[test]
  fn test_bq24() -> Result<()> {
    run_case(|case, random| case.test_bq24(random))
  }

  #[test]
  fn test_bq25() -> Result<()> {
    run_case(|case, random| case.test_bq25(random))
  }

  #[test]
  fn test_bq26() -> Result<()> {
    run_case(|case, random| case.test_bq26(random))
  }

  #[test]
  fn test_multi_field_bq1() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq1(random))
  }

  #[test]
  fn test_multi_field_bq2() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq2(random))
  }

  #[test]
  fn test_multi_field_bq3() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq3(random))
  }

  #[test]
  fn test_multi_field_bq4() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq4(random))
  }

  #[test]
  fn test_multi_field_bq5() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq5(random))
  }

  #[test]
  fn test_multi_field_bq6() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq6(random))
  }

  #[test]
  fn test_multi_field_bq7() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq7(random))
  }

  #[test]
  fn test_multi_field_bq8() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq8(random))
  }

  #[test]
  fn test_multi_field_bq9() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq9(random))
  }

  #[test]
  fn test_multi_field_bq10() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq10(random))
  }

  #[test]
  fn test_multi_field_bqof_pq1() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bqof_pq1(random))
  }

  #[test]
  fn test_multi_field_bqof_pq2() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bqof_pq2(random))
  }

  #[test]
  fn test_multi_field_bqof_pq3() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bqof_pq3(random))
  }

  #[test]
  fn test_multi_field_bqof_pq4() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bqof_pq4(random))
  }

  #[test]
  fn test_multi_field_bqof_pq5() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bqof_pq5(random))
  }

  #[test]
  fn test_multi_field_bqof_pq6() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bqof_pq6(random))
  }

  #[test]
  fn test_multi_field_bqof_pq7() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bqof_pq7(random))
  }

  #[test]
  fn test_synonym_query() -> Result<()> {
    run_case(|case, random| case.test_synonym_query(random))
  }

  #[test]
  fn test_equality() -> Result<()> {
    run_case(|case, _random| case.test_equality())
  }
}
