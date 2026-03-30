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
use crate::core::search::top_docs::TopDocs;
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
    &self,
    visited_limit: usize,
    context: LeafReaderContext<LR>,
  ) -> lucene_error::Result<Self::KnnCollector<'_>>
  where
    LR: LeafReader;
}

pub enum KnnCollectorEnum<A, B> {
  A(A),
  B(B),
}

impl<A, B> KnnCollector for KnnCollectorEnum<A, B>
where
  A: KnnCollector,
  B: KnnCollector<ScoreDocLike = A::ScoreDocLike>,
{
  fn early_terminated(&self) -> bool {
    match self {
      KnnCollectorEnum::A(a) => a.early_terminated(),
      KnnCollectorEnum::B(b) => b.early_terminated(),
    }
  }

  fn inc_visited_count(&mut self, count: usize) {
    match self {
      KnnCollectorEnum::A(a) => a.inc_visited_count(count),
      KnnCollectorEnum::B(b) => b.inc_visited_count(count),
    }
  }

  fn visited_count(&self) -> usize {
    match self {
      KnnCollectorEnum::A(a) => a.visited_count(),
      KnnCollectorEnum::B(b) => b.visited_count(),
    }
  }

  fn visit_limit(&self) -> usize {
    match self {
      KnnCollectorEnum::A(a) => a.visit_limit(),
      KnnCollectorEnum::B(b) => b.visit_limit(),
    }
  }

  fn k(&self) -> usize {
    match self {
      KnnCollectorEnum::A(a) => a.k(),

      KnnCollectorEnum::B(b) => b.k(),
    }
  }

  fn collect(&mut self, doc_id: usize, similarity: f32) -> lucene_error::Result<bool> {
    match self {
      KnnCollectorEnum::A(a) => a.collect(doc_id, similarity),
      KnnCollectorEnum::B(b) => b.collect(doc_id, similarity),
    }
  }

  fn min_competitive_similarity(&self) -> lucene_error::Result<f32> {
    match self {
      KnnCollectorEnum::A(a) => a.min_competitive_similarity(),
      KnnCollectorEnum::B(b) => b.min_competitive_similarity(),
    }
  }

  type ScoreDocLike = A::ScoreDocLike;

  fn top_docs(&mut self) -> lucene_error::Result<TopDocs<Self::ScoreDocLike>> {
    match self {
      KnnCollectorEnum::A(a) => a.top_docs(),
      KnnCollectorEnum::B(b) => b.top_docs(),
    }
  }
}
