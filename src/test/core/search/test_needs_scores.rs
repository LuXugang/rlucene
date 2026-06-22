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
use crate::core::document::text_field::TextField;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{IntoBoxQuery, Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::{SortField, SortFieldType};
use crate::core::search::sort_field_enum::SortFieldEnum;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::search::weight::Weight;
use crate::core::store::directory::DirEnum;
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::DefaultIndexSearchCR;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_reader, random,
};
use std::hash::{Hash, Hasher};
use std::mem;
use std::sync::Arc;

pub struct TestNeedsScores {
  #[allow(dead_code)]
  dir: Arc<DirEnum>,
  searcher: DefaultIndexSearchCR,
}

impl TestNeedsScores {
  fn set_up() -> Result<Self> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let analyzer = MockAnalyzer::new(&mut random);
    let iw = RandomIndexWriter::with_analyzer(&mut random, dir.clone(), analyzer);
    for i in 0..5 {
      let mut doc = Document::new();
      doc.add(TextField::from_string(
        "field",
        format!("this is document {i}"),
        Store::No,
      )?);
      iw.add_document(&mut random, doc)?;
    }
    let reader = iw.get_reader(&mut random)?;
    let mut searcher = new_searcher_with_reader(reader)?;
    searcher.set_query_cache(None);
    iw.close(&mut random)?;
    Ok(Self { dir, searcher })
  }
}

#[test]
fn test_prohibited_clause() -> Result<()> {
  let case = TestNeedsScores::set_up()?;
  let required = TermQuery::new(Term::from_text("field", "this"));
  let prohibited = TermQuery::new(Term::from_text("field", "3"));
  let mut bq = BooleanQueryBuilder::new();
  bq.add(
    AssertNeedsScores::new(required, ScoreMode::TopScores),
    Occur::Must,
  )?;
  bq.add(
    AssertNeedsScores::new(prohibited, ScoreMode::CompleteNoScores),
    Occur::MustNot,
  )?;
  assert_eq!(4, case.searcher.search(bq.build(), 5)?.total_hits.value());
  Ok(())
}

#[test]
fn test_constant_score_query() -> Result<()> {
  let case = TestNeedsScores::set_up()?;
  let term = TermQuery::new(Term::from_text("field", "this"));

  let constant_score = ConstantScoreQuery::new(AssertNeedsScores::new(
    term.clone(),
    ScoreMode::CompleteNoScores,
  ));
  assert_eq!(5, case.searcher.count(constant_score.clone())?);

  let manager = TopScoreDocCollectorManager::with_after(5, None, i32::MAX as usize)?;
  let hits = case
    .searcher
    .search_with_collector_manager(constant_score, &manager)?;
  assert_eq!(5, hits.total_hits.value());

  let constant_score = ConstantScoreQuery::new(AssertNeedsScores::new(term, ScoreMode::TopDocs));
  assert_eq!(
    5,
    case
      .searcher
      .search(constant_score.clone(), 5)?
      .total_hits
      .value()
  );
  assert_eq!(
    5,
    case
      .searcher
      .search_with_sort(constant_score.clone(), 5, Sort::get_index_order()?)?
      .base
      .total_hits
      .value()
  );
  assert_eq!(
    5,
    case
      .searcher
      .search_with_sort(
        constant_score,
        5,
        Sort::with_fields(vec![
          SortFieldEnum::from(SortField::get_field_doc()?),
          SortFieldEnum::from(SortField::new(None::<String>, SortFieldType::Score)?),
        ])?,
      )?
      .base
      .total_hits
      .value()
  );
  Ok(())
}

#[test]
fn test_sort_by_field() -> Result<()> {
  let case = TestNeedsScores::set_up()?;
  let query = AssertNeedsScores::new(MatchAllDocsQuery::new(), ScoreMode::TopDocs);
  assert_eq!(
    5,
    case
      .searcher
      .search_with_sort(query, 5, Sort::get_index_order()?)?
      .base
      .total_hits
      .value()
  );
  Ok(())
}

#[test]
fn test_sort_by_score() -> Result<()> {
  let case = TestNeedsScores::set_up()?;
  let query = AssertNeedsScores::new(MatchAllDocsQuery::new(), ScoreMode::TopScores);
  assert_eq!(
    5,
    case
      .searcher
      .search_with_sort(query, 5, Sort::get_relevance()?)?
      .base
      .total_hits
      .value()
  );
  Ok(())
}

/// Wraps a query, checking that the score mode passed to `Weight` is the expected value.
#[derive(Clone, Debug)]
pub struct AssertNeedsScores {
  query: Box<Query>,
  value: ScoreMode,
  id: Identity,
}

impl AssertNeedsScores {
  pub(crate) fn new<T>(query: T, value: ScoreMode) -> Self
  where
    T: IntoBoxQuery,
  {
    Self {
      query: query.into_box_query(),
      value,
      id: Identity::new(),
    }
  }
}

impl HasIdentity for AssertNeedsScores {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl PartialEq for AssertNeedsScores {
  fn eq(&self, other: &Self) -> bool {
    self.query == other.query && self.value == other.value
  }
}

impl Eq for AssertNeedsScores {}

impl Hash for AssertNeedsScores {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.query.hash(state);
    mem::discriminant(&self.value).hash(state);
  }
}

impl QueryBase for AssertNeedsScores {
  fn to_string(&self, field: &str) -> Result<String> {
    Ok(format!("asserting({})", self.query.to_string(field)?))
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
    assert_eq!(
      self.value,
      *score_mode,
      "query={}",
      self.query.to_string("")?
    );
    let inner_weight = (*self.query)
      .clone()
      .create_weight(searcher, score_mode, boost)?;
    assert_eq!(self.value, *score_mode);
    Ok(Box::new(AssertNeedsScoresWeight::new(self, inner_weight)))
  }

  fn rewrite<IRC>(mut self, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    let query_id = self.query.identity().clone();
    let rewritten = self.query.rewrite(searcher)?;
    if rewritten.identity() != &query_id {
      Ok(AssertNeedsScores::new(rewritten, self.value).into())
    } else {
      self.query = Box::new(rewritten);
      Ok(self.into())
    }
  }

  fn visit<QV>(&self, visitor: &QV)
  where
    QV: QueryVisitor,
  {
    self.query.visit(visitor);
  }
}

impl IntoBoxQuery for AssertNeedsScores {
  fn into_box_query(self) -> Box<Query> {
    Box::new(self.into())
  }
}

struct AssertNeedsScoresWeight<IRC>
where
  IRC: IndexReaderContext,
{
  query: Arc<Query>,
  inner_weight: QueryWeight<IRC>,
}

impl<IRC> AssertNeedsScoresWeight<IRC>
where
  IRC: IndexReaderContext,
{
  fn new(query: AssertNeedsScores, inner_weight: QueryWeight<IRC>) -> Self {
    Self {
      query: Arc::new(query.into()),
      inner_weight,
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for AssertNeedsScoresWeight<IRC>
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    self.inner_weight.is_cacheable(ctx)
  }
}

impl<IRC> Weight<IRC> for AssertNeedsScoresWeight<IRC>
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
    self.inner_weight.matches(context, doc, searcher)
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    self.inner_weight.explain(context, doc, searcher)
  }

  fn get_query(&self) -> Arc<Query> {
    self.query.clone()
  }

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    self.inner_weight.scorer_supplier(context, searcher)
  }

  fn count(&self, context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
    self.inner_weight.count(context)
  }
}
