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
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::collector::Collector;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_caching_wrapping_scorer::ScoreCachingWrappingScorer;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

/// A collector wrapper that only forwards hits whose score is strictly positive.
pub struct PositiveScoresOnlyCollector<C> {
  inner: C,
}

impl<C> PositiveScoresOnlyCollector<C> {
  pub fn new(inner: C) -> Self {
    Self { inner }
  }

  pub fn into_inner(self) -> C {
    self.inner
  }
}

impl<C> Collector for PositiveScoresOnlyCollector<C>
where
  C: Collector,
{
  type LeafCollector<'a, IRC>
    = PositiveScoresOnlyLeafCollector<C::LeafCollector<'a, IRC>>
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    Ok(PositiveScoresOnlyLeafCollector::new(
      self.inner.get_leaf_collector(context, weight)?,
    ))
  }

  fn score_mode(&self) -> ScoreMode {
    if self.inner.score_mode().is_exhaustive() {
      ScoreMode::Complete
    } else {
      ScoreMode::TopScores
    }
  }

  fn set_weight<W, IRC>(&self, weight: Option<&W>) -> Result<()>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    self.inner.set_weight(weight)
  }
}

pub struct PositiveScoresOnlyLeafCollector<LC> {
  inner: LC,
}

impl<LC> PositiveScoresOnlyLeafCollector<LC> {
  fn new(inner: LC) -> Self {
    Self { inner }
  }
}

impl<LC> Display for PositiveScoresOnlyLeafCollector<LC> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl<LC> LeafCollector for PositiveScoresOnlyLeafCollector<LC>
where
  LC: LeafCollector,
{
  fn set_scorer(&mut self, scorer: &mut dyn Scorable) -> Result<()> {
    self.inner.set_scorer(scorer)
  }

  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let mut scorer = ScoreCachingWrappingScorer::new(scorer);
    if scorer.score()? > 0.0 {
      self.inner.collect(doc, &mut scorer)?;
    }
    Ok(())
  }

  fn finish(&mut self) -> Result<()> {
    self.inner.finish()
  }
}
