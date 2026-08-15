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
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::score_mode::ScoreMode::TopScores;
#[cfg(test)]
use crate::core::search::scorer::ScorerKind;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::scorer_util::ScorerUtil;
use crate::core::search::two_phase_iterator::{
  TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
};
use crate::core::util::error::lucene_error::Result;

pub type ReqOptSumScorerDisi<S1, S2> = DocIdSetIteratorEnum2<
  DocIdSetIteratorImpl<S1, S2>,
  TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<S1, S2>>,
>;
/// A scorer for queries with a required part and an optional part.
/// Delays advance on the optional part until a score is needed.
pub struct ReqOptSumScorer<S1, S2> {
  disi: ReqOptSumScorerDisi<S1, S2>,
  tpi_state: TwoPhaseState,
}
impl<S1, S2> ReqOptSumScorer<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  /// Construct a `ReqOptScorer`.
  ///
  /// * `req_scorer` — the required scorer, which must match
  /// * `opt_scorer` — the optional scorer, used only for scoring
  /// * `score_mode` — how the produced scorers will be consumed
  pub(crate) fn new(mut req_scorer: S1, mut opt_scorer: S2, score_mode: ScoreMode) -> Result<Self> {
    let (req_max_score, wrapper) = if score_mode != TopScores {
      (f32::MAX, false)
    } else {
      req_scorer.advance_shallow(0)?;
      opt_scorer.advance_shallow(0)?;
      (req_scorer.get_max_score(NO_MORE_DOCS)?, true)
    };
    let has_tpi = req_scorer.has_two_phase_iterator() == TwoPhaseState::Yes
      || opt_scorer.has_two_phase_iterator() == TwoPhaseState::Yes;
    let approximation = DocIdSetIteratorImpl::new(req_scorer, opt_scorer, req_max_score, wrapper)?;
    match has_tpi {
      true => Ok(Self {
        disi: DocIdSetIteratorEnum2::B(TwoPhaseIteratorAsDocIdSetIterator::new(
          TwoPhaseIteratorImpl::new(approximation),
        )),
        tpi_state: TwoPhaseState::Yes,
      }),
      false => Ok(Self {
        disi: DocIdSetIteratorEnum2::A(approximation),
        tpi_state: TwoPhaseState::No,
      }),
    }
  }
  #[cfg(test)]
  pub(crate) fn with_fixed_max_score(
    req_scorer: S1,
    opt_scorer: S2,
    score_mode: ScoreMode,
  ) -> Result<Self> {
    let mut v = Self::new(req_scorer, opt_scorer, score_mode)?;
    match v.disi {
      DocIdSetIteratorEnum2::A(ref mut disi) => disi.fixed_max_score = true,
      DocIdSetIteratorEnum2::B(ref mut wrapper) => {
        wrapper.two_phase_iterator.disi.fixed_max_score = true
      },
    }
    Ok(v)
  }
}

impl<S1, S2> Scorable for ReqOptSumScorer<S1, S2>
where
  S1: Scorer + 'static,
  S2: Scorer + 'static,
{
  fn score(&mut self) -> Result<f32> {
    match self.disi {
      DocIdSetIteratorEnum2::A(ref mut disi) => disi.score(),
      DocIdSetIteratorEnum2::B(ref mut wrapper) => wrapper.two_phase_iterator.disi.score(),
    }
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    match self.disi {
      DocIdSetIteratorEnum2::A(ref mut disi) => disi.set_min_competitive_score(min_score),
      DocIdSetIteratorEnum2::B(ref mut wrapper) => wrapper
        .two_phase_iterator
        .disi
        .set_min_competitive_score(min_score),
    }
  }

  fn cost(&self) -> Result<i64> {
    self.iterator().cost()
  }
}

impl<S1, S2> crate::core::search::scorable::FixedScore for ReqOptSumScorer<S1, S2> {}

impl<S1, S2> Scorer for ReqOptSumScorer<S1, S2>
where
  S1: Scorer + 'static,
  S2: Scorer + 'static,
{
  fn doc_id(&mut self) -> Result<i32> {
    match self.disi {
      DocIdSetIteratorEnum2::A(ref mut disi) => disi.req_scorer.doc_id(),
      DocIdSetIteratorEnum2::B(ref mut wrapper) => {
        wrapper.two_phase_iterator.disi.req_scorer.doc_id()
      },
    }
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    match &self.disi {
      DocIdSetIteratorEnum2::A(v) => Box::new(v),
      DocIdSetIteratorEnum2::B(v) => Box::new(v),
    }
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    match &mut self.disi {
      DocIdSetIteratorEnum2::A(v) => Box::new(v),
      DocIdSetIteratorEnum2::B(v) => Box::new(v),
    }
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let ReqOptSumScorer { disi, .. } = *self;
    match disi {
      DocIdSetIteratorEnum2::A(v) => Box::new(v),
      DocIdSetIteratorEnum2::B(v) => Box::new(v),
    }
  }

  fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    match self.tpi_state {
      TwoPhaseState::No => None,
      _ => match &self.disi {
        DocIdSetIteratorEnum2::A(_) => {
          debug_assert!(false, "should not be here");
          None
        },
        DocIdSetIteratorEnum2::B(wrapper) => Some(Box::new(&wrapper.two_phase_iterator)),
      },
    }
  }

  fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    match self.tpi_state {
      TwoPhaseState::No => None,
      _ => match &mut self.disi {
        DocIdSetIteratorEnum2::A(_) => {
          debug_assert!(false, "should not be here");
          None
        },
        DocIdSetIteratorEnum2::B(wrapper) => Some(Box::new(&mut wrapper.two_phase_iterator)),
      },
    }
  }

  fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>>
  where
    Self: Sized,
  {
    let ReqOptSumScorer {
      disi, tpi_state, ..
    } = *self;
    match tpi_state {
      TwoPhaseState::No => None,
      _ => match disi {
        DocIdSetIteratorEnum2::A(_) => {
          debug_assert!(false, "should not be here");
          None
        },
        DocIdSetIteratorEnum2::B(wrapper) => Some(Box::new(wrapper.two_phase_iterator)),
      },
    }
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    match self.disi {
      DocIdSetIteratorEnum2::A(ref mut disi) => disi.advance_shallow(target),
      DocIdSetIteratorEnum2::B(ref mut wrapper) => {
        wrapper.two_phase_iterator.disi.advance_shallow(target)
      },
    }
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    match self.disi {
      DocIdSetIteratorEnum2::A(ref mut disi) => disi.get_max_score(upto),
      DocIdSetIteratorEnum2::B(ref mut wrapper) => {
        wrapper.two_phase_iterator.disi.get_max_score(upto)
      },
    }
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    self.tpi_state
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    match self.tpi_state {
      TwoPhaseState::No => self.iterator_mut(),
      _ => match self.disi {
        DocIdSetIteratorEnum2::A(_) => self.iterator_mut(),
        DocIdSetIteratorEnum2::B(ref mut wrapper) => wrapper.two_phase_iterator.approximation_mut(),
      },
    }
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    match self.tpi_state {
      TwoPhaseState::No => self.iterator(),
      _ => match self.disi {
        DocIdSetIteratorEnum2::A(_) => self.iterator(),
        DocIdSetIteratorEnum2::B(ref wrapper) => wrapper.two_phase_iterator.approximation(),
      },
    }
  }
  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    ScorerKind::ReqOptSum
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
  fn new(req_scorer: S1, opt_scorer: S2, req_max_score: f32, wrapper: bool) -> Result<Self> {
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
    Ok(disi)
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

    let opt_doc = self.opt_scorer.doc_id()?;

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

    if self.opt_scorer.doc_id()? <= upto {
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

      {
        if ScorerUtil::doc_id(&self.req_scorer) < req_doc {
          req_doc = ScorerUtil::advance(&mut self.req_scorer, req_doc)?;
        }
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

      loop {
        let mut opt_doc = ScorerUtil::doc_id(&self.opt_scorer);

        if opt_doc < req_doc {
          opt_doc = ScorerUtil::advance(&mut self.opt_scorer, req_doc)?;
        }

        if opt_doc > upper_bound {
          req_doc = upper_bound + 1;
          continue 'advance_head;
        }

        if opt_doc != req_doc {
          req_doc = ScorerUtil::advance(&mut self.req_scorer, opt_doc)?;
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
    let cur_doc = self.req_scorer.doc_id()?;
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
  fn new(disi: DocIdSetIteratorImpl<S1, S2>) -> Self {
    Self { disi }
  }
}
impl<S1, S2> TwoPhaseIterator for TwoPhaseIteratorImpl<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.disi)
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.disi)
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
      let req_doc = self.disi.req_scorer.doc_id()?;
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
        let matches = {
          let mut tpi = self.disi.opt_scorer.two_phase_iterator_mut();
          tpi.as_mut().unwrap().matches()?
        };
        if !matches {
          // Advance the iterator to make it clear it doesn't match the current doc id
          ScorerUtil::next_doc(&mut self.disi.opt_scorer)?;
          return Ok(false);
        }
      } else if opt_doc == req_doc
        && !self
          .disi
          .opt_scorer
          .two_phase_iterator_mut()
          .as_mut()
          .unwrap()
          .matches()?
      {
        // Advance the iterator to make it clear it doesn't match the current doc id
        ScorerUtil::next_doc(&mut self.disi.opt_scorer)?;
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
