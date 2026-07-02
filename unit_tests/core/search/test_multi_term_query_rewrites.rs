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
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::index::BytesRef;
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::directory_reader;
use crate::core::index::filtered_terms_enum::{
  AcceptStatus, FilteredTermsEnum, FilteredTermsEnumBase,
};
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::multi_reader::MultiReader;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::search::boolean_query::BooleanQuery;
use crate::core::search::index_searcher;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::index_searcher::{DefaultIndexSearcher, set_max_clause_count};
use crate::core::search::multi_term_query::{
  CONSTANT_SCORE_BOOLEAN_REWRITE, ConstantScoreBlendedRewrite, ConstantScoreRewrite,
  MultiTermQuery, MultiTermQuerySet, RewriteMethod, RewriteMethodEnum, SCORING_BOOLEAN_REWRITE,
  TopTermsBoostOnlyBooleanQueryRewrite, TopTermsScoringBooleanQueryRewrite,
};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::term_range_query::TermRangeQuery;
use crate::core::store::directory::DirEnum;
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
pub use crate::test_framework::core::search::multi_term::BoostCheckingQuery;
use crate::test_framework::core::util::DefaultIndexSearchCR;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_reader, new_string_field, random,
};
use rand::Rng;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub struct TestMultiTermQueryRewrites;

type MultiTermRewriteSearcher = DefaultIndexSearcher<
  CompositeReaderContext<
    MultiReader<DefaultLeafReader<DirEnum>, StandardDirectoryReaderType<DirEnum>>,
  >,
>;

fn set_up<R: Rng + ?Sized>(
  random: &mut R,
) -> Result<(
  DefaultIndexSearchCR,
  MultiTermRewriteSearcher,
  MultiTermRewriteSearcher,
)> {
  let dir = new_directory_shared(random)?;
  let sdir1 = new_directory_shared(random)?;
  let sdir2 = new_directory_shared(random)?;

  let mock = MockAnalyzer::new(random);
  let writer = RandomIndexWriter::with_analyzer(random, dir.clone(), mock)?;

  let mock = MockAnalyzer::new(random);
  let swriter1 = RandomIndexWriter::with_analyzer(random, sdir1.clone(), mock)?;

  let mock = MockAnalyzer::new(random);
  let swriter2 = RandomIndexWriter::with_analyzer(random, sdir2.clone(), mock)?;

  let mut field_to_type = HashMap::new();

  for i in 0..10 {
    let mut doc = Document::new();
    doc.add(new_string_field(
      random,
      "data",
      i.to_string(),
      Store::No,
      &mut field_to_type,
    )?);

    writer.add_document(random, doc.clone())?;

    if i % 2 == 0 {
      swriter1.add_document(random, doc)?;
    } else {
      swriter2.add_document(random, doc)?;
    }
  }

  writer.force_merge(random, 1)?;
  swriter1.force_merge(random, 1)?;
  swriter2.force_merge(random, 1)?;

  writer.close(random)?;
  swriter1.close(random)?;
  swriter2.close(random)?;

  let reader = directory_reader::open(dir.clone())?;
  let searcher = new_searcher_with_reader(reader)?;
  // TODO IMPORTANT 这里没有调用close方法，有必要吗

  let multi_reader = MultiReader::with_composite_reader(vec![
    directory_reader::open(sdir1.clone())?,
    directory_reader::open(sdir2.clone())?,
  ])?;
  let multi_searcher = new_searcher_with_reader(multi_reader)?;

  let multi_reader_dupls = MultiReader::with_composite_reader(vec![
    directory_reader::open(sdir1)?,
    directory_reader::open(dir)?,
  ])?;
  let multi_searcher_dupls = new_searcher_with_reader(multi_reader_dupls)?;

  Ok((searcher, multi_searcher, multi_searcher_dupls))
}
fn extract_inner_query(q: Query) -> Query {
  match q {
    Query::ConstantScore(q) => q.into_inner(),
    _ => q,
  }
}

fn extract_term(q: Query) -> Arc<Term> {
  let q = extract_inner_query(q);
  match q {
    Query::Term(q) => q.get_term(),
    _ => unreachable!("expected TermQuery"),
  }
}

fn check_boolean_query_order(q: Query) {
  let q = extract_inner_query(q);
  let bq = match q {
    Query::Boolean(q) => q,
    _ => unreachable!("expected BooleanQuery"),
  };
  let mut last: Option<Arc<Term>> = None;
  for clause in bq.clauses().iter() {
    let act = extract_term(clause.query.clone());

    if let Some(last) = last {
      assert!(last < act, "sort order of terms in BQ violated");
    }

    last = Some(act.clone());
  }
}
fn check_duplicate_terms<R, T>(random: &mut R, method: T) -> Result<()>
where
  R: Rng + ?Sized,
  T: Into<RewriteMethodEnum>,
{
  let mtq = TermRangeQuery::new_string_range_with_rewrite(
    "data",
    Some("2"),
    Some("7"),
    true,
    true,
    method,
  )?;
  let (searcher, multi_searcher, multi_searcher_dupls) = set_up(random)?;

  let q1 = searcher.rewrite(mtq.clone())?;
  let q2 = multi_searcher.rewrite(mtq.clone())?;
  let q3 = multi_searcher_dupls.rewrite(mtq)?;

  assert_eq!(
    q1, q2,
    "The multi-segment case must produce same rewritten query"
  );
  assert_eq!(
    q1, q3,
    "The multi-segment case with duplicates must produce same rewritten query"
  );

  check_boolean_query_order(q1);
  check_boolean_query_order(q2);
  check_boolean_query_order(q3);

  Ok(())
}
#[test]
fn test_rewrites_with_duplicate_terms() -> Result<()> {
  let mut random = random();

  check_duplicate_terms(&mut random, SCORING_BOOLEAN_REWRITE)?;

  check_duplicate_terms(&mut random, CONSTANT_SCORE_BOOLEAN_REWRITE)?;
  // use a large PQ here to only test duplicate terms and dont mix up when all scores are equal
  check_duplicate_terms(&mut random, TopTermsScoringBooleanQueryRewrite::new(1024))?;

  check_duplicate_terms(&mut random, TopTermsBoostOnlyBooleanQueryRewrite::new(1024))?;

  Ok(())
}
fn check_boolean_query_boosts(bq: &BooleanQuery) -> Result<()> {
  for clause in bq.clauses() {
    let boost_q = match clause.query.clone() {
      Query::Boost(q) => q,
      _ => unreachable!("expected BoostQuery"),
    };

    let mtq = match boost_q.get_query() {
      Query::Term(q) => q,
      _ => unreachable!("expected TermQuery"),
    };

    assert_eq!(
      mtq.get_term().text()?.parse::<f32>().unwrap(),
      boost_q.get_boost(),
      "Parallel sorting of boosts in rewrite mode broken"
    );
  }
  Ok(())
}

fn check_boosts<R, T>(random: &mut R, method: T) -> Result<()>
where
  R: Rng + ?Sized,
  T: Into<RewriteMethodEnum>,
{
  let mtq = BoostCheckingQuery::new("data", method);
  let (searcher, multi_searcher, multi_searcher_dupls) = set_up(random)?;

  let q1 = searcher.rewrite(mtq.clone())?;
  let q2 = multi_searcher.rewrite(mtq.clone())?;
  let q3 = multi_searcher_dupls.rewrite(mtq)?;

  assert_eq!(
    q1, q2,
    "The multi-segment case must produce same rewritten query"
  );
  assert_eq!(
    q1, q3,
    "The multi-segment case with duplicates must produce same rewritten query"
  );

  if matches!(q1, Query::MatchNoDocs(_)) {
    assert!(matches!(q2, Query::MatchNoDocs(_)));
    assert!(matches!(q3, Query::MatchNoDocs(_)));
  } else {
    check_boolean_query_order(q1);
    check_boolean_query_order(q2);
    check_boolean_query_order(q3);
  }

  Ok(())
}
#[test]
fn test_boosts() -> Result<()> {
  let mut random = random();

  check_boosts(&mut random, SCORING_BOOLEAN_REWRITE)?;

  // use a large PQ here to only test boosts and dont mix up when all scores are equal
  check_boosts(&mut random, TopTermsScoringBooleanQueryRewrite::new(1024))?;

  Ok(())
}
fn check_max_clause_limitation<R, T>(random: &mut R, method: T) -> Result<()>
where
  R: Rng + ?Sized,
  T: Into<RewriteMethodEnum>,
{
  let saved_max_clause_count = index_searcher::get_max_clause_count();
  set_max_clause_count(3)?;

  let result: Result<()> = (|| {
    let mtq = TermRangeQuery::new_string_range_with_rewrite(
      "data",
      Some("2"),
      Some("7"),
      true,
      true,
      method,
    )?;
    let (_, _, multi_searcher_dupls) = set_up(random)?;

    let err = multi_searcher_dupls.rewrite(mtq);
    assert!(matches!(err, Err(LuceneError::TooManyClauses(_))));

    Ok(())
  })();

  set_max_clause_count(saved_max_clause_count)?;

  result
}

fn check_no_max_clause_limitation<T, R>(random: &mut R, method: T) -> Result<()>
where
  R: Rng + ?Sized,
  T: Into<RewriteMethodEnum>,
{
  let saved_max_clause_count = index_searcher::get_max_clause_count();
  set_max_clause_count(3)?;

  let result: Result<()> = (|| {
    let mtq = TermRangeQuery::new_string_range_with_rewrite(
      "data",
      Some("2"),
      Some("7"),
      true,
      true,
      method,
    )?;
    let (_, _, multi_searcher_dupls) = set_up(random)?;

    multi_searcher_dupls.rewrite(mtq)?;

    Ok(())
  })();

  set_max_clause_count(saved_max_clause_count)?;

  result
}
#[test]
fn test_max_clause_limitations() -> Result<()> {
  let mut random = random();

  check_max_clause_limitation(&mut random, SCORING_BOOLEAN_REWRITE)?;
  check_max_clause_limitation(&mut random, CONSTANT_SCORE_BOOLEAN_REWRITE)?;

  check_no_max_clause_limitation(&mut random, ConstantScoreRewrite)?;
  check_no_max_clause_limitation(&mut random, ConstantScoreBlendedRewrite)?;
  check_no_max_clause_limitation(&mut random, TopTermsScoringBooleanQueryRewrite::new(1024))?;
  check_no_max_clause_limitation(&mut random, TopTermsBoostOnlyBooleanQueryRewrite::new(1024))?;

  Ok(())
}
