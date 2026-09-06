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
use crate::core::index::impacts_source::ImpactsSource;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::max_score_cache::MaxScoreCache;
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::util::error::lucene_error::Result;
/// [`DocIdSetIterator`] that skips non-competitive docs thanks to the indexed impacts.
/// Call [`set_min_competitive_score`](ImpactsDISI::set_min_competitive_score) in order to give this
/// iterator the ability to skip low-scoring documents.
///
/// @lucene.internal
pub(crate) struct ImpactsDISI<M> {
  mode: M,
  state: CompetitiveScoreState,
}

/// Keeps document iteration separate from the impacts source, as Java does for phrase and synonym
/// scorers.
pub(crate) struct SeparateIteratorMode<D, IS, SS> {
  iterator: D,
  max_score_cache: MaxScoreCache<IS, SS>,
}

/// Iterates the impacts source itself when Rust owns only one value for both roles.
pub(crate) struct SourceIteratorMode<IS, SS> {
  max_score_cache: MaxScoreCache<IS, SS>,
}

pub(crate) type SeparateImpactsDISI<D, IS, SS> = ImpactsDISI<SeparateIteratorMode<D, IS, SS>>;
pub(crate) type SourceImpactsDISI<IS, SS> = ImpactsDISI<SourceIteratorMode<IS, SS>>;

pub(crate) trait ImpactsDISIMode {
  type Iterator: DocIdSetIterator;
  type ImpactsSource: ImpactsSource;
  type Scorer: SimScorer;

  fn iterator(&self) -> &Self::Iterator;

  fn iterator_mut(&mut self) -> &mut Self::Iterator;

  fn into_iterator(self) -> Self::Iterator;

  fn max_score_cache(&self) -> &MaxScoreCache<Self::ImpactsSource, Self::Scorer>;

  fn max_score_cache_mut(&mut self) -> &mut MaxScoreCache<Self::ImpactsSource, Self::Scorer>;
}

struct CompetitiveScoreState {
  min_competitive_score: f32,
  upto: i32,
  max_score: f32,
}

impl<D, IS, SS> ImpactsDISI<SeparateIteratorMode<D, IS, SS>> {
  pub(crate) fn new(iterator: D, max_score_cache: MaxScoreCache<IS, SS>) -> Self {
    Self {
      mode: SeparateIteratorMode {
        iterator,
        max_score_cache,
      },
      state: CompetitiveScoreState::new(),
    }
  }
}

impl<IS, SS> ImpactsDISI<SourceIteratorMode<IS, SS>> {
  pub(crate) fn from_source(max_score_cache: MaxScoreCache<IS, SS>) -> Self {
    Self {
      mode: SourceIteratorMode { max_score_cache },
      state: CompetitiveScoreState::new(),
    }
  }
}

impl<M> ImpactsDISI<M>
where
  M: ImpactsDISIMode,
{
  pub(crate) fn iterator(&self) -> &M::Iterator {
    self.mode.iterator()
  }

  pub(crate) fn iterator_mut(&mut self) -> &mut M::Iterator {
    self.mode.iterator_mut()
  }

  pub(crate) fn into_iterator(self) -> M::Iterator {
    self.mode.into_iterator()
  }

  pub(crate) fn max_score_cache(&self) -> &MaxScoreCache<M::ImpactsSource, M::Scorer> {
    self.mode.max_score_cache()
  }

  pub(crate) fn max_score_cache_mut(&mut self) -> &mut MaxScoreCache<M::ImpactsSource, M::Scorer> {
    self.mode.max_score_cache_mut()
  }

  /// Set the minimum competitive score.
  ///
  /// See also [`Scorable::set_min_competitive_score`](crate::core::search::scorable::Scorable::set_min_competitive_score).
  pub(crate) fn set_min_competitive_score(&mut self, min_competitive_score: f32) {
    self.state.set_min_competitive_score(min_competitive_score);
  }
}

impl<D, IS, SS> ImpactsDISIMode for SeparateIteratorMode<D, IS, SS>
where
  D: DocIdSetIterator,
  IS: ImpactsSource,
  SS: SimScorer,
{
  type Iterator = D;
  type ImpactsSource = IS;
  type Scorer = SS;

  fn iterator(&self) -> &Self::Iterator {
    &self.iterator
  }

  fn iterator_mut(&mut self) -> &mut Self::Iterator {
    &mut self.iterator
  }

  fn into_iterator(self) -> Self::Iterator {
    self.iterator
  }

  fn max_score_cache(&self) -> &MaxScoreCache<Self::ImpactsSource, Self::Scorer> {
    &self.max_score_cache
  }

  fn max_score_cache_mut(&mut self) -> &mut MaxScoreCache<Self::ImpactsSource, Self::Scorer> {
    &mut self.max_score_cache
  }
}

impl<IS, SS> ImpactsDISIMode for SourceIteratorMode<IS, SS>
where
  IS: DocIdSetIterator + ImpactsSource,
  SS: SimScorer,
{
  type Iterator = IS;
  type ImpactsSource = IS;
  type Scorer = SS;

  fn iterator(&self) -> &Self::Iterator {
    &self.max_score_cache.impacts_source
  }

  fn iterator_mut(&mut self) -> &mut Self::Iterator {
    &mut self.max_score_cache.impacts_source
  }

  fn into_iterator(self) -> Self::Iterator {
    self.max_score_cache.impacts_source
  }

  fn max_score_cache(&self) -> &MaxScoreCache<Self::ImpactsSource, Self::Scorer> {
    &self.max_score_cache
  }

  fn max_score_cache_mut(&mut self) -> &mut MaxScoreCache<Self::ImpactsSource, Self::Scorer> {
    &mut self.max_score_cache
  }
}

impl CompetitiveScoreState {
  fn new() -> Self {
    Self {
      min_competitive_score: 0.0,
      upto: NO_MORE_DOCS,
      max_score: f32::MAX,
    }
  }

  fn set_min_competitive_score(&mut self, min_competitive_score: f32) {
    debug_assert!(min_competitive_score >= self.min_competitive_score);
    if min_competitive_score > self.min_competitive_score {
      self.min_competitive_score = min_competitive_score;
      // force `upto` and `max_score` to be recomputed so that we will skip
      // documents if the current block of documents is not competitive
      // only if the min competitive score actually increased
      self.upto = -1;
    }
  }

  fn advance_target<IS, SS>(
    &mut self,
    max_score_cache: &mut MaxScoreCache<IS, SS>,
    mut target: i32,
  ) -> Result<i32>
  where
    IS: ImpactsSource,
    SS: SimScorer,
  {
    if target <= self.upto {
      // we are still in the current block, which is considered competitive
      // according to impacts, no skipping
      return Ok(target);
    }
    self.upto = max_score_cache.advance_shallow(target)?;
    self.max_score = max_score_cache.get_max_score_with_level_zero()?;

    loop {
      debug_assert!(self.upto >= target);

      if self.max_score >= self.min_competitive_score {
        return Ok(target);
      }

      if self.upto == NO_MORE_DOCS {
        return Ok(NO_MORE_DOCS);
      }

      let skip_up_to = max_score_cache.get_skip_up_to(self.min_competitive_score)?;
      if skip_up_to == -1 {
        // no further skipping
        target = self.upto + 1;
      } else if skip_up_to == NO_MORE_DOCS {
        return Ok(NO_MORE_DOCS);
      } else {
        target = skip_up_to + 1;
      }

      self.upto = max_score_cache.advance_shallow(target)?;
      self.max_score = max_score_cache.get_max_score_with_level_zero()?;
    }
  }
}

impl<M> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for ImpactsDISI<M> where
  M: ImpactsDISIMode
{
}
impl<M> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess for ImpactsDISI<M> where
  M: ImpactsDISIMode
{
}

impl<M> DocIdSetIterator for ImpactsDISI<M>
where
  M: ImpactsDISIMode,
{
  fn doc_id(&self) -> i32 {
    self.mode.iterator().doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    let doc = self.mode.iterator().doc_id();
    if doc < self.state.upto {
      return self.mode.iterator_mut().next_doc();
    }
    self.advance(doc + 1)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    let doc_id = self
      .state
      .advance_target(self.mode.max_score_cache_mut(), target)?;
    self.mode.iterator_mut().advance(doc_id)
  }

  fn cost(&self) -> Result<i64> {
    self.mode.iterator().cost()
  }
}
