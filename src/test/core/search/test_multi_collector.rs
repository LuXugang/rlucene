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
use crate::core::index::composite_reader::get_context;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::collector::Collector;
use crate::core::search::dummy::dummy_weight::DummyWeight;
use crate::core::search::leaf_collector::{LeafCollector, LeafCollectorEnum2};
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::multi_collector::{
  MinCompetitiveScoreAwareScorable, OneOrMultiCollector, wrap,
};
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::score::Score;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::dummy_total_hit_count_collector::DummyTotalHitCountCollector;
use crate::test::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_searcher_with_reader, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
use rand::prelude::SliceRandom;
use std::cell::Cell;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[allow(dead_code)] // for quick search
struct TestMultiCollector;

struct TerminateAfterCollector<C> {
  in_: C,
  count: i32,
  terminate_after: i32,
}

impl<C> TerminateAfterCollector<C> {
  fn new(in_: C, terminate_after: i32) -> Self {
    Self {
      in_,
      count: 0,
      terminate_after,
    }
  }
}

impl<C> Collector for TerminateAfterCollector<C>
where
  C: Collector,
{
  type LeafCollector<'a, IRC>
    = TerminateAfterLeafCollector<'a, C::LeafCollector<'a, IRC>>
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
    let Self {
      in_,
      count,
      terminate_after,
    } = self;
    if *count >= *terminate_after {
      return Err(LuceneError::collection_terminated(""));
    }
    let leaf_collector = in_.get_leaf_collector(context, weight)?;
    Ok(TerminateAfterLeafCollector::new(
      leaf_collector,
      count,
      *terminate_after,
    ))
  }

  fn score_mode(&self) -> ScoreMode {
    self.in_.score_mode()
  }
}

impl<C> Display for TerminateAfterCollector<C> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "TerminateAfterCollector")
  }
}

struct TerminateAfterLeafCollector<'a, LC> {
  in_: LC,
  count: &'a mut i32,
  terminate_after: i32,
}

impl<'a, LC> TerminateAfterLeafCollector<'a, LC> {
  fn new(in_: LC, count: &'a mut i32, terminate_after: i32) -> Self {
    Self {
      in_,
      count,
      terminate_after,
    }
  }
}

impl<LC> Display for TerminateAfterLeafCollector<'_, LC>
where
  LC: LeafCollector,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "TerminateAfterLeafCollector")
  }
}

impl<LC> LeafCollector for TerminateAfterLeafCollector<'_, LC>
where
  LC: LeafCollector,
{
  fn set_scorer(&mut self, scorer: &mut dyn Scorable) -> Result<()> {
    self.in_.set_scorer(scorer)
  }

  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    if *self.count >= self.terminate_after {
      return Err(LuceneError::collection_terminated(""));
    }
    self.in_.collect(doc, scorer)?;
    *self.count += 1;
    Ok(())
  }

  fn finish(&mut self) -> Result<()> {
    self.in_.finish()
  }
}

struct SetScorerLeafCollector<LC> {
  in_: LC,
  set_scorer_called: Arc<AtomicBool>,
}

impl<LC> SetScorerLeafCollector<LC> {
  fn new(in_: LC, set_scorer_called: Arc<AtomicBool>) -> Self {
    Self {
      in_,
      set_scorer_called,
    }
  }
}

impl<LC> Display for SetScorerLeafCollector<LC>
where
  LC: LeafCollector,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "SetScorerLeafCollector")
  }
}

impl<LC> LeafCollector for SetScorerLeafCollector<LC>
where
  LC: LeafCollector,
{
  fn set_scorer(&mut self, scorer: &mut dyn Scorable) -> Result<()> {
    self.in_.set_scorer(scorer)?;
    self.set_scorer_called.store(true, Ordering::SeqCst);
    Ok(())
  }

  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    self.in_.collect(doc, scorer)
  }

  fn finish(&mut self) -> Result<()> {
    self.in_.finish()
  }
}

enum CollectorEnum2<A, B> {
  A(A),
  B(B),
}

impl<A, B> Collector for CollectorEnum2<A, B>
where
  A: Collector,
  B: Collector,
{
  type LeafCollector<'a, IRC>
    = LeafCollectorEnum2<A::LeafCollector<'a, IRC>, B::LeafCollector<'a, IRC>>
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
    match self {
      Self::A(collector) => collector
        .get_leaf_collector(context, weight)
        .map(LeafCollectorEnum2::A),
      Self::B(collector) => collector
        .get_leaf_collector(context, weight)
        .map(LeafCollectorEnum2::B),
    }
  }

  fn score_mode(&self) -> ScoreMode {
    match self {
      Self::A(collector) => collector.score_mode(),
      Self::B(collector) => collector.score_mode(),
    }
  }

  fn set_weight<W, IRC>(&self, weight: Option<&W>) -> Result<()>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    match self {
      Self::A(collector) => collector.set_weight(weight),
      Self::B(collector) => collector.set_weight(weight),
    }
  }
}

struct SetMinScoreCollector {
  min_score: f32,
}

impl SetMinScoreCollector {
  fn new() -> Self {
    Self { min_score: 0.0 }
  }
}

impl Collector for SetMinScoreCollector {
  type LeafCollector<'a, IRC>
    = &'a mut Self
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    Ok(self)
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreMode::TopScores
  }
}

impl Display for SetMinScoreCollector {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "SetMinScoreCollector")
  }
}

impl LeafCollector for SetMinScoreCollector {
  fn set_scorer(&mut self, _scorer: &mut dyn Scorable) -> Result<()> {
    Ok(())
  }

  fn collect(&mut self, _doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    self.min_score = self.min_score.next_up();
    scorer.set_min_competitive_score(self.min_score)
  }
}

struct SetScorerCollector<C> {
  in_: C,
  set_scorer_called: Arc<AtomicBool>,
}

impl<C> SetScorerCollector<C> {
  fn new(in_: C, set_scorer_called: Arc<AtomicBool>) -> Self {
    Self {
      in_,
      set_scorer_called,
    }
  }
}

impl<C> Collector for SetScorerCollector<C>
where
  C: Collector,
{
  type LeafCollector<'a, IRC>
    = SetScorerLeafCollector<C::LeafCollector<'a, IRC>>
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
    let leaf_collector = self.in_.get_leaf_collector(context, weight)?;
    Ok(SetScorerLeafCollector::new(
      leaf_collector,
      self.set_scorer_called.clone(),
    ))
  }

  fn score_mode(&self) -> ScoreMode {
    self.in_.score_mode()
  }
}

impl<C> Display for SetScorerCollector<C> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "SetScorerCollector")
  }
}

struct PanicOnMinCompetitiveScoreScorable;

impl Scorable for PanicOnMinCompetitiveScoreScorable {
  fn score(&mut self) -> Result<f32> {
    Ok(0.0)
  }

  fn set_min_competitive_score(&mut self, _min_score: f32) -> Result<()> {
    panic!("set_min_competitive_score must not be called")
  }
}

impl FixedScore for PanicOnMinCompetitiveScoreScorable {}

struct MinCompetitiveScoreScorable {
  min_competitive_score: f32,
}

impl MinCompetitiveScoreScorable {
  fn new() -> Self {
    Self {
      min_competitive_score: 0.0,
    }
  }
}

impl Scorable for MinCompetitiveScoreScorable {
  fn score(&mut self) -> Result<f32> {
    Ok(0.0)
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    self.min_competitive_score = min_score;
    Ok(())
  }
}

impl FixedScore for MinCompetitiveScoreScorable {}

#[derive(Clone, Copy)]
enum ExpectedScorer {
  Score,
  ScoreCachingWrappingScorer,
  MinCompetitiveScoreAwareScorable,
}

impl ExpectedScorer {
  fn matches(&self, type_name: &str) -> bool {
    match self {
      Self::Score => type_name == std::any::type_name::<Score>(),
      Self::ScoreCachingWrappingScorer => type_name.contains("ScoreCachingWrappingScorer"),
      Self::MinCompetitiveScoreAwareScorable => {
        type_name.contains("MinCompetitiveScoreAwareScorable")
      },
    }
  }
}

struct ExpectedScorerCollector {
  score_mode: ScoreMode,
  expected_scorer: ExpectedScorer,
}

fn collector(score_mode: ScoreMode, expected_scorer: ExpectedScorer) -> ExpectedScorerCollector {
  ExpectedScorerCollector {
    score_mode,
    expected_scorer,
  }
}

impl Collector for ExpectedScorerCollector {
  type LeafCollector<'a, IRC>
    = &'a mut Self
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    Ok(self)
  }

  fn score_mode(&self) -> ScoreMode {
    self.score_mode
  }
}

impl Display for ExpectedScorerCollector {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "ExpectedScorerCollector")
  }
}

impl LeafCollector for ExpectedScorerCollector {
  fn set_scorer(&mut self, scorer: &mut dyn Scorable) -> Result<()> {
    let type_name = scorer.scorable_test_type_name();
    assert!(
      self.expected_scorer.matches(type_name),
      "unexpected scorer type {type_name}"
    );
    Ok(())
  }

  fn collect(&mut self, _doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
    Ok(())
  }
}

struct DummyCollector {
  collect_called: Rc<Cell<bool>>,
  set_next_reader_called: Rc<Cell<bool>>,
  set_scorer_called: Rc<Cell<bool>>,
}

impl DummyCollector {
  fn new() -> Self {
    Self {
      collect_called: Rc::new(Cell::new(false)),
      set_next_reader_called: Rc::new(Cell::new(false)),
      set_scorer_called: Rc::new(Cell::new(false)),
    }
  }
}

impl Collector for DummyCollector {
  type LeafCollector<'a, IRC>
    = &'a mut Self
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    self.set_next_reader_called.set(true);
    Ok(self)
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreMode::Complete
  }
}

impl Display for DummyCollector {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "DummyCollector")
  }
}

impl LeafCollector for DummyCollector {
  fn set_scorer(&mut self, _scorer: &mut dyn Scorable) -> Result<()> {
    self.set_scorer_called.set(true);
    Ok(())
  }

  fn collect(&mut self, _doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
    self.collect_called.set(true);
    Ok(())
  }
}

struct TerminatingDummyCollector {
  base: DummyCollector,
  terminate_on_doc: i32,
  score_mode: ScoreMode,
}

impl TerminatingDummyCollector {
  fn new(terminate_on_doc: i32, score_mode: ScoreMode) -> Self {
    Self {
      base: DummyCollector::new(),
      terminate_on_doc,
      score_mode,
    }
  }
}

impl Collector for TerminatingDummyCollector {
  type LeafCollector<'a, IRC>
    = &'a mut Self
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    self.base.set_next_reader_called.set(true);
    Ok(self)
  }

  fn score_mode(&self) -> ScoreMode {
    self.score_mode
  }
}

impl Display for TerminatingDummyCollector {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "TerminatingDummyCollector")
  }
}

impl LeafCollector for TerminatingDummyCollector {
  fn set_scorer(&mut self, scorer: &mut dyn Scorable) -> Result<()> {
    self.base.set_scorer(scorer)
  }

  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    if doc == self.terminate_on_doc {
      return Err(LuceneError::collection_terminated(""));
    }
    self.base.collect(doc, scorer)
  }
}

#[test]
fn test_null_collectors() -> Result<()> {
  // Tests that the collector rejects all None collectors.
  assert!(matches!(
    wrap::<DummyCollector>(vec![None, None]),
    Err(LuceneError::IllegalArgument(_))
  ));

  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  writer.add_document(&mut random, Document::new())?;
  let reader = Rc::new(writer.get_reader(&mut random)?);
  let ctx = get_context(reader)?;
  let leaves = ctx.leaves()?;
  let dummy_weight = DummyWeight::<LeafReaderContext<_>>::new(leaves[0].reader().clone());

  // Tests that the collector handles some None collectors well. If it
  // does not, the test would fail on an absent collector.
  let mut c = wrap(vec![
    Some(DummyCollector::new()),
    None,
    Some(DummyCollector::new()),
  ])?;
  assert!(matches!(c, OneOrMultiCollector::Multi(_)));
  let mut scorer = Score::new(0.0);
  let mut ac = c.get_leaf_collector(&leaves[0], Some(&dummy_weight))?;
  ac.collect(1, &mut scorer)?;
  drop(ac);
  c.get_leaf_collector(&leaves[0], Some(&dummy_weight))?;
  c.get_leaf_collector(&leaves[0], Some(&dummy_weight))?
    .set_scorer(&mut scorer)?;
  Ok(())
}

#[test]
fn test_single_collector() -> Result<()> {
  // Tests that if a single Collector is input, it is returned (and not MultiCollector).
  let dc = DummyCollector::new();
  assert!(matches!(wrap(vec![Some(dc)])?, OneOrMultiCollector::One(_)));
  let dc = DummyCollector::new();
  assert!(matches!(
    wrap(vec![Some(dc), None])?,
    OneOrMultiCollector::One(_)
  ));
  Ok(())
}

#[test]
fn test_collector() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  writer.add_document(&mut random, Document::new())?;
  let reader = Rc::new(writer.get_reader(&mut random)?);
  let ctx = get_context(reader)?;
  let leaves = ctx.leaves()?;
  let dummy_weight = DummyWeight::<LeafReaderContext<_>>::new(leaves[0].reader().clone());

  // Tests that the collector delegates calls to input collectors properly.

  // Tests that the collector handles some None collectors well. If it
  // does not, the test would fail on an absent collector.
  let mut c = wrap(vec![
    Some(DummyCollector::new()),
    Some(DummyCollector::new()),
  ])?;
  let mut scorer = Score::new(0.0);
  let mut ac = c.get_leaf_collector(&leaves[0], Some(&dummy_weight))?;
  ac.collect(1, &mut scorer)?;
  drop(ac);
  let mut ac = c.get_leaf_collector(&leaves[0], Some(&dummy_weight))?;
  ac.set_scorer(&mut scorer)?;
  drop(ac);

  match c {
    OneOrMultiCollector::One(_) => panic!("expected MultiCollector"),
    OneOrMultiCollector::Multi(ref c) => {
      for dc in c.get_collectors() {
        assert!(dc.collect_called.get());
        assert!(dc.set_next_reader_called.get());
        assert!(dc.set_scorer_called.get());
      }
    },
  }
  Ok(())
}
#[test]
fn test_cache_scores_if_necessary() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  writer.add_document(&mut random, Document::new())?;
  writer.commit(&mut random)?;
  let reader = Rc::new(writer.get_reader(&mut random)?);
  let ctx = get_context(reader)?;
  let leaves = ctx.leaves()?;
  let dummy_weight = DummyWeight::<LeafReaderContext<_>>::new(leaves[0].reader().clone());

  let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    let mut collector = collector(
      ScoreMode::CompleteNoScores,
      ExpectedScorer::ScoreCachingWrappingScorer,
    );
    let leaf_collector = collector
      .get_leaf_collector(&leaves[0], Some(&dummy_weight))
      .unwrap();
    let mut scorer = Score::new(0.0);
    leaf_collector.set_scorer(&mut scorer).unwrap();
  }));
  assert!(result.is_err());

  // no collector needs scores => no caching
  let c1 = collector(ScoreMode::CompleteNoScores, ExpectedScorer::Score);
  let c2 = collector(ScoreMode::CompleteNoScores, ExpectedScorer::Score);
  let mut multi_collector = wrap(vec![Some(c1), Some(c2)])?;
  multi_collector
    .get_leaf_collector(&leaves[0], Some(&dummy_weight))?
    .set_scorer(&mut Score::new(0.0))?;

  // only one collector needs scores => no caching
  let c1 = collector(ScoreMode::Complete, ExpectedScorer::Score);
  let c2 = collector(ScoreMode::CompleteNoScores, ExpectedScorer::Score);
  let mut multi_collector = wrap(vec![Some(c1), Some(c2)])?;
  multi_collector
    .get_leaf_collector(&leaves[0], Some(&dummy_weight))?
    .set_scorer(&mut Score::new(0.0))?;

  // several collectors need scores => caching
  let c1 = collector(
    ScoreMode::Complete,
    ExpectedScorer::ScoreCachingWrappingScorer,
  );
  let c2 = collector(
    ScoreMode::Complete,
    ExpectedScorer::ScoreCachingWrappingScorer,
  );
  let mut multi_collector = wrap(vec![Some(c1), Some(c2)])?;
  multi_collector
    .get_leaf_collector(&leaves[0], Some(&dummy_weight))?
    .set_scorer(&mut Score::new(0.0))?;
  Ok(())
}

#[test]
fn test_collection_terminated_exception_handling() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 3);
  for _ in 0..iters {
    let dir = new_directory_shared(&mut random)?;
    let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
    let num_docs = TestUtil::next_int(&mut random, 100, 1000);
    for _ in 0..num_docs {
      writer.add_document(&mut random, Document::new())?;
    }
    let reader = writer.get_reader(&mut random)?;
    writer.close(&mut random)?;
    let searcher = new_searcher_with_reader(reader)?;
    let mut expected_counts = Vec::new();
    let mut collectors = Vec::new();
    let num_collectors = TestUtil::next_int(&mut random, 1, 5);
    for _ in 0..num_collectors {
      let terminate_after = random.random_range(0..num_docs + 10);
      let expected_count = if terminate_after > num_docs {
        num_docs
      } else {
        terminate_after
      };
      expected_counts.push(expected_count);
      collectors.push(Some(TerminateAfterCollector::new(
        DummyTotalHitCountCollector::new(),
        terminate_after,
      )));
    }
    let mut collector = wrap(collectors)?;
    searcher.search_with_collector(MatchAllDocsQuery::new(), &mut collector)?;
    match collector {
      OneOrMultiCollector::One(collector) => {
        assert_eq!(expected_counts[0], collector.in_.get_total_hits());
      },
      OneOrMultiCollector::Multi(collector) => {
        for (collector, expected_count) in collector.get_collectors().iter().zip(expected_counts) {
          assert_eq!(expected_count, collector.in_.get_total_hits());
        }
      },
    }
  }
  Ok(())
}

#[test]
fn test_set_scorer_after_collection_terminated() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  writer.add_document(&mut random, Document::new())?;
  let reader = Rc::new(writer.get_reader(&mut random)?);
  let ctx = get_context(reader)?;
  let leaves = ctx.leaves()?;
  let dummy_weight = DummyWeight::<LeafReaderContext<_>>::new(leaves[0].reader().clone());

  let set_scorer_called1 = Arc::new(AtomicBool::new(false));
  let collector1 = SetScorerCollector::new(DummyCollector::new(), set_scorer_called1.clone());

  let set_scorer_called2 = Arc::new(AtomicBool::new(false));
  let collector2 = SetScorerCollector::new(DummyCollector::new(), set_scorer_called2.clone());

  let collector1 = TerminateAfterCollector::new(collector1, 1);
  let collector2 = TerminateAfterCollector::new(collector2, 2);

  let mut scorer = Score::new(0.0);
  let mut collector = wrap(vec![Some(collector1), Some(collector2)])?;

  let mut leaf_collector = collector.get_leaf_collector(&leaves[0], Some(&dummy_weight))?;
  leaf_collector.set_scorer(&mut scorer)?;
  assert!(set_scorer_called1.load(Ordering::SeqCst));
  assert!(set_scorer_called2.load(Ordering::SeqCst));

  leaf_collector.collect(0, &mut scorer)?;
  leaf_collector.collect(1, &mut scorer)?;

  set_scorer_called1.store(false, Ordering::SeqCst);
  set_scorer_called2.store(false, Ordering::SeqCst);
  leaf_collector.set_scorer(&mut scorer)?;
  assert!(!set_scorer_called1.load(Ordering::SeqCst));
  assert!(set_scorer_called2.load(Ordering::SeqCst));

  assert!(matches!(
    leaf_collector.collect(1, &mut scorer),
    Err(LuceneError::CollectionTerminated(_))
  ));

  set_scorer_called1.store(false, Ordering::SeqCst);
  set_scorer_called2.store(false, Ordering::SeqCst);
  leaf_collector.set_scorer(&mut scorer)?;
  assert!(!set_scorer_called1.load(Ordering::SeqCst));
  assert!(!set_scorer_called2.load(Ordering::SeqCst));
  Ok(())
}

#[test]
fn test_disables_set_min_score() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  writer.add_document(&mut random, Document::new())?;
  let reader = Rc::new(writer.get_reader(&mut random)?);
  let ctx = get_context(reader)?;
  let leaves = ctx.leaves()?;
  let dummy_weight = DummyWeight::<LeafReaderContext<_>>::new(leaves[0].reader().clone());

  let collector = CollectorEnum2::A(SetMinScoreCollector::new());
  let collector2 = CollectorEnum2::B(DummyTotalHitCountCollector::new());
  let mut multi_collector = wrap(vec![Some(collector), Some(collector2)])?;
  let mut leaf_collector = multi_collector.get_leaf_collector(&leaves[0], Some(&dummy_weight))?;
  let mut scorer = PanicOnMinCompetitiveScoreScorable;
  leaf_collector.set_scorer(&mut scorer)?;
  leaf_collector.collect(0, &mut scorer)?;
  Ok(())
}

#[test]
fn test_disables_set_min_score_with_early_termination() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  writer.add_document(&mut random, Document::new())?;
  let reader = Rc::new(writer.get_reader(&mut random)?);
  let ctx = get_context(reader)?;
  let leaves = ctx.leaves()?;
  let dummy_weight = DummyWeight::<LeafReaderContext<_>>::new(leaves[0].reader().clone());

  for num_col in 1..4 {
    let mut cols = Vec::new();
    cols.push(Some(CollectorEnum2::A(SetMinScoreCollector::new())));
    for _ in 0..num_col {
      cols.push(Some(CollectorEnum2::B(TerminateAfterCollector::new(
        DummyTotalHitCountCollector::new(),
        0,
      ))));
    }
    cols.shuffle(&mut random);
    let mut multi_collector = wrap(cols)?;
    let mut leaf_collector = multi_collector.get_leaf_collector(&leaves[0], Some(&dummy_weight))?;
    let mut scorer = PanicOnMinCompetitiveScoreScorable;
    leaf_collector.set_scorer(&mut scorer)?;
    leaf_collector.collect(0, &mut scorer)?;
  }
  Ok(())
}

#[test]
fn test_scorer_wrapping_for_top_scores() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  writer.add_document(&mut random, Document::new())?;
  let reader = Rc::new(writer.get_reader(&mut random)?);
  let ctx = get_context(reader)?;
  let leaves = ctx.leaves()?;
  let dummy_weight = DummyWeight::<LeafReaderContext<_>>::new(leaves[0].reader().clone());

  let c1 = collector(
    ScoreMode::TopScores,
    ExpectedScorer::MinCompetitiveScoreAwareScorable,
  );
  let c2 = collector(
    ScoreMode::TopScores,
    ExpectedScorer::MinCompetitiveScoreAwareScorable,
  );
  let mut multi_collector = wrap(vec![Some(c1), Some(c2)])?;
  multi_collector
    .get_leaf_collector(&leaves[0], Some(&dummy_weight))?
    .set_scorer(&mut Score::new(0.0))?;

  let c1 = collector(
    ScoreMode::TopScores,
    ExpectedScorer::ScoreCachingWrappingScorer,
  );
  let c2 = collector(
    ScoreMode::Complete,
    ExpectedScorer::ScoreCachingWrappingScorer,
  );
  let mut multi_collector = wrap(vec![Some(c1), Some(c2)])?;
  multi_collector
    .get_leaf_collector(&leaves[0], Some(&dummy_weight))?
    .set_scorer(&mut Score::new(0.0))?;
  Ok(())
}

#[test]
fn test_min_competitive_score() -> Result<()> {
  let mut current_min_scores = [0.0; 3];
  let mut scorer = MinCompetitiveScoreScorable::new();
  assert_eq!(0.0, scorer.min_competitive_score);
  {
    let mut s0 = MinCompetitiveScoreAwareScorable::new(&mut scorer, 0, &mut current_min_scores);
    s0.set_min_competitive_score(0.5)?;
  }
  assert_eq!(0.0, scorer.min_competitive_score);
  {
    let mut s1 = MinCompetitiveScoreAwareScorable::new(&mut scorer, 1, &mut current_min_scores);
    s1.set_min_competitive_score(0.8)?;
  }
  assert_eq!(0.0, scorer.min_competitive_score);
  {
    let mut s2 = MinCompetitiveScoreAwareScorable::new(&mut scorer, 2, &mut current_min_scores);
    s2.set_min_competitive_score(0.3)?;
  }
  assert_eq!(0.3, scorer.min_competitive_score);
  {
    let mut s2 = MinCompetitiveScoreAwareScorable::new(&mut scorer, 2, &mut current_min_scores);
    s2.set_min_competitive_score(0.1)?;
  }
  assert_eq!(0.3, scorer.min_competitive_score);
  {
    let mut s1 = MinCompetitiveScoreAwareScorable::new(&mut scorer, 1, &mut current_min_scores);
    s1.set_min_competitive_score(f32::MAX)?;
  }
  assert_eq!(0.3, scorer.min_competitive_score);
  {
    let mut s2 = MinCompetitiveScoreAwareScorable::new(&mut scorer, 2, &mut current_min_scores);
    s2.set_min_competitive_score(f32::MAX)?;
  }
  assert_eq!(0.5, scorer.min_competitive_score);
  {
    let mut s0 = MinCompetitiveScoreAwareScorable::new(&mut scorer, 0, &mut current_min_scores);
    s0.set_min_competitive_score(f32::MAX)?;
  }
  assert_eq!(f32::MAX, scorer.min_competitive_score);
  Ok(())
}

#[test]
fn test_collection_termination() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  writer.add_document(&mut random, Document::new())?;
  let reader = Rc::new(writer.get_reader(&mut random)?);
  let ctx = get_context(reader)?;
  let leaves = ctx.leaves()?;
  let dummy_weight = DummyWeight::<LeafReaderContext<_>>::new(leaves[0].reader().clone());

  let c1 = TerminatingDummyCollector::new(1, ScoreMode::Complete);
  let c1_collect_called = c1.base.collect_called.clone();
  let c2 = TerminatingDummyCollector::new(2, ScoreMode::Complete);
  let c2_collect_called = c2.base.collect_called.clone();

  let mut mc = wrap(vec![Some(c1), Some(c2)])?;
  let mut lc = mc.get_leaf_collector(&leaves[0], Some(&dummy_weight))?;
  let mut scorer = Score::new(0.0);
  lc.set_scorer(&mut scorer)?;
  lc.collect(0, &mut scorer)?;
  assert!(c1_collect_called.get(), "c1's collect should be called");
  assert!(c2_collect_called.get(), "c2's collect should be called");
  c1_collect_called.set(false);
  c2_collect_called.set(false);
  lc.collect(1, &mut scorer)?;
  assert!(!c1_collect_called.get(), "c1 should be removed already");
  assert!(c2_collect_called.get(), "c2's collect should be called");
  c2_collect_called.set(false);

  assert!(matches!(
    lc.collect(2, &mut scorer),
    Err(LuceneError::CollectionTerminated(_))
  ));
  assert!(!c1_collect_called.get(), "c1 should be removed already");
  assert!(!c2_collect_called.get(), "c2 should be removed already");
  Ok(())
}

#[test]
fn test_set_scorer_on_collection_termination_skip_non_competitive() -> Result<()> {
  do_test_set_scorer_on_collection_termination(true)
}

#[test]
fn test_set_scorer_on_collection_termination_skip_no_skips() -> Result<()> {
  do_test_set_scorer_on_collection_termination(false)
}

fn do_test_set_scorer_on_collection_termination(allow_skip_non_competitive: bool) -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  writer.add_document(&mut random, Document::new())?;
  let reader = Rc::new(writer.get_reader(&mut random)?);
  let ctx = get_context(reader)?;
  let leaves = ctx.leaves()?;
  let dummy_weight = DummyWeight::<LeafReaderContext<_>>::new(leaves[0].reader().clone());

  let score_mode = if allow_skip_non_competitive {
    ScoreMode::TopScores
  } else {
    ScoreMode::Complete
  };
  let c1 = TerminatingDummyCollector::new(1, score_mode);
  let c1_set_scorer_called = c1.base.set_scorer_called.clone();
  let c2 = TerminatingDummyCollector::new(2, score_mode);
  let c2_set_scorer_called = c2.base.set_scorer_called.clone();

  let mut mc = wrap(vec![Some(c1), Some(c2)])?;
  let mut lc = mc.get_leaf_collector(&leaves[0], Some(&dummy_weight))?;
  assert!(!c1_set_scorer_called.get());
  assert!(!c2_set_scorer_called.get());
  let mut scorer = Score::new(0.0);
  lc.set_scorer(&mut scorer)?;
  assert!(c1_set_scorer_called.get());
  assert!(c2_set_scorer_called.get());
  c1_set_scorer_called.set(false);
  c2_set_scorer_called.set(false);
  lc.collect(0, &mut scorer)?;

  lc.set_scorer(&mut scorer)?;
  assert!(c1_set_scorer_called.get());
  assert!(c2_set_scorer_called.get());
  c1_set_scorer_called.set(false);
  c2_set_scorer_called.set(false);

  lc.collect(1, &mut scorer)?;
  lc.set_scorer(&mut scorer)?;
  assert!(!c1_set_scorer_called.get());
  assert!(c2_set_scorer_called.get());
  c2_set_scorer_called.set(false);

  assert!(matches!(
    lc.collect(2, &mut scorer),
    Err(LuceneError::CollectionTerminated(_))
  ));
  lc.set_scorer(&mut scorer)?;
  assert!(!c1_set_scorer_called.get());
  assert!(!c2_set_scorer_called.get());
  Ok(())
}

#[test]
fn test_merge_score_modes() -> Result<()> {
  for sm1 in ScoreMode::values() {
    for sm2 in ScoreMode::values() {
      let c1 = TerminatingDummyCollector::new(0, *sm1);
      let c2 = TerminatingDummyCollector::new(0, *sm2);
      let c = wrap(vec![Some(c1), Some(c2)])?;
      if sm1 == sm2 {
        assert_eq!(*sm1, c.score_mode());
      } else if sm1.needs_scores() || sm2.needs_scores() {
        assert_eq!(ScoreMode::Complete, c.score_mode());
      } else {
        assert_eq!(ScoreMode::CompleteNoScores, c.score_mode());
      }
    }
  }
  Ok(())
}
