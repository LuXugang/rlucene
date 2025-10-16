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
use crate::core::search::dummy::dummy_scorable::DummyScorable;
use crate::core::search::filter_leaf_collector::FilterLeafCollectorOwned;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};
/// A [`Scorer`](crate::core::search::scorer::Scorer) that wraps another scorer and caches the score of the current document.
///
/// Successive calls to `score()` will return the same result and will not invoke
/// the wrapped scorer’s `score()` method, unless the current document has changed.
///
/// This struct is useful due to changes in the [`Collector`](crate::core::search::collector::Collector) interface, where the score
/// is not computed for a document by default—only if the collector explicitly requests it.
///
/// Some collectors may need to use the score in multiple places, but they only have a
/// [`Scorer`](crate::core::search::scorer::Scorer) reference and could otherwise end up computing the score of the same
/// document more than once.
pub struct ScoreCachingWrappingScorer<S>
where
    S: Scorable,
{
    score_is_cached: bool,
    cur_score: f32,
    in_: S,
}
/// Creates a new instance by wrapping the given scorer.
impl<S> ScoreCachingWrappingScorer<S>
where
    S: Scorable,
{
    pub fn new(in_: S) -> Self {
        Self {
            score_is_cached: false,
            cur_score: 0.0,
            in_,
        }
    }
}

impl<S> Scorable for ScoreCachingWrappingScorer<S>
where
    S: Scorable,
{
    fn score(&mut self) -> Result<f32> {
        if !self.score_is_cached {
            self.cur_score = self.in_.score()?;
            self.score_is_cached = true;
        }
        Ok(self.cur_score)
    }

    fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
        self.in_.set_min_competitive_score(min_score)
    }

    type Scorable = DummyScorable;

    fn get_children(&self) -> Result<Vec<ChildScorable<Self::Scorable>>> {
        todo!()
    }
}
pub struct ScoreCachingWrappingLeafCollector<LC>
where
    LC: LeafCollector,
{
    base: FilterLeafCollectorOwned<LC>,
}
impl<LC> ScoreCachingWrappingLeafCollector<LC>
where
    LC: LeafCollector,
{
    pub(crate) fn new(collector: LC) -> Self {
        Self {
            base: collector.into(),
        }
    }
}

impl<LC> Display for ScoreCachingWrappingLeafCollector<LC>
where
    LC: LeafCollector,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.base.fmt(f)
    }
}

impl<LC> LeafCollector for ScoreCachingWrappingLeafCollector<LC>
where
    LC: LeafCollector,
{
    fn set_scorer<S>(&mut self, scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        self.base.set_scorer(scorer)
    }

    fn collect<S>(&mut self, doc: i32, scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        // TODO: IMPORTANT 这里不对
        self.base.collect(doc, scorer)
    }

    type DocIdSetIterator = <FilterLeafCollectorOwned<LC> as LeafCollector>::DocIdSetIterator;

    fn competitive_iterator(&mut self) -> Result<Option<Self::DocIdSetIterator>> {
        self.base.competitive_iterator()
    }

    fn finish(&mut self) -> Result<()> {
        self.base.finish()
    }
}
