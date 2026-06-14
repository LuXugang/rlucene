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
use crate::core::search::disi_priority_queue::DisiPriorityQueue;
use crate::core::search::disi_wrapper::DisiWrapper;
use crate::core::search::disjunction_disi_approximation::DisjunctionDISIApproximation;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum2};
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::search::score_mode::ScoreMode;
#[cfg(test)]
use crate::core::search::scorer::ScorerKind;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::{
  TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator, as_doc_id_set_iterator,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};

pub type Disi<S> = DocIdSetIteratorEnum2<
  DisjunctionDISIApproximation<S>,
  TwoPhaseIteratorAsDocIdSetIterator<TwoPhase<S>>,
>;
/// Base trait for scorers that score disjunctions.
pub struct DisjunctionScorer<S, T>
where
  S: Scorer,
  T: DisjunctionScorerBase,
{
  disi: Disi<S>,
  sub: T,
  tpi_state: TwoPhaseState,
}

impl<S, T> DisjunctionScorer<S, T>
where
  S: Scorer,
  T: DisjunctionScorerBase,
{
  pub(crate) fn new(sub_scorers: Vec<S>, score_mode: ScoreMode, sub: T) -> Result<Self> {
    let sub_scorers_len = sub_scorers.len();
    if sub_scorers_len <= 1 {
      return Err(LuceneError::illegal_argument(
        "There must be at least 2 subScorers",
      ));
    }
    let mut dpq = DisiPriorityQueue::new(sub_scorers_len);
    let mut all_scorers = Vec::with_capacity(sub_scorers_len);
    for (i, scorer) in sub_scorers.into_iter().enumerate() {
      let w = DisiWrapper::new(scorer)?;
      all_scorers.push(w);
      dpq.add(i, all_scorers.as_slice());
    }
    let mut approximation = DisjunctionDISIApproximation::new(dpq, all_scorers);
    let needs_scores = score_mode != ScoreMode::CompleteNoScores;
    let mut has_approximation = false;
    let mut sum_match_cost = 0f32;
    let mut sum_approx_cost = 0i64;
    // Compute matchCost as the average over the matchCost of the subScorers.
    // This is weighted by the cost, which is an expected number of matching documents.
    for idx in approximation.sub_iterators.iter() {
      let w = &mut approximation.all_scores[idx];
      let cost_weight = if w.cost <= 1 { 1 } else { w.cost };
      sum_approx_cost += cost_weight;
      if w.scorer.has_two_phase_iterator() == TwoPhaseState::Yes
        || w.scorer.two_phase_iterator().is_some()
      {
        has_approximation = true;
        sum_match_cost += w.match_cost * cost_weight as f32;
      }
    }
    let (disi, tpi_state) = if !has_approximation {
      (Disi::A(approximation), TwoPhaseState::No)
    } else {
      let match_cost = sum_match_cost / sum_approx_cost as f32;
      let two_phase = TwoPhase::new(approximation, match_cost, needs_scores)?;
      let v = as_doc_id_set_iterator(two_phase);
      (Disi::B(v), TwoPhaseState::Yes)
    };
    Ok(Self {
      disi,
      sub,
      tpi_state,
    })
  }

  fn get_sub_matched(&mut self) -> Result<Option<usize>> {
    match self.disi {
      Disi::A(ref mut v) => Ok(Some(v.sub_iterators.top_list_root(&mut v.all_scores))),
      Disi::B(ref mut v) => {
        let two_phase = &mut v.two_phase_iterator;
        two_phase.get_sub_matches()
      },
    }
  }
}

impl<S, T> Scorable for DisjunctionScorer<S, T>
where
  S: Scorer + 'static,
  T: DisjunctionScorerBase,
{
  fn score(&mut self) -> Result<f32> {
    let idx = self.get_sub_matched()?;
    match self.disi {
      Disi::A(ref mut v) => self.sub.score(&mut v.all_scores, idx),
      Disi::B(ref mut v) => self.sub.score(
        v.two_phase_iterator
          .unverified_matches
          .compare
          .approximation
          .all_scores
          .as_mut_slice(),
        idx,
      ),
    }
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    match self.disi {
      Disi::A(ref mut v) => self
        .sub
        .set_min_competitive_score(min_score, &mut v.all_scores),
      Disi::B(ref mut v) => self.sub.set_min_competitive_score(
        min_score,
        v.two_phase_iterator
          .unverified_matches
          .compare
          .approximation
          .all_scores
          .as_mut_slice(),
      ),
    }
  }

  fn get_children(&self) -> Result<Vec<ChildScorable<Box<dyn Scorable>>>> {
    todo!()
  }

  fn cost(&self) -> Result<i64> {
    self.iterator().cost()
  }
}

impl<S, T> crate::core::search::scorable::FixedScore for DisjunctionScorer<S, T>
where
  S: Scorer + 'static,
  T: DisjunctionScorerBase,
{
}

impl<S, T> Scorer for DisjunctionScorer<S, T>
where
  S: Scorer + 'static,
  T: DisjunctionScorerBase,
{
  fn doc_id(&mut self) -> Result<i32> {
    match self.disi {
      Disi::A(ref v) => Ok(v.doc_id()),
      Disi::B(ref v) => {
        let approximation = &v
          .two_phase_iterator
          .unverified_matches
          .compare
          .approximation;
        Ok(approximation.doc_id())
      },
    }
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    match &self.disi {
      Disi::A(v) => Box::new(v),
      Disi::B(v) => Box::new(v),
    }
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    match &mut self.disi {
      Disi::A(v) => Box::new(v),
      Disi::B(v) => Box::new(v),
    }
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let DisjunctionScorer { disi, .. } = *self;
    match disi {
      Disi::A(v) => Box::new(v),
      Disi::B(v) => Box::new(v),
    }
  }

  fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    match self.tpi_state {
      TwoPhaseState::No => None,
      _ => match self.disi {
        Disi::B(ref v) => Some(Box::new(&v.two_phase_iterator)),
        _ => {
          debug_assert!(false, "should not be here");
          None
        },
      },
    }
  }

  fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    match self.tpi_state {
      TwoPhaseState::No => None,
      _ => match self.disi {
        Disi::B(ref mut v) => Some(Box::new(&mut v.two_phase_iterator)),
        _ => {
          debug_assert!(false, "should not be here");
          None
        },
      },
    }
  }

  fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>> {
    let DisjunctionScorer {
      disi, tpi_state, ..
    } = *self;
    match tpi_state {
      TwoPhaseState::No => None,
      _ => match disi {
        Disi::B(v) => Some(Box::new(v.two_phase_iterator)),
        _ => {
          debug_assert!(false, "should not be here");
          None
        },
      },
    }
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    match self.disi {
      Disi::A(ref mut v) => match self.sub.advance_shallow(target, &mut v.all_scores) {
        Ok(doc) => Ok(doc),
        Err(e) => match e {
          LuceneError::NotImplemented(_) => self.default_advance_shallow(target),
          _ => Err(e),
        },
      },
      Disi::B(ref mut v) => {
        match self.sub.advance_shallow(
          target,
          v.two_phase_iterator
            .unverified_matches
            .compare
            .approximation
            .all_scores
            .as_mut_slice(),
        ) {
          Ok(doc) => Ok(doc),
          Err(e) => match e {
            LuceneError::NotImplemented(_) => self.default_advance_shallow(target),
            _ => Err(e),
          },
        }
      },
    }
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    match self.disi {
      Disi::A(ref mut v) => self.sub.get_max_score(upto, &mut v.all_scores),
      Disi::B(ref mut v) => self.sub.get_max_score(
        upto,
        v.two_phase_iterator
          .unverified_matches
          .compare
          .approximation
          .all_scores
          .as_mut_slice(),
      ),
    }
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    self.tpi_state
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    match self.tpi_state {
      TwoPhaseState::No => self.iterator_mut(),
      _ => match self.disi {
        Disi::A(_) => self.iterator_mut(),
        Disi::B(ref mut v) => v.two_phase_iterator.approximation_mut(),
      },
    }
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    match self.tpi_state {
      TwoPhaseState::No => self.iterator(),
      _ => match self.disi {
        Disi::A(_) => self.iterator(),
        Disi::B(ref v) => v.two_phase_iterator.approximation(),
      },
    }
  }
  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    ScorerKind::Disjunction
  }
}

pub struct TwoPhase<S>
where
  S: Scorer,
{
  match_cost: f32,
  // list of verified matches on the current doc
  verified_matches: Option<usize>,
  // priority queue of approximations on the current doc that have not been verified yet
  unverified_matches: PriorityQueue<usize, DisiWrapperCmp<S>>,
  needs_scores: bool,
}
impl<S> TwoPhase<S>
where
  S: Scorer,
{
  fn new(
    approximation: DisjunctionDISIApproximation<S>,
    match_cost: f32,
    needs_scores: bool,
  ) -> Result<Self> {
    let cmp = DisiWrapperCmp { approximation };
    let size = cmp.approximation.all_scores.len();
    let unverified_matches = PriorityQueue::new(size, cmp)?;
    Ok(Self {
      match_cost,
      verified_matches: None,
      unverified_matches,
      needs_scores,
    })
  }
  pub(crate) fn get_sub_matches(&mut self) -> Result<Option<usize>> {
    // iteration order does not matter
    // TODO IMPORTANT Due to borrow checker, we have to collect first, could we avoid it?
    let v: Vec<usize> = self.unverified_matches.iter_ref().cloned().collect();
    for i in v {
      let w = &mut self.unverified_matches.compare.approximation.all_scores[i];

      if w.matches()? {
        w.next = self.verified_matches;

        self.verified_matches = Some(i);
      }
    }
    self.unverified_matches.clear();
    Ok(self.verified_matches)
  }
  fn all_scores_ref(&mut self) -> &[DisiWrapper<S>] {
    &self.unverified_matches.compare.approximation.all_scores
  }
  fn all_scores_mut(&mut self) -> &mut [DisiWrapper<S>] {
    &mut self.unverified_matches.compare.approximation.all_scores
  }
}
impl<S> TwoPhaseIterator for TwoPhase<S>
where
  S: Scorer,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.unverified_matches.compare.approximation)
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.unverified_matches.compare.approximation)
  }

  fn matches(&mut self) -> Result<bool> {
    self.verified_matches = None;
    self.unverified_matches.clear();

    let root_idx = {
      self
        .unverified_matches
        .compare
        .approximation
        .sub_iterators
        .top_list_root(&mut self.unverified_matches.compare.approximation.all_scores)
    };
    let mut w_idx_opt = Some(root_idx);
    while w_idx_opt.is_some() {
      let w_idx = w_idx_opt.unwrap();
      let w = &mut self.unverified_matches.compare.approximation.all_scores[w_idx];
      let next = w.next;
      let has_no_two_phase_view = (w.scorer.has_two_phase_iterator() == TwoPhaseState::No)
        || w.scorer.two_phase_iterator().is_none();
      if has_no_two_phase_view {
        // implicitly verified, move it to verifiedMatches
        w.next = self.verified_matches;
        self.verified_matches = Some(w_idx);

        if !self.needs_scores {
          // we can stop here
          return Ok(true);
        }
      } else {
        self.unverified_matches.add(w_idx)?;
      }
      w_idx_opt = next;
    }

    if self.verified_matches.is_some() {
      return Ok(true);
    }

    // verify subs that have an two-phase iterator
    // least-costly ones first
    while let Some(w_idx) = self.unverified_matches.pop()? {
      let w = &mut self.all_scores_mut()[w_idx];

      if w.matches()? {
        w.next = None;
        self.verified_matches = Some(w_idx);
        return Ok(true);
      }
    }

    Ok(false)
  }

  fn match_cost(&self) -> f32 {
    self.match_cost
  }
}

struct DisiWrapperCmp<S>
where
  S: Scorer,
{
  approximation: DisjunctionDISIApproximation<S>,
}
impl<S> Compare<usize> for DisiWrapperCmp<S>
where
  S: Scorer,
{
  fn less_than(&self, a: &usize, b: &usize) -> Result<bool> {
    Ok(self.approximation.all_scores[*a].match_cost < self.approximation.all_scores[*b].match_cost)
  }
}

pub trait DisjunctionScorerBase {
  fn score<S>(&self, disi_wrapper: &mut [DisiWrapper<S>], top_list: Option<usize>) -> Result<f32>
  where
    S: Scorer;
  fn advance_shallow<S>(&mut self, target: i32, disi_wrapper: &mut [DisiWrapper<S>]) -> Result<i32>
  where
    S: Scorer;
  fn get_max_score<S>(&mut self, upto: i32, disi_wrapper: &mut [DisiWrapper<S>]) -> Result<f32>
  where
    S: Scorer;
  fn set_min_competitive_score<S>(
    &mut self,
    min_score: f32,
    disi_wrapper: &mut [DisiWrapper<S>],
  ) -> Result<()>
  where
    S: Scorer;
}
