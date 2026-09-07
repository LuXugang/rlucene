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
use crate::core::index::impact::Impact;
use crate::core::index::impacts::Impacts;
use crate::core::index::impacts_source::ImpactsSource;
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::core_helper::CoreHelper;
use crate::core::util::error::lucene_error::Result;
/// Compute maximum scores based on [`Impacts`] and keep them in a cache
/// in order not to run expensive similarity score computations multiple times
/// on the same data.
///
/// @lucene.internal
pub struct MaxScoreCache<IS, SS> {
  pub(crate) impacts_source: IS,
  pub(crate) scorer: SS,
  global_max_score: f32,
  max_score_cache: Vec<f32>,
  max_score_cache_upto: Vec<i32>,
}

impl<IS, SS> MaxScoreCache<IS, SS>
where
  SS: SimScorer,
{
  pub fn new(impacts_source: IS, scorer: SS) -> Self {
    let global_max_score = scorer.score(f32::MAX, 1);

    Self {
      impacts_source,
      scorer,
      global_max_score,
      max_score_cache: Vec::new(),
      max_score_cache_upto: Vec::new(),
    }
  }
}

impl<IS, SS> MaxScoreCache<IS, SS>
where
  IS: ImpactsSource,
  SS: SimScorer,
{
  /// Implement the contract of [`Scorer::advance_shallow`](ImpactsSource::advance_shallow) based on the wrapped [`ImpactsSource`].
  ///
  /// See also [`Scorer::advance_shallow`].
  pub fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    self.impacts_source.advance_shallow(target)?;
    let impacts = self.impacts_source.get_impacts()?;
    Ok(impacts.get_doc_id_upto(0))
  }

  fn ensure_cache_size(&mut self, size: usize) -> Result<()> {
    if self.max_score_cache.len() < size {
      let old_len = self.max_score_cache.len();
      ArrayUtil::grow_with_len(&mut self.max_score_cache, size)?;
      let len = self.max_score_cache.len();
      ArrayUtil::grow_exact(&mut self.max_score_cache_upto, len)?;
      self.max_score_cache_upto[old_len..].fill(-1);
    }
    Ok(())
  }

  fn compute_max_score(&self, impacts: &[Impact]) -> f32 {
    let mut max_score = 0.0;
    for impact in impacts {
      let score = self.scorer.score(impact.freq as f32, impact.norm);
      max_score = CoreHelper::max_f32(score, max_score);
    }
    max_score
  }
  /// Return the maximum score up to upTo included.
  pub fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    match self.get_level(upto)? {
      Some(level) => self.get_max_score_with_level(level),
      None => Ok(self.global_max_score),
    }
  }

  /// Return the first level that includes all doc IDs up to `upto`,
  /// or None if there is no such level.
  fn get_level(&self, upto: i32) -> Result<Option<usize>> {
    let impacts = self.impacts_source.get_impacts()?;
    let num_levels = impacts.num_levels();
    for level in 0..num_levels {
      let impacts_up_to = impacts.get_doc_id_upto(level);
      if upto <= impacts_up_to {
        return Ok(Some(level as usize));
      }
    }
    Ok(None)
  }

  pub fn get_max_score_with_level_zero(&mut self) -> Result<f32> {
    self.get_max_score_with_level(0)
  }
  /// Return the maximum score for the given `level`.
  fn get_max_score_with_level(&mut self, level: usize) -> Result<f32> {
    self.ensure_cache_size(level + 1)?;
    let impacts = self.impacts_source.get_impacts()?;
    let level_up_to = impacts.get_doc_id_upto(level as i32);
    if self.max_score_cache_upto[level] < level_up_to {
      let max_score = self.compute_max_score(impacts.get_impacts(level as i32)?.as_ref());
      self.max_score_cache[level] = max_score;
      self.max_score_cache_upto[level] = level_up_to;
    }
    Ok(self.max_score_cache[level])
  }

  /// Return the maximum level at which scores are all less than `min_score`,
  /// or None if none.
  fn get_skip_level(&mut self, min_score: f32) -> Result<Option<usize>> {
    let num_levels = {
      let impacts = self.impacts_source.get_impacts()?;
      impacts.num_levels()
    };
    let mut skip_level = None;
    for level in 0..num_levels {
      if self.get_max_score_with_level(level as usize)? >= min_score {
        return Ok(skip_level);
      }
      skip_level = Some(level as usize);
    }
    Ok(skip_level)
  }

  /// Return an inclusive upper bound of documents that all have a score less than `min_score`,
  /// or -1 if the current document may be competitive.
  pub fn get_skip_up_to(&mut self, min_score: f32) -> Result<i32> {
    match self.get_skip_level(min_score)? {
      Some(level) => {
        let impacts = self.impacts_source.get_impacts()?;
        Ok(impacts.get_doc_id_upto(level as i32))
      },
      None => Ok(-1),
    }
  }
}
