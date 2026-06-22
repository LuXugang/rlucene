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
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::term::Term;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{IntoBoxQuery, Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::term_query::TermQuery;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::search::base_explanation_test_case::{
  BaseExplanationTestCase, BaseExplanationTestContext, FIELD, before_class_test_explanations,
};
use crate::test::core::search::check_hits::CheckHits;
use crate::test::core::util::lucene_test_case::random;
use rand::Rng;
use std::hash::{Hash, Hasher};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

/// Tests that the [`BaseExplanationTestCase`] helper code, as well as
/// [`CheckHits::check_no_match_explanations`] are checking what they are suppose to.
#[allow(dead_code)] // for quick search
struct TestBaseExplanationTestCase {
  context: BaseExplanationTestContext,
}

impl TestBaseExplanationTestCase {
  fn new<R>(random: &mut R) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    Ok(Self {
      context: before_class_test_explanations(random)?,
    })
  }
}

impl BaseExplanationTestCase for TestBaseExplanationTestCase {}

#[test]
fn test_query_no_match_when_expected() -> Result<()> {
  let mut random = random();
  let test = TestBaseExplanationTestCase::new(&mut random)?;
  let result = catch_unwind(AssertUnwindSafe(|| {
    test
      .q_test(
        &mut random,
        &test.context.searcher,
        TermQuery::new(Term::from_text(FIELD, "BOGUS")),
        &[3 /* none */],
      )
      .unwrap();
  }));
  assert!(result.is_err());
  Ok(())
}

#[test]
fn test_query_match_when_not_expected() -> Result<()> {
  let mut random = random();
  let test = TestBaseExplanationTestCase::new(&mut random)?;
  let result = catch_unwind(AssertUnwindSafe(|| {
    test
      .q_test(
        &mut random,
        &test.context.searcher,
        TermQuery::new(Term::from_text(FIELD, "w1")),
        &[0, 1 /*, 2, 3 */],
      )
      .unwrap();
  }));
  assert!(result.is_err());
  Ok(())
}
// TODO IMPORTANT Matches 未实现
fn test_incorrect_explain_scores() -> Result<()> {
  let mut random = random();
  let test = TestBaseExplanationTestCase::new(&mut random)?;
  // sanity check what a real TermQuery matches
  test.q_test(
    &mut random,
    &test.context.searcher,
    TermQuery::new(Term::from_text(FIELD, "zz")),
    &[1, 3],
  )?;

  // ensure when the Explanations are broken, we get an error about those matches
  let result = catch_unwind(AssertUnwindSafe(|| {
    test
      .q_test(
        &mut random,
        &test.context.searcher,
        BrokenExplainTermQuery::new(Term::from_text(FIELD, "zz"), false, true),
        &[1, 3],
      )
      .unwrap();
  }));
  assert!(result.is_err());
  Ok(())
}

#[test]
fn test_incorrect_explain_matches() -> Result<()> {
  let mut random = random();
  let test = TestBaseExplanationTestCase::new(&mut random)?;
  // sanity check what a real TermQuery matches
  test.q_test(
    &mut random,
    &test.context.searcher,
    TermQuery::new(Term::from_text(FIELD, "zz")),
    &[1, 3],
  )?;

  // ensure when the Explanations are broken, we get an error about the non matches
  let result = catch_unwind(AssertUnwindSafe(|| {
    CheckHits::check_no_match_explanations(
      BrokenExplainTermQuery::new(Term::from_text(FIELD, "zz"), true, false).into(),
      FIELD,
      &test.context.searcher,
      &[1, 3],
    )
    .unwrap();
  }));
  assert!(result.is_err());
  Ok(())
}

#[derive(Clone)]
pub struct BrokenExplainTermQuery {
  id: Identity,
  term_query: TermQuery,
  pub(crate) toggle_explain_match: bool,
  pub(crate) break_explain_scores: bool,
}

impl BrokenExplainTermQuery {
  pub(crate) fn new<T>(term: T, toggle_explain_match: bool, break_explain_scores: bool) -> Self
  where
    T: Into<Arc<Term>>,
  {
    Self {
      id: Identity::new(),
      term_query: TermQuery::new(term),
      toggle_explain_match,
      break_explain_scores,
    }
  }
}

impl std::fmt::Debug for BrokenExplainTermQuery {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self.to_string("") {
      Ok(s) => write!(f, "{}", s),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl PartialEq for BrokenExplainTermQuery {
  fn eq(&self, other: &Self) -> bool {
    self.term_query == other.term_query
      && self.toggle_explain_match == other.toggle_explain_match
      && self.break_explain_scores == other.break_explain_scores
  }
}

impl Eq for BrokenExplainTermQuery {}

impl Hash for BrokenExplainTermQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.term_query.hash(state);
    self.toggle_explain_match.hash(state);
    self.break_explain_scores.hash(state);
  }
}

impl HasIdentity for BrokenExplainTermQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for BrokenExplainTermQuery {
  fn to_string(&self, field: &str) -> Result<String> {
    self.term_query.to_string(field)
  }

  fn create_weight<IRC>(
    self,
    searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    let inner_weight = self
      .term_query
      .clone()
      .create_weight(searcher, score_mode, boost)?;
    Ok(Box::new(BrokenExplainWeight::new(self, inner_weight)))
  }

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Ok(self.into())
  }

  fn visit<QV>(&self, visitor: &QV)
  where
    QV: QueryVisitor,
  {
    self.term_query.visit(visitor);
  }
}

impl IntoBoxQuery for BrokenExplainTermQuery {
  fn into_box_query(self) -> Box<Query> {
    Box::new(self.into())
  }
}

pub(crate) struct BrokenExplainWeight<IRC>
where
  IRC: IndexReaderContext,
{
  query: Arc<Query>,
  in_: QueryWeight<IRC>,
}

impl<IRC> BrokenExplainWeight<IRC>
where
  IRC: IndexReaderContext,
{
  fn new(query: BrokenExplainTermQuery, inner_weight: QueryWeight<IRC>) -> Self {
    Self {
      query: Arc::new(query.into()),
      in_: inner_weight,
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for BrokenExplainWeight<IRC>
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    self.in_.is_cacheable(ctx)
  }
}

impl<IRC> Weight<IRC> for BrokenExplainWeight<IRC>
where
  IRC: IndexReaderContext,
{
  type Matches = MatchWithNoTerms;
  type ScorerSupplier = QueryWeightSs<IRC>;

  fn matches(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::Matches>> {
    self.in_.matches(context, doc, searcher)
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    let query = match self.query.as_ref() {
      Query::BrokenExplainTerm(query) => query,
      _ => {
        return Err(LuceneError::illegal_state(
          "expected BrokenExplainTermQuery",
        ));
      },
    };
    let mut result = self.in_.explain(context, doc, searcher)?;
    if result.is_match() {
      if query.break_explain_scores {
        let value = result.get_value().to_f64().ok_or_else(|| {
          LuceneError::illegal_state(format!(
            "Explanation value is not a number: {:?}",
            result.get_value()
          ))
        })?;
        result = Explanation::match_(-value, "Broken Explanation Score", vec![result]);
      }
      if query.toggle_explain_match {
        result = Explanation::no_match("Broken Explanation Matching", vec![result]);
      }
    } else if query.toggle_explain_match {
      result = Explanation::match_(-42.0f32, "Broken Explanation Matching", vec![result]);
    }
    Ok(result)
  }

  fn get_query(&self) -> Arc<Query> {
    self.query.clone()
  }

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    match self.in_.scorer(context, searcher)? {
      Some(scorer) => Ok(Some(Box::new(DefaultScorerSupplier::new(scorer)))),
      None => Ok(None),
    }
  }
}
