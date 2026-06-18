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
use crate::test::core::search::base_explanation_test_case::{
  ALTFIELD, BaseExplanationTestCase, BaseExplanationTestContext, FIELD,
  before_class_test_explanations,
};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use rand::Rng;

/// TestExplanations implementation focusing on basic query types
#[allow(dead_code)] // for quick search
struct TestSimpleExplanations {
  context: BaseExplanationTestContext,
}

impl TestSimpleExplanations {
  fn new<R>(random: &mut R) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    Ok(Self {
      context: before_class_test_explanations(random)?,
    })
  }
}

impl BaseExplanationTestCase for TestSimpleExplanations {}

// we focus on queries that don't rewrite to other queries.
// if we get those covered well, then the ones that rewrite should
// also be covered.

/* simple term tests */

#[test]
fn test_t1() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    TermQuery::new(Term::from_text(FIELD, "w1")),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_t2() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let term_query = TermQuery::new(Term::from_text(FIELD, "w1"));
  test.q_test(
    &mut random,
    &test.context.searcher,
    BoostQuery::new(term_query, 100.0)?,
    &[0, 1, 2, 3],
  )
}

/* MatchAllDocs */

#[test]
fn test_ma1() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    MatchAllDocsQuery::new(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_ma2() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let q = MatchAllDocsQuery::new();
  test.q_test(
    &mut random,
    &test.context.searcher,
    BoostQuery::new(q, 1000.0)?,
    &[0, 1, 2, 3],
  )
}

/* some simple phrase tests */

#[test]
fn test_p1() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let phrase_query = PhraseQuery::from_terms_no_slop(FIELD, &["w1", "w2"])?;
  test.q_test(&mut random, &test.context.searcher, phrase_query, &[0])
}

#[test]
fn test_p2() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let phrase_query = PhraseQuery::from_terms_no_slop(FIELD, &["w1", "w3"])?;
  test.q_test(&mut random, &test.context.searcher, phrase_query, &[1, 3])
}

#[test]
fn test_p3() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let phrase_query = PhraseQuery::from_terms(1, FIELD, &["w1", "w2"])?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    phrase_query,
    &[0, 1, 2],
  )
}

#[test]
fn test_p4() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let phrase_query = PhraseQuery::from_terms(1, FIELD, &["w2", "w3"])?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    phrase_query,
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_p5() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let phrase_query = PhraseQuery::from_terms(1, FIELD, &["w3", "w2"])?;
  test.q_test(&mut random, &test.context.searcher, phrase_query, &[1, 3])
}

#[test]
fn test_p6() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let phrase_query = PhraseQuery::from_terms(2, FIELD, &["w3", "w2"])?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    phrase_query,
    &[0, 1, 3],
  )
}

#[test]
fn test_p7() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let phrase_query = PhraseQuery::from_terms(3, FIELD, &["w3", "w2"])?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    phrase_query,
    &[0, 1, 2, 3],
  )
}

/* ConstantScoreQueries */

#[test]
fn test_csq1() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let q = ConstantScoreQuery::new(test.match_these_items(&[0, 1, 2, 3])?);
  test.q_test(&mut random, &test.context.searcher, q, &[0, 1, 2, 3])
}

#[test]
fn test_csq2() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let q = ConstantScoreQuery::new(test.match_these_items(&[1, 3])?);
  test.q_test(&mut random, &test.context.searcher, q, &[1, 3])
}

#[test]
fn test_csq3() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let q = ConstantScoreQuery::new(test.match_these_items(&[0, 2])?);
  test.q_test(
    &mut random,
    &test.context.searcher,
    BoostQuery::new(q, 1000.0)?,
    &[0, 2],
  )
}

/* DisjunctionMaxQuery */

#[test]
fn test_dmq1() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let q = DisjunctionMaxQuery::new(
    vec![
      TermQuery::new(Term::from_text(FIELD, "w1")).into(),
      TermQuery::new(Term::from_text(FIELD, "w5")).into(),
    ],
    0.0,
  )?;
  test.q_test(&mut random, &test.context.searcher, q, &[0, 1, 2, 3])
}

#[test]
fn test_dmq2() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let q = DisjunctionMaxQuery::new(
    vec![
      TermQuery::new(Term::from_text(FIELD, "w1")).into(),
      TermQuery::new(Term::from_text(FIELD, "w5")).into(),
    ],
    0.5,
  )?;
  test.q_test(&mut random, &test.context.searcher, q, &[0, 1, 2, 3])
}

#[test]
fn test_dmq3() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let q = DisjunctionMaxQuery::new(
    vec![
      TermQuery::new(Term::from_text(FIELD, "QQ")).into(),
      TermQuery::new(Term::from_text(FIELD, "w5")).into(),
    ],
    0.5,
  )?;
  test.q_test(&mut random, &test.context.searcher, q, &[0])
}

#[test]
fn test_dmq4() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let q = DisjunctionMaxQuery::new(
    vec![
      TermQuery::new(Term::from_text(FIELD, "QQ")).into(),
      TermQuery::new(Term::from_text(FIELD, "xx")).into(),
    ],
    0.5,
  )?;
  test.q_test(&mut random, &test.context.searcher, q, &[2, 3])
}

#[test]
fn test_dmq5() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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
  test.q_test(&mut random, &test.context.searcher, q, &[2, 3])
}

#[test]
fn test_dmq6() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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
  test.q_test(&mut random, &test.context.searcher, q, &[0, 1, 2, 3])
}

#[test]
fn test_dmq7() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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
  test.q_test(&mut random, &test.context.searcher, q, &[0, 1, 2, 3])
}

#[test]
fn test_dmq8() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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
  test.q_test(&mut random, &test.context.searcher, q, &[0, 2, 3])
}

#[test]
fn test_dmq9() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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

  test.q_test(&mut random, &test.context.searcher, q, &[0, 2, 3])
}

/* MultiPhraseQuery */

#[test]
fn test_mpq1() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut qb = MultiPhraseQuery::builder();
  qb.add_terms(&test.ta(&["w1"]))?;
  qb.add_terms(&test.ta(&["w2", "w3", "xx"]))?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    qb.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_mpq2() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut qb = MultiPhraseQuery::builder();
  qb.add_terms(&test.ta(&["w1"]))?;
  qb.add_terms(&test.ta(&["w2", "w3"]))?;
  test.q_test(&mut random, &test.context.searcher, qb.build(), &[0, 1, 3])
}

#[test]
fn test_mpq3() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut qb = MultiPhraseQuery::builder();
  qb.add_terms(&test.ta(&["w1", "xx"]))?;
  qb.add_terms(&test.ta(&["w2", "w3"]))?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    qb.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_mpq4() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut qb = MultiPhraseQuery::builder();
  qb.add_terms(&test.ta(&["w1"]))?;
  qb.add_terms(&test.ta(&["w2"]))?;
  test.q_test(&mut random, &test.context.searcher, qb.build(), &[0])
}

#[test]
fn test_mpq5() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut qb = MultiPhraseQuery::builder();
  qb.add_terms(&test.ta(&["w1"]))?;
  qb.add_terms(&test.ta(&["w2"]))?;
  qb.set_slop(1)?;
  test.q_test(&mut random, &test.context.searcher, qb.build(), &[0, 1, 2])
}

#[test]
fn test_mpq6() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut qb = MultiPhraseQuery::builder();
  qb.add_terms(&test.ta(&["w1", "w3"]))?;
  qb.add_terms(&test.ta(&["w2"]))?;
  qb.set_slop(1)?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    qb.build(),
    &[0, 1, 2, 3],
  )
}

/* some simple tests of boolean queries containing term queries */

#[test]
fn test_bq1() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();
  query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Must)?;
  query.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Must)?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_bq2() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();
  query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Must)?;
  query.add(TermQuery::new(Term::from_text(FIELD, "w3")), Occur::Must)?;
  test.q_test(&mut random, &test.context.searcher, query.build(), &[2, 3])
}

#[test]
fn test_bq3() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();
  query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Should)?;
  query.add(TermQuery::new(Term::from_text(FIELD, "w3")), Occur::Must)?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_bq4() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut outer_query = BooleanQueryBuilder::new();
  outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;

  let mut inner_query = BooleanQueryBuilder::new();
  inner_query.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::MustNot)?;
  inner_query.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Should)?;
  outer_query.add(inner_query.build(), Occur::Should)?;

  test.q_test(
    &mut random,
    &test.context.searcher,
    outer_query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_bq5() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut outer_query = BooleanQueryBuilder::new();
  outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;

  let mut inner_query = BooleanQueryBuilder::new();
  inner_query.add(TermQuery::new(Term::from_text(FIELD, "qq")), Occur::Must)?;
  inner_query.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Should)?;
  outer_query.add(inner_query.build(), Occur::Should)?;

  test.q_test(
    &mut random,
    &test.context.searcher,
    outer_query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_bq6() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut outer_query = BooleanQueryBuilder::new();
  outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;

  let mut inner_query = BooleanQueryBuilder::new();
  inner_query.add(TermQuery::new(Term::from_text(FIELD, "qq")), Occur::MustNot)?;
  inner_query.add(TermQuery::new(Term::from_text(FIELD, "w5")), Occur::Should)?;
  outer_query.add(inner_query.build(), Occur::MustNot)?;

  test.q_test(
    &mut random,
    &test.context.searcher,
    outer_query.build(),
    &[1, 2, 3],
  )
}

#[test]
fn test_bq7() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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

  test.q_test(
    &mut random,
    &test.context.searcher,
    outer_query.build(),
    &[0],
  )
}

#[test]
fn test_bq8() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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

  test.q_test(
    &mut random,
    &test.context.searcher,
    outer_query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_bq9() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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

  test.q_test(
    &mut random,
    &test.context.searcher,
    outer_query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_bq10() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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

  test.q_test(
    &mut random,
    &test.context.searcher,
    outer_query.build(),
    &[1],
  )
}

#[test]
fn test_bq11() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();
  query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;
  let boosted_query = TermQuery::new(Term::from_text(FIELD, "w1"));
  query.add(BoostQuery::new(boosted_query, 1000.0)?, Occur::Should)?;

  test.q_test(
    &mut random,
    &test.context.searcher,
    query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_bq14() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut q = BooleanQueryBuilder::new();
  q.add(
    TermQuery::new(Term::from_text(FIELD, "QQQQQ")),
    Occur::Should,
  )?;
  q.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    q.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_bq15() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut q = BooleanQueryBuilder::new();
  q.add(
    TermQuery::new(Term::from_text(FIELD, "QQQQQ")),
    Occur::MustNot,
  )?;
  q.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    q.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_bq16() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut q = BooleanQueryBuilder::new();
  q.add(
    TermQuery::new(Term::from_text(FIELD, "QQQQQ")),
    Occur::Should,
  )?;

  let mut boolean_query = BooleanQueryBuilder::new();
  boolean_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;
  boolean_query.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::MustNot)?;

  q.add(boolean_query.build(), Occur::Should)?;
  test.q_test(&mut random, &test.context.searcher, q.build(), &[0, 1])
}

#[test]
fn test_bq17() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut q = BooleanQueryBuilder::new();
  q.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Should)?;

  let mut boolean_query = BooleanQueryBuilder::new();
  boolean_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;
  boolean_query.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::MustNot)?;

  q.add(boolean_query.build(), Occur::Should)?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    q.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_bq19() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();
  query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::MustNot)?;
  query.add(TermQuery::new(Term::from_text(FIELD, "w3")), Occur::Should)?;

  test.q_test(&mut random, &test.context.searcher, query.build(), &[0, 1])
}

#[test]
fn test_bq20() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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

  test.q_test(&mut random, &test.context.searcher, q.build(), &[0, 3])
}

#[test]
fn test_bq21() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut q = BooleanQueryBuilder::new();
  q.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Should)?;
  q.add(TermQuery::new(Term::from_text(FIELD, "zz")), Occur::Should)?;

  test.q_test(&mut random, &test.context.searcher, q.build(), &[1, 2, 3])
}

#[test]
fn test_bq23() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();
  query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Filter)?;
  query.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Filter)?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_bq24() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();
  query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Filter)?;
  query.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Should)?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_bq25() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();
  query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Filter)?;
  query.add(TermQuery::new(Term::from_text(FIELD, "w2")), Occur::Must)?;
  test.q_test(
    &mut random,
    &test.context.searcher,
    query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_bq26() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();
  query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Filter)?;
  query.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::MustNot)?;
  test.q_test(&mut random, &test.context.searcher, query.build(), &[0, 1])
}

/* BQ of TQ: using alt so some fields have zero boost and some don't */

#[test]
fn test_multi_field_bq1() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();
  query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Must)?;
  query.add(TermQuery::new(Term::from_text(ALTFIELD, "w2")), Occur::Must)?;

  test.q_test(
    &mut random,
    &test.context.searcher,
    query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_multi_field_bq2() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();
  query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Must)?;
  query.add(TermQuery::new(Term::from_text(ALTFIELD, "w3")), Occur::Must)?;

  test.q_test(&mut random, &test.context.searcher, query.build(), &[2, 3])
}

#[test]
fn test_multi_field_bq3() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();
  query.add(TermQuery::new(Term::from_text(FIELD, "yy")), Occur::Should)?;
  query.add(TermQuery::new(Term::from_text(ALTFIELD, "w3")), Occur::Must)?;

  test.q_test(
    &mut random,
    &test.context.searcher,
    query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_multi_field_bq4() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut outer_query = BooleanQueryBuilder::new();
  outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;

  let mut inner_query = BooleanQueryBuilder::new();
  inner_query.add(TermQuery::new(Term::from_text(FIELD, "xx")), Occur::MustNot)?;
  inner_query.add(
    TermQuery::new(Term::from_text(ALTFIELD, "w2")),
    Occur::Should,
  )?;
  outer_query.add(inner_query.build(), Occur::Should)?;

  test.q_test(
    &mut random,
    &test.context.searcher,
    outer_query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_multi_field_bq5() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut outer_query = BooleanQueryBuilder::new();
  outer_query.add(TermQuery::new(Term::from_text(FIELD, "w1")), Occur::Should)?;

  let mut inner_query = BooleanQueryBuilder::new();
  inner_query.add(TermQuery::new(Term::from_text(ALTFIELD, "qq")), Occur::Must)?;
  inner_query.add(
    TermQuery::new(Term::from_text(ALTFIELD, "w2")),
    Occur::Should,
  )?;
  outer_query.add(inner_query.build(), Occur::Should)?;

  test.q_test(
    &mut random,
    &test.context.searcher,
    outer_query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_multi_field_bq6() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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

  test.q_test(
    &mut random,
    &test.context.searcher,
    outer_query.build(),
    &[1, 2, 3],
  )
}

#[test]
fn test_multi_field_bq7() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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

  test.q_test(
    &mut random,
    &test.context.searcher,
    outer_query.build(),
    &[0],
  )
}

#[test]
fn test_multi_field_bq8() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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

  test.q_test(
    &mut random,
    &test.context.searcher,
    outer_query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_multi_field_bq9() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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

  test.q_test(
    &mut random,
    &test.context.searcher,
    outer_query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_multi_field_bq10() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
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

  test.q_test(
    &mut random,
    &test.context.searcher,
    outer_query.build(),
    &[1],
  )
}

/* BQ of PQ: using alt so some fields have zero boost and some don't */

#[test]
fn test_multi_field_bqof_pq1() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();

  let left_child = PhraseQuery::from_terms_no_slop(FIELD, &["w1", "w2"])?;
  query.add(left_child, Occur::Should)?;

  let right_child = PhraseQuery::from_terms_no_slop(ALTFIELD, &["w1", "w2"])?;
  query.add(right_child, Occur::Should)?;

  test.q_test(&mut random, &test.context.searcher, query.build(), &[0])
}

#[test]
fn test_multi_field_bqof_pq2() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();

  let left_child = PhraseQuery::from_terms_no_slop(FIELD, &["w1", "w3"])?;
  query.add(left_child, Occur::Should)?;

  let right_child = PhraseQuery::from_terms_no_slop(ALTFIELD, &["w1", "w3"])?;
  query.add(right_child, Occur::Should)?;

  test.q_test(&mut random, &test.context.searcher, query.build(), &[1, 3])
}

#[test]
fn test_multi_field_bqof_pq3() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();

  let left_child = PhraseQuery::from_terms(1, FIELD, &["w1", "w2"])?;
  query.add(left_child, Occur::Should)?;

  let right_child = PhraseQuery::from_terms(1, ALTFIELD, &["w1", "w2"])?;
  query.add(right_child, Occur::Should)?;

  test.q_test(
    &mut random,
    &test.context.searcher,
    query.build(),
    &[0, 1, 2],
  )
}

#[test]
fn test_multi_field_bqof_pq4() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();

  let left_child = PhraseQuery::from_terms(1, FIELD, &["w2", "w3"])?;
  query.add(left_child, Occur::Should)?;

  let right_child = PhraseQuery::from_terms(1, ALTFIELD, &["w2", "w3"])?;
  query.add(right_child, Occur::Should)?;

  test.q_test(
    &mut random,
    &test.context.searcher,
    query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_multi_field_bqof_pq5() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();

  let left_child = PhraseQuery::from_terms(1, FIELD, &["w3", "w2"])?;
  query.add(left_child, Occur::Should)?;

  let right_child = PhraseQuery::from_terms(1, ALTFIELD, &["w3", "w2"])?;
  query.add(right_child, Occur::Should)?;

  test.q_test(&mut random, &test.context.searcher, query.build(), &[1, 3])
}

#[test]
fn test_multi_field_bqof_pq6() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();

  let left_child = PhraseQuery::from_terms(2, FIELD, &["w3", "w2"])?;
  query.add(left_child, Occur::Should)?;

  let right_child = PhraseQuery::from_terms(2, ALTFIELD, &["w3", "w2"])?;
  query.add(right_child, Occur::Should)?;

  test.q_test(
    &mut random,
    &test.context.searcher,
    query.build(),
    &[0, 1, 3],
  )
}

#[test]
fn test_multi_field_bqof_pq7() -> Result<()> {
  let mut random = random();
  let test = TestSimpleExplanations::new(&mut random)?;
  let mut query = BooleanQueryBuilder::new();

  let left_child = PhraseQuery::from_terms(3, FIELD, &["w3", "w2"])?;
  query.add(left_child, Occur::Should)?;

  let right_child = PhraseQuery::from_terms(1, ALTFIELD, &["w3", "w2"])?;
  query.add(right_child, Occur::Should)?;

  test.q_test(
    &mut random,
    &test.context.searcher,
    query.build(),
    &[0, 1, 2, 3],
  )
}

#[test]
fn test_synonym_query() -> Result<()> {
  // TODO: SynonymQuery 未实现
  Ok(())
}

#[test]
fn test_equality() {
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
}
