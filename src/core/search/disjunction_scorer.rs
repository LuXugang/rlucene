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

/// A disjunction whose iterator implementation is selected during construction.
pub struct DisjunctionScorer<D: DocIdSetIterator, T> {
  disi: D,
  sub: T,
}

impl<S: Scorer, T> DisjunctionScorer<DisjunctionDISIApproximation<S>, T> {
  /// Prepare the shared queue and choose the iterator before entering a document loop.
  pub(crate) fn prepare(
    sub_scorers: Vec<S>,
    score_mode: ScoreMode,
  ) -> Result<
    DocIdSetIteratorEnum2<
      DisjunctionDISIApproximation<S>,
      TwoPhaseIteratorAsDocIdSetIterator<TwoPhase<S>>,
    >,
  > {
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
    let mut approximation = DisjunctionDISIApproximation::new(dpq, all_scorers)?;
    let needs_scores = score_mode != ScoreMode::CompleteNoScores;
    let mut has_approximation = false;
    let mut sum_match_cost = 0f32;
    let mut sum_approx_cost = 0i64;
    // Compute matchCost as the average over the matchCost of the subScorers.
    // This is weighted by the cost, which is an expected number of matching documents.
    let (sub_iterators, all_scores) = approximation.sub_iterators_and_all_scores_mut();
    for idx in sub_iterators.iter() {
      let w = &mut all_scores[idx];
      let cost_weight = if w.cost <= 1 { 1 } else { w.cost };
      sum_approx_cost += cost_weight;
      if w.scorer.has_two_phase_iterator() == TwoPhaseState::Yes
        || w.scorer.two_phase_iterator().is_some()
      {
        has_approximation = true;
        sum_match_cost += w.match_cost * cost_weight as f32;
      }
    }
    if !has_approximation {
      Ok(DocIdSetIteratorEnum2::A(approximation))
    } else {
      let match_cost = sum_match_cost / sum_approx_cost as f32;
      let two_phase = TwoPhase::new(approximation, match_cost, needs_scores)?;
      Ok(DocIdSetIteratorEnum2::B(as_doc_id_set_iterator(two_phase)))
    }
  }

  pub(crate) fn from_disi(disi: DisjunctionDISIApproximation<S>, sub: T) -> Self {
    Self { disi, sub }
  }
}
impl<S: Scorer, T> DisjunctionScorer<TwoPhaseIteratorAsDocIdSetIterator<TwoPhase<S>>, T> {
  pub(crate) fn from_tpi(disi: TwoPhaseIteratorAsDocIdSetIterator<TwoPhase<S>>, sub: T) -> Self {
    Self { disi, sub }
  }
}

/// Static access to the queue and matching state owned by the selected iterator.
#[doc(hidden)]
pub trait DisjunctionState: DocIdSetIterator + 'static {
  type SubScorer: Scorer + 'static;
  fn get_sub_matched(&mut self) -> Result<Option<usize>>;
  fn all_scores_mut(&mut self) -> &mut [DisiWrapper<Self::SubScorer>];
  fn approximation(&self) -> &DisjunctionDISIApproximation<Self::SubScorer>;
  fn approximation_mut(&mut self) -> &mut DisjunctionDISIApproximation<Self::SubScorer>;
  fn two_phase_iterator(&self) -> Option<&TwoPhase<Self::SubScorer>>;
  fn two_phase_iterator_mut(&mut self) -> Option<&mut TwoPhase<Self::SubScorer>>;
  fn take_two_phase_iterator(self) -> Option<TwoPhase<Self::SubScorer>>;
  fn has_two_phase_iterator(&self) -> TwoPhaseState;
}
impl<S: Scorer + 'static> DisjunctionState for DisjunctionDISIApproximation<S> {
  type SubScorer = S;
  fn get_sub_matched(&mut self) -> Result<Option<usize>> {
    Ok(Some(self.top_list_root()))
  }
  fn all_scores_mut(&mut self) -> &mut [DisiWrapper<S>] {
    self.all_scores_mut()
  }
  fn approximation(&self) -> &Self {
    self
  }
  fn approximation_mut(&mut self) -> &mut Self {
    self
  }
  fn two_phase_iterator(&self) -> Option<&TwoPhase<S>> {
    None
  }
  fn two_phase_iterator_mut(&mut self) -> Option<&mut TwoPhase<S>> {
    None
  }
  fn take_two_phase_iterator(self) -> Option<TwoPhase<S>> {
    None
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }
}
impl<S: Scorer + 'static> DisjunctionState for TwoPhaseIteratorAsDocIdSetIterator<TwoPhase<S>> {
  type SubScorer = S;
  fn get_sub_matched(&mut self) -> Result<Option<usize>> {
    self.two_phase_iterator.get_sub_matches()
  }
  fn all_scores_mut(&mut self) -> &mut [DisiWrapper<S>] {
    self.two_phase_iterator.all_scores_mut()
  }
  fn approximation(&self) -> &DisjunctionDISIApproximation<S> {
    self.two_phase_iterator.approximation()
  }
  fn approximation_mut(&mut self) -> &mut DisjunctionDISIApproximation<S> {
    self.two_phase_iterator.approximation_mut()
  }
  fn two_phase_iterator(&self) -> Option<&TwoPhase<S>> {
    Some(&self.two_phase_iterator)
  }
  fn two_phase_iterator_mut(&mut self) -> Option<&mut TwoPhase<S>> {
    Some(&mut self.two_phase_iterator)
  }
  fn take_two_phase_iterator(self) -> Option<TwoPhase<S>> {
    Some(self.two_phase_iterator)
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::Yes
  }
}

impl<D: DisjunctionState, T: DisjunctionScorerBase> Scorable for DisjunctionScorer<D, T> {
  type Child<'a>
    = crate::core::search::dummy::dummy_scorable::DummyScorable
  where
    Self: 'a;
  fn score(&mut self) -> Result<f32> {
    let idx = self.disi.get_sub_matched()?;
    self.sub.score(self.disi.all_scores_mut(), idx)
  }
  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    self
      .sub
      .set_min_competitive_score(min_score, self.disi.all_scores_mut())
  }
  fn get_children(&mut self) -> Result<Vec<ChildScorable<Self::Child<'_>>>> {
    todo!()
  }
  fn cost(&self) -> Result<i64> {
    self.disi.cost()
  }
}
impl<D: DocIdSetIterator, T> crate::core::search::scorable::FixedScore for DisjunctionScorer<D, T> {}
impl<D: DocIdSetIterator, T> DocIdSetIterator for DisjunctionScorer<D, T> {
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
impl<D: DocIdSetIterator, T> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for DisjunctionScorer<D, T>
{
}
impl<D: DocIdSetIterator, T> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for DisjunctionScorer<D, T>
{
}
impl<D: DisjunctionState, T: DisjunctionScorerBase> Scorer for DisjunctionScorer<D, T> {
  type Iterator = D;
  type OwnedTwoPhase = TwoPhase<D::SubScorer>;
  type ApproximationRef<'a>
    = &'a DisjunctionDISIApproximation<D::SubScorer>
  where
    Self: 'a;
  type ApproximationMut<'a>
    = &'a mut DisjunctionDISIApproximation<D::SubScorer>
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
    Ok(self.disi.approximation().doc_id())
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
    match self.sub.advance_shallow(target, self.disi.all_scores_mut()) {
      Ok(doc) => Ok(doc),
      Err(e) => match e {
        LuceneError::NotImplemented(_) => self.default_advance_shallow(target),
        _ => Err(e),
      },
    }
  }
  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    self.sub.get_max_score(upto, self.disi.all_scores_mut())
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
    ScorerKind::Disjunction
  }
}

pub struct TwoPhase<S> {
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
    let size = cmp.approximation.all_scores().len();
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
    for heap_index in 1..=self.unverified_matches.size() {
      let i = *self.unverified_matches.get_heap_array()[heap_index]
        .as_ref()
        .ok_or_else(|| LuceneError::illegal_state("priority queue element should exist"))?;
      let w = &mut self
        .unverified_matches
        .compare
        .approximation
        .all_scores_mut()[i];

      if w.matches()? {
        w.next = self.verified_matches;

        self.verified_matches = Some(i);
      }
    }
    self.unverified_matches.clear();
    Ok(self.verified_matches)
  }
  fn all_scores_mut(&mut self) -> &mut [DisiWrapper<S>] {
    self
      .unverified_matches
      .compare
      .approximation
      .all_scores_mut()
  }
}
impl<S> TwoPhaseIterator for TwoPhase<S>
where
  S: Scorer,
{
  type ApproximationRef<'a>
    = &'a DisjunctionDISIApproximation<S>
  where
    Self: 'a;
  type ApproximationMut<'a>
    = &'a mut DisjunctionDISIApproximation<S>
  where
    Self: 'a;

  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    &mut self.unverified_matches.compare.approximation
  }

  fn approximation(&self) -> Self::ApproximationRef<'_> {
    &self.unverified_matches.compare.approximation
  }

  fn matches(&mut self) -> Result<bool> {
    self.verified_matches = None;
    self.unverified_matches.clear();

    let root_idx = self
      .unverified_matches
      .compare
      .approximation
      .top_list_root();
    let mut w_idx_opt = Some(root_idx);
    while let Some(w_idx) = w_idx_opt {
      let w = &mut self
        .unverified_matches
        .compare
        .approximation
        .all_scores_mut()[w_idx];
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

struct DisiWrapperCmp<S> {
  approximation: DisjunctionDISIApproximation<S>,
}
impl<S> Compare<usize> for DisiWrapperCmp<S>
where
  S: Scorer,
{
  fn less_than(&self, a: &usize, b: &usize) -> Result<bool> {
    Ok(
      self.approximation.all_scores()[*a].match_cost
        < self.approximation.all_scores()[*b].match_cost,
    )
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
