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
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::query::Query;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::search::random_approximation_query::RandomApproximationQuery;
use crate::test_framework::core::search::search_equivalence_test_base::{
  SearchEquivalenceTestBase, SearchEquivalenceTestBaseMeta,
};
use crate::test_framework::core::util::lucene_test_case::random;
use rand::Rng;
use std::sync::LazyLock;
/// Basic equivalence tests for approximations.
pub struct TestApproximationSearchEquivalence {
  meta: SearchEquivalenceTestBaseMeta,
}

static CONTEXT: LazyLock<TestApproximationSearchEquivalence> = LazyLock::new(|| {
  let mut random = random();
  TestApproximationSearchEquivalence {
    meta: SearchEquivalenceTestBaseMeta::new(&mut random)
      .expect("failed to initialize TestApproximationSearchEquivalence"),
  }
});

impl TestApproximationSearchEquivalence {
  fn new<R>(_random: &mut R) -> &'static Self
  where
    R: Rng + ?Sized,
  {
    &CONTEXT
  }

  fn random_approximation_query<R>(&self, query: impl Into<Query>, random: &mut R) -> Query
  where
    R: Rng + ?Sized,
  {
    RandomApproximationQuery::new(query.into(), random).into()
  }

  fn random_term_other_than<R>(&self, random: &mut R, term: &Term) -> Term
  where
    R: Rng + ?Sized,
  {
    loop {
      let other = self.random_term(random);
      if &other != term {
        return other;
      }
    }
  }
}

impl SearchEquivalenceTestBase for TestApproximationSearchEquivalence {
  fn get_meta(&self) -> &SearchEquivalenceTestBaseMeta {
    &self.meta
  }
}

#[test]
fn test_conjunction() -> Result<()> {
  let mut random = random();
  let case = TestApproximationSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let q1 = TermQuery::new(t1);
  let q2 = TermQuery::new(t2);

  let mut bq1 = BooleanQueryBuilder::new();
  bq1.add(q1.clone(), Occur::Must)?;
  bq1.add(q2.clone(), Occur::Must)?;

  let mut bq2 = BooleanQueryBuilder::new();
  bq2.add(
    case.random_approximation_query(q1, &mut random),
    Occur::Must,
  )?;
  bq2.add(
    case.random_approximation_query(q2, &mut random),
    Occur::Must,
  )?;

  case.assert_same_scores(&mut random, &bq1.build().into(), &bq2.build().into())
}

#[test]
fn test_nested_conjunction() -> Result<()> {
  let mut random = random();
  let case = TestApproximationSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term_other_than(&mut random, &t1);
  let t3 = case.random_term(&mut random);
  let q1 = TermQuery::new(t1);
  let q2 = TermQuery::new(t2);
  let q3 = TermQuery::new(t3);

  let mut bq1 = BooleanQueryBuilder::new();
  bq1.add(q1.clone(), Occur::Must)?;
  bq1.add(q2.clone(), Occur::Must)?;

  let mut bq2 = BooleanQueryBuilder::new();
  bq2.add(bq1.build(), Occur::Must)?;
  bq2.add(q3.clone(), Occur::Must)?;

  let mut bq3 = BooleanQueryBuilder::new();
  bq3.add(
    case.random_approximation_query(q1, &mut random),
    Occur::Must,
  )?;
  bq3.add(
    case.random_approximation_query(q2, &mut random),
    Occur::Must,
  )?;

  let mut bq4 = BooleanQueryBuilder::new();
  bq4.add(bq3.build(), Occur::Must)?;
  bq4.add(q3, Occur::Must)?;

  case.assert_same_scores(&mut random, &bq2.build().into(), &bq4.build().into())
}

#[test]
fn test_disjunction() -> Result<()> {
  let mut random = random();
  let case = TestApproximationSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let q1 = TermQuery::new(t1);
  let q2 = TermQuery::new(t2);

  let mut bq1 = BooleanQueryBuilder::new();
  bq1.add(q1.clone(), Occur::Should)?;
  bq1.add(q2.clone(), Occur::Should)?;

  let mut bq2 = BooleanQueryBuilder::new();
  bq2.add(
    case.random_approximation_query(q1, &mut random),
    Occur::Should,
  )?;
  bq2.add(
    case.random_approximation_query(q2, &mut random),
    Occur::Should,
  )?;

  case.assert_same_scores(&mut random, &bq1.build().into(), &bq2.build().into())
}

#[test]
fn test_nested_disjunction() -> Result<()> {
  let mut random = random();
  let case = TestApproximationSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term_other_than(&mut random, &t1);
  let t3 = case.random_term(&mut random);
  let q1 = TermQuery::new(t1);
  let q2 = TermQuery::new(t2);
  let q3 = TermQuery::new(t3);

  let mut bq1 = BooleanQueryBuilder::new();
  bq1.add(q1.clone(), Occur::Should)?;
  bq1.add(q2.clone(), Occur::Should)?;

  let mut bq2 = BooleanQueryBuilder::new();
  bq2.add(bq1.build(), Occur::Should)?;
  bq2.add(q3.clone(), Occur::Should)?;

  let mut bq3 = BooleanQueryBuilder::new();
  bq3.add(
    case.random_approximation_query(q1, &mut random),
    Occur::Should,
  )?;
  bq3.add(
    case.random_approximation_query(q2, &mut random),
    Occur::Should,
  )?;

  let mut bq4 = BooleanQueryBuilder::new();
  bq4.add(bq3.build(), Occur::Should)?;
  bq4.add(q3, Occur::Should)?;

  case.assert_same_scores(&mut random, &bq2.build().into(), &bq4.build().into())
}

#[test]
fn test_disjunction_in_conjunction() -> Result<()> {
  let mut random = random();
  let case = TestApproximationSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term_other_than(&mut random, &t1);
  let t3 = case.random_term(&mut random);
  let q1 = TermQuery::new(t1);
  let q2 = TermQuery::new(t2);
  let q3 = TermQuery::new(t3);

  let mut bq1 = BooleanQueryBuilder::new();
  bq1.add(q1.clone(), Occur::Should)?;
  bq1.add(q2.clone(), Occur::Should)?;

  let mut bq2 = BooleanQueryBuilder::new();
  bq2.add(bq1.build(), Occur::Must)?;
  bq2.add(q3.clone(), Occur::Must)?;

  let mut bq3 = BooleanQueryBuilder::new();
  bq3.add(
    case.random_approximation_query(q1, &mut random),
    Occur::Should,
  )?;
  bq3.add(
    case.random_approximation_query(q2, &mut random),
    Occur::Should,
  )?;

  let mut bq4 = BooleanQueryBuilder::new();
  bq4.add(bq3.build(), Occur::Must)?;
  bq4.add(q3, Occur::Must)?;

  case.assert_same_scores(&mut random, &bq2.build().into(), &bq4.build().into())
}

#[test]
fn test_conjunction_in_disjunction() -> Result<()> {
  let mut random = random();
  let case = TestApproximationSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term_other_than(&mut random, &t1);
  let t3 = case.random_term(&mut random);
  let q1 = TermQuery::new(t1);
  let q2 = TermQuery::new(t2);
  let q3 = TermQuery::new(t3);

  let mut bq1 = BooleanQueryBuilder::new();
  bq1.add(q1.clone(), Occur::Must)?;
  bq1.add(q2.clone(), Occur::Must)?;

  let mut bq2 = BooleanQueryBuilder::new();
  bq2.add(bq1.build(), Occur::Should)?;
  bq2.add(q3.clone(), Occur::Should)?;

  let mut bq3 = BooleanQueryBuilder::new();
  bq3.add(
    case.random_approximation_query(q1, &mut random),
    Occur::Must,
  )?;
  bq3.add(
    case.random_approximation_query(q2, &mut random),
    Occur::Must,
  )?;

  let mut bq4 = BooleanQueryBuilder::new();
  bq4.add(bq3.build(), Occur::Should)?;
  bq4.add(q3, Occur::Should)?;

  case.assert_same_scores(&mut random, &bq2.build().into(), &bq4.build().into())
}

#[test]
fn test_constant_score() -> Result<()> {
  let mut random = random();
  let case = TestApproximationSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let q1 = TermQuery::new(t1);
  let q2 = TermQuery::new(t2);

  let mut bq1 = BooleanQueryBuilder::new();
  bq1.add(ConstantScoreQuery::new(q1.clone()), Occur::Must)?;
  bq1.add(ConstantScoreQuery::new(q2.clone()), Occur::Must)?;

  let mut bq2 = BooleanQueryBuilder::new();
  bq2.add(
    ConstantScoreQuery::new(case.random_approximation_query(q1, &mut random)),
    Occur::Must,
  )?;
  bq2.add(
    ConstantScoreQuery::new(case.random_approximation_query(q2, &mut random)),
    Occur::Must,
  )?;

  case.assert_same_scores(&mut random, &bq1.build().into(), &bq2.build().into())
}

#[test]
fn test_exclusion() -> Result<()> {
  let mut random = random();
  let case = TestApproximationSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term(&mut random);
  let q1 = TermQuery::new(t1);
  let q2 = TermQuery::new(t2);

  let mut bq1 = BooleanQueryBuilder::new();
  bq1.add(q1.clone(), Occur::Must)?;
  bq1.add(q2.clone(), Occur::MustNot)?;

  let mut bq2 = BooleanQueryBuilder::new();
  bq2.add(
    case.random_approximation_query(q1, &mut random),
    Occur::Must,
  )?;
  bq2.add(
    case.random_approximation_query(q2, &mut random),
    Occur::MustNot,
  )?;

  case.assert_same_scores(&mut random, &bq1.build().into(), &bq2.build().into())
}

#[test]
fn test_nested_exclusion() -> Result<()> {
  let mut random = random();
  let case = TestApproximationSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term_other_than(&mut random, &t1);
  let t3 = case.random_term(&mut random);
  let q1 = TermQuery::new(t1);
  let q2 = TermQuery::new(t2);
  let q3 = TermQuery::new(t3);

  let mut bq1 = BooleanQueryBuilder::new();
  bq1.add(q1.clone(), Occur::Must)?;
  bq1.add(q2.clone(), Occur::MustNot)?;

  let mut bq2 = BooleanQueryBuilder::new();
  bq2.add(bq1.build(), Occur::Must)?;
  bq2.add(q3.clone(), Occur::Must)?;
  let expected: Query = bq2.build().into();

  let mut bq3 = BooleanQueryBuilder::new();
  bq3.add(
    case.random_approximation_query(q1.clone(), &mut random),
    Occur::Must,
  )?;
  bq3.add(
    case.random_approximation_query(q2.clone(), &mut random),
    Occur::MustNot,
  )?;
  let mut bq4 = BooleanQueryBuilder::new();
  bq4.add(bq3.build(), Occur::Must)?;
  bq4.add(q3.clone(), Occur::Must)?;
  case.assert_same_scores(&mut random, &expected, &bq4.build().into())?;

  let mut bq3 = BooleanQueryBuilder::new();
  bq3.add(
    case.random_approximation_query(q1.clone(), &mut random),
    Occur::Must,
  )?;
  bq3.add(q2.clone(), Occur::MustNot)?;
  let mut bq4 = BooleanQueryBuilder::new();
  bq4.add(bq3.build(), Occur::Must)?;
  bq4.add(q3.clone(), Occur::Must)?;
  case.assert_same_scores(&mut random, &expected, &bq4.build().into())?;

  let mut bq3 = BooleanQueryBuilder::new();
  bq3.add(q1, Occur::Must)?;
  bq3.add(
    case.random_approximation_query(q2, &mut random),
    Occur::MustNot,
  )?;
  let mut bq4 = BooleanQueryBuilder::new();
  bq4.add(bq3.build(), Occur::Must)?;
  bq4.add(q3, Occur::Must)?;
  case.assert_same_scores(&mut random, &expected, &bq4.build().into())
}

#[test]
fn test_req_opt() -> Result<()> {
  let mut random = random();
  let case = TestApproximationSearchEquivalence::new(&mut random);
  let t1 = case.random_term(&mut random);
  let t2 = case.random_term_other_than(&mut random, &t1);
  let t3 = case.random_term(&mut random);
  let q1 = TermQuery::new(t1);
  let q2 = TermQuery::new(t2);
  let q3 = TermQuery::new(t3);

  let mut bq1 = BooleanQueryBuilder::new();
  bq1.add(q1.clone(), Occur::Must)?;
  bq1.add(q2.clone(), Occur::Should)?;

  let mut bq2 = BooleanQueryBuilder::new();
  bq2.add(bq1.build(), Occur::Must)?;
  bq2.add(q3.clone(), Occur::Must)?;

  let mut bq3 = BooleanQueryBuilder::new();
  bq3.add(
    case.random_approximation_query(q1, &mut random),
    Occur::Must,
  )?;
  bq3.add(
    case.random_approximation_query(q2, &mut random),
    Occur::Should,
  )?;

  let mut bq4 = BooleanQueryBuilder::new();
  bq4.add(bq3.build(), Occur::Must)?;
  bq4.add(q3, Occur::Must)?;

  case.assert_same_scores(&mut random, &bq2.build().into(), &bq4.build().into())
}
