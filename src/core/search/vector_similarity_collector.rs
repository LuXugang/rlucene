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
use crate::core::search::abstract_knn_collector::{AbstractKnnCollector, AbstractKnnCollectorBase};
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::top_docs::TopDocs;
use crate::core::search::total_hits::{Relation, TotalHits};
use crate::core::util::CoreHelper;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
/// Perform a similarity-based graph search.
pub struct VectorSimilarityCollector {
  traversal_similarity: f32,
  result_similarity: f32,
  max_similarity: f32,
  score_doc_list: Vec<ScoreDoc>,
  base: AbstractKnnCollectorBase,
}

impl VectorSimilarityCollector {
  /// Perform a similarity-based graph search.
  ///
  /// The graph is traversed until no better scoring nodes are available, or
  /// the best candidate has a score below `traversalSimilarity`. All
  /// traversed nodes with scores above `resultSimilarity` are collected.
  ///
  /// # Arguments
  ///
  /// * `traversal_similarity` - (lower) similarity score threshold for graph
  ///   traversal.
  /// * `result_similarity` - (higher) similarity score threshold for result
  ///   collection.
  /// * `visit_limit` - The maximum number of nodes to visit.
  pub fn new(
    traversal_similarity: f32,
    result_similarity: f32,
    visit_limit: usize,
  ) -> Result<Self> {
    let base = AbstractKnnCollectorBase::new(1, visit_limit);
    if traversal_similarity > result_similarity {
      return Err(LuceneError::illegal_argument(
        "traversalSimilarity should be <= resultSimilarity",
      ));
    }
    Ok(Self {
      traversal_similarity,
      result_similarity,
      max_similarity: f32::NEG_INFINITY,
      score_doc_list: Vec::new(),
      base,
    })
  }
}

impl KnnCollector for VectorSimilarityCollector {
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
    self.max_similarity = CoreHelper::max_f32(self.max_similarity, similarity);
    if similarity >= self.result_similarity {
      debug_assert!(doc_id <= i32::MAX as usize);
      self
        .score_doc_list
        .push(ScoreDoc::new(doc_id as i32, similarity));
    }
    Ok(true)
  }

  fn min_competitive_similarity(&self) -> Result<f32> {
    Ok(CoreHelper::min_f32(
      self.traversal_similarity,
      self.max_similarity,
    ))
  }

  fn top_docs(&mut self) -> Result<TopDocs<ScoreDoc>> {
    // Results are not returned in a sorted order to prevent unnecessary
    // calculations (because we do not need to maintain the topK)
    let relation = if AbstractKnnCollector::early_terminated(self) {
      Relation::GreaterThanOrEqualTo
    } else {
      Relation::EqualTo
    };
    Ok(TopDocs::new(
      TotalHits::new(AbstractKnnCollector::visited_count(self), relation),
      std::mem::take(&mut self.score_doc_list),
    ))
  }
}
impl AbstractKnnCollector for VectorSimilarityCollector {
  fn num_collected(&self) -> usize {
    self.score_doc_list.len()
  }

  fn base(&self) -> &AbstractKnnCollectorBase {
    &self.base
  }

  fn base_mut(&mut self) -> &mut AbstractKnnCollectorBase {
    &mut self.base
  }
}
