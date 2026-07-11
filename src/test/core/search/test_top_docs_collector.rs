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
use crate::core::index::composite_reader::{CompositeReader, get_context};
use crate::core::index::directory_reader;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::multi_terms::get_terms;
use crate::core::index::terms::Terms;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_index_writer_config, new_searcher_with_reader,
  new_searcher_with_threads, random,
};

use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::standard_directory_reader::StandardDirectoryReader;
use crate::core::index::term::Term;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;

use crate::core::document::string_field::StringField;
use crate::core::index::BytesRef;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::dummy::dummy_weight::DummyWeight;
use crate::core::search::hit_queue::{self, HitQueueComparator};
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::max_score_accumulator::{DEFAULT_INTERVAL, MaxScoreAccumulator};
use crate::core::search::query::Query;
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::score_mode::ScoreMode::CompleteNoScores;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::{TopDocs, TopDocsLike};
use crate::core::search::top_docs_collector::{TopDocsCollector, TopDocsCollectorBase};
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::search::total_hits::Relation::{EqualTo, GreaterThanOrEqualTo};
use crate::core::search::total_hits::{Relation, TotalHits};
use crate::core::search::weight::Weight;
use crate::core::store::directory::Directory;
use crate::core::util::TryIntoInt;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::PriorityQueue;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::check_hits::CheckHits;
use crate::test_framework::core::util::line_file_docs::LineFileDocs;
use rand::{Rng, RngExt};
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[allow(dead_code)] // for quick search
struct TestTopDocsCollector;

struct MyTopDocsCollectorMananger {
  num_hits: i32,
}
impl MyTopDocsCollectorMananger {
  fn new(num_hits: i32) -> Self {
    Self { num_hits }
  }
}
impl CollectorManager for MyTopDocsCollectorMananger {
  type C = MyTopDocsCollector;
  type T = MyTopDocsCollector;

  fn new_collector(&self) -> Result<Self::C> {
    MyTopDocsCollector::new(self.num_hits.try_convert()?)
  }

  fn reduce(&self, collectors: Vec<Self::C>) -> Result<Self::T> {
    let mut total_hits = 0;
    let mut my_top_docs_collector = MyTopDocsCollector::new(self.num_hits.try_convert()?)?;
    for collector in collectors {
      total_hits += collector.base.total_hits;
      for score_doc in collector.base.pq.iter() {
        my_top_docs_collector
          .pq_mut()
          .insert_with_overflow(score_doc)?;
      }
    }
    my_top_docs_collector.base.total_hits = total_hits;
    Ok(my_top_docs_collector)
  }
}

pub const SCORES: [f32; 30] = [
  0.7767749, 1.7839992, 8.9925785, 7.9608946, 0.07948637, 2.6356435, 7.4950366, 7.1490803,
  8.108544, 4.961808, 2.2423935, 7.285586, 4.6699767, 2.9655676, 6.953706, 5.383931, 6.9916306,
  8.365894, 7.888485, 8.723962, 3.1796896, 0.39971232, 1.3077754, 6.8489285, 9.17561, 5.060466,
  7.9793315, 8.601509, 4.1858315, 0.28146625,
];

struct LeafCollectorImpl<'a> {
  base: &'a mut MyTopDocsCollector,
  doc_base: usize,
  scores: [f32; 30],
}
impl<'a> LeafCollectorImpl<'a> {
  fn new(base: &'a mut MyTopDocsCollector, doc_base: usize, scores: [f32; 30]) -> Self {
    Self {
      base,
      doc_base,
      scores,
    }
  }
}

impl<'a> Display for LeafCollectorImpl<'a> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "LeafCollectorImpl")
  }
}

impl<'a> LeafCollector for LeafCollectorImpl<'a> {
  fn collect(&mut self, doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
    self.base.base.total_hits += 1;
    let sd = ScoreDoc::new(
      doc + self.doc_base as i32,
      self.scores[self.doc_base + doc as usize],
    );
    self.base.pq_mut().insert_with_overflow(sd)?;
    Ok(())
  }
}
struct MyTopDocsCollector {
  base: TopDocsCollectorBase<ScoreDoc, HitQueueComparator>,
}
impl MyTopDocsCollector {
  fn new(size: usize) -> Result<Self> {
    let pq = hit_queue::new(size, true)?;
    let base = TopDocsCollectorBase::new(pq);
    Ok(Self { base })
  }
}

impl Collector for MyTopDocsCollector {
  type LeafCollector<'a, IRC>
    = LeafCollectorImpl<'a>
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    let base = context.doc_base;
    Ok(LeafCollectorImpl::new(self, base, SCORES))
  }

  fn score_mode(&self) -> ScoreMode {
    CompleteNoScores
  }
}

impl TopDocsCollector for MyTopDocsCollector {
  type Item = ScoreDoc;
  type Cmp = HitQueueComparator;
  type TopDocsLike = TopDocs<Self::Item>;

  fn pq(&self) -> &PriorityQueue<Self::Item, Self::Cmp> {
    &self.base.pq
  }

  fn pq_mut(&mut self) -> &mut PriorityQueue<Self::Item, Self::Cmp> {
    &mut self.base.pq
  }

  fn total_hits(&self) -> usize {
    self.base.total_hits
  }

  fn get_total_hits_relation(&self) -> Relation {
    self.base.total_hits_relation
  }

  fn new_top_docs(&self, results: Option<Vec<Self::Item>>, _start: i32) -> Self::TopDocsLike
  where
    Self: Sized,
  {
    match results {
      None => Self::empty_top_docs(),
      Some(res) => TopDocs::new(
        TotalHits::new(self.base.total_hits, self.base.total_hits_relation),
        res,
      ),
    }
  }
}
fn get_reader<D>(dir: Arc<D>) -> Result<StandardDirectoryReader<D>>
where
  D: Directory + 'static,
{
  let mut random = random();
  let writer = RandomIndexWriter::new(&mut random, dir)?;
  for _ in 0..30 {
    let _ = writer.add_document(&mut random, Document::new())?;
  }
  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;
  Ok(reader)
}
fn do_search<R>(random: &mut R, num_results: i32) -> Result<MyTopDocsCollector>
where
  R: Rng + ?Sized,
{
  let query = MatchAllDocsQuery::new();
  let dir = new_directory_shared(random)?;
  let reader = get_reader(dir)?;
  let searcher = new_searcher_with_reader(reader)?;
  let cm = MyTopDocsCollectorMananger::new(num_results);
  searcher.search_with_collector_manager(query, &cm)
}
fn do_search_with_threshold<R, CR>(
  random: &mut R,
  num_results: usize,
  threshold: usize,
  query: Query,
  index_reader: CR,
) -> Result<TopDocs<ScoreDoc>>
where
  CR: CompositeReader + 'static + std::marker::Sync,
  <CR as CompositeReader>::LeafReader: 'static,
  R: Rng + ?Sized,
  <CR as CompositeReader>::LeafReader: std::marker::Sync,
{
  let searcher = new_searcher_with_threads(random, index_reader, true, true, false)?;
  let collector_manager = TopScoreDocCollectorManager::with_after(num_results, None, threshold)?;
  searcher.search_with_collector_manager(query, &collector_manager)
}
fn do_concurrent_search_with_threshold<R, CR>(
  random: &mut R,
  num_results: usize,
  threshold: usize,
  query: Query,
  index_reader: CR,
) -> Result<TopDocs<ScoreDoc>>
where
  CR: CompositeReader + 'static + std::marker::Sync,
  <CR as CompositeReader>::LeafReader: 'static,
  R: Rng + ?Sized,
  <CR as CompositeReader>::LeafReader: std::marker::Sync,
{
  let searcher = new_searcher_with_threads(random, index_reader, true, true, true)?;
  let collector_manager = TopScoreDocCollectorManager::with_after(num_results, None, threshold)?;
  searcher.search_with_collector_manager(query, &collector_manager)
}

#[test]
fn test_invalid_arguments() -> Result<()> {
  let mut random = random();
  let num_results = 5;
  let mut tdc = do_search(&mut random, num_results)?;

  // start < 0
  let result = tdc.top_docs_with_start(-1);
  assert!(
    matches!(result, Err(LuceneError::IllegalArgument(msg)) if msg.message.eq(
            "Expected value of starting position is between 0 and 5, got -1",
    ))
  );

  // start == pq.size()
  let td = tdc.top_docs_with_start(num_results)?;
  assert_eq!(td.score_docs.len(), 0);

  // howMany < 0
  let result = tdc.top_docs_with_start_limit(0, -1);
  assert!(
    matches!(result, Err(LuceneError::IllegalArgument(msg)) if msg.message.eq(
            "Number of hits requested must be greater than 0 but value was -1",
    ))
  );

  Ok(())
}
#[test]
fn test_zero_results() -> Result<()> {
  let mut tdc = MyTopDocsCollector::new(5)?;
  let td = tdc.top_docs_with_start_limit(0, 1)?;
  assert_eq!(td.score_docs.len(), 0);
  Ok(())
}
#[test]
fn test_first_results_page() -> Result<()> {
  let mut random = random();
  let mut tdc = do_search(&mut random, 15)?;
  let td = tdc.top_docs_with_start_limit(0, 10)?;
  assert_eq!(td.score_docs.len(), 10);
  Ok(())
}
#[test]
fn test_second_results_pages() -> Result<()> {
  let mut random = random();

  // ask for more results than are available
  let mut tdc = do_search(&mut random, 15)?;
  let td = tdc.top_docs_with_start_limit(10, 10)?;
  assert_eq!(td.score_docs.len(), 5);

  // ask for 5 results (exactly what there should be)
  let mut tdc = do_search(&mut random, 15)?;
  let td = tdc.top_docs_with_start_limit(10, 5)?;
  assert_eq!(td.score_docs.len(), 5);

  // ask for less results than there are
  let mut tdc = do_search(&mut random, 15)?;
  let td = tdc.top_docs_with_start_limit(10, 4)?;
  assert_eq!(td.score_docs.len(), 4);

  Ok(())
}
#[test]
fn test_get_all_results() -> Result<()> {
  let mut random = random();
  let mut tdc = do_search(&mut random, 15)?;
  let td = tdc.top_docs()?;
  assert_eq!(td.score_docs.len(), 15);
  Ok(())
}

#[test]
fn test_get_results_from_start() -> Result<()> {
  let mut random = random();

  // should bring all results
  let mut tdc = do_search(&mut random, 15)?;
  let td = tdc.top_docs_with_start(0)?;
  assert_eq!(td.score_docs.len(), 15);

  // get the last 5 only
  let mut tdc = do_search(&mut random, 15)?;
  let td = tdc.top_docs_with_start(10)?;
  assert_eq!(td.score_docs.len(), 5);

  Ok(())
}
#[test]
fn test_illegal_arguments() -> Result<()> {
  let mut random = random();
  let mut tdc = do_search(&mut random, 15)?;

  // start < 0
  let result = tdc.top_docs_with_start(-1);
  assert!(
    matches!(result, Err(LuceneError::IllegalArgument(msg)) if msg.message.eq(
        "Expected value of starting position is between 0 and 15, got -1",
    ))
  );

  // how_many < 0
  let result = tdc.top_docs_with_start_limit(9, -1);
  assert!(
    matches!(result, Err(LuceneError::IllegalArgument(msg)) if msg.message.eq(
        "Number of hits requested must be greater than 0 but value was -1",
    ))
  );

  Ok(())
}
#[test]
fn test_results_order() -> Result<()> {
  let mut random = random();
  let mut tdc = do_search(&mut random, 15)?;
  let td = tdc.top_docs()?;
  let sd = td.score_docs;

  assert_eq!(MAX_SCORE, sd[0].score);
  for i in 1..sd.len() {
    assert!(sd[i - 1].score >= sd[i].score);
  }

  Ok(())
}
const MAX_SCORE: f32 = 9.17561;

struct Score {
  score: f32,
  min_competitive_score: Option<f32>,
}
impl Score {
  fn new() -> Self {
    Self {
      score: 0.0,
      min_competitive_score: None,
    }
  }
}
impl Scorable for Score {
  fn score(&mut self) -> Result<f32> {
    Ok(self.score)
  }

  fn set_min_competitive_score(&mut self, score: f32) -> Result<()> {
    assert!(
      self.min_competitive_score.is_none()
        || score >= *self.min_competitive_score.as_ref().unwrap()
    );
    self.min_competitive_score = Some(score);
    Ok(())
  }

  fn cost(&self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl FixedScore for Score {}
#[test]
fn test_set_min_competitive_score() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  writer.add_documents(vec![
    Document::new(),
    Document::new(),
    Document::new(),
    Document::new(),
  ])?;
  writer.flush()?;
  writer.add_documents(vec![Document::new(), Document::new()])?;
  writer.flush()?;

  let reader = directory_reader::open_from_writer(&writer)?;
  let v = get_context(reader)?;
  assert_eq!(v.leaves()?.len(), 2);
  writer.close()?;

  let collector_manager = TopScoreDocCollectorManager::new(2, 2)?;
  let mut collector = collector_manager.new_collector()?;
  let mut scorer = Score::new();
  let dummy_weight = Box::new(DummyWeight::<LeafReaderContext<_>>::new(
    v.leaves()?[0].reader().clone(),
  ));
  let mut leaf_collector = collector.get_leaf_collector(&v.leaves()?[0], Some(&dummy_weight))?;
  leaf_collector.set_scorer(&mut scorer)?;
  assert!(scorer.min_competitive_score.is_none());

  scorer.score = 1.0;
  leaf_collector.collect(0, &mut scorer)?;
  assert!(scorer.min_competitive_score.is_none());

  scorer.score = 2.0;
  leaf_collector.collect(1, &mut scorer)?;
  assert!(scorer.min_competitive_score.is_none());

  scorer.score = 3.0;
  leaf_collector.collect(2, &mut scorer)?;
  assert_eq!(scorer.min_competitive_score, Some(2.0f32.next_up()));

  scorer.score = 0.5;
  scorer.min_competitive_score = None;
  leaf_collector.collect(3, &mut scorer)?;
  assert!(scorer.min_competitive_score.is_none());

  scorer.score = 4.0;
  leaf_collector.collect(4, &mut scorer)?;
  assert_eq!(scorer.min_competitive_score, Some(3.0f32.next_up()));

  // Make sure the min score is set on scorers on new segments
  scorer = Score::new();
  let mut leaf_collector = collector.get_leaf_collector(&v.leaves()?[1], Some(&dummy_weight))?;
  leaf_collector.set_scorer(&mut scorer)?;
  assert_eq!(scorer.min_competitive_score, Some(3.0f32.next_up()));

  scorer.score = 1.0;
  leaf_collector.collect(0, &mut scorer)?;
  assert_eq!(scorer.min_competitive_score, Some(3.0f32.next_up()));

  scorer.score = 4.0;
  leaf_collector.collect(1, &mut scorer)?;
  assert_eq!(scorer.min_competitive_score, Some(4.0f32.next_up()));

  Ok(())
}
#[test]
fn test_shared_count_collector_manager() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  writer.add_documents(vec![
    Document::new(),
    Document::new(),
    Document::new(),
    Document::new(),
  ])?;
  writer.flush()?;
  writer.add_documents(vec![Document::new(), Document::new()])?;
  writer.flush()?;

  let reader = directory_reader::open_from_writer(&writer)?;
  let reader2 = directory_reader::open_from_writer(&writer)?;
  let v = get_context(&reader)?;
  assert_eq!(v.leaves()?.len(), 2);
  writer.close()?;

  let query = MatchAllDocsQuery::new();
  let tdc = do_concurrent_search_with_threshold(&mut random, 5, 10, query.into(), reader)?;
  let query = MatchAllDocsQuery::new();
  let tdc2 = do_search_with_threshold(&mut random, 5, 10, query.into(), reader2)?;

  let query = MatchAllDocsQuery::new();
  CheckHits::check_equal(&query.into(), &tdc.score_docs, &tdc2.score_docs)?;
  Ok(())
}
#[test]
fn test_total_hits() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  writer.add_documents(vec![
    Document::new(),
    Document::new(),
    Document::new(),
    Document::new(),
  ])?;
  writer.flush()?;
  writer.add_documents(vec![
    Document::new(),
    Document::new(),
    Document::new(),
    Document::new(),
    Document::new(),
    Document::new(),
  ])?;
  writer.flush()?;

  let reader = directory_reader::open_from_writer(&writer)?;
  let v = get_context(reader)?;
  assert_eq!(v.leaves()?.len(), 2);
  writer.close()?;
  let dummy_weight = DummyWeight::<LeafReaderContext<_>>::new(v.leaves()?[0].reader().clone());

  for total_hits_threshold in 0..20 {
    let collector_manager = TopScoreDocCollectorManager::new(2, total_hits_threshold)?;
    let mut collector = collector_manager.new_collector()?;
    let mut scorer = Score::new();
    let mut leaf_collector = collector.get_leaf_collector(&v.leaves()?[0], Some(&dummy_weight))?;
    leaf_collector.set_scorer(&mut scorer)?;

    scorer.score = 3.0;
    leaf_collector.collect(0, &mut scorer)?;

    scorer.score = 3.0;
    leaf_collector.collect(1, &mut scorer)?;

    let mut leaf_collector = collector.get_leaf_collector(&v.leaves()?[1], Some(&dummy_weight))?;
    leaf_collector.set_scorer(&mut scorer)?;

    scorer.score = 3.0;
    leaf_collector.collect(1, &mut scorer)?;

    scorer.score = 4.0;
    leaf_collector.collect(1, &mut scorer)?;

    let top_docs = collector.top_docs()?;
    assert_eq!(top_docs.total_hits.value, 4);
    assert_eq!(
      scorer.min_competitive_score.is_some(),
      total_hits_threshold < 4
    );
    assert_eq!(
      top_docs.total_hits,
      if total_hits_threshold < 4 {
        TotalHits::new(4, GreaterThanOrEqualTo)
      } else {
        TotalHits::new(4, EqualTo)
      }
    );
  }
  Ok(())
}
#[test]
fn test_relation_vs_top_docs_count() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let mut doc = Document::new();
  doc.add(TextField::from_string("f", "foo bar", Store::No)?);

  writer.add_documents(vec![doc.clone(); 5])?;
  writer.flush()?;
  writer.add_documents(vec![doc.clone(); 5])?;
  writer.flush()?;

  let reader = writer.get_reader(false, false)?;
  let searcher = IndexSearcher::from_cr(reader)?;

  let manager = TopScoreDocCollectorManager::new(2, 10)?;
  let top_docs = searcher
    .search_with_collector_manager(TermQuery::new(Term::from_text("f", "foo")), &manager)?;
  assert_eq!(10, top_docs.total_hits().value());
  assert_eq!(EqualTo, top_docs.total_hits().relation());

  let manager = TopScoreDocCollectorManager::new(2, 2)?;
  let top_docs = searcher
    .search_with_collector_manager(TermQuery::new(Term::from_text("f", "foo")), &manager)?;
  assert!(10 >= top_docs.total_hits().value());
  assert_eq!(GreaterThanOrEqualTo, top_docs.total_hits().relation());

  let manager = TopScoreDocCollectorManager::new(10, 2)?;
  let top_docs = searcher
    .search_with_collector_manager(TermQuery::new(Term::from_text("f", "foo")), &manager)?;
  assert_eq!(10, top_docs.total_hits().value());
  assert_eq!(EqualTo, top_docs.total_hits().relation());

  writer.close()?;
  Ok(())
}

#[test]
fn test_concurrent_min_score() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(NoMergePolicy::default());
  let w = IndexWriter::new(dir.clone(), iwc)?;

  let doc = Document::new();

  w.add_documents(vec![doc.clone(); 5])?;
  w.flush()?;
  w.add_documents(vec![doc.clone(); 6])?;
  w.flush()?;
  w.add_documents(vec![doc.clone(); 2])?;
  w.flush()?;

  let reader = directory_reader::open_from_writer(&w)?;
  let reader = get_context(reader)?;
  assert_eq!(3, reader.leaves()?.len());
  w.close()?;

  // TopScoreDocCollector — no sort; just score descending, then doc
  DEFAULT_INTERVAL.store(0, Ordering::Relaxed);
  let manager = TopScoreDocCollectorManager::new(2, 0)?;
  let mut collector = manager.new_collector()?;
  let mut collector2 = manager.new_collector()?;

  // both collectors share same MaxScoreAccumulator
  assert!(Arc::ptr_eq(
    collector.min_score_acc.as_ref().unwrap(),
    collector2.min_score_acc.as_ref().unwrap()
  ));
  let min_value_checker = collector.min_score_acc.clone().unwrap();
  assert_eq!(min_value_checker.mod_interval, 0);

  let mut scorer = Score::new();
  let mut scorer2 = Score::new();

  let leaves = reader.leaves()?;
  let dummy_weight = DummyWeight::<LeafReaderContext<_>>::new(leaves[0].reader().clone());

  let mut leaf_collector = collector.get_leaf_collector(&leaves[0], Some(&dummy_weight))?;
  leaf_collector.set_scorer(&mut scorer)?;

  let mut leaf_collector2 = collector2.get_leaf_collector(&leaves[1], Some(&dummy_weight))?;
  leaf_collector2.set_scorer(&mut scorer2)?;

  scorer.score = 3.0;
  leaf_collector.collect(0, &mut scorer)?;
  assert_eq!(i64::MIN, min_value_checker.get_raw());
  assert!(scorer.min_competitive_score.is_none());

  scorer2.score = 6.0;
  leaf_collector2.collect(0, &mut scorer2)?;
  assert_eq!(i64::MIN, min_value_checker.get_raw());
  assert!(scorer2.min_competitive_score.is_none());

  scorer.score = 2.0;
  leaf_collector.collect(1, &mut scorer)?;
  assert_eq!(i64::MIN, min_value_checker.get_raw());
  assert!(scorer.min_competitive_score.is_none());

  scorer2.score = 9.0;
  leaf_collector2.collect(1, &mut scorer2)?;
  assert_eq!(i64::MIN, min_value_checker.get_raw());
  assert!(scorer2.min_competitive_score.is_none());

  scorer2.score = 7.0;
  leaf_collector2.collect(2, &mut scorer2)?;
  assert!((MaxScoreAccumulator::to_score(min_value_checker.get_raw()) - 7.0).abs() < f32::EPSILON);
  assert!(scorer.min_competitive_score.is_none());
  assert!((7f32.next_up() - scorer2.min_competitive_score.unwrap()).abs() < f32::EPSILON);

  scorer2.score = 1.0;
  leaf_collector2.collect(3, &mut scorer2)?;
  assert!((MaxScoreAccumulator::to_score(min_value_checker.get_raw()) - 7.0).abs() < f32::EPSILON);
  assert!(scorer.min_competitive_score.is_none());
  assert!((7f32.next_up() - scorer2.min_competitive_score.unwrap()).abs() < f32::EPSILON);

  scorer.score = 10.0;
  leaf_collector.collect(2, &mut scorer)?;
  assert!((MaxScoreAccumulator::to_score(min_value_checker.get_raw()) - 7.0).abs() < f32::EPSILON);
  assert!((7.0 - scorer.min_competitive_score.unwrap()).abs() < f32::EPSILON);
  assert!((7f32.next_up() - scorer2.min_competitive_score.unwrap()).abs() < f32::EPSILON);

  scorer.score = 11.0;
  leaf_collector.collect(3, &mut scorer)?;
  assert!((MaxScoreAccumulator::to_score(min_value_checker.get_raw()) - 10.0).abs() < f32::EPSILON);
  assert!((10f32.next_up() - scorer.min_competitive_score.unwrap()).abs() < f32::EPSILON);
  assert!((7f32.next_up() - scorer2.min_competitive_score.unwrap()).abs() < f32::EPSILON);

  let mut collector3 = manager.new_collector()?;
  let mut leaf_collector3 = collector3.get_leaf_collector(&leaves[2], Some(&dummy_weight))?;
  let mut scorer3 = Score::new();
  leaf_collector3.set_scorer(&mut scorer3)?;
  assert!((10f32.next_up() - scorer3.min_competitive_score.unwrap()).abs() < f32::EPSILON);

  scorer3.score = 1.0;
  leaf_collector3.collect(0, &mut scorer3)?;
  assert!((MaxScoreAccumulator::to_score(min_value_checker.get_raw()) - 10.0).abs() < f32::EPSILON);
  assert!((10f32.next_up() - scorer3.min_competitive_score.unwrap()).abs() < f32::EPSILON);

  scorer.score = 11.0;
  leaf_collector.collect(4, &mut scorer)?;
  assert!((MaxScoreAccumulator::to_score(min_value_checker.get_raw()) - 11.0).abs() < f32::EPSILON);
  assert!((11f32.next_up() - scorer.min_competitive_score.unwrap()).abs() < f32::EPSILON);
  assert!((7f32.next_up() - scorer2.min_competitive_score.unwrap()).abs() < f32::EPSILON);
  assert!((10f32.next_up() - scorer3.min_competitive_score.unwrap()).abs() < f32::EPSILON);

  scorer3.score = 2.0;
  leaf_collector3.collect(1, &mut scorer3)?;
  assert!((MaxScoreAccumulator::to_score(min_value_checker.get_raw()) - 11.0).abs() < f32::EPSILON);
  assert!((11f32.next_up() - scorer.min_competitive_score.unwrap()).abs() < f32::EPSILON);
  assert!((7f32.next_up() - scorer2.min_competitive_score.unwrap()).abs() < f32::EPSILON);
  assert!((11f32.next_up() - scorer3.min_competitive_score.unwrap()).abs() < f32::EPSILON);

  let top_docs = manager.reduce(vec![collector, collector2, collector3])?;
  assert_eq!(11, top_docs.total_hits().value());
  assert_eq!(
    TotalHits::new(11, GreaterThanOrEqualTo),
    *top_docs.total_hits()
  );

  Ok(())
}

#[test]
fn test_random_min_competitive_score() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone())?;

  let num_docs = at_least(&mut random, 1000);
  for _ in 0..num_docs {
    let num_as = 1 + random.random_range(0..5);
    let num_bs = if random.random::<f32>() < 0.5 {
      0
    } else {
      1 + random.random_range(0..5)
    };
    let num_cs = if random.random::<f32>() < 0.1 {
      0
    } else {
      1 + random.random_range(0..5)
    };

    let mut doc = Document::new();
    for _ in 0..num_as {
      doc.add(StringField::from_string("f", "A", Store::No)?);
    }
    for _ in 0..num_bs {
      doc.add(StringField::from_string("f", "B", Store::No)?);
    }
    for _ in 0..num_cs {
      doc.add(StringField::from_string("f", "C", Store::No)?);
    }
    w.add_document(&mut random, doc)?;
  }

  let index_reader = Arc::new(w.get_reader(&mut random)?);
  w.close(&mut random)?;

  let queries: Vec<Query> = vec![
    TermQuery::new(Term::from_text("f", "A")).into(),
    TermQuery::new(Term::from_text("f", "B")).into(),
    TermQuery::new(Term::from_text("f", "C")).into(),
    {
      let mut b = Builder::new();
      b.add(TermQuery::new(Term::from_text("f", "A")), Occur::Must)?;
      b.add(TermQuery::new(Term::from_text("f", "B")), Occur::Should)?;
      b.build().into()
    },
  ];

  for query in queries {
    let tdc =
      do_concurrent_search_with_threshold(&mut random, 5, 0, query.clone(), index_reader.clone())?;
    let tdc2 = do_search_with_threshold(&mut random, 5, 0, query.clone(), index_reader.clone())?;

    assert!(tdc.total_hits.value() > 0);
    assert!(tdc2.total_hits.value() > 0);
    CheckHits::check_equal(&query, &tdc.score_docs, &tdc2.score_docs)?;
  }

  Ok(())
}
#[test]
fn test_realistic_concurrent_minimum_score() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  {
    let mut line_docs = LineFileDocs::new(&mut random)?;
    let num_docs = at_least(&mut random, 100);
    for _ in 0..num_docs {
      writer.add_document(&mut random, line_docs.next_doc()?)?;
    }
  }

  let index_reader = Arc::new(writer.get_reader(&mut random)?);
  writer.close(&mut random)?;

  let terms = get_terms(index_reader.clone(), "body")?
    .ok_or_else(|| LuceneError::illegal_state("no terms for field 'body'"))?;

  let mut term_count = 0;
  {
    let mut terms_enum = terms.iterator()?;
    while terms_enum.next()?.is_some() {
      term_count += 1;
    }
  }
  assert!(term_count > 0);

  let chance = 10.0 / term_count as f64;
  let mut terms_enum = terms.iterator()?;
  while let Some(term) = terms_enum.next()? {
    if random.random::<f64>() <= chance {
      let term_bytes = BytesRef::deep_copy_of(&*term);
      let query: Query = TermQuery::new(Term::new("body", term_bytes)).into();

      let tdc = do_concurrent_search_with_threshold(
        &mut random,
        5,
        0,
        query.clone(),
        index_reader.clone(),
      )?;
      let tdc2 = do_search_with_threshold(&mut random, 5, 0, query.clone(), index_reader.clone())?;

      CheckHits::check_equal(&query, &tdc.score_docs, &tdc2.score_docs)?;
    }
  }

  Ok(())
}
