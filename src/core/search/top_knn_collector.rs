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

use crate::core::search::abstract_knn_collector::{AbstractKnnCollector, AbstractKnnCollectorBase};
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::top_docs::TopDocs;
use crate::core::search::total_hits::{Relation, TotalHits};
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::neighbor_queue::NeighborQueue;

/// `TopKnnCollector` is a specific [`KnnCollector`] implementation.
/// A min-heap is used to keep track of the currently collected vectors,
/// allowing for efficient updates as better vectors are collected.
pub struct TopKnnCollector {
  queue: NeighborQueue,
  base: AbstractKnnCollectorBase,
}

impl TopKnnCollector {
  /// # Arguments
  ///
  /// * `k` - the number of neighbors to collect
  /// * `visit_limit` - how many vector nodes the results are allowed to visit
  pub fn new(k: usize, visit_limit: usize) -> Result<Self> {
    let base = AbstractKnnCollectorBase::new(k, visit_limit);
    Ok(Self {
      queue: NeighborQueue::new(k, false)?,
      base,
    })
  }
}

impl KnnCollector for TopKnnCollector {
  fn early_terminated(&self) -> bool {
    AbstractKnnCollector::early_terminated(self)
  }

  fn inc_visited_count(&mut self, count: usize) {
    AbstractKnnCollector::inc_visited_count(self, count)
  }

  fn visited_count(&self) -> usize {
    AbstractKnnCollector::visited_count(self)
  }

  fn visit_limit(&self) -> usize {
    AbstractKnnCollector::visit_limit(self)
  }

  fn k(&self) -> usize {
    AbstractKnnCollector::k(self)
  }

  fn collect(&mut self, doc_id: usize, similarity: f32) -> Result<bool> {
    self.queue.insert_with_overflow(doc_id, similarity)
  }

  fn min_competitive_similarity(&self) -> Result<f32> {
    if self.queue.size() >= AbstractKnnCollector::k(self) {
      Ok(self.queue.top_score())
    } else {
      Ok(f32::NEG_INFINITY)
    }
  }

  fn top_docs(&mut self) -> Result<TopDocs<ScoreDoc>> {
    debug_assert!(
      self.queue.size() <= AbstractKnnCollector::k(self),
      "Tried to collect more results than the maximum number allowed"
    );

    let mut score_docs = vec![ScoreDoc::default(); self.queue.size()];
    for i in 1..=score_docs.len() {
      let doc_id = self.queue.top_node();
      let score = self.queue.top_score();
      let len = score_docs.len() - i;
      score_docs[len] = ScoreDoc::new(doc_id.try_convert()?, score);
      self.queue.pop()?;
    }

    let relation = if AbstractKnnCollector::early_terminated(self) {
      Relation::GreaterThanOrEqualTo
    } else {
      Relation::EqualTo
    };

    let total_hits = TotalHits::new(AbstractKnnCollector::visited_count(self), relation);
    Ok(TopDocs::new(total_hits, score_docs))
  }
}
impl AbstractKnnCollector for TopKnnCollector {
  fn num_collected(&self) -> usize {
    self.queue.size()
  }

  fn base(&self) -> &AbstractKnnCollectorBase {
    &self.base
  }

  fn base_mut(&mut self) -> &mut AbstractKnnCollectorBase {
    &mut self.base
  }
}

impl fmt::Display for TopKnnCollector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "TopKnnCollector[k={}, size={}]",
      AbstractKnnCollector::k(self),
      self.queue.size()
    )
  }
}
