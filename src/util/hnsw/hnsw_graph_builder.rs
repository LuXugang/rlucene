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
use crate::search::knn_collector::KnnCollector;
use crate::search::top_docs::TopDocs;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::hnsw::neighbor_queue::NeighborQueue;

pub struct HnswGraphBuilder;
/// A restricted, specialized [`KnnCollector`] that can be used when building a
/// graph.
///
/// This collector does **not** support [`TopDocs`].
pub struct GraphBuilderKnnCollector {
    queue: NeighborQueue,
    k: i32,
    visited_count: usize,
}
impl GraphBuilderKnnCollector {
    pub fn new(k: i32) -> Result<Self> {
        Ok(Self {
            queue: NeighborQueue::new(k, false)?,
            k,
            visited_count: 0,
        })
    }

    pub fn size(&self) -> usize {
        self.queue.size() as usize
    }

    pub fn pop_node(&mut self) -> Result<i32> {
        self.queue.pop()
    }

    pub fn pop_until_nearest_k_nodes(&mut self) -> Result<Vec<i32>> {
        while self.size() as i32 > self.k {
            self.queue.pop()?;
        }
        Ok(self.queue.nodes())
    }

    pub fn minimum_score(&self) -> f32 {
        self.queue.top_score()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.visited_count = 0;
    }
}
impl KnnCollector for GraphBuilderKnnCollector {
    fn early_terminated(&self) -> bool {
        false
    }

    fn inc_visited_count(&mut self, count: usize) {
        self.visited_count += count;
    }

    fn visited_count(&self) -> usize {
        self.visited_count
    }

    fn visit_limit(&self) -> usize {
        i64::MAX as usize
    }

    fn k(&self) -> i32 {
        self.k
    }

    fn collect(&mut self, doc_id: i32, similarity: f32) -> bool {
        self.queue.insert_with_overflow(doc_id, similarity)
    }

    fn min_competitive_similarity(&self) -> f32 {
        if self.queue.size() >= self.k {
            self.queue.top_score()
        } else {
            f32::NEG_INFINITY
        }
    }

    fn top_docs(&mut self) -> Result<TopDocs> {
        Err(LuceneError::illegal_state(""))
    }
}
