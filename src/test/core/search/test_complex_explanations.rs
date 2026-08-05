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
use crate::core::search::index_searcher::get_default_similarity;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::multi_phrase_query::MultiPhraseQuery;
use crate::core::search::query::Query;
use crate::core::search::similarities_impl::classic_similarity;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::search::base_explanation_test_case::{
  BaseExplanationTestCase, BaseExplanationTestContext, FIELD, before_class_test_explanations,
};
use crate::test_framework::core::util::lucene_test_case::random;
use parking_lot::Mutex;
use rand::Rng;
use rand::prelude::StdRng;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::LazyLock;

#[allow(dead_code)] // for quick search
pub(crate) struct TestComplexExplanations {
  context: BaseExplanationTestContext,
}

impl TestComplexExplanations {
  pub(crate) fn new<R>(random: &mut R) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    let context = before_class_test_explanations(random)?;
    Ok(Self { context })
  }

  pub(crate) fn tear_down(&mut self) -> Result<()> {
    self
      .context
      .searcher
      .set_similarity(get_default_similarity()?);
    Ok(())
  }
}

impl BaseExplanationTestCase for TestComplexExplanations {
  fn initialize(&mut self) -> Result<()> {
    self
      .context
      .searcher
      .set_similarity(classic_similarity::new());
    Ok(())
  }
}

impl ComplexExplanations for TestComplexExplanations {
  fn context(&self) -> &BaseExplanationTestContext {
    &self.context
  }
}

static CONTEXT: LazyLock<Mutex<TestComplexExplanations>> = LazyLock::new(|| {
  let mut random = random();
  Mutex::new(
    TestComplexExplanations::new(&mut random)
      .expect("failed to initialize TestComplexExplanations"),
  )
});

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestComplexExplanations, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let mut case = CONTEXT.lock();
  case.initialize()?;
  let result = catch_unwind(AssertUnwindSafe(|| f(&case, &mut random)));
  let tear_down_result = case.tear_down();
  match result {
    Ok(result) => {
      tear_down_result?;
      result
    },
    Err(payload) => {
      let _ = tear_down_result;
      resume_unwind(payload)
    },
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

/// TestExplanations implementation that builds up super crazy complex queries on the assumption
/// that if the explanations work out right for them, they should work for anything.
pub(crate) trait ComplexExplanations: BaseExplanationTestCase {
  fn context(&self) -> &BaseExplanationTestContext;

  // :TODO: we really need more crazy complex cases.

  // //////////////////////////////////////////////////////////////////

  // The rest of these aren't that complex, but they are <i>somewhat</i> complex, and they expose
  // weakness in dealing with queries that match with scores of 0 wrapped in other queries

  fn test_t3<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let query = TermQuery::new(Term::from_text(FIELD, "w1"));
    self.bq_test(
      random,
      &self.context().searcher,
      BoostQuery::new(query, 0.0)?,
      &[0, 1, 2, 3],
    )
  }

  fn test_ma3<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let q = MatchAllDocsQuery::new();
    self.bq_test(
      random,
      &self.context().searcher,
      BoostQuery::new(q, 0.0)?,
      &[0, 1, 2, 3],
    )
  }

  fn test_fq5<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let query = TermQuery::new(Term::from_text(FIELD, "xx"));
    let mut filtered = BooleanQueryBuilder::new();
    filtered.add(BoostQuery::new(query, 0.0)?, Occur::Must)?;
    filtered.add(self.match_these_items(&[1, 3])?, Occur::Filter)?;
    self.bq_test(random, &self.context().searcher, filtered.build(), &[3])
  }

  fn test_csq4<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let q = ConstantScoreQuery::new(self.match_these_items(&[3])?);
    self.bq_test(
      random,
      &self.context().searcher,
      BoostQuery::new(q, 0.0)?,
      &[3],
    )
  }

  fn test_dmq10<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Should)?;
    let boosted_query = TermQuery::new(Term::from_text(FIELD, "w5"));
    query.add(BoostQuery::new(boosted_query, 100.0)?, Occur::Should)?;

    let xx_boosted_query = TermQuery::new(Term::from_text(FIELD, "xx"));

    let q = DisjunctionMaxQuery::new(
      vec![
        query.build().into(),
        BoostQuery::new(xx_boosted_query, 0.0)?.into(),
      ],
      0.5,
    )?;
    self.bq_test(
      random,
      &self.context().searcher,
      BoostQuery::new(q, 0.0)?,
      &[0, 2, 3],
    )
  }

  fn test_mpq7<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut qb = MultiPhraseQuery::builder();
    qb.add_terms(&self.ta(&["w1"]))?;
    qb.add_terms(&self.ta(&["w2"]))?;
    qb.set_slop(1)?;
    let q: Query = qb.build().into();
    self.bq_test(
      random,
      &self.context().searcher,
      BoostQuery::new(q, 0.0)?,
      &[0, 1, 2],
    )
  }

  fn test_bq12<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // NOTE: using qtest not bqtest
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;
    let boosted_query = TermQuery::new(Term::from_text(FIELD, "w2"));
    query.add(BoostQuery::new(boosted_query, 0.0)?, Occur::Should)?;

    self.q_test(
      random,
      &self.context().searcher,
      query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_bq13<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // NOTE: using qtest not bqtest
    let mut query = BooleanQueryBuilder::new();
    query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;
    let boosted_query = TermQuery::new(Term::from_text(FIELD, "w5"));
    query.add(BoostQuery::new(boosted_query, 0.0)?, Occur::MustNot)?;

    self.q_test(random, &self.context().searcher, query.build(), &[1, 2, 3])
  }

  fn test_bq18<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // NOTE: using qtest not bqtest
    let mut query = BooleanQueryBuilder::new();
    let boosted_query = TermQuery::new(Term::from_text(FIELD, "w1"));
    query.add(BoostQuery::new(boosted_query, 0.0)?, Occur::Must)?;
    query.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Should)?;

    self.q_test(
      random,
      &self.context().searcher,
      query.build(),
      &[0, 1, 2, 3],
    )
  }

  fn test_bq21<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut builder = BooleanQueryBuilder::new();
    builder.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Must)?;
    builder.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Should)?;

    let query = builder.build();

    self.bq_test(
      random,
      &self.context().searcher,
      BoostQuery::new(query, 0.0)?,
      &[0, 1, 2, 3],
    )
  }

  fn test_bq22<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut builder = BooleanQueryBuilder::new();
    let boosted_query = TermQuery::new(Term::from_text(FIELD, "w1"));
    builder.add(BoostQuery::new(boosted_query, 0.0)?, Occur::Must)?;
    builder.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Should)?;
    let query = builder.build();

    self.bq_test(
      random,
      &self.context().searcher,
      BoostQuery::new(query, 0.0)?,
      &[0, 1, 2, 3],
    )
  }
}
