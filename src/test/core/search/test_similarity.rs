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
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, new_merge_policy_with_mock_mp,
  new_searcher_with_reader, random,
};
use std::fmt::{Display, Formatter};

use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::query::Query;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::similarities_impl::classic_similarity::idf_explain;
use crate::core::search::similarities_impl::tf_idf_similarity::{
  TFIDFSimilarity, TFIDFSimilarityBase, TFIDFSubEnum,
};
use crate::core::search::simple_collector::SimpleCollector;
use crate::core::search::term_query::TermQuery;
use crate::core::search::term_statistics::TermStatistics;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::dummy_total_hit_count_collector::CollectorManagerImpl;

#[allow(dead_code)] // for quick search
struct TestSimilarity;

#[derive(Clone)]
pub struct SimpleSimilarity;
pub fn new_simple_similarity() -> TFIDFSimilarity {
  let v = TFIDFSubEnum::Simple(SimpleSimilarity);
  TFIDFSimilarity::new(v)
}
impl TFIDFSimilarityBase for SimpleSimilarity {
  fn tf(&self, freq: f32) -> f32 {
    freq
  }

  fn idf_explain(
    &self,
    collection_stats: &CollectionStatistics,
    term_stats: &TermStatistics,
  ) -> Explanation {
    idf_explain(self, collection_stats, term_stats)
  }

  fn idf_explain_from_multi_ts(
    &self,
    _collection_stats: &CollectionStatistics,
    _term_stats: &[TermStatistics],
  ) -> Explanation {
    Explanation::match_no_details(1.0f32, "Inexplicable")
  }

  fn idf(&self, _doc_freq: i64, _doc_count: i64) -> f32 {
    1f32
  }

  fn length_norm(&self, _length: i32) -> f32 {
    1f32
  }
}
#[test]
fn test_similarity() -> Result<()> {
  let mut random = random();
  let store = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  iwc.set_similarity(new_simple_similarity());
  iwc.set_merge_policy(new_merge_policy_with_mock_mp(&mut random, false)?);
  let writer = RandomIndexWriter::with_config(&mut random, store.clone(), iwc);

  let mut d1 = Document::new();
  d1.add(TextField::from_string("field", "a c", Store::Yes)?);

  let mut d2 = Document::new();
  d2.add(TextField::from_string("field", "a c b", Store::Yes)?);

  writer.add_document(&mut random, d1)?;
  writer.add_document(&mut random, d2)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let mut searcher = new_searcher_with_reader(reader)?;
  searcher.set_similarity(new_simple_similarity());

  let a = Term::from_text("field", "a");
  let b = Term::from_text("field", "b");
  let c = Term::from_text("field", "c");

  assert_score(&searcher, TermQuery::new(b.clone()).into(), 1.0)?;

  let mut bq = Builder::new();
  bq.add(TermQuery::new(a.clone()), Occur::Should)?;
  bq.add(TermQuery::new(b.clone()), Occur::Should)?;

  let manager = CollectorManagerImpl;
  searcher.search_with_collector_manager(bq.build(), &manager)?;

  let mut pq =
    PhraseQuery::from_bytes_no_slop(a.field(), vec![a.bytes().clone(), c.bytes().clone()])?;
  assert_score(&searcher, pq.into(), 1.0)?;

  pq = PhraseQuery::from_bytes(2, a.field(), vec![a.bytes().clone(), b.bytes().clone()])?;
  assert_score(&searcher, pq.into(), 0.5)?;
  Ok(())
}
fn assert_score<IRC>(searcher: &IndexSearcher<IRC>, query: Query, score: f32) -> Result<()>
where
  IRC: IndexReaderContext + Sync,
{
  let manager = CollectorManagerImpl2::new(score);
  searcher.search_with_collector_manager(query, &manager)?;
  Ok(())
}

trait ScoreAssertingCollector: SimpleCollector {
  fn score_mode(&self) -> ScoreMode {
    ScoreMode::Complete
  }
}
struct ScoreAssertingCollectorImpl1 {
  base: usize,
}
impl ScoreAssertingCollectorImpl1 {
  fn new() -> Self {
    Self { base: 0 }
  }
}

impl SimpleCollector for ScoreAssertingCollectorImpl1 {
  fn do_set_next_reader<LR>(&mut self, context: &LeafReaderContext<LR>) -> Result<()>
  where
    LR: LeafReader,
  {
    self.base = context.doc_base;
    Ok(())
  }
}

impl Collector for ScoreAssertingCollectorImpl1 {
  type LeafCollector<'a, IRC>
    = &'a mut Self
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    weight: Option<&W>,
  ) -> crate::core::util::error::lucene_error::Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    SimpleCollector::get_leaf_collector(self, context, weight)?;
    Ok(self)
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreAssertingCollector::score_mode(self)
  }
}

impl LeafCollector for ScoreAssertingCollectorImpl1 {
  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    assert_eq!((doc as usize + self.base + 1) as f32, scorer.score()?);
    Ok(())
  }
}

impl Display for ScoreAssertingCollectorImpl1 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl ScoreAssertingCollector for ScoreAssertingCollectorImpl1 {}

struct ScoreAssertingCollectorImpl2 {
  score: f32,
}

impl ScoreAssertingCollectorImpl2 {
  fn new(score: f32) -> Self {
    Self { score }
  }
}

impl SimpleCollector for ScoreAssertingCollectorImpl2 {}

impl Collector for ScoreAssertingCollectorImpl2 {
  type LeafCollector<'a, IRC>
    = &'a mut Self
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    SimpleCollector::get_leaf_collector(self, context, weight)?;
    Ok(self)
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreAssertingCollector::score_mode(self)
  }
}

impl LeafCollector for ScoreAssertingCollectorImpl2 {
  fn collect(&mut self, _doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    assert_eq!(self.score, scorer.score()?);
    Ok(())
  }
}

impl Display for ScoreAssertingCollectorImpl2 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl ScoreAssertingCollector for ScoreAssertingCollectorImpl2 {}

struct CollectorManagerImpl1;
impl CollectorManager for CollectorManagerImpl1 {
  type C = ScoreAssertingCollectorImpl1;
  type T = ();

  fn new_collector(&self) -> Result<Self::C> {
    Ok(ScoreAssertingCollectorImpl1::new())
  }

  fn reduce(&self, _collectors: Vec<Self::C>) -> Result<Self::T> {
    Ok(())
  }
}

struct CollectorManagerImpl2 {
  score: f32,
}
impl CollectorManagerImpl2 {
  fn new(score: f32) -> Self {
    Self { score }
  }
}
impl CollectorManager for CollectorManagerImpl2 {
  type C = ScoreAssertingCollectorImpl2;
  type T = ();

  fn new_collector(&self) -> Result<Self::C> {
    Ok(ScoreAssertingCollectorImpl2::new(self.score))
  }

  fn reduce(&self, _collectors: Vec<Self::C>) -> Result<Self::T> {
    Ok(())
  }
}
