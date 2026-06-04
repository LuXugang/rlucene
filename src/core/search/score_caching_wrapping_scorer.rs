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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::cell::Cell;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

/// A `Scorer`(crate::core::search::scorer::Scorer) that wraps another scorer and caches the score of the current document.
///
/// Successive calls to `score()` will return the same result and will not invoke
/// the wrapped scorer’s `score()` method, unless the current document has changed.
///
/// This struct is useful due to changes in the [`Collector`](crate::core::search::collector::Collector) interface, where the score
/// is not computed for a document by default—only if the collector explicitly requests it.
///
/// Some collectors may need to use the score in multiple places, but they only have a
/// `Scorer`(crate::core::search::scorer::Scorer) reference and could otherwise end up computing the score of the same
/// document more than once.
pub struct ScoreCachingWrappingScorer<S>
where
  S: Scorable,
{
  cache: ScoreCachingWrappingScorerCache,
  in_: S,
}

#[derive(Clone)]
struct ScoreCachingWrappingScorerCache {
  score_is_cached: Rc<Cell<bool>>,
  cur_score: Rc<Cell<f32>>,
}

impl ScoreCachingWrappingScorerCache {
  fn new() -> Self {
    Self {
      score_is_cached: Rc::new(Cell::new(false)),
      cur_score: Rc::new(Cell::new(0.0)),
    }
  }

  fn init(&self) {
    self.score_is_cached.set(false);
  }
}

/// Creates a new instance by wrapping the given scorer.
impl<S> ScoreCachingWrappingScorer<S>
where
  S: Scorable,
{
  fn new_with_cache(in_: S, cache: ScoreCachingWrappingScorerCache) -> Self {
    Self { cache, in_ }
  }
}

impl<S> Scorable for ScoreCachingWrappingScorer<S>
where
  S: Scorable,
{
  fn score(&mut self) -> Result<f32> {
    if !self.cache.score_is_cached.get() {
      self.cache.cur_score.set(self.in_.score()?);
      self.cache.score_is_cached.set(true);
    }
    Ok(self.cache.cur_score.get())
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    self.in_.set_min_competitive_score(min_score)
  }

  fn get_children(&self) -> Result<Vec<ChildScorable<Box<dyn Scorable>>>> {
    todo!()
  }

  fn cost(&self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }

  #[cfg(test)]
  fn scorable_test_type_name(&self) -> &'static str {
    std::any::type_name::<Self>()
  }
}

impl<S> crate::core::search::scorable::FixedScore for ScoreCachingWrappingScorer<S> where S: Scorable
{}
pub struct ScoreCachingWrappingLeafCollector<LC>
where
  LC: LeafCollector,
{
  inner: LC,
  cache: ScoreCachingWrappingScorerCache,
}
impl<LC> ScoreCachingWrappingLeafCollector<LC>
where
  LC: LeafCollector,
{
  pub(crate) fn new(base: LC) -> Self {
    Self {
      inner: base,
      cache: ScoreCachingWrappingScorerCache::new(),
    }
  }
}

impl<LC> Display for ScoreCachingWrappingLeafCollector<LC>
where
  LC: LeafCollector,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    self.inner.fmt(f)
  }
}

impl<LC> LeafCollector for ScoreCachingWrappingLeafCollector<LC>
where
  LC: LeafCollector,
{
  fn set_scorer(&mut self, scorer: &mut dyn Scorable) -> Result<()> {
    self.cache.init();
    let mut wrapper = ScoreCachingWrappingScorer::new_with_cache(scorer, self.cache.clone());
    self.inner.set_scorer(&mut wrapper)
  }

  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let mut wrapper = ScoreCachingWrappingScorer::new_with_cache(scorer, self.cache.clone());
    self.cache.init();
    self.inner.collect(doc, &mut wrapper)
  }
  fn competitive_iterator(&mut self) -> Result<Option<Box<dyn DocIdSetIterator + '_>>> {
    self.inner.competitive_iterator()
  }

  fn finish(&mut self) -> Result<()> {
    self.inner.finish()
  }
}
