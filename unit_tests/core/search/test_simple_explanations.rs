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
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::disjunction_max_query::DisjunctionMaxQuery;
use crate::core::search::explanation::Explanation;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::multi_phrase_query::MultiPhraseQuery;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::term_query::TermQuery;
use crate::core::util::CoreHelper;
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::search::base_explanation_test_case::{
  ALTFIELD, BaseExplanationTestCase, BaseExplanationTestContext, FIELD,
  before_class_test_explanations,
};
use crate::test::support::core::util::lucene_test_case::random;
use rand::Rng;
use rand::prelude::StdRng;

/// TestExplanations implementation focusing on basic query types
#[allow(dead_code)] // for quick search
pub(crate) struct TestSimpleExplanations {
  context: BaseExplanationTestContext,
}

impl TestSimpleExplanations {
  pub(crate) fn new<R>(random: &mut R) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    Ok(Self {
      context: before_class_test_explanations(random)?,
    })
  }
}

impl BaseExplanationTestCase for TestSimpleExplanations {}

impl SimpleExplanations for TestSimpleExplanations {
  fn context(&self) -> &BaseExplanationTestContext {
    &self.context
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestSimpleExplanations, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestSimpleExplanations::new(&mut random)?;
  f(&case, &mut random)
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

/// TestExplanations implementation focusing on basic query types
pub(crate) trait SimpleExplanations: BaseExplanationTestCase {
  fn context(&self) -> &BaseExplanationTestContext;

  // we focus on queries that don't rewrite to other queries.
  // if we get those covered well, then the ones that rewrite should
  // also be covered.

  /* simple term tests */

  fn test_t1<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.q_test(
      random,
      &self.context().searcher,
      TermQuery::new(Term::from_text(FIELD, "w1")),
      &[0, 1, 2, 3],
    )
  }

  fn test_t2<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let term_query = TermQuery::new(Term::from_text(FIELD, "w1"));
    self.q_test(
      random,
      &self.context().searcher,
      BoostQuery::new(term_query, 100.0)?,
      &[0, 1, 2, 3],
    )
  }

  /* MatchAllDocs */

  fn test_ma1<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.default_test_ma1(random)
  }
  fn default_test_ma1<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.q_test(
      random,
      &self.context().searcher,
      MatchAllDocsQuery::new(),
      &[0, 1, 2, 3],
    )
  }

  fn test_ma2<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.default_test_ma2(random)
  }
  fn default_test_ma2<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let q = MatchAllDocsQuery::new();
    self.q_test(
      random,
      &self.context().searcher,
      BoostQuery::new(q, 1000.0)?,
      &[0, 1, 2, 3],
    )
  }

  /* some simple phrase tests */

  fn test_p1<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let phrase_query = PhraseQuery::from_terms_no_slop(FIELD, &["w1", "w2"])?;
    self.q_test(random, &self.context().searcher, phrase_query, &[0])
  }

  fn test_p2<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let phrase_query = PhraseQuery::from_terms_no_slop(FIELD, &["w1", "w3"])?;
    self.q_test(random, &self.context().searcher, phrase_query, &[1, 3])
  }

  fn test_p3<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let phrase_query = PhraseQuery::from_terms(1, FIELD, &["w1", "w2"])?;
    self.q_test(random, &self.context().searcher, phrase_query, &[0, 1, 2])
  }

  fn test_p4<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let phrase_query = PhraseQuery::from_terms(1, FIELD, &["w2", "w3"])?;
    self.q_test(
      random,
      &self.context().searcher,
      phrase_query,
      &[0, 1, 2, 3],
    )
  }

  fn test_p5<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let phrase_query = PhraseQuery::from_terms(1, FIELD, &["w3", "w2"])?;
    self.q_test(random, &self.context().searcher, phrase_query, &[1, 3])
  }

  fn test_p6<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let phrase_query = PhraseQuery::from_terms(2, FIELD, &["w3", "w2"])?;
    self.q_test(random, &self.context().searcher, phrase_query, &[0, 1, 3])
  }

  fn test_p7<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let phrase_query = PhraseQuery::from_terms(3, FIELD, &["w3", "w2"])?;
    self.q_test(
      random,
      &self.context().searcher,
      phrase_query,
      &[0, 1, 2, 3],
    )
  }

  /* ConstantScoreQueries */

  fn test_csq1<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let q = ConstantScoreQuery::new(self.match_these_items(&[0, 1, 2, 3])?);
    self.q_test(random, &self.context().searcher, q, &[0, 1, 2, 3])
  }

  fn test_csq2<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let q = ConstantScoreQuery::new(self.match_these_items(&[1, 3])?);
    self.q_test(random, &self.context().searcher, q, &[1, 3])
  }

  fn test_csq3<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let q = ConstantScoreQuery::new(self.match_these_items(&[0, 2])?);
    self.q_test(
      random,
      &self.context().searcher,
      BoostQuery::new(q, 1000.0)?,
      &[0, 2],
    )
  }

  /* DisjunctionMaxQuery */

  fn test_dmq1<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let q = DisjunctionMaxQuery::new(
      vec![
        TermQuery::new(Term::from_text(FIELD, "w1")).into(),
        TermQuery::new(Term::from_text(FIELD, "w5")).into(),
      ],
      0.0,
    )?;
    self.q_test(random, &self.context().searcher, q, &[0, 1, 2, 3])
  }

  fn test_dmq2<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let q = DisjunctionMaxQuery::new(
      vec![
        TermQuery::new(Term::from_text(FIELD, "w1")).into(),
        TermQuery::new(Term::from_text(FIELD, "w5")).into(),
      ],
      0.5,
    )?;
    self.q_test(random, &self.context().searcher, q, &[0, 1, 2, 3])
  }

  fn test_dmq3<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let q = DisjunctionMaxQuery::new(
      vec![
        TermQuery::new(Term::from_text(FIELD, "QQ")).into(),
        TermQuery::new(Term::from_text(FIELD, "w5")).into(),
      ],
      0.5,
    )?;
    self.q_test(random, &self.context().searcher, q, &[0])
  }

  fn test_dmq4<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let q = DisjunctionMaxQuery::new(
      vec![
        TermQuery::new(Term::from_text(FIELD, "QQ")).into(),
        TermQuery::new(Term::from_text(FIELD, "xx")).into(),
      ],
      0.5,
    )?;
    self.q_test(random, &self.context().searcher, q, &[2, 3])
  }

  fn test_dmq5<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut boolean_query = BooleanQueryBuilder::new();
    boolean_query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Should)?;
    boolean_query.add(TermQuery::new(Term::from_text(FIELD, "QQ")), Occur::MustNot)?;

    let q = DisjunctionMaxQuery::new(
      vec![
        boolean_query.build().into(),
        TermQuery::new(Term::from_text(FIELD, "xx")).into(),
      ],
      0.5,
    )?;
    self.q_test(random, &self.context().searcher, q, &[2, 3])
  }

  fn test_dmq6<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut boolean_query = BooleanQueryBuilder::new();
    boolean_query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::MustNot)?;
    boolean_query.add(TermQuery::new(Term::from_text(FIELD, "w3")), Occur::Should)?;

    let q = DisjunctionMaxQuery::new(
      vec![
        boolean_query.build().into(),
        TermQuery::new(Term::from_text(FIELD, "xx")).into(),
      ],
      0.5,
    )?;
    self.q_test(random, &self.context().searcher, q, &[0, 1, 2, 3])
  }

  fn test_dmq7<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut boolean_query = BooleanQueryBuilder::new();
    boolean_query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::MustNot)?;
    boolean_query.add(TermQuery::new(Term::from_text(FIELD, "w3")), Occur::Should)?;

    let q = DisjunctionMaxQuery::new(
      vec![
        boolean_query.build().into(),
        TermQuery::new(Term::from_text(FIELD, "w2")).into(),
      ],
      0.5,
    )?;
    self.q_test(random, &self.context().searcher, q, &[0, 1, 2, 3])
  }

  fn test_dmq8<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut boolean_query = BooleanQueryBuilder::new();
    boolean_query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Should)?;

    let boosted_query = TermQuery::new(Term::from_text(FIELD, "w5"));
    boolean_query.add(BoostQuery::new(boosted_query, 100.0)?, Occur::Should)?;

    let xx_boosted_query = TermQuery::new(Term::from_text(FIELD, "xx"));

    let q = DisjunctionMaxQuery::new(
      vec![
        boolean_query.build().into(),
        BoostQuery::new(xx_boosted_query, 100000.0)?.into(),
      ],
      0.5,
    )?;
    self.q_test(random, &self.context().searcher, q, &[0, 2, 3])
  }

  fn test_dmq9<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut boolean_query = BooleanQueryBuilder::new();
    boolean_query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Should)?;

    let boosted_query = TermQuery::new(Term::from_text(FIELD, "w5"));
    boolean_query.add(BoostQuery::new(boosted_query, 100.0)?, Occur::Should)?;

    let xx_boosted_query = TermQuery::new(Term::from_text(FIELD, "xx"));

    let q = DisjunctionMaxQuery::new(
      vec![
        boolean_query.build().into(),
        BoostQuery::new(xx_boosted_query, 0.0)?.into(),
      ],
      0.5,
    )?;

    self.q_test(random, &self.context().searcher, q, &[0, 2, 3])
  }

  /* MultiPhraseQuery */

  fn test_mpq1<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut qb = MultiPhraseQuery::builder();
    qb.add_terms(&self.ta(&["w1"]))?;
    qb.add_terms(&self.ta(&["w2", "w3", "xx"]))?;
    self.q_test(random, &self.context().searcher, qb.build(), &[0, 1, 2, 3])
  }

  fn test_mpq2<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut qb = MultiPhraseQuery::builder();
    qb.add_terms(&self.ta(&["w1"]))?;
    qb.add_terms(&self.ta(&["w2", "w3"]))?;
    self.q_test(random, &self.context().searcher, qb.build(), &[0, 1, 3])
  }

  fn test_mpq3<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut qb = MultiPhraseQuery::builder();
    qb.add_terms(&self.ta(&["w1", "xx"]))?;
    qb.add_terms(&self.ta(&["w2", "w3"]))?;
    self.q_test(random, &self.context().searcher, qb.build(), &[0, 1, 2, 3])
  }

  fn test_mpq4<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut qb = MultiPhraseQuery::builder();
    qb.add_terms(&self.ta(&["w1"]))?;
    qb.add_terms(&self.ta(&["w2"]))?;
    self.q_test(random, &self.context().searcher, qb.build(), &[0])
  }

  fn test_mpq5<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut qb = MultiPhraseQuery::builder();
    qb.add_terms(&self.ta(&["w1"]))?;
    qb.add_terms(&self.ta(&["w2"]))?;
    qb.set_slop(1)?;
    self.q_test(random, &self.context().searcher, qb.build(), &[0, 1, 2])
  }

  fn test_mpq6<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut qb = MultiPhraseQuery::builder();
    qb.add_terms(&self.ta(&["w1", "w3"]))?;
    qb.add_terms(&self.ta(&["w2"]))?;
    qb.set_slop(1)?;
    self.q_test(random, &self.context().searcher, qb.build(), &[0, 1, 2, 3])
  }

  /* some simple tests of boolean queries containing term queries */

  fn test_bq1<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Must)?;
    query.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Must)?;
    self.q_test(
      random,
      &self.context().searcher,
      query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_bq2<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Must)?;
    query.add(TermQuery::new(Term::from_text(FIELD, "w3")), Occur::Must)?;
    self.q_test(random, &self.context().searcher, query.build(), &[2, 3])
  }

  fn test_bq3<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Should)?;
    query.add(TermQuery::new(Term::from_text(FIELD, "w3")), Occur::Must)?;
    self.q_test(
      random,
      &self.context().searcher,
      query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_bq4<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut outer_query = BooleanQueryBuilder::new();
    outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;

    let mut inner_query = BooleanQueryBuilder::new();
    inner_query.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::MustNot)?;
    inner_query.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Should)?;
    outer_query.add(inner_query.build(), Occur::Should)?;

    self.q_test(
      random,
      &self.context().searcher,
      outer_query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_bq5<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut outer_query = BooleanQueryBuilder::new();
    outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;

    let mut inner_query = BooleanQueryBuilder::new();
    inner_query.add(TermQuery::new(Term::from_text(FIELD, "qq")), Occur::Must)?;
    inner_query.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Should)?;
    outer_query.add(inner_query.build(), Occur::Should)?;

    self.q_test(
      random,
      &self.context().searcher,
      outer_query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_bq6<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut outer_query = BooleanQueryBuilder::new();
    outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;

    let mut inner_query = BooleanQueryBuilder::new();
    inner_query.add(TermQuery::new(Term::from_text(FIELD, "qq")), Occur::MustNot)?;
    inner_query.add(TermQuery::new(Term::from_text(FIELD, "w5")), Occur::Should)?;
    outer_query.add(inner_query.build(), Occur::MustNot)?;

    self.q_test(
      random,
      &self.context().searcher,
      outer_query.build(),
      &[1, 2, 3],
    )
  }

  fn test_bq7<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut outer_query = BooleanQueryBuilder::new();
    outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Must)?;

    let mut inner_query = BooleanQueryBuilder::new();
    inner_query.add(TermQuery::new(Term::from_text(FIELD, "qq")), Occur::Should)?;

    let mut child_left = BooleanQueryBuilder::new();
    child_left.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::Should)?;
    child_left.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::MustNot)?;
    inner_query.add(child_left.build(), Occur::Should)?;

    let mut child_right = BooleanQueryBuilder::new();
    child_right.add(TermQuery::new(Term::from_text(FIELD, "w3")), Occur::Must)?;
    child_right.add(TermQuery::new(Term::from_text(FIELD, "w4")), Occur::Must)?;
    inner_query.add(child_right.build(), Occur::Should)?;

    outer_query.add(inner_query.build(), Occur::Must)?;

    self.q_test(random, &self.context().searcher, outer_query.build(), &[0])
  }

  fn test_bq8<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut outer_query = BooleanQueryBuilder::new();
    outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Must)?;

    let mut inner_query = BooleanQueryBuilder::new();
    inner_query.add(TermQuery::new(Term::from_text(FIELD, "qq")), Occur::Should)?;

    let mut child_left = BooleanQueryBuilder::new();
    child_left.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::Should)?;
    child_left.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::MustNot)?;
    inner_query.add(child_left.build(), Occur::Should)?;

    let mut child_right = BooleanQueryBuilder::new();
    child_right.add(TermQuery::new(Term::from_text(FIELD, "w3")), Occur::Must)?;
    child_right.add(TermQuery::new(Term::from_text(FIELD, "w4")), Occur::Must)?;
    inner_query.add(child_right.build(), Occur::Should)?;

    outer_query.add(inner_query.build(), Occur::Should)?;

    self.q_test(
      random,
      &self.context().searcher,
      outer_query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_bq9<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut outer_query = BooleanQueryBuilder::new();
    outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Must)?;

    let mut inner_query = BooleanQueryBuilder::new();
    inner_query.add(TermQuery::new(Term::from_text(FIELD, "qq")), Occur::Should)?;

    let mut child_left = BooleanQueryBuilder::new();
    child_left.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::MustNot)?;
    child_left.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Should)?;
    inner_query.add(child_left.build(), Occur::Should)?;

    let mut child_right = BooleanQueryBuilder::new();
    child_right.add(TermQuery::new(Term::from_text(FIELD, "w3")), Occur::Must)?;
    child_right.add(TermQuery::new(Term::from_text(FIELD, "w4")), Occur::Must)?;
    inner_query.add(child_right.build(), Occur::MustNot)?;

    outer_query.add(inner_query.build(), Occur::Should)?;

    self.q_test(
      random,
      &self.context().searcher,
      outer_query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_bq10<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut outer_query = BooleanQueryBuilder::new();
    outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Must)?;

    let mut inner_query = BooleanQueryBuilder::new();
    inner_query.add(TermQuery::new(Term::from_text(FIELD, "qq")), Occur::Should)?;

    let mut child_left = BooleanQueryBuilder::new();
    child_left.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::MustNot)?;
    child_left.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Should)?;
    inner_query.add(child_left.build(), Occur::Should)?;

    let mut child_right = BooleanQueryBuilder::new();
    child_right.add(TermQuery::new(Term::from_text(FIELD, "w3")), Occur::Must)?;
    child_right.add(TermQuery::new(Term::from_text(FIELD, "w4")), Occur::Must)?;
    inner_query.add(child_right.build(), Occur::MustNot)?;

    outer_query.add(inner_query.build(), Occur::Must)?;

    self.q_test(random, &self.context().searcher, outer_query.build(), &[1])
  }

  fn test_bq11<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;
    let boosted_query = TermQuery::new(Term::from_text(FIELD, "w1"));
    query.add(BoostQuery::new(boosted_query, 1000.0)?, Occur::Should)?;

    self.q_test(
      random,
      &self.context().searcher,
      query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_bq14<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut q = BooleanQueryBuilder::new();
    q.add(
      TermQuery::new(Term::from_text(FIELD, "QQQQQ")),
      Occur::Should,
    )?;
    q.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;
    self.q_test(random, &self.context().searcher, q.build(), &[0, 1, 2, 3])
  }

  fn test_bq15<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut q = BooleanQueryBuilder::new();
    q.add(
      TermQuery::new(Term::from_text(FIELD, "QQQQQ")),
      Occur::MustNot,
    )?;
    q.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;
    self.q_test(random, &self.context().searcher, q.build(), &[0, 1, 2, 3])
  }

  fn test_bq16<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut q = BooleanQueryBuilder::new();
    q.add(
      TermQuery::new(Term::from_text(FIELD, "QQQQQ")),
      Occur::Should,
    )?;

    let mut boolean_query = BooleanQueryBuilder::new();
    boolean_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;
    boolean_query.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::MustNot)?;

    q.add(boolean_query.build(), Occur::Should)?;
    self.q_test(random, &self.context().searcher, q.build(), &[0, 1])
  }

  fn test_bq17<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut q = BooleanQueryBuilder::new();
    q.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Should)?;

    let mut boolean_query = BooleanQueryBuilder::new();
    boolean_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;
    boolean_query.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::MustNot)?;

    q.add(boolean_query.build(), Occur::Should)?;
    self.q_test(random, &self.context().searcher, q.build(), &[0, 1, 2, 3])
  }

  fn test_bq19<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::MustNot)?;
    query.add(TermQuery::new(Term::from_text(FIELD, "w3")), Occur::Should)?;

    self.q_test(random, &self.context().searcher, query.build(), &[0, 1])
  }

  fn test_bq20<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut q = BooleanQueryBuilder::new();
    q.set_minimum_number_should_match(2);
    q.add(
      TermQuery::new(Term::from_text(FIELD, "QQQQQ")),
      Occur::Should,
    )?;
    q.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Should)?;
    q.add(TermQuery::new(Term::from_text(FIELD, "zz")), Occur::Should)?;
    q.add(TermQuery::new(Term::from_text(FIELD, "w5")), Occur::Should)?;
    q.add(TermQuery::new(Term::from_text(FIELD, "w4")), Occur::Should)?;

    self.q_test(random, &self.context().searcher, q.build(), &[0, 3])
  }

  fn test_bq21<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut q = BooleanQueryBuilder::new();
    q.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Should)?;
    q.add(TermQuery::new(Term::from_text(FIELD, "zz")), Occur::Should)?;

    self.q_test(random, &self.context().searcher, q.build(), &[1, 2, 3])
  }

  fn test_bq23<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Filter)?;
    query.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Filter)?;
    self.q_test(
      random,
      &self.context().searcher,
      query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_bq24<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Filter)?;
    query.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Should)?;
    self.q_test(
      random,
      &self.context().searcher,
      query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_bq25<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Filter)?;
    query.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Must)?;
    self.q_test(
      random,
      &self.context().searcher,
      query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_bq26<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Filter)?;
    query.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::MustNot)?;
    self.q_test(random, &self.context().searcher, query.build(), &[0, 1])
  }

  /* BQ of TQ: using alt so some fields have zero boost and some don't */

  fn test_multi_field_bq1<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Must)?;
    query.add(TermQuery::new(Term::from_text(ALTFIELD, "w2")), Occur::Must)?;

    self.q_test(
      random,
      &self.context().searcher,
      query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_multi_field_bq2<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Must)?;
    query.add(TermQuery::new(Term::from_text(ALTFIELD, "w3")), Occur::Must)?;

    self.q_test(random, &self.context().searcher, query.build(), &[2, 3])
  }

  fn test_multi_field_bq3<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Should)?;
    query.add(TermQuery::new(Term::from_text(ALTFIELD, "w3")), Occur::Must)?;

    self.q_test(
      random,
      &self.context().searcher,
      query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_multi_field_bq4<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut outer_query = BooleanQueryBuilder::new();
    outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;

    let mut inner_query = BooleanQueryBuilder::new();
    inner_query.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::MustNot)?;
    inner_query.add(
      TermQuery::new(Term::from_text(ALTFIELD, "w2")),
      Occur::Should,
    )?;
    outer_query.add(inner_query.build(), Occur::Should)?;

    self.q_test(
      random,
      &self.context().searcher,
      outer_query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_multi_field_bq5<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut outer_query = BooleanQueryBuilder::new();
    outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;

    let mut inner_query = BooleanQueryBuilder::new();
    inner_query.add(TermQuery::new(Term::from_text(ALTFIELD, "qq")), Occur::Must)?;
    inner_query.add(
      TermQuery::new(Term::from_text(ALTFIELD, "w2")),
      Occur::Should,
    )?;
    outer_query.add(inner_query.build(), Occur::Should)?;

    self.q_test(
      random,
      &self.context().searcher,
      outer_query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_multi_field_bq6<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut outer_query = BooleanQueryBuilder::new();
    outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;

    let mut inner_query = BooleanQueryBuilder::new();
    inner_query.add(
      TermQuery::new(Term::from_text(ALTFIELD, "qq")),
      Occur::MustNot,
    )?;
    inner_query.add(
      TermQuery::new(Term::from_text(ALTFIELD, "w5")),
      Occur::Should,
    )?;
    outer_query.add(inner_query.build(), Occur::MustNot)?;

    self.q_test(
      random,
      &self.context().searcher,
      outer_query.build(),
      &[1, 2, 3],
    )
  }

  fn test_multi_field_bq7<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut outer_query = BooleanQueryBuilder::new();
    outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Must)?;

    let mut inner_query = BooleanQueryBuilder::new();
    inner_query.add(
      TermQuery::new(Term::from_text(ALTFIELD, "qq")),
      Occur::Should,
    )?;

    let mut child_left = BooleanQueryBuilder::new();
    child_left.add(
      TermQuery::new(Term::from_text(ALTFIELD, "xx")),
      Occur::Should,
    )?;
    child_left.add(
      TermQuery::new(Term::from_text(ALTFIELD, "w2")),
      Occur::MustNot,
    )?;
    inner_query.add(child_left.build(), Occur::Should)?;

    let mut child_right = BooleanQueryBuilder::new();
    child_right.add(TermQuery::new(Term::from_text(ALTFIELD, "w3")), Occur::Must)?;
    child_right.add(TermQuery::new(Term::from_text(ALTFIELD, "w4")), Occur::Must)?;
    inner_query.add(child_right.build(), Occur::Should)?;

    outer_query.add(inner_query.build(), Occur::Must)?;

    self.q_test(random, &self.context().searcher, outer_query.build(), &[0])
  }

  fn test_multi_field_bq8<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut outer_query = BooleanQueryBuilder::new();
    outer_query.add(TermQuery::new(Term::from_text(ALTFIELD, "w1")), Occur::Must)?;

    let mut inner_query = BooleanQueryBuilder::new();
    inner_query.add(TermQuery::new(Term::from_text(FIELD, "qq")), Occur::Should)?;

    let mut child_left = BooleanQueryBuilder::new();
    child_left.add(
      TermQuery::new(Term::from_text(ALTFIELD, "xx")),
      Occur::Should,
    )?;
    child_left.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::MustNot)?;
    inner_query.add(child_left.build(), Occur::Should)?;

    let mut child_right = BooleanQueryBuilder::new();
    child_right.add(TermQuery::new(Term::from_text(ALTFIELD, "w3")), Occur::Must)?;
    child_right.add(TermQuery::new(Term::from_text(FIELD, "w4")), Occur::Must)?;
    inner_query.add(child_right.build(), Occur::Should)?;

    outer_query.add(inner_query.build(), Occur::Should)?;

    self.q_test(
      random,
      &self.context().searcher,
      outer_query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_multi_field_bq9<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut outer_query = BooleanQueryBuilder::new();
    outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Must)?;

    let mut inner_query = BooleanQueryBuilder::new();
    inner_query.add(
      TermQuery::new(Term::from_text(ALTFIELD, "qq")),
      Occur::Should,
    )?;

    let mut child_left = BooleanQueryBuilder::new();
    child_left.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::MustNot)?;
    child_left.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Should)?;
    inner_query.add(child_left.build(), Occur::Should)?;

    let mut child_right = BooleanQueryBuilder::new();
    child_right.add(TermQuery::new(Term::from_text(ALTFIELD, "w3")), Occur::Must)?;
    child_right.add(TermQuery::new(Term::from_text(FIELD, "w4")), Occur::Must)?;
    inner_query.add(child_right.build(), Occur::MustNot)?;

    outer_query.add(inner_query.build(), Occur::Should)?;

    self.q_test(
      random,
      &self.context().searcher,
      outer_query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_multi_field_bq10<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut outer_query = BooleanQueryBuilder::new();
    outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Must)?;

    let mut inner_query = BooleanQueryBuilder::new();
    inner_query.add(
      TermQuery::new(Term::from_text(ALTFIELD, "qq")),
      Occur::Should,
    )?;

    let mut child_left = BooleanQueryBuilder::new();
    child_left.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::MustNot)?;
    child_left.add(
      TermQuery::new(Term::from_text(ALTFIELD, "w2")),
      Occur::Should,
    )?;
    inner_query.add(child_left.build(), Occur::Should)?;

    let mut child_right = BooleanQueryBuilder::new();
    child_right.add(TermQuery::new(Term::from_text(ALTFIELD, "w3")), Occur::Must)?;
    child_right.add(TermQuery::new(Term::from_text(FIELD, "w4")), Occur::Must)?;
    inner_query.add(child_right.build(), Occur::MustNot)?;

    outer_query.add(inner_query.build(), Occur::Must)?;

    self.q_test(random, &self.context().searcher, outer_query.build(), &[1])
  }

  /* BQ of PQ: using alt so some fields have zero boost and some don't */

  fn test_multi_field_bqof_pq1<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();

    let left_child = PhraseQuery::from_terms_no_slop(FIELD, &["w1", "w2"])?;
    query.add(left_child, Occur::Should)?;

    let right_child = PhraseQuery::from_terms_no_slop(ALTFIELD, &["w1", "w2"])?;
    query.add(right_child, Occur::Should)?;

    self.q_test(random, &self.context().searcher, query.build(), &[0])
  }

  fn test_multi_field_bqof_pq2<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();

    let left_child = PhraseQuery::from_terms_no_slop(FIELD, &["w1", "w3"])?;
    query.add(left_child, Occur::Should)?;

    let right_child = PhraseQuery::from_terms_no_slop(ALTFIELD, &["w1", "w3"])?;
    query.add(right_child, Occur::Should)?;

    self.q_test(random, &self.context().searcher, query.build(), &[1, 3])
  }

  fn test_multi_field_bqof_pq3<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();

    let left_child = PhraseQuery::from_terms(1, FIELD, &["w1", "w2"])?;
    query.add(left_child, Occur::Should)?;

    let right_child = PhraseQuery::from_terms(1, ALTFIELD, &["w1", "w2"])?;
    query.add(right_child, Occur::Should)?;

    self.q_test(random, &self.context().searcher, query.build(), &[0, 1, 2])
  }

  fn test_multi_field_bqof_pq4<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();

    let left_child = PhraseQuery::from_terms(1, FIELD, &["w2", "w3"])?;
    query.add(left_child, Occur::Should)?;

    let right_child = PhraseQuery::from_terms(1, ALTFIELD, &["w2", "w3"])?;
    query.add(right_child, Occur::Should)?;

    self.q_test(
      random,
      &self.context().searcher,
      query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_multi_field_bqof_pq5<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();

    let left_child = PhraseQuery::from_terms(1, FIELD, &["w3", "w2"])?;
    query.add(left_child, Occur::Should)?;

    let right_child = PhraseQuery::from_terms(1, ALTFIELD, &["w3", "w2"])?;
    query.add(right_child, Occur::Should)?;

    self.q_test(random, &self.context().searcher, query.build(), &[1, 3])
  }

  fn test_multi_field_bqof_pq6<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();

    let left_child = PhraseQuery::from_terms(2, FIELD, &["w3", "w2"])?;
    query.add(left_child, Occur::Should)?;

    let right_child = PhraseQuery::from_terms(2, ALTFIELD, &["w3", "w2"])?;
    query.add(right_child, Occur::Should)?;

    self.q_test(random, &self.context().searcher, query.build(), &[0, 1, 3])
  }

  fn test_multi_field_bqof_pq7<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();

    let left_child = PhraseQuery::from_terms(3, FIELD, &["w3", "w2"])?;
    query.add(left_child, Occur::Should)?;

    let right_child = PhraseQuery::from_terms(1, ALTFIELD, &["w3", "w2"])?;
    query.add(right_child, Occur::Should)?;

    self.q_test(
      random,
      &self.context().searcher,
      query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_synonym_query<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // TODO: SynonymQuery 未实现
    Ok(())
  }

  fn test_equality(&self) -> Result<()> {
    let e1 = Explanation::match_no_details(1f32, "an explanation");
    let e2 = Explanation::match_(
      1f32,
      "an explanation",
      vec![Explanation::match_no_details(1f32, "a subexplanation")],
    );
    let e25 = Explanation::match_(
      1f32,
      "an explanation",
      vec![Explanation::match_(
        1f32,
        "a subexplanation",
        vec![Explanation::match_no_details(1f32, "a subsubexplanation")],
      )],
    );
    let e3 = Explanation::match_no_details(1f32, "an explanation");
    let e4 = Explanation::match_no_details(2f32, "an explanation");
    let e5 = Explanation::no_match_no_details("an explanation");
    let e6 = Explanation::no_match(
      "an explanation",
      vec![Explanation::match_no_details(1f32, "a subexplanation")],
    );
    let e7 = Explanation::no_match_no_details("an explanation");
    let e8 = Explanation::match_no_details(1f32, "another explanation");

    assert!(e1 == e3);
    assert!(e1 != e2);
    assert!(e2 != e25);
    assert!(e1 != e4);
    assert!(e1 != e5);
    assert!(e5 == e7);
    assert!(e5 != e6);
    assert!(e1 != e8);

    assert_eq!(
      CoreHelper::calculate_hash(&e1),
      CoreHelper::calculate_hash(&e3)
    );
    assert_eq!(
      CoreHelper::calculate_hash(&e5),
      CoreHelper::calculate_hash(&e7)
    );

    Ok(())
  }
}
