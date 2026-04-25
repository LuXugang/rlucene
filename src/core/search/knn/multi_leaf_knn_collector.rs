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
use crate::core::search::abstract_knn_collector::AbstractKnnCollector;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::top_docs::TopDocs;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::hnsw::blocking_float_heap::BlockingFloatHeap;
use crate::core::util::hnsw::float_heap::FloatHeap;

/// MultiLeafKnnCollector is a specific KnnCollector that can exchange the top collected results
/// across segments through a shared global queue.
pub struct MultiLeafKnnCollector<'a, A>
where
  A: AbstractKnnCollector,
{
  /// interval to synchronize the local and global queues, as a number of visited vectors
  pub(crate) interval: usize,
  /// the global queue of the highest similarities collected so far across all segments
  pub(crate) global_similarity_queue: &'a BlockingFloatHeap,
  /// the local queue of the highest similarities if we are not competitive globally
  /// the size of this queue is defined by greediness
  pub(crate) non_competitive_queue: FloatHeap,
  /// the queue of the local similarities to periodically update with the global queue
  pub(crate) updates_queue: FloatHeap,
  pub(crate) updates_scratch: Vec<f32>,
  pub(crate) k_results_collected: bool,
  pub(crate) cached_global_min_sim: f32,
  pub(crate) sub_collector: A,
}

impl<'a, A> MultiLeafKnnCollector<'a, A>
where
  A: AbstractKnnCollector,
{
  /// greediness of globally non-competitive search: (0,1]
  const DEFAULT_GREEDINESS: f32 = 0.9f32;
  const DEFAULT_INTERVAL: usize = 0xff;

  /// Create a new MultiLeafKnnCollector.
  ///
  /// # Arguments
  ///
  /// * `k` - the number of neighbors to collect
  /// * `global_similarity_queue` - the global queue of the highest similarities collected so far
  ///   across all segments
  /// * `sub_collector` - the local collector
  pub fn new(
    k: usize,
    global_similarity_queue: &'a BlockingFloatHeap,
    sub_collector: A,
  ) -> Result<Self> {
    Self::with_params(
      k,
      Self::DEFAULT_GREEDINESS,
      Self::DEFAULT_INTERVAL,
      global_similarity_queue,
      sub_collector,
    )
  }

  /// Create a new MultiLeafKnnCollector.
  ///
  /// # Arguments
  ///
  /// * `k` - the number of neighbors to collect
  /// * `greediness` - the greediness of the global search
  /// * `interval` - (by number of collected values) the interval to synchronize the local and
  ///   global queues
  /// * `global_similarity_queue` - the global queue of the highest similarities collected so far
  /// * `sub_collector` - the local collector
  pub fn with_params(
    k: usize,
    greediness: f32,
    interval: usize,
    global_similarity_queue: &'a BlockingFloatHeap,
    sub_collector: A,
  ) -> Result<Self> {
    if !(0.0..=1.0).contains(&greediness) {
      return Err(LuceneError::illegal_argument("greediness must be in [0,1]"));
    }
    if interval == 0 {
      return Err(LuceneError::illegal_argument("interval must be positive"));
    }

    Ok(Self {
      interval,
      sub_collector,
      global_similarity_queue,
      non_competitive_queue: FloatHeap::new(std::cmp::max(
        1,
        ((1.0 - greediness) * k as f32).round() as usize,
      ))?,
      updates_queue: FloatHeap::new(k)?,
      updates_scratch: vec![0.0; k],
      k_results_collected: false,
      cached_global_min_sim: f32::NEG_INFINITY,
    })
  }
}

impl<A> KnnCollector for MultiLeafKnnCollector<'_, A>
where
  A: AbstractKnnCollector,
{
  fn early_terminated(&self) -> bool {
    AbstractKnnCollector::early_terminated(&self.sub_collector)
  }

  fn inc_visited_count(&mut self, count: usize) {
    AbstractKnnCollector::inc_visited_count(&mut self.sub_collector, count);
  }

  fn visited_count(&self) -> usize {
    AbstractKnnCollector::visited_count(&self.sub_collector)
  }

  fn visit_limit(&self) -> usize {
    AbstractKnnCollector::visit_limit(&self.sub_collector)
  }

  fn k(&self) -> usize {
    AbstractKnnCollector::k(&self.sub_collector)
  }

  fn collect(&mut self, doc_id: usize, similarity: f32) -> Result<bool> {
    let local_sim_updated = self.sub_collector.collect(doc_id, similarity)?;

    let first_k_results_collected =
      !self.k_results_collected && self.sub_collector.num_collected() == self.k();

    if first_k_results_collected {
      self.k_results_collected = true;
    }

    self.updates_queue.offer(similarity);
    let mut global_sim_updated = self.non_competitive_queue.offer(similarity);

    if self.k_results_collected {
      // as we've collected k results, we can start do periodic updates with the global queue
      if first_k_results_collected
        || (AbstractKnnCollector::visited_count(&self.sub_collector) & self.interval) == 0
      {
        // BlockingFloatHeap#offer requires input to be sorted in ascending order, so we can't
        // pass in the underlying updatesQueue array as-is since it is only partially ordered
        // (see GH#13462):
        let len = self.updates_queue.size();
        if len > 0 {
          for i in 0..len {
            self.updates_scratch[i] = self.updates_queue.poll()?;
          }
          debug_assert!(self.updates_queue.size() == 0);

          self.cached_global_min_sim = self
            .global_similarity_queue
            .offer_array(&self.updates_scratch, len);
          global_sim_updated = true;
        }
      }
    }

    Ok(local_sim_updated || global_sim_updated)
  }

  fn min_competitive_similarity(&self) -> Result<f32> {
    if !self.k_results_collected {
      return Ok(f32::NEG_INFINITY);
    }

    Ok(f32::max(
      self.sub_collector.min_competitive_similarity()?,
      f32::min(
        self.non_competitive_queue.peek(),
        self.cached_global_min_sim,
      ),
    ))
  }

  fn top_docs(&mut self) -> Result<TopDocs<ScoreDoc>> {
    self.sub_collector.top_docs()
  }
}
impl<A> std::fmt::Display for MultiLeafKnnCollector<'_, A>
where
  A: AbstractKnnCollector + std::fmt::Display,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "MultiLeafKnnCollector[subCollector={}]",
      self.sub_collector
    )
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::search::top_knn_collector::TopKnnCollector;

  #[allow(dead_code)] // for quick search
  struct TestMultiLeafKnnCollector;

  /** Validates a fix for GH#13462 */
  #[test]
  fn test_global_score_coordination() -> Result<()> {
    let k = 7;
    let global_heap = BlockingFloatHeap::new(k);
    let mut collector1 =
      MultiLeafKnnCollector::new(k, &global_heap, TopKnnCollector::new(k, i32::MAX as usize)?)?;
    let mut collector2 =
      MultiLeafKnnCollector::new(k, &global_heap, TopKnnCollector::new(k, i32::MAX as usize)?)?;

    // Collect k (7) hits in collector1 with scores [100, 106]:
    for i in 0..k {
      collector1.collect(0, 100.0 + i as f32)?;
    }

    // The global heap should be updated since k hits were collected, and have a min score of
    // 100:
    assert_eq!(100.0, global_heap.peek());
    assert_eq!(100.0, collector1.min_competitive_similarity()?);

    // Collect k (7) hits in collector2 with only two that are competitive (200 and 300),
    // which also forces an update of the global heap with collector2's hits. This is a tricky
    // case where the heap will not be fully ordered, so it ensures global queue updates don't
    // incorrectly short-circuit (see GH#13462):
    collector2.collect(0, 10.0)?;
    collector2.collect(0, 11.0)?;
    collector2.collect(0, 12.0)?;
    collector2.collect(0, 13.0)?;
    collector2.collect(0, 200.0)?;
    collector2.collect(0, 14.0)?;
    collector2.collect(0, 300.0)?;

    // At this point, our global heap should contain [102, 103, 104, 105, 106, 200, 300] since
    // values 200 and 300 from collector2 should have pushed out 100 and 101 from collector1.
    // The min value on the global heap should be 102:
    assert_eq!(102.0, global_heap.peek());
    assert_eq!(102.0, collector2.min_competitive_similarity()?);
    Ok(())
  }
}
