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
use std::fmt;

use crate::search::abstract_knn_collector::AbstractKnnCollectorBase;
use crate::search::knn_collector::KnnCollector;
use crate::search::score_doc::ScoreDoc;
use crate::search::top_docs::TopDocs;
use crate::search::total_hits::{Relation, TotalHits};
use crate::util::error::lucene_error::Result;
use crate::util::hnsw::neighbor_queue::NeighborQueue;

/// `TopKnnCollector` is a specific [`KnnCollector`] implementation.
/// A min-heap is used to keep track of the currently collected vectors,
/// allowing for efficient updates as better vectors are collected.
pub struct TopKnnCollector {
    queue: NeighborQueue,
}

impl TopKnnCollector {
    /// # Arguments
    ///
    /// * `k` - the number of neighbors to collect
    /// * `visit_limit` - how many vector nodes the results are allowed to visit
    pub fn new(k: i32) -> Result<Self> {
        Ok(Self {
            queue: NeighborQueue::new(k, false)?,
        })
    }
}
impl AbstractKnnCollectorBase for TopKnnCollector {
    fn num_collected(&self) -> usize {
        self.queue.size() as usize
    }
}
impl KnnCollector for TopKnnCollector {
    fn early_terminated(&self) -> bool {
        unimplemented!()
    }

    fn inc_visited_count(&mut self, count: usize) {
        unimplemented!()
    }

    fn visited_count(&self) -> usize {
        unimplemented!()
    }

    fn visit_limit(&self) -> usize {
        unimplemented!()
    }

    fn k(&self) -> usize {
        unimplemented!()
    }

    fn collect(&mut self, doc_id: i32, similarity: f32) -> bool {
        self.queue.insert_with_overflow(doc_id, similarity)
    }

    fn min_competitive_similarity(&self) -> f32 {
        if self.queue.size() as usize >= self.k() {
            self.queue.top_score()
        } else {
            f32::NEG_INFINITY
        }
    }

    fn top_docs(&mut self) -> Result<TopDocs> {
        assert!(
            self.queue.size() as usize <= self.k(),
            "Tried to collect more results than the maximum number allowed"
        );

        let mut score_docs = vec![ScoreDoc::default(); self.queue.size() as usize];
        for i in 1..score_docs.len() {
            let doc_id = self.queue.top_node();
            let score = self.queue.top_score();
            let len = score_docs.len() - i;
            score_docs[len] = ScoreDoc::new(doc_id, score);
            self.queue.pop()?;
        }

        let relation = if self.early_terminated() {
            Relation::GreaterThanOrEqualTo
        } else {
            Relation::EqualTo
        };

        let total_hits = TotalHits::new(self.visited_count(), relation);
        Ok(TopDocs::new(total_hits, score_docs))
    }
}

impl fmt::Display for TopKnnCollector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TopKnnCollector[k={}, size={}]",
            self.k(),
            self.queue.size()
        )
    }
}
