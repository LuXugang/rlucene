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

use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::search::approximation_disi::ApproximationBitSetAccess;
use crate::core::search::approximation_disi::ApproximationDISI;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::phrase_matcher::PhraseScorerMatcher;
use crate::core::search::scorable::Scorable;
#[cfg(test)]
use crate::core::search::scorer::ScorerKind;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::search::two_phase_iterator::{
  TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
};
use crate::core::util::error::lucene_error::Result;
#[cfg(test)]
use std::any::Any;
pub type Disi<M, SS, N, const TOP_SCORES: bool = false> =
  TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<M, SS, N, TOP_SCORES>>;
pub struct PhraseScorer<M, SS, N, const TOP_SCORES: bool = false> {
  pub(crate) disi: Disi<M, SS, N, TOP_SCORES>,
}
impl<M, SS, N, const TOP_SCORES: bool> PhraseScorer<M, SS, N, TOP_SCORES>
where
  M: PhraseScorerMatcher,
  SS: SimScorer,
  N: NumericDocValues,
  TwoPhaseIteratorImpl<M, SS, N, TOP_SCORES>: TwoPhaseIterator,
{
  pub(crate) fn new(matcher: M, sim_scorer: SS, norms: Option<N>) -> Self {
    let v = TwoPhaseIteratorImpl::new(matcher, sim_scorer, norms);
    let disi = TwoPhaseIteratorAsDocIdSetIterator::new(v);
    Self { disi }
  }
}

impl<M, SS, N, const TOP_SCORES: bool> Scorable for PhraseScorer<M, SS, N, TOP_SCORES>
where
  M: PhraseScorerMatcher + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
  TwoPhaseIteratorImpl<M, SS, N, TOP_SCORES>: TwoPhaseIterator,
{
  type Child<'a>
    = crate::core::search::dummy::dummy_scorable::DummyScorable
  where
    Self: 'a;

  fn score(&mut self) -> Result<f32> {
    {
      let tpi = &mut self.disi.two_phase_iterator;
      if tpi.freq == 0.0 {
        tpi.freq = tpi.matcher.sloppy_weight();
        while tpi.matcher.next_match()? {
          tpi.freq += tpi.matcher.sloppy_weight();
        }
      }
    }

    let mut norm: i64 = 1;
    let doc_id = self.disi.doc_id();
    let tpi = &mut self.disi.two_phase_iterator;
    if let Some(norms) = tpi.norms.as_mut()
      && norms.advance_exact(doc_id)?
    {
      norm = norms.long_value()?;
    }

    Ok(tpi.sim_scorer.score(tpi.freq, norm))
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    self.disi.two_phase_iterator.min_competitive_score = min_score;
    self
      .disi
      .two_phase_iterator
      .matcher
      .impacts_approximation_mut()
      .set_min_competitive_score(min_score);
    Ok(())
  }

  fn cost(&self) -> Result<i64> {
    self.disi.cost()
  }
  #[cfg(test)]
  fn as_any(&mut self) -> &mut dyn Any {
    self
  }
}

impl<M, SS, N, const TOP_SCORES: bool> crate::core::search::scorable::FixedScore
  for PhraseScorer<M, SS, N, TOP_SCORES>
{
}

impl<M, SS, N, const TOP_SCORES: bool> DocIdSetIterator for PhraseScorer<M, SS, N, TOP_SCORES>
where
  M: PhraseScorerMatcher + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
  TwoPhaseIteratorImpl<M, SS, N, TOP_SCORES>: TwoPhaseIterator,
{
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
impl<M, SS, N, const TOP_SCORES: bool>
  crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for PhraseScorer<M, SS, N, TOP_SCORES>
where
  M: PhraseScorerMatcher + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
  TwoPhaseIteratorImpl<M, SS, N, TOP_SCORES>: TwoPhaseIterator,
{
}
impl<M, SS, N, const TOP_SCORES: bool>
  crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for PhraseScorer<M, SS, N, TOP_SCORES>
where
  M: PhraseScorerMatcher + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
  TwoPhaseIteratorImpl<M, SS, N, TOP_SCORES>: TwoPhaseIterator,
{
}
impl<M, SS, N, const TOP_SCORES: bool> ApproximationDISI for PhraseScorer<M, SS, N, TOP_SCORES>
where
  M: PhraseScorerMatcher + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
  TwoPhaseIteratorImpl<M, SS, N, TOP_SCORES>: TwoPhaseIterator,
{
  fn approx_doc_id(&self) -> i32 {
    self.disi.two_phase_iterator.approx_doc_id()
  }
  #[inline]
  fn approx_next_doc(&mut self) -> Result<i32> {
    self.disi.two_phase_iterator.approx_next_doc()
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    self.disi.two_phase_iterator.approx_advance(target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.disi.two_phase_iterator.approx_slow_advance(target)
  }
  fn approx_cost(&self) -> Result<i64> {
    self.disi.two_phase_iterator.approx_cost()
  }
}
impl<M, SS, N, const TOP_SCORES: bool> ApproximationBitSetAccess
  for PhraseScorer<M, SS, N, TOP_SCORES>
where
  M: PhraseScorerMatcher + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
  TwoPhaseIteratorImpl<M, SS, N, TOP_SCORES>: TwoPhaseIterator,
{
  fn approx_is_bit_iter(&self) -> bool {
    self.disi.two_phase_iterator.approx_is_bit_iter()
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    self.disi.two_phase_iterator.approx_get(index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.disi.two_phase_iterator.approx_set_doc_id(doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    self.disi.two_phase_iterator.approx_bit_set_length()
  }
}

impl<M, SS, N, const TOP_SCORES: bool> TwoPhaseIterator for PhraseScorer<M, SS, N, TOP_SCORES>
where
  M: PhraseScorerMatcher + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
  TwoPhaseIteratorImpl<M, SS, N, TOP_SCORES>: TwoPhaseIterator,
{
  fn matches(&mut self) -> Result<bool> {
    self.disi.two_phase_iterator.matches()
  }
  fn match_cost(&self) -> f32 {
    self.disi.two_phase_iterator.match_cost()
  }
}

impl<M, SS, N, const TOP_SCORES: bool> Scorer for PhraseScorer<M, SS, N, TOP_SCORES>
where
  M: PhraseScorerMatcher + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
  TwoPhaseIteratorImpl<M, SS, N, TOP_SCORES>: TwoPhaseIterator,
{
  fn scoring_doc_id(&mut self) -> Result<i32> {
    Ok(self.disi.two_phase_iterator.doc_id())
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    self
      .disi
      .two_phase_iterator
      .matcher
      .impacts_approximation_mut()
      .max_score_cache_mut()
      .advance_shallow(target)
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    self
      .disi
      .two_phase_iterator
      .matcher
      .impacts_approximation_mut()
      .max_score_cache_mut()
      .get_max_score(upto)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::Yes
  }

  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    ScorerKind::Phrase
  }
}

pub struct TwoPhaseIteratorImpl<M, SS, N, const TOP_SCORES: bool = false> {
  pub(crate) matcher: M,
  sim_scorer: SS,
  norms: Option<N>,
  match_cost: f32,
  freq: f32,
  min_competitive_score: f32,
}
impl<M, SS, N, const TOP_SCORES: bool> TwoPhaseIteratorImpl<M, SS, N, TOP_SCORES>
where
  M: PhraseScorerMatcher,
  SS: SimScorer,
  N: NumericDocValues,
{
  fn new(matcher: M, sim_scorer: SS, norms: Option<N>) -> Self {
    let match_cost = matcher.get_match_cost();
    Self {
      matcher,
      sim_scorer,
      norms,
      match_cost,
      freq: 0.0,
      min_competitive_score: 0.0,
    }
  }

  fn doc_id(&self) -> i32
  where
    Self: TwoPhaseIterator,
  {
    self.approx_doc_id()
  }
}
impl<M, SS, N, const TOP_SCORES: bool> ApproximationDISI
  for TwoPhaseIteratorImpl<M, SS, N, TOP_SCORES>
where
  M: PhraseScorerMatcher,
  SS: SimScorer,
  N: NumericDocValues,
{
  fn approx_doc_id(&self) -> i32 {
    self.matcher.approx_doc_id()
  }
  #[inline]
  fn approx_next_doc(&mut self) -> Result<i32> {
    self.matcher.approx_next_doc()
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    self.matcher.approx_advance(target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.matcher.approx_slow_advance(target)
  }
  fn approx_cost(&self) -> Result<i64> {
    self.matcher.approx_cost()
  }
}

impl<M, SS, N, const TOP_SCORES: bool> ApproximationBitSetAccess
  for TwoPhaseIteratorImpl<M, SS, N, TOP_SCORES>
where
  M: PhraseScorerMatcher,
  SS: SimScorer,
  N: NumericDocValues,
{
  fn approx_is_bit_iter(&self) -> bool {
    self.matcher.approx_is_bit_iter()
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    self.matcher.approx_get(index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.matcher.approx_set_doc_id(doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    self.matcher.approx_bit_set_length()
  }
}

impl<M, SS, N, const TOP_SCORES: bool> TwoPhaseIterator
  for TwoPhaseIteratorImpl<M, SS, N, TOP_SCORES>
where
  M: PhraseScorerMatcher,
  SS: SimScorer,
  N: NumericDocValues,
{
  fn matches(&mut self) -> Result<bool> {
    self.matcher.reset()?;

    if TOP_SCORES && self.min_competitive_score > 0.0 {
      let max_freq = self.matcher.max_freq()?;

      let mut norm: i64 = 1;
      let doc_id = self.doc_id();
      if let Some(norms) = self.norms.as_mut()
        && norms.advance_exact(doc_id)?
      {
        norm = norms.long_value()?;
      }

      if self.sim_scorer.score(max_freq, norm) < self.min_competitive_score {
        // The maximum score we could get is less than the min competitive score
        return Ok(false);
      }
    }

    self.freq = 0.0;
    self.matcher.next_match()
  }

  fn match_cost(&self) -> f32 {
    self.match_cost
  }
}
