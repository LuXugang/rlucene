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
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::TextField;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::boolean_scorer_supplier::BooleanScorerSupplier;
use crate::core::search::bulk_scorer::{BulkScorer, BulkScorerKind};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::term_query::TermQuery;
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_directory_shared, new_searcher_with_reader, random,
};

use crate::core::index::index_reader::Identity;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{
  Query, QueryBase, QueryWeight, QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer,
};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score::Score;
use crate::core::search::scorer::ScorerKind;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::Weight;
use crate::core::util::HasIdentity;
use crate::core::util::bits::Bits;
use crate::test::core::search::query_utils::QueryUtils;
use rand::RngExt;

#[allow(dead_code)] // for quick search
struct TestBooleanScorer;

const FIELD: &str = "category";
#[test]
fn test_method() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;

  let values = ["1", "2", "3", "4"];

  let writer = RandomIndexWriter::new(&mut random, directory.clone());
  for value in values {
    let mut doc = Document::new();
    doc.add(StringField::from_string(FIELD, value, Store::Yes)?);
    writer.add_document(doc)?;
  }
  let ir = writer.get_reader()?;
  writer.close()?;

  let mut boolean_query1 = Builder::new();
  boolean_query1.add(TermQuery::new(Term::from_text(FIELD, "1")), Occur::Should)?;
  boolean_query1.add(TermQuery::new(Term::from_text(FIELD, "2")), Occur::Should)?;

  let mut query = Builder::new();
  query.add(boolean_query1.build(), Occur::Must)?;
  query.add(TermQuery::new(Term::from_text(FIELD, "9")), Occur::MustNot)?;

  let index_searcher = new_searcher_with_reader(ir)?;
  let hits = index_searcher.search(query.build(), 1000)?.score_docs;
  assert_eq!(2, hits.len(), "Number of matched documents");
  Ok(())
}
#[test]
fn test_embedded_boolean_scorer() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone());

  let mut doc = Document::new();
  doc.add(TextField::from_string(
        "field",
        "doctors are people who prescribe medicines of which they know little, to cure diseases of which they know less, in human beings of whom they know nothing",
        Store::No,
    )?);
  w.add_document(doc)?;

  let reader = w.get_reader()?;
  w.close()?;

  let searcher = new_searcher_with_reader(reader)?;
  let mut q1 = Builder::new();
  q1.add(
    TermQuery::new(Term::from_text("field", "little")),
    Occur::Should,
  )?;
  q1.add(
    TermQuery::new(Term::from_text("field", "diseases")),
    Occur::Should,
  )?;

  let mut q2 = Builder::new();
  q2.add(q1.build(), Occur::Should)?;
  q2.add(CrazyMustUseBulkScorerQuery::new(), Occur::Should)?;

  assert_eq!(1, searcher.count(q2.build())?);
  Ok(())
}
#[test]
fn test_optimize_top_level_clause_or_null() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone());

  let mut doc = Document::new();
  doc.add(StringField::from_string("foo", "bar", Store::No)?);
  w.add_document(doc)?;

  let reader = w.get_reader()?;
  let mut searcher = new_searcher_with_reader(reader)?;
  searcher.set_query_cache(None);
  let leaves = searcher.get_top_reader_context().leaves()?;
  let ctx = &leaves[0];

  let mut query = Builder::new();
  query.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
  query.add(
    TermQuery::new(Term::from_text("missing_field", "baz")),
    Occur::Should,
  )?;
  let query = query.build();

  let rewritten = searcher.rewrite(query)?;
  let weight = searcher.create_weight(rewritten, ScoreMode::CompleteNoScores, 1.0)?;
  let mut ss = weight.scorer_supplier(ctx, &searcher)?.unwrap();
  let scorer = ss
        .as_any()
        .downcast_mut::<BooleanScorerSupplier<
            CompositeReaderContext<StandardDirectoryReaderType<DirEnum>>,
        >>()
        .unwrap();
  let bs = scorer.boolean_scorer(ctx, &searcher)?.unwrap();
  assert!(matches!(bs.kind(), BulkScorerKind::Default));

  let mut query = Builder::new();
  query.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
  query.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
  let query = query.build();

  let rewritten = searcher.rewrite(query)?;
  let weight = searcher.create_weight(rewritten, ScoreMode::Complete, 1.0)?;
  let mut ss = weight.scorer_supplier(ctx, &searcher)?.unwrap();
  let scorer = ss
        .as_any()
        .downcast_mut::<BooleanScorerSupplier<
            CompositeReaderContext<StandardDirectoryReaderType<DirEnum>>,
        >>()
        .unwrap();
  let bs = scorer.boolean_scorer(ctx, &searcher)?.unwrap();
  assert!(matches!(bs.kind(), BulkScorerKind::Default));
  w.close()?;
  Ok(())
}
#[test]
fn test_optimize_prohibited_clauses() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone());

  let mut doc = Document::new();
  doc.add(StringField::from_string("foo", "bar", Store::No)?);
  doc.add(StringField::from_string("foo", "baz", Store::No)?);
  w.add_document(doc)?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("foo", "baz", Store::No)?);
  w.add_document(doc)?;

  w.force_merge(1)?;
  let reader = w.get_reader()?;
  let mut searcher = new_searcher_with_reader(reader)?;
  searcher.set_query_cache(None);
  let leaves = searcher.get_top_reader_context().leaves()?;
  let ctx = &leaves[0];

  let mut query = Builder::new();
  query.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
  query.add(
    TermQuery::new(Term::from_text("foo", "bar")),
    Occur::MustNot,
  )?;
  let query = query.build();

  let rewritten = searcher.rewrite(query)?;
  let weight = searcher.create_weight(rewritten, ScoreMode::Complete, 1.0)?;
  let mut ss = weight.scorer_supplier(ctx, &searcher)?.unwrap();
  let scorer = ss
        .as_any()
        .downcast_mut::<BooleanScorerSupplier<
            CompositeReaderContext<StandardDirectoryReaderType<DirEnum>>,
        >>()
        .unwrap();
  let bs = scorer.boolean_scorer(ctx, &searcher)?.unwrap();
  assert!(matches!(bs.kind(), BulkScorerKind::ReqExcl));

  let mut query = Builder::new();
  query.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
  query.add(MatchAllDocsQuery::new(), Occur::Should)?;
  query.add(
    TermQuery::new(Term::from_text("foo", "bar")),
    Occur::MustNot,
  )?;
  let query = query.build();

  let rewritten = searcher.rewrite(query)?;
  let weight = searcher.create_weight(rewritten, ScoreMode::Complete, 1.0)?;
  let mut ss = weight.scorer_supplier(ctx, &searcher)?.unwrap();
  let scorer = ss
        .as_any()
        .downcast_mut::<BooleanScorerSupplier<
            CompositeReaderContext<StandardDirectoryReaderType<DirEnum>>,
        >>()
        .unwrap();
  let bs = scorer.boolean_scorer(ctx, &searcher)?.unwrap();
  assert!(matches!(bs.kind(), BulkScorerKind::ReqExcl));

  let mut query = Builder::new();
  query.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
  query.add(
    TermQuery::new(Term::from_text("foo", "bar")),
    Occur::MustNot,
  )?;
  let query = query.build();

  let rewritten = searcher.rewrite(query)?;
  let weight = searcher.create_weight(rewritten, ScoreMode::Complete, 1.0)?;
  let mut ss = weight.scorer_supplier(ctx, &searcher)?.unwrap();
  let scorer = ss
        .as_any()
        .downcast_mut::<BooleanScorerSupplier<
            CompositeReaderContext<StandardDirectoryReaderType<DirEnum>>,
        >>()
        .unwrap();
  let bs = scorer.boolean_scorer(ctx, &searcher)?.unwrap();
  assert!(matches!(bs.kind(), BulkScorerKind::ReqExcl));

  let mut query = Builder::new();
  query.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
  query.add(
    TermQuery::new(Term::from_text("foo", "bar")),
    Occur::MustNot,
  )?;
  let query = query.build();

  let rewritten = searcher.rewrite(query)?;
  let weight = searcher.create_weight(rewritten, ScoreMode::Complete, 1.0)?;
  let mut ss = weight.scorer_supplier(ctx, &searcher)?.unwrap();
  let scorer = ss
        .as_any()
        .downcast_mut::<BooleanScorerSupplier<
            CompositeReaderContext<StandardDirectoryReaderType<DirEnum>>,
        >>()
        .unwrap();
  let bs = scorer.boolean_scorer(ctx, &searcher)?.unwrap();
  assert!(matches!(bs.kind(), BulkScorerKind::ReqExcl));

  w.close()?;
  Ok(())
}
#[test]
fn test_sparse_clause_optimization() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone());

  let empty_doc = Document::new();
  let num_docs = at_least(&mut random, 10);
  let mut num_empty_docs = at_least(&mut random, 200);

  for _ in 0..num_docs {
    for _ in (0..=num_empty_docs).rev() {
      w.add_document(empty_doc.clone())?;
    }

    let mut doc = Document::new();
    for value in ["foo", "bar", "baz"] {
      if random.random_bool(0.5) {
        doc.add(StringField::from_string("field", value, Store::No)?);
      }
    }
    w.add_document(doc)?;
  }

  num_empty_docs = at_least(&mut random, 200);
  for _ in (0..=num_empty_docs).rev() {
    w.add_document(empty_doc.clone())?;
  }

  if random.random_bool(0.5) {
    w.force_merge(1)?;
  }

  let reader = w.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut query = Builder::new();
  query.add(
    BoostQuery::new(TermQuery::new(Term::from_text("field", "foo")), 3.0)?,
    Occur::Should,
  )?;
  query.add(
    BoostQuery::new(TermQuery::new(Term::from_text("field", "bar")), 3.0)?,
    Occur::Should,
  )?;
  query.add(
    BoostQuery::new(TermQuery::new(Term::from_text("field", "baz")), 3.0)?,
    Occur::Should,
  )?;
  let query = query.build();

  QueryUtils::check_from_searcher(&mut random, query, &searcher)?;

  w.close()?;
  Ok(())
}
#[test]
fn test_filter_constant_score() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone());

  let mut doc = Document::new();
  doc.add(StringField::from_string("foo", "bar", Store::No)?);
  doc.add(StringField::from_string("foo", "bat", Store::No)?);
  doc.add(StringField::from_string("foo", "baz", Store::No)?);
  w.add_document(doc)?;

  let reader = w.get_reader()?;
  let mut searcher = new_searcher_with_reader(reader)?;
  searcher.set_query_cache(None);

  {
    let mut query = Builder::new();
    query.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
    let query = query.build();

    let rewrite = searcher.rewrite(query)?;
    match rewrite {
      Query::Boost(b) => {
        matches!(*b.get_query(), Query::Term(_))
      },
      _ => unreachable!(""),
    };
  }

  let queries = vec![
    {
      let mut query = Builder::new();
      query.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
      query.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
      query.build()
    },
    {
      let mut query = Builder::new();
      query.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
      query.add(TermQuery::new(Term::from_text("foo", "arf")), Occur::Should)?;
      query.build()
    },
    {
      let mut query = Builder::new();
      query.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
      query.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
      query.add(TermQuery::new(Term::from_text("foo", "arf")), Occur::Should)?;
      query.add(TermQuery::new(Term::from_text("foo", "arw")), Occur::Should)?;
      query.build()
    },
  ];

  let leaves = searcher.get_top_reader_context().leaves()?;
  let ctx = &leaves[0];

  for query in queries {
    let rewrite = searcher.rewrite(query)?;
    for score_mode in ScoreMode::values() {
      let weight = searcher.create_weight(rewrite.clone(), *score_mode, 1.0)?;
      let scorer = weight.scorer(ctx, &searcher)?.unwrap();
      if *score_mode == ScoreMode::TopScores {
        assert!(matches!(scorer.kind(), ScorerKind::ConstantScore));
      } else {
        assert!(!matches!(scorer.kind(), ScorerKind::ConstantScore));
      }
    }
  }

  let queries = vec![
    {
      let mut query = Builder::new();
      query.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
      query.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
      query.build()
    },
    {
      let mut query = Builder::new();
      query.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
      query.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
      query.add(TermQuery::new(Term::from_text("foo", "arf")), Occur::Should)?;
      query.build()
    },
    {
      let mut query = Builder::new();
      query.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
      query.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
      query.add(TermQuery::new(Term::from_text("foo", "arf")), Occur::Must)?;
      query.build()
    },
  ];

  for query in queries {
    let rewrite = searcher.rewrite(query)?;
    for score_mode in ScoreMode::values() {
      let weight = searcher.create_weight(rewrite.clone(), *score_mode, 1.0)?;
      match weight.scorer(ctx, &searcher)? {
        None => continue,
        Some(scorer) => {
          assert!(!matches!(scorer.kind(), ScorerKind::ConstantScore));
        },
      }
    }
  }

  w.close()?;
  Ok(())
}
#[derive(Clone, Debug)]
pub struct CrazyMustUseBulkScorerQuery {
  id: Identity,
}

impl CrazyMustUseBulkScorerQuery {
  pub(crate) fn new() -> Self {
    Self {
      id: Identity::new(),
    }
  }
}

impl Default for CrazyMustUseBulkScorerQuery {
  fn default() -> Self {
    Self::new()
  }
}

impl PartialEq for CrazyMustUseBulkScorerQuery {
  fn eq(&self, other: &Self) -> bool {
    self.identity() == other.identity()
  }
}

impl Eq for CrazyMustUseBulkScorerQuery {}

impl Hash for CrazyMustUseBulkScorerQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.identity().hash(state);
  }
}

impl HasIdentity for CrazyMustUseBulkScorerQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for CrazyMustUseBulkScorerQuery {
  fn to_string(&self, _field: &str) -> Result<String> {
    Ok("MustUseBulkScorerQuery".to_string())
  }

  fn create_weight<IRC>(
    self,
    _searcher: &IndexSearcher<IRC>,
    _score_mode: &ScoreMode,
    _boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Ok(Box::new(CrazyMustUseBulkScorerWeight::new(self)))
  }

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Ok(self.into())
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
  }
}

struct CrazyMustUseBulkScorerWeight {
  query: Arc<Query>,
}

impl CrazyMustUseBulkScorerWeight {
  fn new(query: CrazyMustUseBulkScorerQuery) -> Self {
    Self {
      query: Arc::new(query.into()),
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for CrazyMustUseBulkScorerWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    Ok(false)
  }
}

impl<IRC> Weight<IRC> for CrazyMustUseBulkScorerWeight
where
  IRC: IndexReaderContext,
{
  type Matches = MatchWithNoTerms;

  fn matches(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _doc: i32,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::Matches>> {
    Ok(None)
  }

  fn explain(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _doc: i32,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_query(&self) -> Arc<Query> {
    self.query.clone()
  }

  type ScorerSupplier = QueryWeightSs<IRC>;

  fn scorer_supplier(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    Ok(Some(Box::new(CrazyMustUseBulkScorerSupplier)))
  }
}

struct CrazyMustUseBulkScorerSupplier;

impl<IRC> ScorerSupplier<IRC> for CrazyMustUseBulkScorerSupplier
where
  IRC: IndexReaderContext,
{
  type Scorer = QueryWeightSsScorer;
  type BulkScorer = QueryWeightSsBulkScorer;

  fn get(
    &mut self,
    _lead_cost: i64,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::Scorer> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn bulk_scorer(
    &mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::BulkScorer>> {
    Ok(Some(Box::new(CrazyMustUseBulkScorer)))
  }

  fn cost(
    &mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }
}

struct CrazyMustUseBulkScorer;

impl BulkScorer for CrazyMustUseBulkScorer {
  fn score(
    &mut self,
    collector: &mut dyn LeafCollector,
    accept_docs: Option<&dyn Bits>,
    min: i32,
    max: i32,
  ) -> Result<i32> {
    if min <= 0 && max > 0 && accept_docs.map_or(Ok(true), |bits| bits.get(0))? {
      let mut score = Score::default();
      collector.set_scorer(&mut score)?;
      collector.collect(0, &mut score)?;
    }
    Ok(NO_MORE_DOCS)
  }

  fn cost(&mut self) -> Result<i64> {
    Ok(1)
  }
}
