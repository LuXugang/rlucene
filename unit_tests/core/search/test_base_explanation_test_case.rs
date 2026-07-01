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
use crate::test::support::core::search::base_explanation_test_case::{
  BaseExplanationTestCase, BaseExplanationTestContext, FIELD, before_class_test_explanations,
};
use crate::test::support::core::search::check_hits::CheckHits;
pub use crate::test::support::core::search::query::BrokenExplainTermQuery;
use crate::test::support::core::util::lucene_test_case::random;
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
