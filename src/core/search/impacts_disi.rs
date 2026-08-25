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
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum2};
use crate::core::search::max_score_cache::MaxScoreCache;
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::util::error::lucene_error::Result;
/// [`DocIdSetIterator`] that skips non-competitive docs thanks to the indexed impacts.
/// Call [`set_min_competitive_score`](ImpactsDISI::set_min_competitive_score) in order to give this
/// iterator the ability to skip low-scoring documents.
///
/// @lucene.internal
pub struct ImpactsDISI<D, IE, SS> {
  pub(crate) in_: D,
  pub(crate) max_score_cache: MaxScoreCache<IE, SS>,
  min_competitive_score: f32,
  upto: i32,
  max_score: f32,
  pub(crate) use_disi: bool,
}
impl<D, IE, SS> ImpactsDISI<D, IE, SS> {
  pub fn new(in_: D, max_score_cache: MaxScoreCache<IE, SS>, use_disi: bool) -> Self {
    Self {
      in_,
      max_score_cache,
      min_competitive_score: 0.0,
      upto: NO_MORE_DOCS,
      max_score: f32::INFINITY,
      use_disi,
    }
  }
  /// Get the [`MaxScoreCache`].
  pub fn max_score_cache(&self) -> &MaxScoreCache<IE, SS> {
    &self.max_score_cache
  }

  /// Set the minimum competitive score.
  ///
  /// See also [`Scorable::set_min_competitive_score`](crate::core::search::scorable::Scorable::set_min_competitive_score).
  pub fn set_min_competitive_score(&mut self, min_competitive_score: f32) {
    debug_assert!(min_competitive_score >= self.min_competitive_score);
    if min_competitive_score > self.min_competitive_score {
      self.min_competitive_score = min_competitive_score;
      // force `upto` and `max_score` to be recomputed so that we will skip
      // documents if the current block of documents is not competitive
      // only if the min competitive score actually increased
      self.upto = -1;
    }
  }
}

impl<D, IE, SS> ImpactsDISI<D, IE, SS>
where
  D: DocIdSetIterator,
  IE: ImpactsEnum,
  SS: SimScorer,
{
  fn advance_target(&mut self, mut target: i32) -> Result<i32> {
    if target <= self.upto {
      // we are still in the current block, which is considered competitive
      // according to impacts, no skipping
      return Ok(target);
    }
    self.upto = self.max_score_cache.advance_shallow(target)?;
    self.max_score = self.max_score_cache.get_max_score_with_level_zero()?;

    loop {
      debug_assert!(self.upto >= target);

      if self.max_score >= self.min_competitive_score {
        return Ok(target);
      }

      if self.upto == NO_MORE_DOCS {
        return Ok(NO_MORE_DOCS);
      }

      let skip_up_to = self
        .max_score_cache
        .get_skip_up_to(self.min_competitive_score)?;
      if skip_up_to == -1 {
        // no further skipping
        target = self.upto + 1;
      } else if skip_up_to == NO_MORE_DOCS {
        return Ok(NO_MORE_DOCS);
      } else {
        target = skip_up_to + 1;
      }

      self.upto = self.max_score_cache.advance_shallow(target)?;
      self.max_score = self.max_score_cache.get_max_score_with_level_zero()?;
    }
  }

  fn disi_mut(&mut self) -> Disi<&mut D, &mut IE> {
    match self.use_disi {
      true => Disi::A(&mut self.in_),
      false => Disi::B(&mut self.max_score_cache.impacts_source),
    }
  }
}
impl<D, IE, SS> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for ImpactsDISI<D, IE, SS>
where
  D: DocIdSetIterator,
  IE: ImpactsEnum,
  SS: SimScorer,
{
}
impl<D, IE, SS> DocIdSetIterator for ImpactsDISI<D, IE, SS>
where
  D: DocIdSetIterator,
  IE: ImpactsEnum,
  SS: SimScorer,
{
  fn doc_id(&self) -> i32 {
    match self.use_disi {
      true => self.in_.doc_id(),
      false => self.max_score_cache.impacts_source.doc_id(),
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    let upto = self.upto;
    let doc = {
      let mut disi = self.disi_mut();
      let doc = disi.doc_id();
      if doc < upto {
        return disi.next_doc();
      }
      doc
    };
    self.advance(doc + 1)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    let doc_id = self.advance_target(target)?;
    match self.use_disi {
      true => self.in_.advance(doc_id),
      false => self.max_score_cache.impacts_source.advance(doc_id),
    }
  }

  fn cost(&self) -> Result<i64> {
    match self.use_disi {
      true => self.in_.cost(),
      false => self.max_score_cache.impacts_source.cost(),
    }
  }
}

type Disi<I, IE> = DocIdSetIteratorEnum2<I, IE>;
