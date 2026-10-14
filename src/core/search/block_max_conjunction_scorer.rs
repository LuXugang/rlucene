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

use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum2};
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::{
  TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
};
use crate::core::util::core_helper::CoreHelper;
use crate::core::util::error::lucene_error::{LuceneError, Result};

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

    let mut match_cost = Vec::with_capacity(scorers.len());
    for (i, s) in scorers.iter_mut().enumerate() {
      if let Some(tpi) = s.two_phase_iterator() {
        match_cost.push((i, tpi.match_cost()));
      }
    }
    match_cost.sort_by(|a, b| CoreHelper::compare_f32(a.1, b.1));
    let approx = DocIdSetIteratorImpl::new(scorers);
    if match_cost.is_empty() {
      Ok(BlockMaxConjunctionScorerDisi::A(approx))
    } else {
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

/// Access to the block state owned by the selected iterator.
#[doc(hidden)]
pub trait BlockMaxConjunctionState: DocIdSetIterator + 'static {
  type SubScorer: Scorer + 'static;
  fn approximation(&self) -> &DocIdSetIteratorImpl<Self::SubScorer>;
  fn approximation_mut(&mut self) -> &mut DocIdSetIteratorImpl<Self::SubScorer>;
  fn two_phase_iterator(&self) -> Option<&TwoPhaseIteratorImpl<Self::SubScorer>>;
  fn two_phase_iterator_mut(&mut self) -> Option<&mut TwoPhaseIteratorImpl<Self::SubScorer>>;
  fn take_two_phase_iterator(self) -> Option<TwoPhaseIteratorImpl<Self::SubScorer>>;
  fn has_two_phase_iterator(&self) -> TwoPhaseState;
}
impl<S: Scorer + 'static> BlockMaxConjunctionState for DocIdSetIteratorImpl<S> {
  type SubScorer = S;
  fn approximation(&self) -> &Self {
    self
  }
  fn approximation_mut(&mut self) -> &mut Self {
    self
  }
  fn two_phase_iterator(&self) -> Option<&TwoPhaseIteratorImpl<S>> {
    None
  }
  fn two_phase_iterator_mut(&mut self) -> Option<&mut TwoPhaseIteratorImpl<S>> {
    None
  }
  fn take_two_phase_iterator(self) -> Option<TwoPhaseIteratorImpl<S>> {
    None
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }
}
impl<S: Scorer + 'static> BlockMaxConjunctionState
  for TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<S>>
{
  type SubScorer = S;
  fn approximation(&self) -> &DocIdSetIteratorImpl<S> {
    &self.two_phase_iterator.approx
  }
  fn approximation_mut(&mut self) -> &mut DocIdSetIteratorImpl<S> {
    &mut self.two_phase_iterator.approx
  }
  fn two_phase_iterator(&self) -> Option<&TwoPhaseIteratorImpl<S>> {
    Some(&self.two_phase_iterator)
  }
  fn two_phase_iterator_mut(&mut self) -> Option<&mut TwoPhaseIteratorImpl<S>> {
    Some(&mut self.two_phase_iterator)
  }
  fn take_two_phase_iterator(self) -> Option<TwoPhaseIteratorImpl<S>> {
    Some(self.two_phase_iterator)
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::Yes
  }
}
impl<D: BlockMaxConjunctionState> BlockMaxConjunctionScorer<D> {
  fn do_score(s: &mut [D::SubScorer]) -> Result<f32> {
    let mut sum: f64 = 0.0;
    for scorer in s.iter_mut() {
      sum += scorer.score()? as f64;
    }
    Ok(sum as f32)
  }
}
impl<D: BlockMaxConjunctionState> Scorable for BlockMaxConjunctionScorer<D> {
  type Child<'a>
    = &'a mut D::SubScorer
  where
    Self: 'a;
  fn score(&mut self) -> Result<f32> {
    Self::do_score(self.disi.approximation_mut().scorers.as_mut())
  }
  fn set_min_competitive_score(&mut self, score: f32) -> Result<()> {
    self.disi.approximation_mut().min_score = score;
    Ok(())
  }
  fn get_children(&mut self) -> Result<Vec<ChildScorable<Self::Child<'_>>>> {
    let scorers = &mut self.disi.approximation_mut().scorers;
    Ok(
      scorers
        .iter_mut()
        .map(|scorer| ChildScorable::new(scorer, "MUST"))
        .collect(),
    )
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
impl<D: BlockMaxConjunctionState> Scorer for BlockMaxConjunctionScorer<D> {
  type Iterator = D;
  type OwnedTwoPhase = TwoPhaseIteratorImpl<D::SubScorer>;
  type ApproximationRef<'a>
    = &'a DocIdSetIteratorImpl<D::SubScorer>
  where
    Self: 'a;
  type ApproximationMut<'a>
    = &'a mut DocIdSetIteratorImpl<D::SubScorer>
  where
    Self: 'a;
  type TwoPhaseRef<'a>
    = &'a Self::OwnedTwoPhase
  where
    Self: 'a;
  type TwoPhaseMut<'a>
    = &'a mut Self::OwnedTwoPhase
  where
    Self: 'a;
  fn scoring_doc_id(&mut self) -> Result<i32> {
    self.disi.approximation_mut().scorer_doc_id()
  }
  fn take_iterator(self) -> Self::Iterator {
    self.disi
  }
  fn two_phase_iterator(&self) -> Option<Self::TwoPhaseRef<'_>> {
    self.disi.two_phase_iterator()
  }
  fn two_phase_iterator_mut(&mut self) -> Option<Self::TwoPhaseMut<'_>> {
    self.disi.two_phase_iterator_mut()
  }
  fn take_two_phase_iterator(self) -> Option<Self::OwnedTwoPhase> {
    self.disi.take_two_phase_iterator()
  }
  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    self.disi.approximation_mut().advance_shallow(target)
  }
  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    self.disi.approximation_mut().get_max_score(upto)
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    self.disi.has_two_phase_iterator()
  }
  fn approximation(&self) -> Self::ApproximationRef<'_> {
    self.disi.approximation()
  }
  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    self.disi.approximation_mut()
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
      debug_assert_eq!(doc, self.scorers[0].approximation().doc_id());

      if doc == NO_MORE_DOCS {
        return Ok(NO_MORE_DOCS);
      }

      if doc > self.upto {
        // This check is useful when scorers return information about blocks
        // that do not actually have any matches. Otherwise `doc` will always
        // be in the current block already since it is always the result of
        let next_target = self.advance_target(doc)?;
        if next_target != doc {
          doc = self.scorers[0].approximation_mut().advance(next_target)?;
          continue;
        }
      }

      debug_assert!(doc <= self.upto);
      let len = self.scorers.len();
      // then find agreement with other iterators
      for i in 1..len {
        let mut other = self.scorers[i].approximation_mut();
        // other.doc may already be equal to doc if we "continued advanceHead"
        // on the previous iteration and the advance on the lead scorer exactly matched.
        if other.doc_id() < doc {
          let next = other.advance(doc)?;
          if next > doc {
            drop(other);
            // iterator beyond the current doc - advance lead and continue to the new highest
            // doc.
            let v = self.advance_target(next)?;
            doc = self.scorers[0].approximation_mut().advance(v)?;
            continue 'advance_head;
          }
        }
        debug_assert_eq!(other.doc_id(), doc);
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
    self.scorers[0].approximation().doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.advance(self.doc_id() + 1)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    let doc = self.advance_target(target)?;
    let v = self.scorers[0].approximation_mut().advance(doc)?;
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
impl<S> TwoPhaseIterator for TwoPhaseIteratorImpl<S>
where
  S: Scorer,
{
  type ApproximationRef<'a>
    = &'a DocIdSetIteratorImpl<S>
  where
    Self: 'a;
  type ApproximationMut<'a>
    = &'a mut DocIdSetIteratorImpl<S>
  where
    Self: 'a;

  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    &mut self.approx
  }

  fn approximation(&self) -> Self::ApproximationRef<'_> {
    &self.approx
  }

  fn matches(&mut self) -> Result<bool> {
    #[cfg(debug_assertions)]
    let doc = self.approx.scorer_doc_id()?;
    for (idx, _) in &self.has_tpi_idx {
      match self.approx.scorers[*idx].two_phase_iterator_mut() {
        Some(ref mut tpi) => {
          #[cfg(debug_assertions)]
          {
            debug_assert!(tpi.approximation().doc_id() == doc);
          }
          if !tpi.matches()? {
            return Ok(false);
          }
        },
        None => {
          // should not happen
          return Err(LuceneError::illegal_state(
            "two_phase_iterator should not be None",
          ));
        },
      }
    }
    Ok(true)
  }

  fn match_cost(&self) -> f32 {
    self.match_cost
  }
}
