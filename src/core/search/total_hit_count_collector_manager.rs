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
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::index_searcher::LeafSlice;
use crate::core::search::total_hit_count_collector::TotalHitCountCollector;

/// Collector manager based on [`TotalHitCountCollector`](crate::core::search::total_hit_count_collector::TotalHitCountCollector) that allows users to parallelize
/// counting the number of hits, expected to be used mostly wrapped in [`MultiCollectorManager`](crate::core::search::multi_collector_manager::MultiCollectorManager).
///
/// For cases when this is the only collector manager used, [`IndexSearcher::count(query)`](crate::core::search::index_searcher::IndexSearcher::count)
/// should be called instead of [`IndexSearcher::search(query, CollectorManager)`](crate::core::search::index_searcher::search) as the former is
/// faster whenever the count can be returned directly from the index statistics.
pub struct TotalHitCountCollectorManager {
    has_segment_partitions: bool,
}
impl TotalHitCountCollectorManager {
    /// Creates a new total hit count collector manager, providing the array of leaf slices that search
    /// targets, which can be retrieved via [`IndexSearcher::get_slices`](crate::core::search::index_searcher::IndexSearcher::get_slices) for the searcher.
    ///
    /// # Parameters
    /// - `leaf_slices`: the slices that the searcher targets.
    ///   Used to optimize the collection depending on whether segments have been partitioned into
    ///   partitions or not.
    pub fn new(leaf_slices: &[LeafSlice]) -> Self {
        let has_segment_partitions = Self::has_segment_partitions(leaf_slices);
        Self {
            has_segment_partitions,
        }
    }
    pub fn has_segment_partitions(leaf_slices: &[LeafSlice]) -> bool {
        for slice in leaf_slices {
            for partition in &slice.partitions {
                if partition.min_doc_id > 0 || partition.max_doc_id < partition.ctx_max_doc {
                    return true;
                }
            }
        }
        false
    }
}
impl CollectorManager for TotalHitCountCollectorManager {
    type C = TotalHitCountCollector;
    type T = i32;

    fn new_collector(&self) -> crate::core::util::error::lucene_error::Result<Self::C> {
        if self.has_segment_partitions {
            todo!()
            // TODO
        }
        Ok(TotalHitCountCollector::new())
    }

    fn reduce(
        &self,
        collectors: Vec<Self::C>,
    ) -> crate::core::util::error::lucene_error::Result<Self::T> {
        // Make the same collector manager instance reusable across multiple searches.
        // It isn't a strict requirement but is generally supported as collector managers normally
        // don't hold state, as opposed to collectors.

        // TODO
        // assert has_segment_partitions || early_terminated_map.is_empty();
        if self.has_segment_partitions {
            todo!()
        }
        let mut total_hits = 0;
        for collector in collectors {
            total_hits += collector.total_hit;
        }
        Ok(total_hits)
    }
}
