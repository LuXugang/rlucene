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
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::top_docs::TopDocs;
use crate::core::util::error::lucene_error;
use crate::core::util::error::lucene_error::Result;

/// KnnCollector is a knn collector used for gathering kNN results and providing
/// topDocs from the gathered neighbors
pub trait KnnCollector {
  /// If search visits too many documents, the results collector will
  /// terminate early.
  ///
  /// Usually, this is due to some restricted filter on the document set.
  ///
  /// When collection is early terminated, the results are not a correct
  /// representation of k nearest neighbors.
  ///
  /// # Returns
  ///
  /// Whether the current result set is marked as incomplete.
  fn early_terminated(&self) -> bool;

  /// Increments the visited vector count.
  ///
  /// # Arguments
  ///
  /// * `count` - must be greater than 0.
  fn inc_visited_count(&mut self, count: usize);

  /// Returns the current visited vector count.
  fn visited_count(&self) -> usize;

  /// Returns the visited vector limit.
  fn visit_limit(&self) -> usize;

  /// Returns the expected number of collected results.
  fn k(&self) -> usize;

  /// Collects the provided `doc_id` and includes it in the result set.
  ///
  /// # Arguments
  ///
  /// * `doc_id` - ID of the vector to collect.
  /// * `similarity` - its calculated similarity.
  ///
  /// # Returns
  ///
  /// `true` if the vector is collected.
  fn collect(&mut self, doc_id: usize, similarity: f32) -> Result<bool>;

  /// This method is utilized during search to ensure only competitive results
  /// are explored.
  ///
  /// If this results collector wants to collect `k` results, this should
  /// return [`f32::NEG_INFINITY`] when not full. When full, the minimum
  /// score should be returned.
  ///
  /// # Returns
  ///
  /// The current minimum competitive similarity in the collection.
  fn min_competitive_similarity(&self) -> Result<f32>;

  /// This drains the collected nearest kNN results and returns them as a
  /// [`TopDocs`] collection, ordered by score descending.
  ///
  /// **Note:** This is generally a destructive action and the collector
  /// should not be used after `top_docs()` is called.
  ///
  /// # Returns
  ///
  /// The collected top documents.
  fn top_docs(&mut self) -> Result<TopDocs<ScoreDoc>>;
}

pub enum KnnCollectorEnum<A, B> {
  A(A),
  B(B),
}

impl<A, B> KnnCollector for KnnCollectorEnum<A, B>
where
  A: KnnCollector,
  B: KnnCollector,
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

  fn top_docs(&mut self) -> lucene_error::Result<TopDocs<ScoreDoc>> {
    match self {
      KnnCollectorEnum::A(a) => a.top_docs(),
      KnnCollectorEnum::B(b) => b.top_docs(),
    }
  }
}
