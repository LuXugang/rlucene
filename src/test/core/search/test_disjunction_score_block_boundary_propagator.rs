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
use crate::core::search::disjunction_score_block_boundary_propagator::DisjunctionScoreBlockBoundaryPropagator;
use crate::test_framework::core::util::lucene_test_case::random;

use crate::core::search::disi_wrapper::DisiWrapper;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, EmptyDISI};

use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use rand::prelude::SliceRandom;

#[allow(dead_code)] // for quick search
struct TestDisjunctionScoreBlockBoundaryPropagator;
#[test]
fn test_basics() -> Result<()> {
  let mut random = random();
  let scorer1 = FakeScorer::new(20, 0.5);
  let scorer2 = FakeScorer::new(50, 1.5);
  let scorer3 = FakeScorer::new(30, 2.0);
  let scorer4 = FakeScorer::new(80, 3.0);
  let mut scorers = [scorer1, scorer2, scorer3, scorer4];
  scorers.shuffle(&mut random);

  let mut propagator = DisjunctionScoreBlockBoundaryPropagator::new(scorers.as_mut_slice())?;
  let mut disi_wrapper = Vec::new();
  for s in scorers.into_iter() {
    disi_wrapper.push(DisiWrapper::new(s)?);
  }

  assert_eq!(20, propagator.advance_shallow(0, &mut disi_wrapper)?);

  propagator.set_min_competitive_score(0.2);
  assert_eq!(20, propagator.advance_shallow(0, &mut disi_wrapper)?);

  propagator.set_min_competitive_score(0.7);
  assert_eq!(30, propagator.advance_shallow(0, &mut disi_wrapper)?);

  propagator.set_min_competitive_score(1.2);
  assert_eq!(30, propagator.advance_shallow(0, &mut disi_wrapper)?);

  propagator.set_min_competitive_score(1.7);
  assert_eq!(30, propagator.advance_shallow(0, &mut disi_wrapper)?);

  propagator.set_min_competitive_score(2.2);
  assert_eq!(80, propagator.advance_shallow(0, &mut disi_wrapper)?);

  propagator.set_min_competitive_score(5.0);
  assert_eq!(80, propagator.advance_shallow(0, &mut disi_wrapper)?);

  Ok(())
}

struct FakeScorer {
  boundary: i32,
  max_score: f32,
  disi: EmptyDISI,
}
impl FakeScorer {
  fn new(boundary: i32, max_score: f32) -> Self {
    Self {
      boundary,
      max_score,
      disi: EmptyDISI::default(),
    }
  }
}

impl Scorable for FakeScorer {
  fn score(&mut self) -> Result<f32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn cost(&self) -> Result<i64> {
    crate::core::search::doc_id_set_iterator::DocIdSetIterator::cost(&self)
  }
}

impl FixedScore for FakeScorer {}

impl DocIdSetIterator for FakeScorer {
  fn doc_id(&self) -> i32 {
    DocIdSetIterator::doc_id(&self.disi)
  }
  fn next_doc(&mut self) -> Result<i32> {
    DocIdSetIterator::next_doc(&mut self.disi)
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    DocIdSetIterator::advance(&mut self.disi, target)
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    DocIdSetIterator::slow_advance(&mut self.disi, target)
  }
  fn cost(&self) -> Result<i64> {
    DocIdSetIterator::cost(&self.disi)
  }
}
impl crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for FakeScorer {
  fn get_fixed_bit_set(&self) -> Option<&crate::core::util::fixed_bit_set::FixedBitSet> {
    crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions::get_fixed_bit_set(
      &self.disi,
    )
  }
  fn get_sparse_fixed_bit_set(
    &self,
  ) -> Option<&crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet> {
    crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions::get_sparse_fixed_bit_set(
      &self.disi,
    )
  }
  fn get_doc_base_fixed_bit_set(
    &self,
  ) -> Option<(usize, &crate::core::util::fixed_bit_set::FixedBitSet)> {
    crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions::get_doc_base_fixed_bit_set(
      &self.disi,
    )
  }
}
impl crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for FakeScorer {
  fn is_bit_iter(&self) -> bool {
    crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::is_bit_iter(&self.disi)
  }
  fn get(&self, index: usize) -> Result<bool> {
    crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::get(&self.disi, index)
  }
  fn set_doc_id(&mut self, doc: i32) -> Result<()> {
    crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::set_doc_id(&mut self.disi, doc)
  }
  fn bit_set_length(&self) -> Result<usize> {
    crate::core::search::doc_id_set_iterator::BitSetIteratorAccess::bit_set_length(&self.disi)
  }
}
impl Scorer for FakeScorer {
  fn scoring_doc_id(&mut self) -> Result<i32> {
    Ok(0)
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let FakeScorer { disi, .. } = *self;
    Box::new(disi)
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    assert!(target <= self.boundary);
    Ok(self.boundary)
  }

  fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
    Ok(self.max_score)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }

  fn approximation(&self) -> &dyn DocIdSetIterator {
    &self.disi
  }

  fn approximation_mut(&mut self) -> &mut dyn DocIdSetIterator {
    &mut self.disi
  }
}
