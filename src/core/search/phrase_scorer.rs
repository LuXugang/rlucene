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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::phrase_matcher::{PhraseMatcher, PhraseMatcherEnum};
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::score_mode::ScoreMode::TopScores;
#[cfg(test)]
use crate::core::search::scorer::ScorerKind;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::search::two_phase_iterator::{
  TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
};
use crate::core::util::error::lucene_error::Result;
pub type Disi<IE, SS, N> = TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<IE, SS, N>>;
pub struct PhraseScorer<IE, SS, N>
where
  IE: ImpactsEnum,
  SS: SimScorer,
  N: NumericDocValues,
{
  disi: Disi<IE, SS, N>,
}
impl<IE, SS, N> PhraseScorer<IE, SS, N>
where
  IE: ImpactsEnum,
  SS: SimScorer,
  N: NumericDocValues,
{
  pub(crate) fn new(
    matcher: PhraseMatcherEnum<IE, SS>,
    score_mode: ScoreMode,
    sim_scorer: SS,
    norms: Option<N>,
  ) -> Self {
    let v = TwoPhaseIteratorImpl::new(matcher, score_mode, sim_scorer, norms);
    let disi = TwoPhaseIteratorAsDocIdSetIterator::new(v);
    Self { disi }
  }
}

impl<IE, SS, N> Scorable for PhraseScorer<IE, SS, N>
where
  IE: ImpactsEnum + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
{
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
    self.iterator().cost()
  }
}

impl<IE, SS, N> Scorer for PhraseScorer<IE, SS, N>
where
  IE: ImpactsEnum + 'static,
  SS: SimScorer + 'static,
  N: NumericDocValues + 'static,
{
  fn doc_id(&mut self) -> Result<i32> {
    Ok(self.disi.two_phase_iterator.doc_id())
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.disi)
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.disi)
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let PhraseScorer { disi, .. } = *self;
    Box::new(disi)
  }

  fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    Some(Box::new(&self.disi.two_phase_iterator))
  }

  fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    Some(Box::new(&mut self.disi.two_phase_iterator))
  }

  fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>> {
    let PhraseScorer { disi, .. } = *self;
    Some(Box::new(disi.two_phase_iterator))
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    match self.disi.two_phase_iterator.matcher {
      PhraseMatcherEnum::Exact(ref mut m) => m
        .impacts_approximation
        .max_score_cache
        .advance_shallow(target),
      PhraseMatcherEnum::Sloppy(ref mut m) => m
        .impacts_approximation
        .max_score_cache
        .advance_shallow(target),
    }
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    match self.disi.two_phase_iterator.matcher {
      PhraseMatcherEnum::Exact(ref mut m) => {
        m.impacts_approximation.max_score_cache.get_max_score(upto)
      },
      PhraseMatcherEnum::Sloppy(ref mut m) => {
        m.impacts_approximation.max_score_cache.get_max_score(upto)
      },
    }
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::Yes
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    self.disi.two_phase_iterator.approximation()
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    self.disi.two_phase_iterator.approximation_mut()
  }
  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    ScorerKind::Phrase
  }
}

pub struct TwoPhaseIteratorImpl<IE, SS, N>
where
  IE: ImpactsEnum,
  SS: SimScorer,
  N: NumericDocValues,
{
  matcher: PhraseMatcherEnum<IE, SS>,
  sim_scorer: SS,
  norms: Option<N>,
  match_cost: f32,
  freq: f32,
  min_competitive_score: f32,
  score_mode: ScoreMode,
}
impl<IE, SS, N> TwoPhaseIteratorImpl<IE, SS, N>
where
  IE: ImpactsEnum,
  SS: SimScorer,
  N: NumericDocValues,
{
  fn new(
    matcher: PhraseMatcherEnum<IE, SS>,
    score_mode: ScoreMode,
    sim_scorer: SS,
    norms: Option<N>,
  ) -> Self {
    let match_cost = matcher.get_match_cost();
    Self {
      matcher,
      sim_scorer,
      norms,
      match_cost,
      freq: 0.0,
      min_competitive_score: 0.0,
      score_mode,
    }
  }

  fn doc_id(&self) -> i32 {
    self.approximation().doc_id()
  }
}
impl<IE, SS, N> TwoPhaseIterator for TwoPhaseIteratorImpl<IE, SS, N>
where
  IE: ImpactsEnum,
  SS: SimScorer,
  N: NumericDocValues,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    self.matcher.approximation_mut()
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    self.matcher.approximation()
  }

  fn matches(&mut self) -> Result<bool> {
    self.matcher.reset()?;

    if self.score_mode == TopScores && self.min_competitive_score > 0.0 {
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
