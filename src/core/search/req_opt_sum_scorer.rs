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
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::score_mode::ScoreMode::TopScores;
#[cfg(test)]
use crate::core::search::scorer::ScorerKind;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::scorer_util::ScorerUtil;
use crate::core::search::two_phase_iterator::{
  TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// A scorer whose ordinary or two-phase iterator is selected at construction.
pub struct ReqOptSumScorer<S: DocIdSetIterator> {
  disi: S,
}

impl<S1: Scorer, S2: Scorer> ReqOptSumScorer<DocIdSetIteratorImpl<S1, S2>> {
  /// Takes the prepared ordinary iterator without a runtime mode enum.
  pub(crate) fn from_disi(disi: DocIdSetIteratorImpl<S1, S2>) -> Self {
    Self { disi }
  }
}

impl<S1: Scorer, S2: Scorer>
  ReqOptSumScorer<TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<S1, S2>>>
{
  /// Takes the prepared two-phase iterator without a runtime mode enum.
  pub(crate) fn from_tpi(two_phase_iterator: TwoPhaseIteratorImpl<S1, S2>) -> Self {
    Self {
      disi: TwoPhaseIteratorAsDocIdSetIterator::new(two_phase_iterator),
    }
  }
}

/// State-specific access to the same required/optional scorers and two-phase view.
#[doc(hidden)]
pub trait ReqOptSumState: DocIdSetIterator + 'static {
  type Required: Scorer + 'static;
  type Optional: Scorer + 'static;
  type ApproximationRef<'a>: DocIdSetIterator
  where
    Self: 'a;
  type ApproximationMut<'a>: DocIdSetIterator
  where
    Self: 'a;
  fn scorer_mut(&mut self) -> &mut DocIdSetIteratorImpl<Self::Required, Self::Optional>;
  fn approximation(&self) -> Self::ApproximationRef<'_>;
  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_>;
  fn two_phase_iterator(&self) -> Option<&TwoPhaseIteratorImpl<Self::Required, Self::Optional>>;
  fn two_phase_iterator_mut(
    &mut self,
  ) -> Option<&mut TwoPhaseIteratorImpl<Self::Required, Self::Optional>>;
  fn take_two_phase_iterator(self) -> Option<TwoPhaseIteratorImpl<Self::Required, Self::Optional>>;
  fn has_two_phase_iterator(&self) -> TwoPhaseState;
}

impl<S: ReqOptSumState> Scorable for ReqOptSumScorer<S> {
  type Child<'a>
    = crate::core::search::scorable::ScorableEnum2<&'a mut S::Required, &'a mut S::Optional>
  where
    Self: 'a;
  fn score(&mut self) -> Result<f32> {
    self.disi.scorer_mut().score()
  }
  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    self.disi.scorer_mut().set_min_competitive_score(min_score)
  }
  fn get_children(&mut self) -> Result<Vec<ChildScorable<Self::Child<'_>>>> {
    let disi = self.disi.scorer_mut();
    Ok(vec![
      ChildScorable::new(
        crate::core::search::scorable::ScorableEnum2::A(&mut disi.req_scorer),
        "MUST",
      ),
      ChildScorable::new(
        crate::core::search::scorable::ScorableEnum2::B(&mut disi.opt_scorer),
        "SHOULD",
      ),
    ])
  }
  fn cost(&self) -> Result<i64> {
    self.disi.cost()
  }
}
impl<S: DocIdSetIterator> crate::core::search::scorable::FixedScore for ReqOptSumScorer<S> {}
impl<S: DocIdSetIterator> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for ReqOptSumScorer<S>
{
}
impl<S: DocIdSetIterator> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for ReqOptSumScorer<S>
{
}
impl<S: DocIdSetIterator> DocIdSetIterator for ReqOptSumScorer<S> {
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
impl<S: ReqOptSumState> Scorer for ReqOptSumScorer<S> {
  type Iterator = S;
  type OwnedTwoPhase = TwoPhaseIteratorImpl<S::Required, S::Optional>;
  type ApproximationRef<'a>
    = S::ApproximationRef<'a>
  where
    Self: 'a;
  type ApproximationMut<'a>
    = S::ApproximationMut<'a>
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
    self.disi.scorer_mut().req_scorer.scoring_doc_id()
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
    self.disi.scorer_mut().advance_shallow(target)
  }
  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    self.disi.scorer_mut().get_max_score(upto)
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    self.disi.has_two_phase_iterator()
  }
  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    self.disi.approximation_mut()
  }
  fn approximation(&self) -> Self::ApproximationRef<'_> {
    self.disi.approximation()
  }
  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    ScorerKind::ReqOptSum
  }
}

impl<S1: Scorer + 'static, S2: Scorer + 'static> ReqOptSumState for DocIdSetIteratorImpl<S1, S2> {
  type Required = S1;
  type Optional = S2;
  type ApproximationRef<'a>
    = &'a Self
  where
    Self: 'a;
  type ApproximationMut<'a>
    = &'a mut Self
  where
    Self: 'a;
  fn scorer_mut(&mut self) -> &mut Self {
    self
  }
  fn approximation(&self) -> Self::ApproximationRef<'_> {
    self
  }
  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    self
  }
  fn two_phase_iterator(&self) -> Option<&TwoPhaseIteratorImpl<S1, S2>> {
    None
  }
  fn two_phase_iterator_mut(&mut self) -> Option<&mut TwoPhaseIteratorImpl<S1, S2>> {
    None
  }
  fn take_two_phase_iterator(self) -> Option<TwoPhaseIteratorImpl<S1, S2>> {
    None
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }
}
impl<S1: Scorer + 'static, S2: Scorer + 'static> ReqOptSumState
  for TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<S1, S2>>
{
  type Required = S1;
  type Optional = S2;
  type ApproximationRef<'a>
    = &'a DocIdSetIteratorImpl<S1, S2>
  where
    Self: 'a;
  type ApproximationMut<'a>
    = &'a mut DocIdSetIteratorImpl<S1, S2>
  where
    Self: 'a;
  fn scorer_mut(&mut self) -> &mut DocIdSetIteratorImpl<S1, S2> {
    &mut self.two_phase_iterator.disi
  }
  fn approximation(&self) -> Self::ApproximationRef<'_> {
    &self.two_phase_iterator.disi
  }
  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    &mut self.two_phase_iterator.disi
  }
  fn two_phase_iterator(&self) -> Option<&TwoPhaseIteratorImpl<S1, S2>> {
    Some(&self.two_phase_iterator)
  }
  fn two_phase_iterator_mut(&mut self) -> Option<&mut TwoPhaseIteratorImpl<S1, S2>> {
    Some(&mut self.two_phase_iterator)
  }
  fn take_two_phase_iterator(self) -> Option<TwoPhaseIteratorImpl<S1, S2>> {
    Some(self.two_phase_iterator)
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::Yes
  }
}
pub struct DocIdSetIteratorImpl<S1, S2> {
  upto: i32,
  max_score: f32,
  opt_is_required: bool,
  min_score: f32,
  req_scorer: S1,
  opt_scorer: S2,
  req_max_score: f32,
  wrapper: bool,
  #[cfg(test)]
  fixed_max_score: bool,
}
impl<S1, S2> DocIdSetIteratorImpl<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  pub(crate) fn new(
    mut req_scorer: S1,
    mut opt_scorer: S2,
    score_mode: ScoreMode,
  ) -> Result<(Self, bool)> {
    let (req_max_score, wrapper) = if score_mode != TopScores {
      (f32::INFINITY, false)
    } else {
      req_scorer.advance_shallow(0)?;
      opt_scorer.advance_shallow(0)?;
      (req_scorer.get_max_score(NO_MORE_DOCS)?, true)
    };
    let has_tpi = req_scorer.has_two_phase_iterator() == TwoPhaseState::Yes
      || opt_scorer.has_two_phase_iterator() == TwoPhaseState::Yes;
    let disi = Self {
      upto: -1,
      max_score: 0.0,
      opt_is_required: false,
      min_score: 0.0,
      req_scorer,
      opt_scorer,
      req_max_score,
      wrapper,
      #[cfg(test)]
      fixed_max_score: false,
    };
    Ok((disi, has_tpi))
  }

  #[cfg(test)]
  pub(crate) fn with_fixed_max_score(mut self) -> Self {
    self.fixed_max_score = true;
    self
  }

  fn move_to_next_block(&mut self, target: i32) -> Result<()> {
    self.upto = self.advance_shallow(target)?;
    let req_max_score_block = self.req_scorer.get_max_score(self.upto)?;
    self.max_score = self.get_max_score(self.upto)?;
    self.opt_is_required = req_max_score_block < self.min_score;
    Ok(())
  }
  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    let mut upto = self.req_scorer.advance_shallow(target)?;

    let opt_doc = (self.opt_scorer).scoring_doc_id()?;

    if opt_doc <= target {
      let v = self.opt_scorer.advance_shallow(target)?;
      upto = upto.min(v);
    } else if opt_doc != NO_MORE_DOCS {
      upto = upto.min(opt_doc - 1);
    }

    Ok(upto)
  }
  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    #[cfg(test)]
    {
      if self.fixed_max_score {
        return Ok(f32::INFINITY);
      }
    }
    let mut max_score = self.req_scorer.get_max_score(upto)?;

    if (self.opt_scorer).scoring_doc_id()? <= upto {
      max_score += self.opt_scorer.get_max_score(upto)?;
    }

    Ok(max_score)
  }
  fn advance_impacts(&mut self, mut target: i32) -> Result<i32> {
    if target > self.upto {
      self.move_to_next_block(target)?;
    }

    loop {
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
  fn advance_internal(&mut self, target: i32) -> Result<i32> {
    if target == NO_MORE_DOCS {
      ScorerUtil::advance(&mut self.req_scorer, target)?;
      return Ok(NO_MORE_DOCS);
    }

    let mut req_doc = target;

    'advance_head: loop {
      if self.min_score != 0.0 {
        req_doc = self.advance_impacts(req_doc)?;
      }

      let mut req_approximation = self.req_scorer.approximation_mut();
      if req_approximation.doc_id() < req_doc {
        req_doc = req_approximation.advance(req_doc)?;
      }

      if req_doc == NO_MORE_DOCS || !self.opt_is_required {
        return Ok(req_doc);
      }

      let upper_bound = if self.req_max_score < self.min_score {
        NO_MORE_DOCS
      } else {
        self.upto
      };

      if req_doc > upper_bound {
        continue;
      }
      // Find the next common doc within the current block

      let mut opt_approximation = self.opt_scorer.approximation_mut();
      loop {
        let mut opt_doc = opt_approximation.doc_id();

        if opt_doc < req_doc {
          opt_doc = opt_approximation.advance(req_doc)?;
        }

        if opt_doc > upper_bound {
          req_doc = upper_bound + 1;
          continue 'advance_head;
        }

        if opt_doc != req_doc {
          req_doc = req_approximation.advance(opt_doc)?;
          if req_doc > upper_bound {
            continue 'advance_head;
          }
        }

        if req_doc == NO_MORE_DOCS || opt_doc == req_doc {
          return Ok(req_doc);
        }
      }
    }
  }
  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    self.min_score = min_score;
    // Potentially move to a conjunction
    if self.req_max_score < self.min_score {
      self.opt_is_required = true;
      if self.req_max_score == 0.0 {
        // If the required clause doesn't contribute scores, we can propagate the minimum
        // competitive score to the optional clause. This happens when the required clause is a
        // FILTER clause.
        // In theory we could generalize this and set minScore - reqMaxScore as a minimum
        // competitive score, but it's unlikely to help in practice unless reqMaxScore is much
        // smaller than typical scores of the optional clause.
        self.opt_scorer.set_min_competitive_score(self.min_score)?;
      }
    }
    Ok(())
  }
  fn score(&mut self) -> Result<f32> {
    let cur_doc = (self.req_scorer).scoring_doc_id()?;
    let mut score = self.req_scorer.score()?;
    let mut opt_scorer_doc = ScorerUtil::doc_id(&self.opt_scorer);

    if opt_scorer_doc < cur_doc {
      opt_scorer_doc = ScorerUtil::advance(&mut self.opt_scorer, cur_doc)?;
      let should_skip = {
        if let Some(mut opt_tpi) = self.opt_scorer.two_phase_iterator_mut() {
          opt_scorer_doc == cur_doc && !opt_tpi.matches()?
        } else {
          false
        }
      };
      if should_skip {
        opt_scorer_doc = ScorerUtil::next_doc(&mut self.opt_scorer)?;
      }
    }

    if opt_scorer_doc == cur_doc {
      score += self.opt_scorer.score()?;
    }

    Ok(score)
  }
}
impl<S1, S2> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for DocIdSetIteratorImpl<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
}
impl<S1, S2> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for DocIdSetIteratorImpl<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
}

impl<S1, S2> DocIdSetIterator for DocIdSetIteratorImpl<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  fn doc_id(&self) -> i32 {
    ScorerUtil::doc_id(&self.req_scorer)
  }

  fn next_doc(&mut self) -> Result<i32> {
    if self.wrapper {
      let next = ScorerUtil::doc_id(&self.req_scorer) + 1;
      self.advance_internal(next)
    } else {
      ScorerUtil::next_doc(&mut self.req_scorer)
    }
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    if self.wrapper {
      self.advance_internal(target)
    } else {
      ScorerUtil::advance(&mut self.req_scorer, target)
    }
  }

  fn cost(&self) -> Result<i64> {
    ScorerUtil::cost(&self.req_scorer)
  }
}

pub struct TwoPhaseIteratorImpl<S1, S2> {
  disi: DocIdSetIteratorImpl<S1, S2>,
}
impl<S1, S2> TwoPhaseIteratorImpl<S1, S2> {
  pub(crate) fn new(disi: DocIdSetIteratorImpl<S1, S2>) -> Self {
    Self { disi }
  }
}
impl<S1, S2> TwoPhaseIterator for TwoPhaseIteratorImpl<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  type ApproximationRef<'a>
    = &'a DocIdSetIteratorImpl<S1, S2>
  where
    Self: 'a;
  type ApproximationMut<'a>
    = &'a mut DocIdSetIteratorImpl<S1, S2>
  where
    Self: 'a;

  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    &mut self.disi
  }

  fn approximation(&self) -> Self::ApproximationRef<'_> {
    &self.disi
  }

  fn matches(&mut self) -> Result<bool> {
    if let Some(mut req_tpi) = self.disi.req_scorer.two_phase_iterator_mut()
      && !req_tpi.matches()?
    {
      return Ok(false);
    }
    let opt_had_tpi = self.disi.opt_scorer.has_two_phase_iterator() == TwoPhaseState::Yes;
    if opt_had_tpi {
      // The below condition is rare and can only happen if we transitioned to
      // optIsRequired=true
      // after the opt approximation was advanced and before it was confirmed.
      let req_doc = (self.disi.req_scorer).scoring_doc_id()?;
      let opt_doc = ScorerUtil::doc_id(&self.disi.opt_scorer);
      if self.disi.opt_is_required {
        if req_doc != opt_doc {
          if opt_doc < req_doc {
            ScorerUtil::advance(&mut self.disi.opt_scorer, req_doc)?;
          }
          if req_doc != ScorerUtil::doc_id(&self.disi.opt_scorer) {
            return Ok(false);
          }
        }
        let matches = match self.disi.opt_scorer.two_phase_iterator_mut() {
          Some(mut tpi) => tpi.matches()?,
          None => {
            return Err(LuceneError::illegal_state(
              "optional scorer reported a two-phase iterator but did not provide one",
            ));
          },
        };
        if !matches {
          // Advance the iterator to make it clear it doesn't match the current doc id
          ScorerUtil::next_doc(&mut self.disi.opt_scorer)?;
          return Ok(false);
        }
      } else if opt_doc == req_doc {
        let matches = match self.disi.opt_scorer.two_phase_iterator_mut() {
          Some(mut tpi) => tpi.matches()?,
          None => {
            return Err(LuceneError::illegal_state(
              "optional scorer reported a two-phase iterator but did not provide one",
            ));
          },
        };
        if !matches {
          // Advance the iterator to make it clear it doesn't match the current doc id
          ScorerUtil::next_doc(&mut self.disi.opt_scorer)?;
        }
      }
    }

    Ok(true)
  }

  fn match_cost(&self) -> f32 {
    let mut cost = 1.0;

    if let Some(req_tpi) = self.disi.req_scorer.two_phase_iterator() {
      cost += req_tpi.match_cost();
    }

    if let Some(opt_tpi) = self.disi.opt_scorer.two_phase_iterator() {
      cost += opt_tpi.match_cost();
    }

    cost
  }
}
