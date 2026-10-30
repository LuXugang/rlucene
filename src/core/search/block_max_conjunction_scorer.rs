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

use crate::core::search::approximation_disi::ApproximationBitSetAccess;
use crate::core::search::approximation_disi::ApproximationDISI;
use crate::core::search::doc_id_set_iterator::BitSetIteratorAccess;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum2};
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::{
  TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
};
use crate::core::util::core_helper::CoreHelper;
use crate::core::util::error::lucene_error::Result;
use smallvec::SmallVec;

pub type BlockMaxConjunctionScorerDisi<S> = DocIdSetIteratorEnum2<
  DocIdSetIteratorImpl<S>,
  TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<S>>,
>;
/// Scorer for conjunctions that checks the maximum scores of each clause
/// in order to potentially skip over blocks that cannot have competitive matches.
pub struct BlockMaxConjunctionScorer<D: DocIdSetIterator> {
  disi: D,
}
impl<S: Scorer> BlockMaxConjunctionScorer<DocIdSetIteratorImpl<S>> {
  pub(crate) fn prepare(scorers_list: Vec<S>) -> Result<BlockMaxConjunctionScorerDisi<S>> {
    let mut scorers_with_cost = Vec::with_capacity(scorers_list.len());
    for v in scorers_list {
      let cost = DocIdSetIterator::cost(&v)?;
      scorers_with_cost.push((cost, v));
    }
    // Lead with the lowest-cost scorer, preserving input order for equal costs.
    scorers_with_cost.sort_by_key(|entry| entry.0);

    let mut scorers = Vec::with_capacity(scorers_with_cost.len());
    for (_, mut v) in scorers_with_cost {
      v.advance_shallow(0)?;
      scorers.push(v);
    }

    let mut match_cost: SmallVec<[_; 8]> = SmallVec::with_capacity(scorers.len());
    for (i, s) in scorers.iter_mut().enumerate() {
      if s.has_two_phase_iterator() == TwoPhaseState::Yes {
        match_cost.push((i, s.match_cost()));
      }
    }
    match_cost.sort_by(|a, b| CoreHelper::compare_f32(a.1, b.1));
    let approx = DocIdSetIteratorImpl::new(scorers);
    if match_cost.is_empty() {
      Ok(BlockMaxConjunctionScorerDisi::A(approx))
    } else {
      // Move spilled storage; copy inline indices into an exactly sized Vec.
      let match_cost = if match_cost.spilled() {
        match_cost.into_vec()
      } else {
        match_cost.to_vec()
      };
      Ok(BlockMaxConjunctionScorerDisi::B(
        TwoPhaseIteratorAsDocIdSetIterator::new(TwoPhaseIteratorImpl::new(approx, match_cost)),
      ))
    }
  }
  pub(crate) fn from_disi(disi: DocIdSetIteratorImpl<S>) -> Self {
    Self { disi }
  }
}
impl<S: Scorer>
  BlockMaxConjunctionScorer<TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<S>>>
{
  pub(crate) fn from_tpi(
    disi: TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<S>>,
  ) -> Self {
    Self { disi }
  }
}

/// Scoring and pruning operations on the state owned by the selected iterator.
#[doc(hidden)]
pub trait BlockMaxConjunctionState:
  DocIdSetIterator + ApproximationDISI + ApproximationBitSetAccess + 'static
{
  fn match_cost(&self) -> f32;

  fn matches(&mut self) -> Result<bool>;

  type SubScorer: Scorer + 'static;
  fn score(&mut self) -> Result<f32>;
  fn set_min_competitive_score(&mut self, score: f32) -> Result<()>;
  fn get_children(&mut self) -> Result<Vec<ChildScorable<&mut Self::SubScorer>>>;
  fn scoring_doc_id(&mut self) -> Result<i32>;
  fn advance_shallow(&mut self, target: i32) -> Result<i32>;
  fn get_max_score(&mut self, upto: i32) -> Result<f32>;

  fn has_two_phase_iterator(&self) -> TwoPhaseState;
}
impl<S: Scorer> ApproximationDISI for DocIdSetIteratorImpl<S> {
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
impl<S: Scorer> ApproximationBitSetAccess for DocIdSetIteratorImpl<S> {
  fn approx_is_bit_iter(&self) -> bool {
    BitSetIteratorAccess::is_bit_iter(self)
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    BitSetIteratorAccess::get(self, index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    BitSetIteratorAccess::set_doc_id(self, doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    BitSetIteratorAccess::bit_set_length(self)
  }
}
impl<S: Scorer> ApproximationDISI for TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<S>> {
  fn approx_doc_id(&self) -> i32 {
    self.two_phase_iterator.approx_doc_id()
  }
  fn approx_next_doc(&mut self) -> Result<i32> {
    self.two_phase_iterator.approx_next_doc()
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    self.two_phase_iterator.approx_advance(target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.two_phase_iterator.approx_slow_advance(target)
  }
  fn approx_cost(&self) -> Result<i64> {
    self.two_phase_iterator.approx_cost()
  }
}
impl<S: Scorer> ApproximationBitSetAccess
  for TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<S>>
{
  fn approx_is_bit_iter(&self) -> bool {
    self.two_phase_iterator.approx_is_bit_iter()
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    self.two_phase_iterator.approx_get(index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.two_phase_iterator.approx_set_doc_id(doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    self.two_phase_iterator.approx_bit_set_length()
  }
}
impl<S: Scorer + 'static> BlockMaxConjunctionState for DocIdSetIteratorImpl<S> {
  fn match_cost(&self) -> f32 {
    0.0
  }

  fn matches(&mut self) -> Result<bool> {
    Ok(true)
  }

  type SubScorer = S;
  fn score(&mut self) -> Result<f32> {
    let mut sum: f64 = 0.0;
    for scorer in self.scorers.iter_mut() {
      sum += scorer.score()? as f64;
    }
    Ok(sum as f32)
  }

  fn set_min_competitive_score(&mut self, score: f32) -> Result<()> {
    self.min_score = score;
    Ok(())
  }

  fn get_children(&mut self) -> Result<Vec<ChildScorable<&mut Self::SubScorer>>> {
    Ok(
      self
        .scorers
        .iter_mut()
        .map(|scorer| ChildScorable::new(scorer, "MUST"))
        .collect(),
    )
  }

  fn scoring_doc_id(&mut self) -> Result<i32> {
    self.scorer_doc_id()
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    DocIdSetIteratorImpl::advance_shallow(self, target)
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    DocIdSetIteratorImpl::get_max_score(self, upto)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }
}
impl<S: Scorer + 'static> BlockMaxConjunctionState
  for TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<S>>
{
  fn match_cost(&self) -> f32 {
    self.two_phase_iterator.match_cost()
  }

  fn matches(&mut self) -> Result<bool> {
    self.two_phase_iterator.matches()
  }

  type SubScorer = S;
  fn score(&mut self) -> Result<f32> {
    BlockMaxConjunctionState::score(&mut self.two_phase_iterator.approx)
  }

  fn set_min_competitive_score(&mut self, score: f32) -> Result<()> {
    self
      .two_phase_iterator
      .approx
      .set_min_competitive_score(score)
  }

  fn get_children(&mut self) -> Result<Vec<ChildScorable<&mut Self::SubScorer>>> {
    self.two_phase_iterator.approx.get_children()
  }

  fn scoring_doc_id(&mut self) -> Result<i32> {
    self.two_phase_iterator.approx.scorer_doc_id()
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    self.two_phase_iterator.approx.advance_shallow(target)
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    self.two_phase_iterator.approx.get_max_score(upto)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::Yes
  }
}
impl<D: BlockMaxConjunctionState> Scorable for BlockMaxConjunctionScorer<D> {
  type Child<'a>
    = &'a mut D::SubScorer
  where
    Self: 'a;
  fn score(&mut self) -> Result<f32> {
    self.disi.score()
  }
  fn set_min_competitive_score(&mut self, score: f32) -> Result<()> {
    self.disi.set_min_competitive_score(score)
  }
  fn get_children(&mut self) -> Result<Vec<ChildScorable<Self::Child<'_>>>> {
    self.disi.get_children()
  }
  fn cost(&self) -> Result<i64> {
    self.disi.cost()
  }
}
impl<D: DocIdSetIterator> crate::core::search::scorable::FixedScore
  for BlockMaxConjunctionScorer<D>
{
}
impl<D: DocIdSetIterator> DocIdSetIterator for BlockMaxConjunctionScorer<D> {
  fn doc_id(&self) -> i32 {
    self.disi.doc_id()
  }
  fn next_doc(&mut self) -> Result<i32> {
    self.disi.next_doc()
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    self.disi.advance(target)
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.disi.slow_advance(target)
  }
  fn cost(&self) -> Result<i64> {
    self.disi.cost()
  }
}
impl<D: DocIdSetIterator> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for BlockMaxConjunctionScorer<D>
{
}
impl<D: DocIdSetIterator> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for BlockMaxConjunctionScorer<D>
{
}
impl<D: BlockMaxConjunctionState> ApproximationDISI for BlockMaxConjunctionScorer<D> {
  fn approx_doc_id(&self) -> i32 {
    self.disi.approx_doc_id()
  }
  fn approx_next_doc(&mut self) -> Result<i32> {
    self.disi.approx_next_doc()
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    self.disi.approx_advance(target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.disi.approx_slow_advance(target)
  }
  fn approx_cost(&self) -> Result<i64> {
    self.disi.approx_cost()
  }
}
impl<D: BlockMaxConjunctionState> ApproximationBitSetAccess for BlockMaxConjunctionScorer<D> {
  fn approx_is_bit_iter(&self) -> bool {
    self.disi.approx_is_bit_iter()
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    self.disi.approx_get(index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.disi.approx_set_doc_id(doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    self.disi.approx_bit_set_length()
  }
}

impl<D: BlockMaxConjunctionState> TwoPhaseIterator for BlockMaxConjunctionScorer<D> {
  fn matches(&mut self) -> Result<bool> {
    self.disi.matches()
  }
  fn match_cost(&self) -> f32 {
    self.disi.match_cost()
  }
}

impl<D: BlockMaxConjunctionState> Scorer for BlockMaxConjunctionScorer<D> {
  fn scoring_doc_id(&mut self) -> Result<i32> {
    self.disi.scoring_doc_id()
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    self.disi.advance_shallow(target)
  }
  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    self.disi.get_max_score(upto)
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    self.disi.has_two_phase_iterator()
  }
}

pub struct DocIdSetIteratorImpl<S> {
  // lead scorer is always the first one
  scorers: Vec<S>,
  upto: i32,
  max_score: f32,
  min_score: f32,
}
impl<S> DocIdSetIteratorImpl<S>
where
  S: Scorer,
{
  fn new(scorers: Vec<S>) -> Self {
    Self {
      scorers,
      upto: -1,
      max_score: 0f32,
      min_score: 0f32,
    }
  }

  fn scorer_doc_id(&mut self) -> Result<i32> {
    (self.scorers[0]).scoring_doc_id()
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    // We use block boundaries of the lead scorer.
    // It is tempting to fold in other clauses as well to have better bounds of
    // the score, but then there is a risk of not progressing fast enough.
    let result = self.scorers[0].advance_shallow(target)?;
    // But we still need to shallow-advance other clauses, in order to have
    // better score upper bounds
    for scorer in self.scorers.iter_mut().skip(1) {
      scorer.advance_shallow(target)?;
    }
    Ok(result)
  }
  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    let mut sum = 0f64;
    for scorer in self.scorers.iter_mut() {
      sum += scorer.get_max_score(upto)? as f64;
    }
    Ok(sum as f32)
  }
  fn do_next(&mut self, mut doc: i32) -> Result<i32> {
    'advance_head: loop {
      debug_assert_eq!(doc, self.scorers[0].approx_doc_id());

      if doc == NO_MORE_DOCS {
        return Ok(NO_MORE_DOCS);
      }

      if doc > self.upto {
        // This check is useful when scorers return information about blocks
        // that do not actually have any matches. Otherwise `doc` will always
        // be in the current block already since it is always the result of
        let next_target = self.advance_target(doc)?;
        if next_target != doc {
          doc = self.scorers[0].approx_advance(next_target)?;
          continue;
        }
      }

      debug_assert!(doc <= self.upto);
      let len = self.scorers.len();
      // then find agreement with other iterators
      for i in 1..len {
        let other = &mut self.scorers[i];
        // other.doc may already be equal to doc if we "continued advanceHead"
        // on the previous iteration and the advance on the lead scorer exactly matched.
        if other.approx_doc_id() < doc {
          let next = other.approx_advance(doc)?;
          if next > doc {
            // iterator beyond the current doc - advance lead and continue to the new highest
            // doc.
            let v = self.advance_target(next)?;
            doc = self.scorers[0].approx_advance(v)?;
            continue 'advance_head;
          }
        }
        debug_assert_eq!(other.approx_doc_id(), doc);
      }
      return Ok(doc);
    }
  }
  fn move_to_next_block(&mut self, target: i32) -> Result<()> {
    if self.min_score == 0.0 {
      self.upto = target;
      self.max_score = f32::INFINITY;
    } else {
      self.upto = self.advance_shallow(target)?;
      self.max_score = self.get_max_score(self.upto)?;
    }
    Ok(())
  }
  fn advance_target(&mut self, mut target: i32) -> Result<i32> {
    if target > self.upto {
      self.move_to_next_block(target)?;
    }

    loop {
      debug_assert!(self.upto >= target);

      if self.max_score >= self.min_score {
        return Ok(target);
      }

      if self.upto == NO_MORE_DOCS {
        return Ok(NO_MORE_DOCS);
      }

      target = self.upto + 1;
      self.move_to_next_block(target)?;
    }
  }
}
impl<S> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for DocIdSetIteratorImpl<S>
where
  S: Scorer,
{
}
impl<S> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for DocIdSetIteratorImpl<S> where
  S: Scorer
{
}

impl<S> DocIdSetIterator for DocIdSetIteratorImpl<S>
where
  S: Scorer,
{
  fn doc_id(&self) -> i32 {
    self.scorers[0].approx_doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.advance(self.doc_id() + 1)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    let doc = self.advance_target(target)?;
    let v = self.scorers[0].approx_advance(doc)?;
    self.do_next(v)
  }

  fn cost(&self) -> Result<i64> {
    Scorable::cost(&self.scorers[0])
  }
}
pub struct TwoPhaseIteratorImpl<S> {
  approx: DocIdSetIteratorImpl<S>,
  match_cost: f32,
  has_tpi_idx: Vec<(usize, f32)>,
}
impl<S> TwoPhaseIteratorImpl<S> {
  fn new(approx: DocIdSetIteratorImpl<S>, has_tpi_idx: Vec<(usize, f32)>) -> Self {
    // Accumulate this cost estimate in f64 without compensated summation.
    let match_cost = has_tpi_idx
      .iter()
      .map(|&(_, cost)| f64::from(cost))
      .sum::<f64>() as f32;
    Self {
      approx,
      match_cost,
      has_tpi_idx,
    }
  }
}
impl<S> ApproximationDISI for TwoPhaseIteratorImpl<S>
where
  S: Scorer,
{
  fn approx_doc_id(&self) -> i32 {
    self.approx.doc_id()
  }

  fn approx_next_doc(&mut self) -> Result<i32> {
    self.approx.next_doc()
  }

  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    self.approx.advance(target)
  }

  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.approx.slow_advance(target)
  }

  fn approx_cost(&self) -> Result<i64> {
    self.approx.cost()
  }
}
impl<S> ApproximationBitSetAccess for TwoPhaseIteratorImpl<S>
where
  S: Scorer,
{
  fn approx_is_bit_iter(&self) -> bool {
    self.approx.is_bit_iter()
  }

  fn approx_get(&self, index: usize) -> Result<bool> {
    self.approx.get(index)
  }

  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.approx.set_doc_id(doc)
  }

  fn approx_bit_set_length(&self) -> Result<usize> {
    self.approx.bit_set_length()
  }
}

impl<S> TwoPhaseIterator for TwoPhaseIteratorImpl<S>
where
  S: Scorer,
{
  fn matches(&mut self) -> Result<bool> {
    #[cfg(debug_assertions)]
    let doc = self.approx.scorer_doc_id()?;
    for (idx, _) in &self.has_tpi_idx {
      let scorer = &mut self.approx.scorers[*idx];
      #[cfg(debug_assertions)]
      debug_assert_eq!(scorer.approx_doc_id(), doc);
      if !scorer.matches()? {
        return Ok(false);
      }
    }
    Ok(true)
  }

  fn match_cost(&self) -> f32 {
    self.match_cost
  }
}
