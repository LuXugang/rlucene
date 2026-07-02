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
use crate::test_framework::core::util::lucene_test_case::{at_least, random};
use std::any::Any;
use std::collections::HashMap;

use crate::core::index::composite_reader_context::CompositeReaderContext;

use crate::core::index::index_reader_context::IRCLeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;

use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_scorer_supplier::BooleanScorerSupplier;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{AllDISI, DocIdSetIterator};
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer};
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::store::nio_fs_directory::NIOFSDirectory;
use crate::core::store::{FSDirectory, NativeFSLockFactory};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use rand::RngExt;
use rand::prelude::IndexedRandom;

#[allow(dead_code)] // for quick search
struct TestBoolean2ScorerSupplier;
type DummyIRC = CompositeReaderContext<
  StandardDirectoryReaderType<FSDirectory<NativeFSLockFactory, NIOFSDirectory>>,
>;

struct FakeScorer {
  it: AllDISI,
}
impl FakeScorer {
  fn new(cost: i64) -> Self {
    let it = AllDISI::new(cost as i32);
    Self { it }
  }
}

impl Scorable for FakeScorer {
  fn score(&mut self) -> Result<f32> {
    Ok(1f32)
  }

  fn cost(&self) -> Result<i64> {
    self.iterator().cost()
  }
}

impl FixedScore for FakeScorer {}

impl Scorer for FakeScorer {
  fn doc_id(&mut self) -> Result<i32> {
    Ok(self.it.doc_id())
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.it)
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.it)
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let FakeScorer { it, .. } = *self;
    Box::new(it)
  }

  fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
    Ok(1f32)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.it)
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.it)
  }
}
#[derive(Clone)]
struct FakeScorerSupplier {
  cost: i64,
  lead_cost: Option<i64>,
  top_level_scoring_clause: bool,
}
impl FakeScorerSupplier {
  fn with_lead_cost(cost: i64, lead_cost: Option<i64>) -> QueryWeightSs<DummyIRC> {
    let v = Self {
      cost,
      lead_cost,
      top_level_scoring_clause: false,
    };
    Box::new(v)
  }
}
fn new_fake_scorer_supplier(cost: i64) -> QueryWeightSs<DummyIRC> {
  let v = FakeScorerSupplier {
    cost,
    lead_cost: None,
    top_level_scoring_clause: false,
  };
  Box::new(v)
}
impl ScorerSupplier<DummyIRC> for FakeScorerSupplier {
  type Scorer = QueryWeightSsScorer;
  type BulkScorer = QueryWeightSsBulkScorer;

  fn get(
    &mut self,
    lead_cost: i64,
    _context: &LeafReaderContext<IRCLeafReader<DummyIRC>>,
    _searcher: &IndexSearcher<DummyIRC>,
  ) -> Result<Self::Scorer> {
    if let Some(v) = self.lead_cost
      && v != lead_cost
    {
      return Err(LuceneError::illegal_state("triggers assert"));
    }
    Ok(Box::new(FakeScorer::new(self.cost)))
  }

  fn bulk_scorer(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<DummyIRC>>,
    searcher: &IndexSearcher<DummyIRC>,
  ) -> Result<Option<Self::BulkScorer>> {
    Ok(Some(Box::new(self.default_bulk_scorer(context, searcher)?)))
  }

  fn cost(
    &mut self,
    _context: &LeafReaderContext<IRCLeafReader<DummyIRC>>,
    _searcher: &IndexSearcher<DummyIRC>,
  ) -> Result<i64> {
    Ok(self.cost)
  }

  fn set_top_level_scoring_clause(&mut self) -> Result<()> {
    self.top_level_scoring_clause = true;
    Ok(())
  }

  fn as_any(&mut self) -> &mut dyn Any {
    self
  }
}
#[test]
fn test_conjunction_cost() -> Result<()> {
  let mut random = random();

  let mut subs = HashMap::new();
  for occur in [Occur::Should, Occur::Must, Occur::Filter, Occur::MustNot] {
    subs.insert(occur, Vec::new());
  }
  let dummy_searcher = crate::test_framework::core::util::dummy_index_searcher(
    crate::test_framework::core::util::dummy_directory()?,
  )?;
  let dummy_lrc = &dummy_searcher.get_leaf_contexts()?[0];
  subs = {
    let occur = *[Occur::Filter, Occur::Must].choose(&mut random).unwrap();
    subs
      .get_mut(&occur)
      .unwrap()
      .push(new_fake_scorer_supplier(42));

    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 0, 100)?;
    assert_eq!(42, supplier.cost(dummy_lrc, &dummy_searcher)?);
    supplier.subs
  };

  subs = {
    let occur = *[Occur::Filter, Occur::Must].choose(&mut random).unwrap();
    subs
      .get_mut(&occur)
      .unwrap()
      .push(new_fake_scorer_supplier(12));

    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 0, 100)?;
    assert_eq!(12, supplier.cost(dummy_lrc, &dummy_searcher)?);
    supplier.subs
  };

  {
    let occur = *[Occur::Filter, Occur::Must].choose(&mut random).unwrap();
    subs
      .get_mut(&occur)
      .unwrap()
      .push(new_fake_scorer_supplier(20));

    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 0, 100)?;
    assert_eq!(12, supplier.cost(dummy_lrc, &dummy_searcher)?);
  }

  Ok(())
}
#[test]
fn test_disjunction_cost() -> Result<()> {
  let mut random = random();

  let mut subs = HashMap::new();
  for occur in [Occur::Should, Occur::Must, Occur::Filter, Occur::MustNot] {
    subs.insert(occur, Vec::new());
  }

  let dummy_searcher = crate::test_framework::core::util::dummy_index_searcher(
    crate::test_framework::core::util::dummy_directory()?,
  )?;
  let dummy_lrc = &dummy_searcher.get_leaf_contexts()?[0];

  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(new_fake_scorer_supplier(42));
  subs = {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 0, 100)?;
    assert_eq!(42, supplier.cost(dummy_lrc, &dummy_searcher)?);

    let scorer = supplier.get(
      random.random_range(0..100) as i64,
      dummy_lrc,
      &dummy_searcher,
    )?;
    assert_eq!(42, scorer.iterator().cost()?);
    supplier.subs
  };

  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(new_fake_scorer_supplier(12));
  subs = {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 0, 100)?;
    assert_eq!(42 + 12, supplier.cost(dummy_lrc, &dummy_searcher)?);

    let scorer = supplier.get(
      random.random_range(0..100) as i64,
      dummy_lrc,
      &dummy_searcher,
    )?;
    assert_eq!(42 + 12, scorer.iterator().cost()?);
    supplier.subs
  };

  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(new_fake_scorer_supplier(20));
  {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 0, 100)?;
    assert_eq!(42 + 12 + 20, supplier.cost(dummy_lrc, &dummy_searcher)?);

    let scorer = supplier.get(
      random.random_range(0..100) as i64,
      dummy_lrc,
      &dummy_searcher,
    )?;
    assert_eq!(42 + 12 + 20, scorer.iterator().cost()?);
  }

  Ok(())
}
#[test]
fn test_disjunction_with_min_should_match_cost() -> Result<()> {
  let mut random = random();

  let mut subs = HashMap::new();
  for occur in [Occur::Should, Occur::Must, Occur::Filter, Occur::MustNot] {
    subs.insert(occur, Vec::new());
  }

  let dummy_searcher = crate::test_framework::core::util::dummy_index_searcher(
    crate::test_framework::core::util::dummy_directory()?,
  )?;
  let dummy_lrc = &dummy_searcher.get_leaf_contexts()?[0];

  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(new_fake_scorer_supplier(42));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(new_fake_scorer_supplier(12));

  subs = {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 1, 100)?;
    assert_eq!(42 + 12, supplier.cost(dummy_lrc, &dummy_searcher)?);

    let scorer = supplier.get(
      random.random_range(0..100) as i64,
      dummy_lrc,
      &dummy_searcher,
    )?;
    assert_eq!(42 + 12, scorer.iterator().cost()?);
    supplier.subs
  };

  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(new_fake_scorer_supplier(20));

  subs = {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 1, 100)?;
    assert_eq!(42 + 12 + 20, supplier.cost(dummy_lrc, &dummy_searcher)?);

    let scorer = supplier.get(
      random.random_range(0..100) as i64,
      dummy_lrc,
      &dummy_searcher,
    )?;
    assert_eq!(42 + 12 + 20, scorer.iterator().cost()?);
    supplier.subs
  };

  subs = {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 2, 100)?;
    assert_eq!(12 + 20, supplier.cost(dummy_lrc, &dummy_searcher)?);

    let scorer = supplier.get(
      random.random_range(0..100) as i64,
      dummy_lrc,
      &dummy_searcher,
    )?;
    assert_eq!(12 + 20, scorer.iterator().cost()?);
    supplier.subs
  };

  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(new_fake_scorer_supplier(30));

  subs = {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 1, 100)?;
    assert_eq!(
      42 + 12 + 20 + 30,
      supplier.cost(dummy_lrc, &dummy_searcher)?
    );

    let scorer = supplier.get(
      random.random_range(0..100) as i64,
      dummy_lrc,
      &dummy_searcher,
    )?;
    assert_eq!(42 + 12 + 20 + 30, scorer.iterator().cost()?);
    supplier.subs
  };

  subs = {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 2, 100)?;
    assert_eq!(12 + 20 + 30, supplier.cost(dummy_lrc, &dummy_searcher)?);

    let scorer = supplier.get(
      random.random_range(0..100) as i64,
      dummy_lrc,
      &dummy_searcher,
    )?;
    assert_eq!(12 + 20 + 30, scorer.iterator().cost()?);
    supplier.subs
  };

  {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 3, 100)?;
    assert_eq!(12 + 20, supplier.cost(dummy_lrc, &dummy_searcher)?);

    let scorer = supplier.get(
      random.random_range(0..100) as i64,
      dummy_lrc,
      &dummy_searcher,
    )?;
    assert_eq!(12 + 20, scorer.iterator().cost()?);
  }

  Ok(())
}
#[test]
fn test_duel_cost() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 1000);

  let dummy_dir = crate::test_framework::core::util::dummy_directory()?;
  let dummy_searcher = crate::test_framework::core::util::dummy_index_searcher(dummy_dir)?;
  let dummy_lrc = &dummy_searcher.get_leaf_contexts()?[0];

  for _i in 0..iters {
    let mut subs = HashMap::new();
    for occur in [Occur::Should, Occur::Must, Occur::Filter, Occur::MustNot] {
      subs.insert(occur, Vec::new());
    }

    let mut num_shoulds = 0;
    let mut num_required = 0;

    let num_clauses = random.random_range(1..=10);
    for _ in 0..num_clauses {
      let occur = *Occur::values().choose(&mut random).unwrap();
      subs
        .get_mut(&occur)
        .unwrap()
        .push(new_fake_scorer_supplier(random.random_range(0..100)));

      if occur == Occur::Should {
        num_shoulds += 1;
      } else if occur == Occur::Filter || occur == Occur::Must {
        num_required += 1;
      }
    }

    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    if !score_mode.needs_scores() && num_required > 0 {
      num_shoulds = 0;
      subs.get_mut(&Occur::Should).unwrap().clear();
    }

    if num_shoulds + num_required == 0 {
      continue;
    }

    let min_should_match = if num_shoulds == 0 {
      0
    } else {
      random.random_range(0..num_shoulds)
    };

    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, min_should_match, 100)?;

    let cost1 = supplier.cost(dummy_lrc, &dummy_searcher)?;
    let scorer = supplier.get(i64::MAX, dummy_lrc, &dummy_searcher)?;
    let cost2 = scorer.iterator().cost()?;

    assert_eq!(cost1, cost2);
  }

  Ok(())
}

#[test]
fn test_fake_scorer_supplier() -> Result<()> {
  let mut random = random();
  let dummy_searcher = crate::test_framework::core::util::dummy_index_searcher(
    crate::test_framework::core::util::dummy_directory()?,
  )?;
  let dummy_lrc = &dummy_searcher.get_leaf_contexts()?[0];

  let mut random_access_supplier =
    FakeScorerSupplier::with_lead_cost(random.random_range(0..100), Some(30));
  assert!(
    random_access_supplier
      .get(70, dummy_lrc, &dummy_searcher)
      .is_err()
  );

  let mut sequential_supplier =
    FakeScorerSupplier::with_lead_cost(random.random_range(0..100), Some(70));
  assert!(
    sequential_supplier
      .get(30, dummy_lrc, &dummy_searcher)
      .is_err()
  );
  Ok(())
}
#[test]
fn test_conjunction_lead_cost() -> Result<()> {
  let mut random = random();
  let dummy_searcher = crate::test_framework::core::util::dummy_index_searcher(
    crate::test_framework::core::util::dummy_directory()?,
  )?;
  let dummy_lrc = &dummy_searcher.get_leaf_contexts()?[0];

  let mut subs = HashMap::new();
  for occur in [Occur::Should, Occur::Must, Occur::Filter, Occur::MustNot] {
    subs.insert(occur, Vec::new());
  }

  subs
    .get_mut([Occur::Filter, Occur::Must].choose(&mut random).unwrap())
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(42, Some(12)));
  subs
    .get_mut([Occur::Filter, Occur::Must].choose(&mut random).unwrap())
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(12, Some(12)));

  {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 0, 100)?;
    let _ = supplier.get(i64::MAX, dummy_lrc, &dummy_searcher)?;
  }

  let mut subs = HashMap::new();
  for occur in [Occur::Should, Occur::Must, Occur::Filter, Occur::MustNot] {
    subs.insert(occur, Vec::new());
  }

  subs
    .get_mut([Occur::Filter, Occur::Must].choose(&mut random).unwrap())
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(42, Some(7)));
  subs
    .get_mut([Occur::Filter, Occur::Must].choose(&mut random).unwrap())
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(12, Some(7)));

  {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 0, 100)?;
    let _ = supplier.get(7, dummy_lrc, &dummy_searcher)?;
  }

  Ok(())
}
#[test]
fn test_disjunction_lead_cost() -> Result<()> {
  let mut random = random();
  let dummy_searcher = crate::test_framework::core::util::dummy_index_searcher(
    crate::test_framework::core::util::dummy_directory()?,
  )?;
  let dummy_lrc = &dummy_searcher.get_leaf_contexts()?[0];

  let mut subs = HashMap::new();
  for occur in [Occur::Should, Occur::Must, Occur::Filter, Occur::MustNot] {
    subs.insert(occur, Vec::new());
  }

  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(42, Some(54)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(12, Some(54)));

  subs = {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 0, 100)?;
    let _ = supplier.get(100, dummy_lrc, &dummy_searcher)?;
    supplier.subs
  };

  subs.get_mut(&Occur::Should).unwrap().clear();
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(42, Some(20)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(12, Some(20)));

  {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 0, 100)?;
    let _ = supplier.get(20, dummy_lrc, &dummy_searcher)?;
  }

  Ok(())
}
#[test]
fn test_disjunction_with_min_should_match_lead_cost() -> Result<()> {
  let mut random = random();
  let dummy_searcher = crate::test_framework::core::util::dummy_index_searcher(
    crate::test_framework::core::util::dummy_directory()?,
  )?;
  let dummy_lrc = &dummy_searcher.get_leaf_contexts()?[0];

  let mut subs = HashMap::new();
  for occur in [Occur::Should, Occur::Must, Occur::Filter, Occur::MustNot] {
    subs.insert(occur, Vec::new());
  }

  // minShouldMatch is 2 so the 2 least costly clauses will lead iteration
  // and their cost will be 30+12=42
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(50, Some(42)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(12, Some(42)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(30, Some(42)));

  {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 2, 100)?;
    let _ = supplier.get(100, dummy_lrc, &dummy_searcher)?;
  }

  let mut subs = HashMap::new();
  for occur in [Occur::Should, Occur::Must, Occur::Filter, Occur::MustNot] {
    subs.insert(occur, Vec::new());
  }

  // If the leadCost is less than the msm cost, then it wins
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(42, Some(20)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(12, Some(20)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(30, Some(20)));

  {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 2, 100)?;
    let _ = supplier.get(20, dummy_lrc, &dummy_searcher)?;
  }

  let mut subs = HashMap::new();
  for occur in [Occur::Should, Occur::Must, Occur::Filter, Occur::MustNot] {
    subs.insert(occur, Vec::new());
  }

  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(42, Some(62)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(12, Some(62)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(30, Some(62)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(20, Some(62)));

  {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 2, 100)?;
    let _ = supplier.get(100, dummy_lrc, &dummy_searcher)?;
  }

  let mut subs = HashMap::new();
  for occur in [Occur::Should, Occur::Must, Occur::Filter, Occur::MustNot] {
    subs.insert(occur, Vec::new());
  }

  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(42, Some(32)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(12, Some(32)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(30, Some(32)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(20, Some(32)));

  {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 3, 100)?;
    let _ = supplier.get(100, dummy_lrc, &dummy_searcher)?;
  }

  Ok(())
}
#[test]
fn test_prohibited_lead_cost() -> Result<()> {
  let mut random = random();
  let dummy_searcher = crate::test_framework::core::util::dummy_index_searcher(
    crate::test_framework::core::util::dummy_directory()?,
  )?;
  let dummy_lrc = &dummy_searcher.get_leaf_contexts()?[0];

  let mut subs = HashMap::new();
  for occur in [Occur::Should, Occur::Must, Occur::Filter, Occur::MustNot] {
    subs.insert(occur, Vec::new());
  }

  subs
    .get_mut(&Occur::Must)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(42, Some(42)));
  subs
    .get_mut(&Occur::MustNot)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(30, Some(42)));

  subs = {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 0, 100)?;
    let _ = supplier.get(100, dummy_lrc, &dummy_searcher)?;
    supplier.subs
  };

  subs.get_mut(&Occur::Must).unwrap().clear();
  subs.get_mut(&Occur::MustNot).unwrap().clear();
  subs
    .get_mut(&Occur::Must)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(42, Some(42)));
  subs
    .get_mut(&Occur::MustNot)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(80, Some(42)));

  subs = {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 0, 100)?;
    let _ = supplier.get(100, dummy_lrc, &dummy_searcher)?;
    supplier.subs
  };

  subs.get_mut(&Occur::Must).unwrap().clear();
  subs.get_mut(&Occur::MustNot).unwrap().clear();
  subs
    .get_mut(&Occur::Must)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(42, Some(20)));
  subs
    .get_mut(&Occur::MustNot)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(30, Some(20)));

  {
    let score_mode = *ScoreMode::values().choose(&mut random).unwrap();
    let mut supplier = BooleanScorerSupplier::new(subs, score_mode, 0, 100)?;
    let _ = supplier.get(20, dummy_lrc, &dummy_searcher)?;
  }

  Ok(())
}
#[test]
fn test_mixed_lead_cost() -> Result<()> {
  let dummy_searcher = crate::test_framework::core::util::dummy_index_searcher(
    crate::test_framework::core::util::dummy_directory()?,
  )?;
  let dummy_lrc = &dummy_searcher.get_leaf_contexts()?[0];

  let mut subs = HashMap::new();
  for occur in Occur::values() {
    subs.insert(*occur, Vec::new());
  }

  subs
    .get_mut(&Occur::Must)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(42, Some(42)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(30, Some(42)));

  subs = {
    let mut supplier = BooleanScorerSupplier::new(subs, ScoreMode::Complete, 0, 100)?;
    let _ = supplier.get(100, dummy_lrc, &dummy_searcher)?;
    supplier.subs
  };

  subs.get_mut(&Occur::Must).unwrap().clear();
  subs.get_mut(&Occur::Should).unwrap().clear();
  subs
    .get_mut(&Occur::Must)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(42, Some(42)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(80, Some(42)));

  subs = {
    let mut supplier = BooleanScorerSupplier::new(subs, ScoreMode::Complete, 0, 100)?;
    let _ = supplier.get(100, dummy_lrc, &dummy_searcher)?;
    supplier.subs
  };

  subs.get_mut(&Occur::Must).unwrap().clear();
  subs.get_mut(&Occur::Should).unwrap().clear();
  subs
    .get_mut(&Occur::Must)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(42, Some(20)));
  subs
    .get_mut(&Occur::Should)
    .unwrap()
    .push(FakeScorerSupplier::with_lead_cost(80, Some(20)));

  {
    let mut supplier = BooleanScorerSupplier::new(subs, ScoreMode::Complete, 0, 100)?;
    let _ = supplier.get(20, dummy_lrc, &dummy_searcher)?;
  }

  Ok(())
}

#[test]
fn test_disjunction_top_level_scoring_clause() -> Result<()> {
  let mut subs = HashMap::new();
  for occur in Occur::values() {
    subs.insert(*occur, Vec::new());
  }

  let clause1 = FakeScorerSupplier::with_lead_cost(10, Some(10));
  subs.get_mut(&Occur::Should).unwrap().push(clause1);
  let clause2 = FakeScorerSupplier::with_lead_cost(10, Some(10));
  subs.get_mut(&Occur::Should).unwrap().push(clause2);

  let mut supplier = BooleanScorerSupplier::new(subs, ScoreMode::TopScores, 0, 100)?;
  supplier.set_top_level_scoring_clause()?;

  assert!(
    !supplier.subs.get_mut(&Occur::Should).unwrap()[0]
      .as_any()
      .downcast_ref::<FakeScorerSupplier>()
      .unwrap()
      .top_level_scoring_clause
  );
  assert!(
    !supplier.subs.get_mut(&Occur::Should).unwrap()[1]
      .as_any()
      .downcast_ref::<FakeScorerSupplier>()
      .unwrap()
      .top_level_scoring_clause
  );
  // assert!(!supplier.subs.get(&Occur::Should).unwrap()[0].top_level_scoring_clause);
  // assert!(!supplier.subs.get(&Occur::Should).unwrap()[1].top_level_scoring_clause);

  Ok(())
}

#[test]
fn test_conjunction_top_level_scoring_clause() -> Result<()> {
  let mut subs = HashMap::new();
  for occur in Occur::values() {
    subs.insert(*occur, Vec::new());
  }

  let clause1 = FakeScorerSupplier::with_lead_cost(10, Some(10));
  subs.get_mut(&Occur::Must).unwrap().push(clause1);
  let clause2 = FakeScorerSupplier::with_lead_cost(10, Some(10));
  subs.get_mut(&Occur::Must).unwrap().push(clause2);

  let mut supplier = BooleanScorerSupplier::new(subs, ScoreMode::TopScores, 0, 100)?;
  supplier.set_top_level_scoring_clause()?;

  assert!(
    !supplier.subs.get_mut(&Occur::Must).unwrap()[0]
      .as_any()
      .downcast_ref::<FakeScorerSupplier>()
      .unwrap()
      .top_level_scoring_clause
  );
  assert!(
    !supplier.subs.get_mut(&Occur::Must).unwrap()[1]
      .as_any()
      .downcast_ref::<FakeScorerSupplier>()
      .unwrap()
      .top_level_scoring_clause
  );

  Ok(())
}

#[test]
fn test_filter_top_level_scoring_clause() -> Result<()> {
  let mut subs = HashMap::new();
  for occur in Occur::values() {
    subs.insert(*occur, Vec::new());
  }

  let clause1 = FakeScorerSupplier::with_lead_cost(10, Some(10));
  subs.get_mut(&Occur::Filter).unwrap().push(clause1);
  let clause2 = FakeScorerSupplier::with_lead_cost(10, Some(10));
  subs.get_mut(&Occur::Filter).unwrap().push(clause2);

  let mut supplier = BooleanScorerSupplier::new(subs, ScoreMode::TopScores, 0, 100)?;
  supplier.set_top_level_scoring_clause()?;
  assert!(
    !supplier.subs.get_mut(&Occur::Filter).unwrap()[0]
      .as_any()
      .downcast_ref::<FakeScorerSupplier>()
      .unwrap()
      .top_level_scoring_clause
  );
  assert!(
    !supplier.subs.get_mut(&Occur::Filter).unwrap()[1]
      .as_any()
      .downcast_ref::<FakeScorerSupplier>()
      .unwrap()
      .top_level_scoring_clause
  );

  Ok(())
}

#[test]
fn test_single_must_scoring_clause() -> Result<()> {
  let mut subs = HashMap::new();
  for occur in Occur::values() {
    subs.insert(*occur, Vec::new());
  }

  let clause1 = FakeScorerSupplier::with_lead_cost(10, Some(10));
  subs.get_mut(&Occur::Must).unwrap().push(clause1);
  let clause2 = FakeScorerSupplier::with_lead_cost(10, Some(10));
  subs.get_mut(&Occur::Filter).unwrap().push(clause2);

  let mut supplier = BooleanScorerSupplier::new(subs, ScoreMode::TopScores, 0, 100)?;
  supplier.set_top_level_scoring_clause()?;
  assert!(
    supplier.subs.get_mut(&Occur::Must).unwrap()[0]
      .as_any()
      .downcast_ref::<FakeScorerSupplier>()
      .unwrap()
      .top_level_scoring_clause
  );
  assert!(
    !supplier.subs.get_mut(&Occur::Filter).unwrap()[0]
      .as_any()
      .downcast_ref::<FakeScorerSupplier>()
      .unwrap()
      .top_level_scoring_clause
  );

  Ok(())
}

#[test]
fn test_single_should_scoring_clause() -> Result<()> {
  let mut subs = HashMap::new();
  for occur in Occur::values() {
    subs.insert(*occur, Vec::new());
  }

  let clause1 = FakeScorerSupplier::with_lead_cost(10, Some(10));
  subs.get_mut(&Occur::Should).unwrap().push(clause1);
  let clause2 = FakeScorerSupplier::with_lead_cost(10, Some(10));
  subs.get_mut(&Occur::MustNot).unwrap().push(clause2);

  let mut supplier = BooleanScorerSupplier::new(subs, ScoreMode::TopScores, 0, 100)?;
  supplier.set_top_level_scoring_clause()?;
  assert!(
    supplier.subs.get_mut(&Occur::Should).unwrap()[0]
      .as_any()
      .downcast_ref::<FakeScorerSupplier>()
      .unwrap()
      .top_level_scoring_clause
  );
  assert!(
    !supplier.subs.get_mut(&Occur::MustNot).unwrap()[0]
      .as_any()
      .downcast_ref::<FakeScorerSupplier>()
      .unwrap()
      .top_level_scoring_clause
  );

  Ok(())
}

#[test]
fn test_max_score_non_top_level_scoring_clause() -> Result<()> {
  let dummy_searcher = crate::test_framework::core::util::dummy_index_searcher(
    crate::test_framework::core::util::dummy_directory()?,
  )?;
  let dummy_lrc = &dummy_searcher.get_leaf_contexts()?[0];

  let mut subs = HashMap::new();
  for occur in Occur::values() {
    subs.insert(*occur, Vec::new());
  }

  let clause1 = FakeScorerSupplier::with_lead_cost(10, Some(10));
  let clause2 = FakeScorerSupplier::with_lead_cost(10, Some(10));
  subs.get_mut(&Occur::Must).unwrap().push(clause1);
  subs.get_mut(&Occur::Must).unwrap().push(clause2);

  let mut supplier = BooleanScorerSupplier::new(subs, ScoreMode::TopScores, 0, 100)?;
  let mut scorer = supplier.get(10, dummy_lrc, &dummy_searcher)?;
  assert_eq!(2.0, scorer.get_max_score(NO_MORE_DOCS)?,);

  let mut subs = HashMap::new();
  for occur in Occur::values() {
    subs.insert(*occur, Vec::new());
  }

  let clause1 = FakeScorerSupplier::with_lead_cost(10, Some(10));
  let clause2 = FakeScorerSupplier::with_lead_cost(10, Some(10));
  subs.get_mut(&Occur::Should).unwrap().push(clause1);
  subs.get_mut(&Occur::Should).unwrap().push(clause2);

  let mut supplier = BooleanScorerSupplier::new(subs, ScoreMode::TopScores, 0, 100)?;
  let mut scorer = supplier.get(10, dummy_lrc, &dummy_searcher)?;
  assert_eq!(2.0, scorer.get_max_score(NO_MORE_DOCS)?,);

  Ok(())
}
