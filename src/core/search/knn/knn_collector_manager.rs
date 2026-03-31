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
use crate::core::search::knn_collector::KnnCollector;
use crate::core::util::error::lucene_error;

/// KnnCollectorManager responsible for creating [`KnnCollector`] instances.
///
/// Useful to create [`KnnCollector`] instances that share global state across leaves,
/// such a global queue of results collected so far.
pub trait KnnCollectorManager {
  type KnnCollector<'a>: KnnCollector
  where
    Self: 'a;
  /// Return a new [`KnnCollector`] instance.
  ///
  /// # Arguments
  ///
  /// * `visitedLimit` - the maximum number of nodes that the search is allowed to visit
  /// * `context` - the leaf reader context
  fn new_collector<LR>(
    &mut self,
    visited_limit: usize,
    context: LeafReaderContext<LR>,
  ) -> lucene_error::Result<Self::KnnCollector<'_>>
  where
    LR: LeafReader;
}
pub type KnnCollectorManagerScoreDocLike<'a, K> =
  <<K as KnnCollectorManager>::KnnCollector<'a> as KnnCollector>::ScoreDocLike;
