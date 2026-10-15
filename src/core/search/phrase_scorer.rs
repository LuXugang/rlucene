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

use crate::core::index::impacts_enum::ImpactsEnum;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::search::conjunction_disi::ConjunctionDISIEnum;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum2};
use crate::core::search::phrase_matcher::{PhraseMatcher, PhraseMatcherEnum};
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
pub type Disi<IE, SS, N, const TOP_SCORES: bool = false> =
  TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<IE, SS, N, TOP_SCORES>>;
pub struct PhraseScorer<IE, SS, N, const TOP_SCORES: bool = false> {
  pub(crate) disi: Disi<IE, SS, N, TOP_SCORES>,
}
impl<IE, SS, N, const TOP_SCORES: bool> PhraseScorer<IE, SS, N, TOP_SCORES>
where
  IE: ImpactsEnum,
  SS: SimScorer,
  N: NumericDocValues,
  TwoPhaseIteratorImpl<IE, SS, N, TOP_SCORES>: TwoPhaseIterator,
{
  pub(crate) fn new(matcher: PhraseMatcherEnum<IE, SS>, sim_scorer: SS, norms: Option<N>) -> Self {
    let v = TwoPhaseIteratorImpl::new(matcher, sim_scorer, norms);
    let disi = TwoPhaseIteratorAsDocIdSetIterator::new(v);
    Self { disi }
  }
}

impl<IE, SS, N, const TOP_SCORES: bool> Scorable for PhraseScorer<IE, SS, N, TOP_SCORES>
where
  IE: ImpactsEnum + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
  TwoPhaseIteratorImpl<IE, SS, N, TOP_SCORES>: TwoPhaseIterator,
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
    match self.disi.two_phase_iterator.matcher {
      PhraseMatcherEnum::Exact(ref mut m) => {
        m.impacts_approximation.set_min_competitive_score(min_score)
      },
      PhraseMatcherEnum::Sloppy(ref mut m) => {
        m.impacts_approximation.set_min_competitive_score(min_score)
      },
    }
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

impl<IE, SS, N, const TOP_SCORES: bool> crate::core::search::scorable::FixedScore
  for PhraseScorer<IE, SS, N, TOP_SCORES>
{
}

impl<IE, SS, N, const TOP_SCORES: bool> DocIdSetIterator for PhraseScorer<IE, SS, N, TOP_SCORES>
where
  IE: ImpactsEnum + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
  TwoPhaseIteratorImpl<IE, SS, N, TOP_SCORES>: TwoPhaseIterator,
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
impl<IE, SS, N, const TOP_SCORES: bool>
  crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for PhraseScorer<IE, SS, N, TOP_SCORES>
where
  IE: ImpactsEnum + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
  TwoPhaseIteratorImpl<IE, SS, N, TOP_SCORES>: TwoPhaseIterator,
{
}
impl<IE, SS, N, const TOP_SCORES: bool>
  crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for PhraseScorer<IE, SS, N, TOP_SCORES>
where
  IE: ImpactsEnum + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
  TwoPhaseIteratorImpl<IE, SS, N, TOP_SCORES>: TwoPhaseIterator,
{
}
impl<IE, SS, N, const TOP_SCORES: bool> Scorer for PhraseScorer<IE, SS, N, TOP_SCORES>
where
  IE: ImpactsEnum + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
  TwoPhaseIteratorImpl<IE, SS, N, TOP_SCORES>: TwoPhaseIterator,
{
  type Iterator = Disi<IE, SS, N, TOP_SCORES>;
  type OwnedTwoPhase = TwoPhaseIteratorImpl<IE, SS, N, TOP_SCORES>;
  type ApproximationRef<'a>
    = <TwoPhaseIteratorImpl<IE, SS, N, TOP_SCORES> as TwoPhaseIterator>::ApproximationRef<'a>
  where
    Self: 'a;
  type ApproximationMut<'a>
    = <TwoPhaseIteratorImpl<IE, SS, N, TOP_SCORES> as TwoPhaseIterator>::ApproximationMut<'a>
  where
    Self: 'a;
  type TwoPhaseRef<'a>
    = &'a TwoPhaseIteratorImpl<IE, SS, N, TOP_SCORES>
  where
    Self: 'a;
  type TwoPhaseMut<'a>
    = &'a mut TwoPhaseIteratorImpl<IE, SS, N, TOP_SCORES>
  where
    Self: 'a;

  fn scoring_doc_id(&mut self) -> Result<i32> {
    Ok(self.disi.two_phase_iterator.doc_id())
  }

  fn take_iterator(self) -> Self::Iterator {
    let PhraseScorer { disi, .. } = self;
    disi
  }

  fn two_phase_iterator(&self) -> Option<Self::TwoPhaseRef<'_>> {
    Some(&self.disi.two_phase_iterator)
  }

  fn two_phase_iterator_mut(&mut self) -> Option<Self::TwoPhaseMut<'_>> {
    Some(&mut self.disi.two_phase_iterator)
  }

  fn take_two_phase_iterator(self) -> Option<Self::OwnedTwoPhase> {
    let PhraseScorer { disi, .. } = self;
    Some(disi.two_phase_iterator)
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    match self.disi.two_phase_iterator.matcher {
      PhraseMatcherEnum::Exact(ref mut m) => m
        .impacts_approximation
        .max_score_cache_mut()
        .advance_shallow(target),
      PhraseMatcherEnum::Sloppy(ref mut m) => m
        .impacts_approximation
        .max_score_cache_mut()
        .advance_shallow(target),
    }
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    match self.disi.two_phase_iterator.matcher {
      PhraseMatcherEnum::Exact(ref mut m) => m
        .impacts_approximation
        .max_score_cache_mut()
        .get_max_score(upto),
      PhraseMatcherEnum::Sloppy(ref mut m) => m
        .impacts_approximation
        .max_score_cache_mut()
        .get_max_score(upto),
    }
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::Yes
  }

  fn approximation(&self) -> Self::ApproximationRef<'_> {
    self.disi.two_phase_iterator.approximation()
  }

  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    self.disi.two_phase_iterator.approximation_mut()
  }
  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    ScorerKind::Phrase
  }
}

pub struct TwoPhaseIteratorImpl<IE, SS, N, const TOP_SCORES: bool = false> {
  pub(crate) matcher: PhraseMatcherEnum<IE, SS>,
  sim_scorer: SS,
  norms: Option<N>,
  match_cost: f32,
  freq: f32,
  min_competitive_score: f32,
}
impl<IE, SS, N, const TOP_SCORES: bool> TwoPhaseIteratorImpl<IE, SS, N, TOP_SCORES>
where
  IE: ImpactsEnum,
  SS: SimScorer,
  N: NumericDocValues,
{
  fn new(matcher: PhraseMatcherEnum<IE, SS>, sim_scorer: SS, norms: Option<N>) -> Self {
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
    self.approximation().doc_id()
  }
}
macro_rules! impl_two_phase_iterator {
  ($top_scores:literal, ($ie:ident, $ss:ident, $n:ident, $a:lifetime), $approximation_ref:ty, $approximation_mut:ty,
   $this:ident, $get_ref:block, $get_mut:block) => {
    impl<$ie, $ss, $n> TwoPhaseIterator for TwoPhaseIteratorImpl<$ie, $ss, $n, $top_scores>
    where
      $ie: ImpactsEnum,
      $ss: SimScorer,
      $n: NumericDocValues,
    {
      type ApproximationRef<$a> = $approximation_ref where Self: $a;
      type ApproximationMut<$a> = $approximation_mut where Self: $a;
      fn approximation(&$this) -> Self::ApproximationRef<'_> $get_ref
      fn approximation_mut(&mut $this) -> Self::ApproximationMut<'_> $get_mut
      fn matches(&mut self) -> Result<bool> {
        self.matcher.reset()?;

        if $top_scores && self.min_competitive_score > 0.0 {
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
  };
}

impl_two_phase_iterator!(
  false,
  (IE, SS, N, 'a),
  &'a ConjunctionDISIEnum<IE>,
  &'a mut ConjunctionDISIEnum<IE>,
  self,
  {
    match &self.matcher {
      PhraseMatcherEnum::Exact(m) => m.approximation(),
      PhraseMatcherEnum::Sloppy(m) => m.approximation(),
    }
  },
  {
    match &mut self.matcher {
      PhraseMatcherEnum::Exact(m) => m.approximation_mut(),
      PhraseMatcherEnum::Sloppy(m) => m.approximation_mut(),
    }
  }
);
impl_two_phase_iterator!(
  true,
  (IE, SS, N, 'a),
  crate::core::search::phrase_matcher::PhraseApproximationRef<'a, IE, SS>,
  crate::core::search::phrase_matcher::PhraseApproximationMut<'a, IE, SS>,
  self,
  {
    match &self.matcher {
      PhraseMatcherEnum::Exact(m) => DocIdSetIteratorEnum2::A(m.approximation_top_scorers()),
      PhraseMatcherEnum::Sloppy(m) => DocIdSetIteratorEnum2::B(m.approximation()),
    }
  },
  {
    match &mut self.matcher {
      PhraseMatcherEnum::Exact(m) => DocIdSetIteratorEnum2::A(m.approximation_top_scorers_mut()),
      PhraseMatcherEnum::Sloppy(m) => DocIdSetIteratorEnum2::B(m.approximation_mut()),
    }
  }
);
