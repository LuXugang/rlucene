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
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::index::slow_impacts_enum::SlowImpactsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::impacts_disi::SourceImpactsDISI;
use crate::core::search::max_score_cache::MaxScoreCache;
use crate::core::search::scorable::Scorable;
use crate::core::search::scorer::{Scorer, ScorerEnum2, TwoPhaseState};
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::util::error::lucene_error::Result;

/// Expert: A Scorer for documents matching a Term.
pub struct TermScorer<S: DocIdSetIterator, N> {
  norms: Option<N>,
  pub(crate) state: S,
}

/// Owns the postings and score cache for a term without competitive skipping.
#[doc(hidden)]
pub struct PostingsTermState<PE, SS> {
  max_score_cache: MaxScoreCache<SlowImpactsEnum<PE>, SS>,
}

/// Owns the impacts iterator and score cache without competitive skipping.
#[doc(hidden)]
pub struct ImpactsTermState<IE, SS> {
  max_score_cache: MaxScoreCache<IE, SS>,
}

/// Keeps the owned iterator's existing specialization boundary without a mode enum.
#[doc(hidden)]
pub struct TermOwnedIterator<I>(I);

impl<I: DocIdSetIterator> DocIdSetIterator for TermOwnedIterator<I> {
  fn doc_id(&self) -> i32 {
    self.0.doc_id()
  }
  fn next_doc(&mut self) -> Result<i32> {
    self.0.next_doc()
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    self.0.advance(target)
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.0.slow_advance(target)
  }
  fn cost(&self) -> Result<i64> {
    DocIdSetIterator::cost(&self.0)
  }
}

// The previous owned DocIdSetIteratorEnum3 did not expose these specialization hooks.
impl<I: DocIdSetIterator> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for TermOwnedIterator<I>
{
}

impl<I: DocIdSetIterator> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for TermOwnedIterator<I>
{
  fn is_bit_iter(&self) -> bool {
    self.0.is_bit_iter()
  }
  fn get(&self, index: usize) -> Result<bool> {
    self.0.get(index)
  }
  fn set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.0.set_doc_id(doc)
  }
  fn bit_set_length(&self) -> Result<usize> {
    self.0.bit_set_length()
  }
}

impl<PE, SS> DocIdSetIterator for PostingsTermState<PE, SS>
where
  PE: PostingsEnum,
{
  fn doc_id(&self) -> i32 {
    self.max_score_cache.impacts_source.delegate.doc_id()
  }
  fn next_doc(&mut self) -> Result<i32> {
    self.max_score_cache.impacts_source.delegate.next_doc()
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    self.max_score_cache.impacts_source.delegate.advance(target)
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self
      .max_score_cache
      .impacts_source
      .delegate
      .slow_advance(target)
  }
  fn cost(&self) -> Result<i64> {
    DocIdSetIterator::cost(&self.max_score_cache.impacts_source.delegate)
  }
}

impl<PE, SS> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for PostingsTermState<PE, SS>
where
  PE: PostingsEnum,
{
  fn get_fixed_bit_set(&self) -> Option<&crate::core::util::fixed_bit_set::FixedBitSet> {
    self
      .max_score_cache
      .impacts_source
      .delegate
      .get_fixed_bit_set()
  }
  fn get_sparse_fixed_bit_set(
    &self,
  ) -> Option<&crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet> {
    self
      .max_score_cache
      .impacts_source
      .delegate
      .get_sparse_fixed_bit_set()
  }
  fn get_doc_base_fixed_bit_set(
    &self,
  ) -> Option<(usize, &crate::core::util::fixed_bit_set::FixedBitSet)> {
    self
      .max_score_cache
      .impacts_source
      .delegate
      .get_doc_base_fixed_bit_set()
  }
}

impl<PE, SS> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for PostingsTermState<PE, SS>
where
  PE: PostingsEnum,
{
  fn is_bit_iter(&self) -> bool {
    self.max_score_cache.impacts_source.delegate.is_bit_iter()
  }
  fn get(&self, index: usize) -> Result<bool> {
    self.max_score_cache.impacts_source.delegate.get(index)
  }
  fn set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.max_score_cache.impacts_source.delegate.set_doc_id(doc)
  }
  fn bit_set_length(&self) -> Result<usize> {
    self
      .max_score_cache
      .impacts_source
      .delegate
      .bit_set_length()
  }
}

impl<IE, SS> DocIdSetIterator for ImpactsTermState<IE, SS>
where
  IE: ImpactsEnum,
{
  fn doc_id(&self) -> i32 {
    self.max_score_cache.impacts_source.doc_id()
  }
  fn next_doc(&mut self) -> Result<i32> {
    self.max_score_cache.impacts_source.next_doc()
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    self.max_score_cache.impacts_source.advance(target)
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.max_score_cache.impacts_source.slow_advance(target)
  }
  fn cost(&self) -> Result<i64> {
    DocIdSetIterator::cost(&self.max_score_cache.impacts_source)
  }
}

impl<IE, SS> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for ImpactsTermState<IE, SS>
where
  IE: ImpactsEnum,
{
  fn get_fixed_bit_set(&self) -> Option<&crate::core::util::fixed_bit_set::FixedBitSet> {
    self.max_score_cache.impacts_source.get_fixed_bit_set()
  }
  fn get_sparse_fixed_bit_set(
    &self,
  ) -> Option<&crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet> {
    self
      .max_score_cache
      .impacts_source
      .get_sparse_fixed_bit_set()
  }
  fn get_doc_base_fixed_bit_set(
    &self,
  ) -> Option<(usize, &crate::core::util::fixed_bit_set::FixedBitSet)> {
    self
      .max_score_cache
      .impacts_source
      .get_doc_base_fixed_bit_set()
  }
}

impl<IE, SS> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for ImpactsTermState<IE, SS>
where
  IE: ImpactsEnum,
{
  fn is_bit_iter(&self) -> bool {
    self.max_score_cache.impacts_source.is_bit_iter()
  }
  fn get(&self, index: usize) -> Result<bool> {
    self.max_score_cache.impacts_source.get(index)
  }
  fn set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.max_score_cache.impacts_source.set_doc_id(doc)
  }
  fn bit_set_length(&self) -> Result<usize> {
    self.max_score_cache.impacts_source.bit_set_length()
  }
}

/// Constructor-selected state for a term scorer. The state type stays fixed while iterating.
#[doc(hidden)]
pub trait TermScorerState: DocIdSetIterator {
  fn freq(&mut self) -> Result<i32>;
  type Iterator: DocIdSetIterator + 'static;
  type Scorer: SimScorer;

  fn into_iterator(self) -> Self::Iterator;
  fn sim_scorer(&self) -> &Self::Scorer;
  fn advance_shallow(&mut self, target: i32) -> Result<i32>;
  fn get_max_score(&mut self, upto: i32) -> Result<f32>;
  fn set_min_competitive_score(&mut self, _min_score: f32) -> Result<()> {
    Ok(())
  }
}

impl<PE, SS, N> TermScorer<PostingsTermState<PE, SS>, N>
where
  PE: PostingsEnum,
  SS: SimScorer,
{
  /// Construct a scorer that will iterate all documents.
  pub fn from_postings(postings_enum: PE, scorer: SS, norms: Option<N>) -> Self {
    let max_score_cache = MaxScoreCache::new(SlowImpactsEnum::new(postings_enum), scorer);
    Self {
      norms,
      state: PostingsTermState { max_score_cache },
    }
  }
}

impl<IE, SS, N> TermScorer<ImpactsTermState<IE, SS>, N>
where
  IE: ImpactsEnum,
  SS: SimScorer,
{
  /// Select the concrete impacts scorer before document iteration.
  pub fn from_impacts(
    impacts_enum: IE,
    scorer: SS,
    norms: Option<N>,
    top_level_scoring_clause: bool,
  ) -> ScorerEnum2<Self, TermScorer<SourceImpactsDISI<IE, SS>, N>> {
    let max_score_cache = MaxScoreCache::new(impacts_enum, scorer);
    if top_level_scoring_clause {
      ScorerEnum2::B(TermScorer {
        norms,
        state: SourceImpactsDISI::from_source(max_score_cache),
      })
    } else {
      ScorerEnum2::A(Self {
        norms,
        state: ImpactsTermState { max_score_cache },
      })
    }
  }
}

impl<PE, SS> TermScorerState for PostingsTermState<PE, SS>
where
  PE: PostingsEnum + 'static,
  SS: SimScorer + 'static,
{
  type Iterator = TermOwnedIterator<PE>;
  type Scorer = SS;

  fn freq(&mut self) -> Result<i32> {
    self.max_score_cache.impacts_source.delegate.freq()
  }

  fn into_iterator(self) -> Self::Iterator {
    TermOwnedIterator(self.max_score_cache.impacts_source.delegate)
  }

  fn sim_scorer(&self) -> &Self::Scorer {
    &self.max_score_cache.scorer
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    self.max_score_cache.advance_shallow(target)
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    self.max_score_cache.get_max_score(upto)
  }
}

impl<IE, SS> TermScorerState for ImpactsTermState<IE, SS>
where
  IE: ImpactsEnum + 'static,
  SS: SimScorer + 'static,
{
  type Iterator = TermOwnedIterator<IE>;
  type Scorer = SS;

  fn freq(&mut self) -> Result<i32> {
    self.max_score_cache.impacts_source.freq()
  }

  fn into_iterator(self) -> Self::Iterator {
    TermOwnedIterator(self.max_score_cache.impacts_source)
  }

  fn sim_scorer(&self) -> &Self::Scorer {
    &self.max_score_cache.scorer
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    self.max_score_cache.advance_shallow(target)
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    self.max_score_cache.get_max_score(upto)
  }
}

impl<IE, SS> TermScorerState for SourceImpactsDISI<IE, SS>
where
  IE: ImpactsEnum + 'static,
  SS: SimScorer + 'static,
{
  type Iterator = TermOwnedIterator<Self>;
  type Scorer = SS;

  fn freq(&mut self) -> Result<i32> {
    SourceImpactsDISI::iterator_mut(self).freq()
  }

  fn into_iterator(self) -> Self::Iterator {
    TermOwnedIterator(self)
  }

  fn sim_scorer(&self) -> &Self::Scorer {
    &self.max_score_cache().scorer
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    self.max_score_cache_mut().advance_shallow(target)
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    self.max_score_cache_mut().get_max_score(upto)
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    SourceImpactsDISI::set_min_competitive_score(self, min_score);
    Ok(())
  }
}

impl<S, N> Scorable for TermScorer<S, N>
where
  S: TermScorerState,
  N: NumericDocValues,
{
  type Child<'a>
    = crate::core::search::dummy::dummy_scorable::DummyScorable
  where
    Self: 'a;

  fn score(&mut self) -> Result<f32> {
    let mut norm = 1;
    if let Some(ref mut norms) = self.norms
      && norms.advance_exact(self.state.doc_id())?
    {
      norm = norms.long_value()?;
    }
    let freq = self.state.freq()?;
    Ok(self.state.sim_scorer().score(freq as f32, norm))
  }

  fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
    let mut norm = 1;
    if let Some(ref mut norms) = self.norms
      && norms.advance_exact(doc_id)?
    {
      norm = norms.long_value()?;
    }
    let scorer = self.state.sim_scorer();
    Ok(scorer.score(0f32, norm))
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    self.state.set_min_competitive_score(min_score)
  }

  fn cost(&self) -> Result<i64> {
    DocIdSetIterator::cost(&self.state)
  }
}

impl<S: DocIdSetIterator, N> crate::core::search::scorable::FixedScore for TermScorer<S, N> {}

impl<S, N> DocIdSetIterator for TermScorer<S, N>
where
  S: TermScorerState,
  N: NumericDocValues,
{
  fn doc_id(&self) -> i32 {
    self.state.doc_id()
  }
  #[inline]
  fn next_doc(&mut self) -> Result<i32> {
    self.state.next_doc()
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    self.state.advance(target)
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.state.slow_advance(target)
  }
  fn cost(&self) -> Result<i64> {
    DocIdSetIterator::cost(&self.state)
  }
}
impl<S, N> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for TermScorer<S, N>
where
  S: TermScorerState,
  N: NumericDocValues,
{
  fn get_fixed_bit_set(&self) -> Option<&crate::core::util::fixed_bit_set::FixedBitSet> {
    self.state.get_fixed_bit_set()
  }
  fn get_sparse_fixed_bit_set(
    &self,
  ) -> Option<&crate::core::util::sparse_fixed_bit_set::SparseFixedBitSet> {
    self.state.get_sparse_fixed_bit_set()
  }
  fn get_doc_base_fixed_bit_set(
    &self,
  ) -> Option<(usize, &crate::core::util::fixed_bit_set::FixedBitSet)> {
    self.state.get_doc_base_fixed_bit_set()
  }
}
impl<S, N> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for TermScorer<S, N>
where
  S: TermScorerState,
  N: NumericDocValues,
{
  fn is_bit_iter(&self) -> bool {
    self.state.is_bit_iter()
  }
  fn get(&self, index: usize) -> Result<bool> {
    self.state.get(index)
  }
  fn set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.state.set_doc_id(doc)
  }
  fn bit_set_length(&self) -> Result<usize> {
    self.state.bit_set_length()
  }
}
impl<S, N> Scorer for TermScorer<S, N>
where
  S: TermScorerState,
  N: NumericDocValues,
{
  type Iterator = S::Iterator;
  type OwnedTwoPhase = crate::core::search::two_phase_iterator::EmptyTPI;
  type ApproximationRef<'a>
    = &'a Self
  where
    Self: 'a;
  type ApproximationMut<'a>
    = &'a mut Self
  where
    Self: 'a;
  type TwoPhaseRef<'a>
    = crate::core::search::two_phase_iterator::EmptyTPI
  where
    Self: 'a;
  type TwoPhaseMut<'a>
    = crate::core::search::two_phase_iterator::EmptyTPI
  where
    Self: 'a;

  fn scoring_doc_id(&mut self) -> Result<i32> {
    Ok(self.state.doc_id())
  }

  fn take_iterator(self) -> Self::Iterator {
    self.state.into_iterator()
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    self.state.advance_shallow(target)
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    self.state.get_max_score(upto)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }

  fn approximation(&self) -> Self::ApproximationRef<'_> {
    self
  }

  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    self
  }
}
