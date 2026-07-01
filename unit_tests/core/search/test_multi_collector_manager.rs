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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::dummy::dummy_weight::DummyWeight;
use crate::core::search::leaf_collector::{LeafCollector, LeafCollectorEnum2};
use crate::core::search::multi_collector_manager::MultiCollectorManager;
use crate::core::search::scorable::Scorable;
use crate::core::search::score::Score;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
use crate::test::support::core::util::lucene_test_case::{new_directory_shared, random};
use crate::test::support::core::util::test_util::TestUtil;
use rand::RngExt;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

#[allow(dead_code)] // for quick search
struct TestMultiCollectorManager;

#[test]
fn test_collection() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  writer.add_document(&mut random, Document::new())?;
  writer.commit(&mut random)?;
  let reader = Rc::new(writer.get_reader(&mut random)?);
  let ctx = get_context(reader)?;
  let leaves = ctx.leaves()?;

  // Setup two collector managers, one that will only collect even doc ids and one that
  // only collects odd. Create some random doc ids and keep track of the ones that we
  // expect each collector manager to collect:
  let even_predicate = Predicate::Even;
  let odd_predicate = Predicate::Odd;

  let cm1 = SimpleCollectorManager::new(even_predicate);
  let cm2 = SimpleCollectorManager::new(odd_predicate);
  for _ in 0..100 {
    let docs = TestUtil::next_int(&mut random, 1000, 10000);
    let expected = generate_doc_ids(docs, &mut random);
    let expected_even: Vec<i32> = expected
      .iter()
      .copied()
      .filter(|doc| even_predicate.test(*doc))
      .collect();
    let expected_odd: Vec<i32> = expected
      .iter()
      .copied()
      .filter(|doc| odd_predicate.test(*doc))
      .collect();

    // Test only wrapping one of the collector managers:
    let mcm = MultiCollectorManager::new(vec![&cm1])?;
    let results = collect_all(&leaves[0], &expected, &mcm)?;
    assert_eq!(1, results.len());
    assert_eq!(expected_even, results[0]);

    // Test wrapping both collector managers:
    let mcm = MultiCollectorManager::new(vec![&cm1, &cm2])?;
    let results = collect_all(&leaves[0], &expected, &mcm)?;
    assert_eq!(2, results.len());
    assert_eq!(expected_even, results[0]);
    assert_eq!(expected_odd, results[1]);
  }
  Ok(())
}

#[test]
fn test_null_collector_managers() -> Result<()> {
  test_not_required_in_rust_lucene!();
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
    let mut collector = ExpectedScorerCollector {
      score_mode: ScoreMode::CompleteNoScores,
      expected_scorer: ExpectedScorer::ScoreCachingWrappingScorer,
    };
    let leaf_collector = collector
      .get_leaf_collector(&leaves[0], Some(&dummy_weight))
      .unwrap();
    let mut scorer = Score::new(0.0);
    leaf_collector.set_scorer(&mut scorer).unwrap();
  }));
  assert!(result.is_err());

  // no collector needs scores => no caching
  let cm1 = collector_manager(ScoreMode::CompleteNoScores, ExpectedScorer::Score);
  let cm2 = collector_manager(ScoreMode::CompleteNoScores, ExpectedScorer::Score);
  let mut collector = MultiCollectorManager::new(vec![&cm1, &cm2])?.new_collector()?;
  collector
    .get_leaf_collector(&leaves[0], Some(&dummy_weight))?
    .set_scorer(&mut Score::new(0.0))?;

  // only one collector needs scores => no caching
  let cm1 = collector_manager(ScoreMode::Complete, ExpectedScorer::Score);
  let cm2 = collector_manager(ScoreMode::CompleteNoScores, ExpectedScorer::Score);
  let mut collector = MultiCollectorManager::new(vec![&cm1, &cm2])?.new_collector()?;
  collector
    .get_leaf_collector(&leaves[0], Some(&dummy_weight))?
    .set_scorer(&mut Score::new(0.0))?;

  // several collectors need scores => caching
  let cm1 = collector_manager(
    ScoreMode::Complete,
    ExpectedScorer::ScoreCachingWrappingScorer,
  );
  let cm2 = collector_manager(
    ScoreMode::Complete,
    ExpectedScorer::ScoreCachingWrappingScorer,
  );
  let mut collector = MultiCollectorManager::new(vec![&cm1, &cm2])?.new_collector()?;
  collector
    .get_leaf_collector(&leaves[0], Some(&dummy_weight))?
    .set_scorer(&mut Score::new(0.0))?;
  Ok(())
}

#[test]
fn test_score_wrapping() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  writer.add_document(&mut random, Document::new())?;
  writer.commit(&mut random)?;
  let reader = Rc::new(writer.get_reader(&mut random)?);
  let ctx = get_context(reader)?;
  let leaves = ctx.leaves()?;
  let dummy_weight = DummyWeight::<LeafReaderContext<_>>::new(leaves[0].reader().clone());

  // all wrapped collector managers are TOP_SCORE score mode, so they should see a
  // MinCompetitiveScoreAwareScorable passed in as their scorer:
  let cm1 = collector_manager(
    ScoreMode::TopScores,
    ExpectedScorer::MinCompetitiveScoreAwareScorable,
  );
  let cm2 = collector_manager(
    ScoreMode::TopScores,
    ExpectedScorer::MinCompetitiveScoreAwareScorable,
  );
  let mut collector = MultiCollectorManager::new(vec![&cm1, &cm2])?.new_collector()?;
  collector
    .get_leaf_collector(&leaves[0], Some(&dummy_weight))?
    .set_scorer(&mut Score::new(0.0))?;

  // both wrapped collector managers need scores, but one is exhaustive, so they should
  // see a ScoreCachingWrappingScorer pass in as their scorer:
  let cm1 = collector_manager(
    ScoreMode::Complete,
    ExpectedScorer::ScoreCachingWrappingScorer,
  );
  let cm2 = collector_manager(
    ScoreMode::TopScores,
    ExpectedScorer::ScoreCachingWrappingScorer,
  );
  let mut collector = MultiCollectorManager::new(vec![&cm1, &cm2])?.new_collector()?;
  collector
    .get_leaf_collector(&leaves[0], Some(&dummy_weight))?
    .set_scorer(&mut Score::new(0.0))?;
  Ok(())
}

#[test]
fn test_early_termination() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  writer.add_document(&mut random, Document::new())?;
  writer.commit(&mut random)?;
  let reader = Rc::new(writer.get_reader(&mut random)?);
  let ctx = get_context(reader)?;
  let leaves = ctx.leaves()?;

  let docs = TestUtil::next_int(&mut random, 1000, 10000);
  let expected = generate_doc_ids(docs, &mut random);

  // The first collector manager should collect all docs even though the second returns
  // collection-terminated error immediately:
  let cm1 = CollectorManagerEnum::Simple(SimpleCollectorManager::default());
  let cm2 = CollectorManagerEnum::Terminating(TerminatingCollectorManager);
  let mcm = MultiCollectorManager::new(vec![&cm1, &cm2])?;
  let results = collect_all(&leaves[0], &expected, &mcm)?;
  assert_eq!(2, results.len());
  assert_eq!(Some(expected.iter().copied().collect()), results[0]);
  assert_eq!(None, results[1]);

  // If multiple wrapped collector managers return collection-terminated errors, the
  // error should be returned by the MultiCollectorManager's collector:
  let cm2 = CollectorManagerEnum::Terminating(TerminatingCollectorManager);
  let cm3 = CollectorManagerEnum::Terminating(TerminatingCollectorManager);
  let mcm = MultiCollectorManager::new(vec![&cm2, &cm3])?;
  assert!(matches!(
    collect_all(&leaves[0], &expected, &mcm),
    Err(LuceneError::CollectionTerminated(_))
  ));
  Ok(())
}

fn collect_all<CM, LR>(
  ctx: &LeafReaderContext<LR>,
  values: &BTreeSet<i32>,
  collector_manager: &CM,
) -> Result<CM::T>
where
  CM: CollectorManager,
  LR: LeafReader + Clone,
{
  let mut random = random();
  let dummy_weight = DummyWeight::<LeafReaderContext<LR>>::new(ctx.reader().clone());
  let mut collectors = vec![collector_manager.new_collector()?];
  for v in values {
    if random.random_range(0..10) == 1 {
      collectors.push(collector_manager.new_collector()?);
    }
    let mut scorer = Score::new(0.0);
    let collector_idx = collectors.len() - 1;
    let mut leaf_collector = collectors[collector_idx]
      .get_leaf_collector::<_, LeafReaderContext<LR>>(ctx, Some(&dummy_weight))?;
    leaf_collector.collect(*v, &mut scorer)?;
  }
  collector_manager.reduce(collectors)
}

/// Generate test doc ids. This will de-dupe and create a sorted collection to be more realistic
/// with real-world use-cases. Note that it's possible this will generate fewer than 'count'
/// entries because of de-duping, but that should be quite rare and probably isn't worth worrying
/// about for these testing purposes.
fn generate_doc_ids(count: i32, random: &mut impl RngExt) -> BTreeSet<i32> {
  let mut generated = BTreeSet::new();
  for _ in 0..count {
    generated.insert(random.random());
  }
  generated
}

#[derive(Clone, Copy)]
enum Predicate {
  All,
  Even,
  Odd,
}

impl Predicate {
  fn test(&self, doc: i32) -> bool {
    match self {
      Self::All => true,
      Self::Even => doc % 2 == 0,
      Self::Odd => doc % 2 != 0,
    }
  }
}

#[derive(Clone, Copy)]
struct SimpleCollectorManager {
  predicate: Predicate,
}

impl Default for SimpleCollectorManager {
  fn default() -> Self {
    Self {
      predicate: Predicate::All,
    }
  }
}

impl SimpleCollectorManager {
  fn new(predicate: Predicate) -> Self {
    Self { predicate }
  }
}

impl CollectorManager for SimpleCollectorManager {
  type C = SimpleListCollector;
  type T = Vec<i32>;

  fn new_collector(&self) -> Result<Self::C> {
    Ok(SimpleListCollector::new(self.predicate))
  }

  fn reduce(&self, collectors: Vec<Self::C>) -> Result<Self::T> {
    let mut all = Vec::new();
    for mut collector in collectors {
      all.append(&mut collector.collected);
    }
    Ok(all)
  }
}

struct SimpleListCollector {
  predicate: Predicate,
  collected: Vec<i32>,
}

impl SimpleListCollector {
  fn new(predicate: Predicate) -> Self {
    Self {
      predicate,
      collected: Vec::new(),
    }
  }
}

impl Collector for SimpleListCollector {
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
    ScoreMode::Complete
  }
}

impl Display for SimpleListCollector {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "SimpleListCollector")
  }
}

impl LeafCollector for SimpleListCollector {
  fn collect(&mut self, doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
    if self.predicate.test(doc) {
      self.collected.push(doc);
    }
    Ok(())
  }
}

struct TerminatingCollectorManager;

impl CollectorManager for TerminatingCollectorManager {
  type C = TerminatingCollector;
  type T = Option<Vec<i32>>;

  fn new_collector(&self) -> Result<Self::C> {
    Ok(TerminatingCollector)
  }

  fn reduce(&self, _collectors: Vec<Self::C>) -> Result<Self::T> {
    Ok(None)
  }
}

struct TerminatingCollector;

impl Collector for TerminatingCollector {
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
    ScoreMode::Complete
  }
}

impl Display for TerminatingCollector {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "TerminatingCollector")
  }
}

impl LeafCollector for TerminatingCollector {
  fn collect(&mut self, _doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
    Err(LuceneError::collection_terminated(""))
  }
}

enum CollectorManagerEnum {
  Simple(SimpleCollectorManager),
  Terminating(TerminatingCollectorManager),
}

impl CollectorManager for CollectorManagerEnum {
  type C = CollectorEnum2<SimpleListCollector, TerminatingCollector>;
  type T = Option<Vec<i32>>;

  fn new_collector(&self) -> Result<Self::C> {
    match self {
      Self::Simple(manager) => manager.new_collector().map(CollectorEnum2::A),
      Self::Terminating(manager) => manager.new_collector().map(CollectorEnum2::B),
    }
  }

  fn reduce(&self, collectors: Vec<Self::C>) -> Result<Self::T> {
    match self {
      Self::Simple(manager) => {
        let mut simple_collectors = Vec::with_capacity(collectors.len());
        for collector in collectors {
          match collector {
            CollectorEnum2::A(collector) => simple_collectors.push(collector),
            CollectorEnum2::B(_) => {
              return Err(LuceneError::illegal_state(
                "expected simple collector while reducing simple manager",
              ));
            },
          }
        }
        manager.reduce(simple_collectors).map(Some)
      },
      Self::Terminating(manager) => {
        let mut terminating_collectors = Vec::with_capacity(collectors.len());
        for collector in collectors {
          match collector {
            CollectorEnum2::A(_) => {
              return Err(LuceneError::illegal_state(
                "expected terminating collector while reducing terminating manager",
              ));
            },
            CollectorEnum2::B(collector) => terminating_collectors.push(collector),
          }
        }
        manager.reduce(terminating_collectors)
      },
    }
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
}

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

struct ExpectedScorerCollectorManager {
  score_mode: ScoreMode,
  expected_scorer: ExpectedScorer,
}

fn collector_manager(
  score_mode: ScoreMode,
  expected_scorer: ExpectedScorer,
) -> ExpectedScorerCollectorManager {
  ExpectedScorerCollectorManager {
    score_mode,
    expected_scorer,
  }
}

impl CollectorManager for ExpectedScorerCollectorManager {
  type C = ExpectedScorerCollector;
  type T = ();

  fn new_collector(&self) -> Result<Self::C> {
    Ok(ExpectedScorerCollector {
      score_mode: self.score_mode,
      expected_scorer: self.expected_scorer,
    })
  }

  fn reduce(&self, _collectors: Vec<Self::C>) -> Result<Self::T> {
    Ok(())
  }
}

struct ExpectedScorerCollector {
  score_mode: ScoreMode,
  expected_scorer: ExpectedScorer,
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
