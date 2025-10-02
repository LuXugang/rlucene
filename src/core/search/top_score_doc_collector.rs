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
use crate::core::search::dummy::dummy_disi::DummyDISI;
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
use crate::core::util::error::lucene_error::Result;
use crate::core::util::priority_queue::PriorityQueue;

pub struct TopScoreDocCollector {
    base: TopDocsCollectorBase<ScoreDoc, HitQueueComparator>,
    after: Option<ScoreDoc>,
    total_hits_threshold: i32,
    min_score_acc: MaxScoreAccumulator,
}
impl TopScoreDocCollector {
    pub fn new(
        num_hits: i32,
        after: Option<ScoreDoc>,
        total_hits_threshold: i32,
        min_score_acc: MaxScoreAccumulator,
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
    type LeafCollector<'a>
        = TopScoreDocLeafCollector<'a>
    where
        Self: 'a;

    fn get_leaf_collector<'a, W, LR>(
        &'a mut self,
        _context: &LeafReaderContext<LR>,
        _weight: Option<&mut W>,
    ) -> Result<Self::LeafCollector<'a>>
    where
        LR: LeafReader,
        W: Weight<LR>,
    {
        todo!()
    }

    fn score_mode(&self) -> ScoreMode {
        match self.total_hits_threshold == i32::MAX {
            true => ScoreMode::Complete,
            false => ScoreMode::TopScores,
        }
    }
}

impl TopDocsCollector for TopScoreDocCollector {
    type Item = ScoreDoc;
    type Cmp = HitQueueComparator;

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

    fn new_top_docs(&self, results: Option<Vec<ScoreDoc>>, _start: i32) -> TopDocs {
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
        self.base.pq.iter().filter(|sd| sd.doc != i32::MAX).count()
    }
}
pub struct TopScoreDocLeafCollector<'a> {
    base: &'a mut TopScoreDocCollector,
}
impl<'a> TopScoreDocLeafCollector<'a> {
    pub fn new(base: &'a mut TopScoreDocCollector) -> Self {
        Self { base }
    }
}
impl LeafCollector for TopScoreDocLeafCollector<'_> {
    fn collect<S>(&mut self, _doc: i32, _scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        todo!()
    }

    type DocIdSetIterator = DummyDISI;
}
