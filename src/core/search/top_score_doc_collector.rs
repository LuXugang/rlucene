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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::collector::Collector;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::hit_queue::{HitQueue, HitQueueComparator};
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::max_score_accumulator::MaxScoreAccumulator;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::top_docs::TopDocs;
use crate::core::search::top_docs_collector::{TopDocsCollector, TopDocsCollectorBase};
use crate::core::search::total_hits::{Relation, TotalHits};
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::PriorityQueue;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

/// A [`Collector`] implementation that collects the top-scoring hits,
/// returning them as a [`TopDocs`].
///
/// This is used by [`IndexSearcher`](crate::core::search::index_searcher::IndexSearcher) to implement [`TopDocs`]-based search.
/// Hits are sorted by score descending and then (when the scores are tied) docID ascending.
/// When you create an instance of this collector you should know in advance whether
/// documents are going to be collected in doc ID order or not.
///
///
/// **NOTE:** The values [`f32::NAN`] and [`f32::NEG_INFINITY`] are not valid scores.
/// This collector will not properly collect hits with such scores.
pub struct TopScoreDocCollector {
    base: TopDocsCollectorBase<ScoreDoc, HitQueueComparator>,
    after: Option<ScoreDoc>,
    total_hits_threshold: usize,
    pub(crate) min_score_acc: Option<Arc<MaxScoreAccumulator>>,
}
impl TopScoreDocCollector {
    pub fn new(
        num_hits: usize,
        after: Option<ScoreDoc>,
        total_hits_threshold: usize,
        min_score_acc: Option<Arc<MaxScoreAccumulator>>,
    ) -> Result<Self> {
        let pq = HitQueue::new(num_hits, true)?;
        let base = TopDocsCollectorBase::new(pq);
        Ok(Self {
            base,
            after,
            total_hits_threshold,
            min_score_acc,
        })
    }
}

impl Collector for TopScoreDocCollector {
    type LeafCollector<'a, LR>
        = TopScoreDocLeafCollector<'a>
    where
        Self: 'a,
        LR: LeafReader;

    fn get_leaf_collector<'a, W, LR>(
        &'a mut self,
        context: &LeafReaderContext<LR>,
        _weight: Option<&W>,
    ) -> Result<Self::LeafCollector<'a, LR>>
    where
        LR: LeafReader,
        W: Weight<LR> + ?Sized,
    {
        let doc_base = context.doc_base;
        let after_score: f32;
        let after_doc: i32;

        if let Some(after) = &self.after {
            after_score = after.score;
            after_doc = after.doc - doc_base as i32
        } else {
            after_score = f32::INFINITY;
            after_doc = NO_MORE_DOCS;
        }
        Ok(TopScoreDocLeafCollector::new(
            self,
            doc_base,
            after_doc,
            after_score,
        ))
    }

    fn score_mode(&self) -> ScoreMode {
        match self.total_hits_threshold == i32::MAX as usize {
            true => ScoreMode::Complete,
            false => ScoreMode::TopScores,
        }
    }
}

impl TopDocsCollector for TopScoreDocCollector {
    type Item = ScoreDoc;
    type Cmp = HitQueueComparator;
    type TopDocsLike = TopDocs<Self::Item>;

    fn pq(&self) -> &PriorityQueue<Self::Item, Self::Cmp> {
        &self.base.pq
    }

    fn pq_mut(&mut self) -> &mut PriorityQueue<Self::Item, Self::Cmp> {
        &mut self.base.pq
    }

    fn total_hits(&self) -> usize {
        self.base.total_hits
    }

    fn get_total_hits_relation(&self) -> Relation {
        self.base.total_hits_relation
    }

    fn new_top_docs(&self, results: Option<Vec<Self::Item>>, _start: i32) -> Self::TopDocsLike
    where
        Self: Sized,
    {
        match results {
            None => TopDocs::new(
                TotalHits::new(self.base.total_hits, self.base.total_hits_relation),
                vec![],
            ),
            Some(res) => TopDocs::new(
                TotalHits::new(self.base.total_hits, self.base.total_hits_relation),
                res,
            ),
        }
    }

    fn top_docs_size(&self) -> usize {
        self.base
            .pq
            .iter_ref()
            .filter(|sd| sd.doc != i32::MAX)
            .count()
    }
}
pub struct TopScoreDocLeafCollector<'a> {
    base: &'a mut TopScoreDocCollector,
    min_competitive_score: f32,
    doc_base: usize,
    after_doc: i32,
    after_score: f32,
}
impl<'a> TopScoreDocLeafCollector<'a> {
    pub fn new(
        base: &'a mut TopScoreDocCollector,
        doc_base: usize,
        after_doc: i32,
        after_score: f32,
    ) -> Self {
        Self {
            base,
            min_competitive_score: 0.0,
            doc_base,
            after_doc,
            after_score,
        }
    }
    fn update_global_min_competitive_score<S: Scorable + ?Sized>(
        &mut self,
        scorer: &mut S,
    ) -> Result<()> {
        debug_assert!(self.base.min_score_acc.is_some());
        let max_min_score = self.base.min_score_acc.as_ref().unwrap().get_raw();
        if max_min_score != i64::MIN {
            // since we tie-break on doc id and collect in doc id order we can require
            // the next float if the global minimum score is set on a document id that is
            // smaller than the ids in the current leaf
            let mut score = MaxScoreAccumulator::to_score(max_min_score);

            if self.doc_base as i32 >= MaxScoreAccumulator::doc_id(max_min_score) {
                score = f32::from_bits(score.to_bits() + 1);
            }
            if score > self.min_competitive_score {
                scorer.set_min_competitive_score(score)?;
                self.min_competitive_score = score;
                self.base.base.total_hits_relation = Relation::GreaterThanOrEqualTo;
            }
        }
        Ok(())
    }
    fn collect_competitive_hit<S: Scorable + ?Sized>(
        &mut self,
        scorer: &mut S,
        doc: i32,
        score: f32,
    ) -> Result<()> {
        match self.base.base.pq.top_mut() {
            None => return Err(LuceneError::illegal_state("Priority queue is empty")),
            Some(pq_top) => {
                pq_top.doc = doc + self.doc_base as i32;
                pq_top.score = score;
            },
        }
        let _ = self.base.base.pq.update_top()?;
        self.update_min_competitive_score(scorer)?;
        Ok(())
    }

    fn update_min_competitive_score<S: Scorable + ?Sized>(&mut self, scorer: &mut S) -> Result<()> {
        if self.base.base.total_hits > self.base.total_hits_threshold
            && let Some(pq_top) = self.base.base.pq.top()
        {
            // since we tie-break on doc id and collect in doc id order, we can require the next float
            // pqTop is never null since TopScoreDocCollector fills the priority queue with sentinel
            // values if the top element is a sentinel value, its score will be -Infty and the below
            // logic is still valid
            let local_min_score = f32::from_bits(pq_top.score.to_bits() + 1);
            if local_min_score > self.min_competitive_score {
                scorer.set_min_competitive_score(local_min_score)?;
                self.base.base.total_hits_relation = Relation::GreaterThanOrEqualTo;
                self.min_competitive_score = local_min_score;
                // we don't use the next float but we register the document id so that other leaves or
                // leaf partitions can require it if they are after the current maximum
                if let Some(acc) = &self.base.min_score_acc {
                    acc.accumulate(pq_top.doc, pq_top.score);
                }
            }
        }
        Ok(())
    }
}

impl Display for TopScoreDocLeafCollector<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::any::type_name::<Self>())
    }
}

impl LeafCollector for TopScoreDocLeafCollector<'_> {
    fn set_scorer(&mut self, scorer: &mut dyn Scorable) -> Result<()> {
        if self.base.total_hits_threshold != i32::MAX as usize {
            self.update_global_min_competitive_score(scorer)?;
        }
        Ok(())
    }

    fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
        let score = scorer.score()?;
        self.base.base.total_hits += 1;
        let hit_count_so_far = self.base.base.total_hits;

        if let Some(acc) = &self.base.min_score_acc
            && (hit_count_so_far as i64 & acc.mod_interval) == 0
        {
            self.update_global_min_competitive_score(scorer)?;
        }

        if let Some(_) = &self.base.after
            && (score > self.after_score || (score == self.after_score && doc <= self.after_doc))
        {
            // hit was collected on a previous page
            if self.base.base.total_hits_relation == Relation::EqualTo {
                // we just reached totalHitsThreshold, we can start setting the min
                // competitive score now
                self.update_min_competitive_score(scorer)?;
            }
            return Ok(());
        }
        match self.base.base.pq.top() {
            None => return Err(LuceneError::illegal_state("Priority queue is empty")),
            Some(pq_top) => {
                if score <= pq_top.score {
                    // Note: for queries that match lots of hits, this is the common case: most hits are not
                    // competitive.
                    if hit_count_so_far == self.base.total_hits_threshold + 1 {
                        self.update_min_competitive_score(scorer)?;
                    }
                    // Since docs are returned in-order (i.e., increasing doc Id), a document
                    // with equal score to pqTop.score cannot compete since HitQueue favors
                    // documents with lower doc Ids. Therefore reject those docs too.
                } else {
                    self.collect_competitive_hit(scorer, doc, score)?;
                }
            },
        }
        Ok(())
    }
}
