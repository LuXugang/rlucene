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

use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::scorable::{ChildScorable, FixedScore, Scorable};
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::util::error::lucene_error::Result;

pub struct DummyScorer;

impl Scorable for DummyScorer {
  type Child<'a>
    = crate::core::search::dummy::dummy_scorable::DummyScorable
  where
    Self: 'a;

  fn score(&mut self) -> Result<f32> {
    dummy_unreachable!()
  }

  fn smoothing_score(&mut self, _doc_id: i32) -> Result<f32> {
    dummy_unreachable!()
  }

  fn set_min_competitive_score(&mut self, _min_score: f32) -> Result<()> {
    dummy_unreachable!()
  }

  fn get_children(&mut self) -> Result<Vec<ChildScorable<Self::Child<'_>>>> {
    dummy_unreachable!()
  }

  fn cost(&self) -> Result<i64> {
    dummy_unreachable!()
  }
}

impl FixedScore for DummyScorer {}

impl DocIdSetIterator for DummyScorer {
  fn doc_id(&self) -> i32 {
    dummy_unreachable!()
  }
  fn next_doc(&mut self) -> Result<i32> {
    dummy_unreachable!()
  }
  fn advance(&mut self, _target: i32) -> Result<i32> {
    dummy_unreachable!()
  }
  fn slow_advance(&mut self, _target: i32) -> Result<i32> {
    dummy_unreachable!()
  }
  fn cost(&self) -> Result<i64> {
    dummy_unreachable!()
  }
}
impl crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for DummyScorer {
  fn get_fixed_bit_set(&self) -> Option<&crate::core::util::fixed_bit_set::FixedBitSet> {
    dummy_unreachable!()
  }
  fn get_sparse_fixed_bit_set(
    &self,
  ) -> Option<&crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet> {
    dummy_unreachable!()
  }
  fn get_doc_base_fixed_bit_set(
    &self,
  ) -> Option<(usize, &crate::core::util::fixed_bit_set::FixedBitSet)> {
    dummy_unreachable!()
  }
}
impl crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for DummyScorer {
  fn is_bit_iter(&self) -> bool {
    dummy_unreachable!()
  }
  fn get(&self, _index: usize) -> Result<bool> {
    dummy_unreachable!()
  }
  fn set_doc_id(&mut self, _doc: i32) -> Result<()> {
    dummy_unreachable!()
  }
  fn bit_set_length(&self) -> Result<usize> {
    dummy_unreachable!()
  }
}
impl Scorer for DummyScorer {
  type Iterator = crate::core::search::doc_id_set_iterator::EmptyDISI;
  type OwnedTwoPhase = crate::core::search::two_phase_iterator::EmptyTPI;
  type ApproximationRef<'a>
    = &'a crate::core::search::doc_id_set_iterator::EmptyDISI
  where
    Self: 'a;
  type ApproximationMut<'a>
    = &'a mut crate::core::search::doc_id_set_iterator::EmptyDISI
  where
    Self: 'a;
  type TwoPhaseRef<'a>
    = crate::core::search::two_phase_iterator::EmptyTPI
  where
    Self: 'a;
  type TwoPhaseMut<'a>
    = crate::core::search::two_phase_iterator::EmptyTPI
  where
    Self: 'a;

  fn scoring_doc_id(&mut self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn take_iterator(self) -> Self::Iterator {
    dummy_unreachable!()
  }

  fn two_phase_iterator(&self) -> Option<Self::TwoPhaseRef<'_>> {
    dummy_unreachable!()
  }

  fn two_phase_iterator_mut(&mut self) -> Option<Self::TwoPhaseMut<'_>> {
    dummy_unreachable!()
  }

  fn take_two_phase_iterator(self) -> Option<Self::OwnedTwoPhase> {
    dummy_unreachable!()
  }

  fn advance_shallow(&mut self, _target: i32) -> Result<i32> {
    dummy_unreachable!()
  }

  fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
    dummy_unreachable!()
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    dummy_unreachable!()
  }

  fn approximation(&self) -> Self::ApproximationRef<'_> {
    dummy_unreachable!()
  }

  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    dummy_unreachable!()
  }
}
