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
use crate::core::index::index_reader::{Identity, IndexReader};
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::bulk_scorer::{BulkScorer, BulkScorerEnum2};
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set_iterator::AllDISI;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{
  Query, QueryBase, QueryWeight, QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer,
};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score::Score;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::{DefaultBulkScorer, Weight};
use crate::core::util::bits::Bits;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::Result;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// A query that matches all documents.
#[derive(Debug, Clone)]
pub struct MatchAllDocsQuery {
  id: Identity,
}
impl Default for MatchAllDocsQuery {
  fn default() -> Self {
    Self::new()
  }
}

impl MatchAllDocsQuery {
  pub fn new() -> Self {
    MatchAllDocsQuery {
      id: Identity::new(),
    }
  }
}

impl PartialEq for MatchAllDocsQuery {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}
impl Eq for MatchAllDocsQuery {}

impl Hash for MatchAllDocsQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    0usize.hash(state);
  }
}

impl HasIdentity for MatchAllDocsQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for MatchAllDocsQuery {
  fn as_string(&self, _field: &str) -> Result<String> {
    Ok("*:*".to_string())
  }

  fn create_weight<IRC>(
    self,
    _searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Ok(Box::new(MatchAllWeight::new(boost, self, *score_mode)))
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
    todo!()
  }
}

pub struct MatchAllWeight {
  base: ConstantScoreWeight,
  parent_query: Arc<Query>,
  score_mode: ScoreMode,
}
impl MatchAllWeight {
  pub fn new(score: f32, query: MatchAllDocsQuery, score_mode: ScoreMode) -> Self {
    Self {
      base: ConstantScoreWeight::new(score),
      parent_query: Arc::new(query.into()),
      score_mode,
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for MatchAllWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    Ok(true)
  }
}

impl<IRC> Weight<IRC> for MatchAllWeight
where
  IRC: IndexReaderContext,
{
  type Matches = MatchWithNoTerms;

  fn matches(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::Matches>> {
    self.default_matches(context, doc, searcher)
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    let scorer = self.scorer(context, searcher)?;
    self
      .base
      .explain(scorer, doc, self.parent_query.as_string("")?)
  }

  fn get_query(&self) -> Arc<Query> {
    self.parent_query.clone()
  }

  type ScorerSupplier = QueryWeightSs<IRC>;

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    let v = Box::new(MatchAllDocsScorerSupplier::new(
      self.score_mode,
      self.base.clone(),
      context.reader().max_doc()?,
    ));
    Ok(Some(v))
  }

  fn count(&self, context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
    context.reader().num_docs()
  }
}
impl Debug for MatchAllWeight {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "weight({:?})", MatchAllDocsQuery::new())
  }
}
pub struct MatchAllDocsScorerSupplier {
  score_mode: ScoreMode,
  weight: ConstantScoreWeight,
  max_doc: i32,
}
impl MatchAllDocsScorerSupplier {
  pub fn new(score_mode: ScoreMode, weight: ConstantScoreWeight, max_doc: i32) -> Self {
    Self {
      score_mode,
      weight,
      max_doc,
    }
  }
}
impl<IRC> ScorerSupplier<IRC> for MatchAllDocsScorerSupplier
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
    let score = self.weight.score();
    let v = ConstantScoreScorer::from_disi(score, self.score_mode, AllDISI::new(self.max_doc));
    Ok(Box::new(v))
  }

  fn bulk_scorer(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::BulkScorer>> {
    let v = if !self.score_mode.is_exhaustive() {
      let opt = self.default_bulk_scorer(context, searcher)?;
      MatchAllBulkScorerEnum::B(opt)
    } else {
      let score = self.weight.score();
      MatchAllBulkScorerEnum::A(MatchAllBulkScorer::new(
        self.score_mode,
        self.max_doc,
        score,
      ))
    };
    Ok(Some(Box::new(v)))
  }

  fn cost(
    &mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<i64> {
    Ok(self.max_doc as i64)
  }
}
pub struct MatchAllBulkScorer {
  score_mode: ScoreMode,
  max_doc: i32,
  score: f32,
}
impl MatchAllBulkScorer {
  pub fn new(score_mode: ScoreMode, max_doc: i32, score: f32) -> Self {
    Self {
      score_mode,
      max_doc,
      score,
    }
  }
}
impl BulkScorer for MatchAllBulkScorer {
  fn score(
    &mut self,
    collector: &mut dyn LeafCollector,
    accept_docs: Option<&dyn Bits>,
    min: i32,
    max: i32,
  ) -> Result<i32> {
    let max = std::cmp::min(max, self.max_doc);
    let mut scorer = Score::new(self.score);
    collector.set_scorer(&mut scorer)?;
    for doc in min..max {
      if match accept_docs {
        None => true,
        Some(bits) => bits.get(doc as usize)?,
      } {
        collector.collect(doc, &mut scorer)?;
      }
    }
    if max == self.max_doc {
      Ok(NO_MORE_DOCS)
    } else {
      Ok(max)
    }
  }

  fn cost(&mut self) -> Result<i64> {
    Ok(self.max_doc as i64)
  }
}
pub type MatchAllBulkScorerEnum =
  BulkScorerEnum2<MatchAllBulkScorer, DefaultBulkScorer<QueryWeightSsScorer>>;

#[cfg(test)]
mod tests {
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::document::field_type::FieldType;

  use crate::core::index::directory_reader;
  use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
  use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
  use crate::core::index::stored_fields::StoredFields;
  use crate::core::index::term::Term;
  use crate::core::search::boolean_clause::Occur;
  use crate::core::search::boolean_query::Builder;
  use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
  use crate::core::search::term_query::TermQuery;
  use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
  use crate::core::search::total_hits::Relation;
  use crate::core::store::directory::Directory;
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_index_writer_config, new_index_writer_config_with_analyzer,
    new_log_merge_policy, new_searcher_with_reader, new_searcher_with_threads, new_text_field,
    random,
  };
  use rand_chacha::rand_core::Rng;
  use std::collections::HashMap;
  use std::sync::Arc;
  use std::vec;

  #[allow(dead_code)] // for quick search
  struct TestMatchAllDocsQuery;
  #[test]
  fn test_query() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_max_buffered_docs(2);
    iwc.set_merge_policy(new_log_merge_policy(&mut random)?);

    let iw = IndexWriter::new(dir.clone(), iwc)?;
    let mut field_types = HashMap::new();

    add_doc(&mut random, "one", &iw, &mut field_types)?;
    add_doc(&mut random, "two", &iw, &mut field_types)?;
    add_doc(&mut random, "three four", &iw, &mut field_types)?;

    let ir = directory_reader::open_from_writer(&iw)?;
    let mut searcher = new_searcher_with_reader(ir)?;

    let mut hits = searcher.search(MatchAllDocsQuery::new(), 1000)?.score_docs;
    assert_eq!(3, hits.len());
    assert_eq!(
      "one",
      searcher
        .stored_fields()?
        .document(hits[0].doc)?
        .get("key")?
        .unwrap()
        .as_ref()
    );
    assert_eq!(
      "two",
      searcher
        .stored_fields()?
        .document(hits[1].doc)?
        .get("key")?
        .unwrap()
        .as_ref()
    );
    assert_eq!(
      "three four",
      searcher
        .stored_fields()?
        .document(hits[2].doc)?
        .get("key")?
        .unwrap()
        .as_ref()
    );

    // some artificial queries to trigger the use of skipTo():

    let mut bq = Builder::new();
    bq.add(MatchAllDocsQuery::new(), Occur::Must)?;
    bq.add(MatchAllDocsQuery::new(), Occur::Must)?;
    hits = searcher.search(bq.build(), 1000)?.score_docs;
    assert_eq!(3, hits.len());

    let mut bq = Builder::new();
    bq.add(MatchAllDocsQuery::new(), Occur::Must)?;
    bq.add(TermQuery::new(Term::from_text("key", "three")), Occur::Must)?;
    hits = searcher.search(bq.build(), 1000)?.score_docs;
    assert_eq!(1, hits.len());

    iw.delete_documents_with_terms(vec![Term::from_text("key", "one")])?;

    let reader = directory_reader::open_from_writer(&iw)?;
    searcher = new_searcher_with_reader(reader)?;

    hits = searcher.search(MatchAllDocsQuery::new(), 1000)?.score_docs;
    assert_eq!(2, hits.len());

    iw.close()?;
    Ok(())
  }
  #[test]
  fn test_equals() -> Result<()> {
    let q1 = MatchAllDocsQuery::new();
    let q2 = MatchAllDocsQuery::new();
    assert_eq!(q1, q2);
    Ok(())
  }
  fn add_doc<D, B, R>(
    random: &mut R,
    text: &str,
    iw: &IndexWriter<D, B>,
    field_to_type: &mut HashMap<String, FieldType>,
  ) -> Result<()>
  where
    D: Directory,
    B: IndexWriterBase,
    R: Rng + ?Sized,
  {
    let mut doc = Document::new();
    let field = new_text_field(random, "key", text, Store::Yes, field_to_type)?;
    doc.add(field);
    iw.add_document(doc)?;
    Ok(())
  }
  #[test]
  fn test_early_termination() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let mut config = new_index_writer_config_with_analyzer(&mut random, mock);
    config.set_max_buffered_docs(2);
    config.set_merge_policy(new_log_merge_policy(&mut random)?);
    let iw = IndexWriter::new(dir.clone(), config)?;
    let mut field_types = HashMap::new();
    let num_docs = 500;
    for i in 0..num_docs {
      let text = format!("doc{}", i);
      add_doc(&mut random, &text, &iw, &mut field_types)?;
    }

    let ir = directory_reader::open_from_writer(&iw)?;
    let ir_arc = Arc::new(ir);

    let single_threaded_searcher = new_searcher_with_threads(ir_arc.clone(), true, true, false)?;

    let total_hits_threshold = 200;
    let collector_mgr = TopScoreDocCollectorManager::new(10, total_hits_threshold)?;

    let top_docs = single_threaded_searcher
      .search_with_collector_manager(MatchAllDocsQuery::new(), &collector_mgr)?;

    assert_eq!(top_docs.total_hits.value(), total_hits_threshold + 1);
    assert_eq!(
      top_docs.total_hits.relation(),
      Relation::GreaterThanOrEqualTo
    );

    let searcher = new_searcher_with_reader(ir_arc.clone())?;
    let collector_mgr = TopScoreDocCollectorManager::new(10, num_docs)?;

    let top_docs =
      searcher.search_with_collector_manager(MatchAllDocsQuery::new(), &collector_mgr)?;

    assert_eq!(top_docs.total_hits.value(), num_docs);
    assert_eq!(top_docs.total_hits.relation(), Relation::EqualTo);
    iw.close()?;
    Ok(())
  }
}
