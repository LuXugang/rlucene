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
use crate::core::index::impacts_enum::{ImpactsEnum, ImpactsEnumEnum2};
use crate::core::index::leaf_reader::{LRImpactsEnum, LRPosting};
use crate::core::index::slow_impacts_enum::SlowImpactsEnum;
use crate::core::search::approximation_disi::{ApproximationBitSetAccess, ApproximationDISI};
use crate::core::search::conjunction_disi::ConjunctionDISIEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::exact_phrase_matcher::ExactPhraseMatcher;
use crate::core::search::impacts_disi::{
  ImpactsDISI, ImpactsDISIMode, SeparateIteratorMode, SourceIteratorMode,
};
use crate::core::search::score_mode::ScoreMode::TopScores;
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::search::sloppy_phrase_matcher::SloppyPhraseMatcher;
use crate::core::util::error::lucene_error::Result;
pub trait PhraseMatcher {
  /// An upper bound on the number of possible matches on this document.
  fn max_freq(&mut self) -> Result<f32>;

  /// Called after `approximation` has been advanced.
  fn reset(&mut self) -> Result<()>;

  /// Find the next match on the current document.
  ///
  /// Returns `false` if there are no more matches.
  fn next_match(&mut self) -> Result<bool>;

  /// The slop-adjusted weight of the current match.
  ///
  /// The sum of the slop-adjusted weights is used as the freq for scoring.
  fn sloppy_weight(&self) -> f32;

  /// The start position of the current match.
  fn start_position(&self) -> i32;

  /// The end position of the current match.
  fn end_position(&self) -> i32;

  /// The start offset of the current match.
  fn start_offset(&self) -> Result<i32>;

  /// The end offset of the current match.
  fn end_offset(&self) -> Result<i32>;

  /// An estimate of the average cost of finding all matches on a document.
  ///
  /// See [`TwoPhaseIterator::match_cost`](crate::core::search::two_phase_iterator::TwoPhaseIterator::match_cost).
  fn get_match_cost(&self) -> f32;
}
/// Matching and candidate iteration on the same concrete owner.
#[doc(hidden)]
pub trait PhraseScorerMatcher:
  PhraseMatcher + ApproximationDISI + ApproximationBitSetAccess
{
  type ImpactsMode: ImpactsDISIMode;

  fn impacts_approximation_mut(&mut self) -> &mut ImpactsDISI<Self::ImpactsMode>;
}

impl<IE: ImpactsEnum, SS: SimScorer, const TOP_SCORES: bool> PhraseScorerMatcher
  for ExactPhraseMatcher<IE, SS, TOP_SCORES>
{
  type ImpactsMode =
    SourceIteratorMode<crate::core::search::exact_phrase_matcher::ImpactsSourceImpl<IE>, SS>;

  fn impacts_approximation_mut(&mut self) -> &mut ImpactsDISI<Self::ImpactsMode> {
    &mut self.impacts_approximation
  }
}

impl<IE: ImpactsEnum, SS: SimScorer> PhraseScorerMatcher for SloppyPhraseMatcher<IE, SS> {
  type ImpactsMode = SeparateIteratorMode<
    ConjunctionDISIEnum<IE>,
    crate::core::search::sloppy_phrase_matcher::ImpactsSourceImpl,
    SS,
  >;

  fn impacts_approximation_mut(&mut self) -> &mut ImpactsDISI<Self::ImpactsMode> {
    &mut self.impacts_approximation
  }
}

pub enum PhraseMatcherEnum<IE, SS, const TOP_SCORES: bool = false> {
  Exact(ExactPhraseMatcher<IE, SS, TOP_SCORES>),
  Sloppy(SloppyPhraseMatcher<IE, SS>),
}
impl<IE, SS, const TOP_SCORES: bool> PhraseMatcherEnum<IE, SS, TOP_SCORES>
where
  IE: ImpactsEnum,
  SS: SimScorer,
{
  pub(crate) fn approx_advance(&mut self, target: i32) -> Result<i32> {
    match self {
      PhraseMatcherEnum::Exact(m) => {
        if m.score_mode == TopScores {
          m.approximation_top_scorers_mut().advance(target)
        } else {
          m.approximation_mut().advance(target)
        }
      },
      PhraseMatcherEnum::Sloppy(m) => m.approximation_mut().advance(target),
    }
  }
}
impl<IE, SS, const TOP_SCORES: bool> PhraseMatcher for PhraseMatcherEnum<IE, SS, TOP_SCORES>
where
  IE: ImpactsEnum,
  SS: SimScorer,
{
  fn max_freq(&mut self) -> Result<f32> {
    match self {
      PhraseMatcherEnum::Exact(m) => m.max_freq(),
      PhraseMatcherEnum::Sloppy(m) => m.max_freq(),
    }
  }

  fn reset(&mut self) -> Result<()> {
    match self {
      PhraseMatcherEnum::Exact(m) => m.reset(),
      PhraseMatcherEnum::Sloppy(m) => m.reset(),
    }
  }

  fn next_match(&mut self) -> Result<bool> {
    match self {
      PhraseMatcherEnum::Exact(m) => m.next_match(),
      PhraseMatcherEnum::Sloppy(m) => m.next_match(),
    }
  }

  fn sloppy_weight(&self) -> f32 {
    match self {
      PhraseMatcherEnum::Exact(m) => m.sloppy_weight(),
      PhraseMatcherEnum::Sloppy(m) => m.sloppy_weight(),
    }
  }

  fn start_position(&self) -> i32 {
    match self {
      PhraseMatcherEnum::Exact(m) => m.start_position(),
      PhraseMatcherEnum::Sloppy(m) => m.start_position(),
    }
  }

  fn end_position(&self) -> i32 {
    match self {
      PhraseMatcherEnum::Exact(m) => m.end_position(),
      PhraseMatcherEnum::Sloppy(m) => m.end_position(),
    }
  }

  fn start_offset(&self) -> Result<i32> {
    match self {
      PhraseMatcherEnum::Exact(m) => m.start_offset(),
      PhraseMatcherEnum::Sloppy(m) => m.start_offset(),
    }
  }

  fn end_offset(&self) -> Result<i32> {
    match self {
      PhraseMatcherEnum::Exact(m) => m.end_offset(),
      PhraseMatcherEnum::Sloppy(m) => m.end_offset(),
    }
  }

  fn get_match_cost(&self) -> f32 {
    match self {
      PhraseMatcherEnum::Exact(m) => m.get_match_cost(),
      PhraseMatcherEnum::Sloppy(m) => m.get_match_cost(),
    }
  }
}

pub type DefaultPhraseMatcherEnum<LR, SS> =
  PhraseMatcherEnum<ImpactsEnumEnum2<LRImpactsEnum<LR>, SlowImpactsEnum<LRPosting<LR>>>, SS>;
