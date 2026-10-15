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
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::approximation_disi::ApproximationBitSetAccess;
use crate::core::search::approximation_disi::ApproximationDISI;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::dummy::dummy_weight::DummyWeight;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::positive_scores_only_collector::PositiveScoresOnlyCollector;
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::top_docs_collector::TopDocsCollector;
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{new_directory_shared, random};
use std::cell::Cell;

#[allow(dead_code)] // for quick search
pub struct TestPositiveScoresOnlyCollector;

const SCORES: [f32; 13] = [
  0.7767749,
  -1.7839992,
  8.9925785,
  7.9608946,
  -0.07948637,
  2.6356435,
  7.4950366,
  7.1490803,
  -8.108544,
  4.961808,
  2.2423935,
  -7.285586,
  4.6699767,
];
const SCORES_LEN: i32 = SCORES.len() as i32;

struct SimpleScorer {
  idx: Cell<i32>,
}

impl SimpleScorer {
  fn new() -> Self {
    Self { idx: Cell::new(-1) }
  }
}

impl FixedScore for SimpleScorer {}

impl Scorable for SimpleScorer {
  type Child<'a>
    = crate::core::search::dummy::dummy_scorable::DummyScorable
  where
    Self: 'a;

  fn score(&mut self) -> Result<f32> {
    let idx = self.idx.get();
    if idx == SCORES_LEN {
      Ok(f32::NAN)
    } else {
      Ok(SCORES[idx as usize])
    }
  }

  fn cost(&self) -> Result<i64> {
    Ok(SCORES.len() as i64)
  }
}

impl ApproximationDISI for SimpleScorer {
  fn approx_doc_id(&self) -> i32 {
    DocIdSetIterator::doc_id(self)
  }
  fn approx_next_doc(&mut self) -> Result<i32> {
    DocIdSetIterator::next_doc(self)
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    DocIdSetIterator::advance(self, target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    DocIdSetIterator::slow_advance(self, target)
  }
  fn approx_cost(&self) -> Result<i64> {
    DocIdSetIterator::cost(self)
  }
}

impl ApproximationBitSetAccess for SimpleScorer {
  fn approx_is_bit_iter(&self) -> bool {
    crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::is_bit_iter(self)
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::get(self, index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::set_doc_id(self, doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::bit_set_length(self)
  }
}

impl TwoPhaseIterator for SimpleScorer {
  fn matches(&mut self) -> Result<bool> {
    Ok(true)
  }
  fn match_cost(&self) -> f32 {
    0.0
  }
}

impl Scorer for SimpleScorer {
  fn scoring_doc_id(&mut self) -> Result<i32> {
    Ok(self.idx.get())
  }

  fn get_max_score(&mut self, _upto: i32) -> Result<f32> {
    Ok(f32::INFINITY)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }
}

impl crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for SimpleScorer {}
impl crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for SimpleScorer {}

impl DocIdSetIterator for SimpleScorer {
  fn doc_id(&self) -> i32 {
    self.idx.get()
  }

  fn next_doc(&mut self) -> Result<i32> {
    let next = self.idx.get() + 1;
    if next == SCORES_LEN {
      self.idx.set(NO_MORE_DOCS);
      Ok(NO_MORE_DOCS)
    } else {
      self.idx.set(next);
      Ok(next)
    }
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.idx.set(target);
    if target < SCORES_LEN {
      Ok(target)
    } else {
      Ok(NO_MORE_DOCS)
    }
  }

  fn cost(&self) -> Result<i64> {
    Ok(SCORES.len() as i64)
  }
}

#[test]
fn test_negative_scores() -> Result<()> {
  let num_positive_scores = SCORES.iter().filter(|score| **score > 0.0).count();

  let mut random = random();
  let directory = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, directory.clone())?;
  writer.add_document(&mut random, Document::new())?;
  writer.commit(&mut random)?;

  let reader = writer.get_reader(&mut random)?;
  let context = reader.get_context()?;
  let leaves = context.leaves()?;
  let searcher = IndexSearcher::new(leaves[0].reader().clone().get_context()?)?;
  writer.close(&mut random)?;

  let mut scorer = SimpleScorer::new();
  let manager = TopScoreDocCollectorManager::new(SCORES.len(), i32::MAX as usize)?;
  let top_docs_collector = manager.new_collector()?;
  let mut collector = PositiveScoresOnlyCollector::new(top_docs_collector);
  let dummy_weight = DummyWeight::new(leaves[0].reader().clone());
  let mut leaf_collector =
    collector.get_leaf_collector(&leaves[0], Some(&dummy_weight), &searcher)?;
  leaf_collector.set_scorer(&mut scorer)?;

  loop {
    let doc = scorer.next_doc()?;
    if doc == NO_MORE_DOCS {
      break;
    }
    leaf_collector.collect(0, &mut scorer)?;
  }

  let mut top_docs_collector = collector.into_inner();
  let top_docs = top_docs_collector.top_docs()?;
  assert_eq!(num_positive_scores, top_docs.total_hits.value);
  for score_doc in top_docs.score_docs {
    assert!(
      score_doc.score > 0.0,
      "only positive scores should return: {}",
      score_doc.score
    );
  }

  Ok(())
}
