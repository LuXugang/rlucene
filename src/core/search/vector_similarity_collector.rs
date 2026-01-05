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
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
/// Perform a similarity-based graph search.
pub struct VectorSimilarityCollector {
    traversal_similarity: f32,
    result_similarity: f32,
    max_similarity: f32,
    score_doc_list: Vec<ScoreDoc>,
    base: AbstractKnnCollector,
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
        let base = AbstractKnnCollector::new(1, visit_limit);
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
        self.base.early_terminated()
    }

    fn inc_visited_count(&mut self, count: usize) {
        self.base.inc_visited_count(count);
    }

    fn visited_count(&self) -> usize {
        self.base.visited_count()
    }

    fn visit_limit(&self) -> usize {
        self.base.visit_limit()
    }

    fn k(&self) -> usize {
        self.base.k()
    }

    fn collect(&mut self, doc_id: usize, similarity: f32) -> bool {
        self.max_similarity = self.max_similarity.max(similarity);
        if similarity >= self.result_similarity {
            debug_assert!(doc_id <= i32::MAX as usize);
            self.score_doc_list
                .push(ScoreDoc::new(doc_id as i32, similarity));
        }
        true
    }

    fn min_competitive_similarity(&self) -> f32 {
        self.traversal_similarity.min(self.max_similarity)
    }

    type Item = ScoreDoc;

    fn top_docs(&mut self) -> Result<TopDocs<Self::Item>> {
        // Results are not returned in a sorted order to prevent unnecessary
        // calculations (because we do not need to maintain the topK)
        let relation = if self.early_terminated() {
            Relation::GreaterThanOrEqualTo
        } else {
            Relation::EqualTo
        };
        Ok(TopDocs::new(
            TotalHits::new(self.visited_count(), relation),
            std::mem::take(&mut self.score_doc_list),
        ))
    }
}

impl AbstractKnnCollectorBase for VectorSimilarityCollector {
    fn num_collected(&self) -> usize {
        self.score_doc_list.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::core::search::knn_collector::KnnCollector;
    use crate::core::search::vector_similarity_collector::VectorSimilarityCollector;
    use crate::core::util::error::lucene_error::Result;

    #[allow(dead_code)] // for quick search
    struct TestVectorSimilarityCollector;
    #[test]
    fn test_result_collection() -> Result<()> {
        let traversal_similarity = 0.3f32;
        let result_similarity = 0.5f32;

        let mut collector = VectorSimilarityCollector::new(
            traversal_similarity,
            result_similarity,
            i32::MAX as usize,
        )?;

        let nodes = [1, 5, 10, 4, 8, 3, 2, 6, 7, 9];
        let scores = [0.1, 0.2, 0.3, 0.5, 0.2, 0.6, 0.9, 0.3, 0.7, 0.8];

        let mut min_competitive_similarities = vec![];

        for (&node, &score) in nodes.iter().zip(scores.iter()) {
            collector.collect(node, score);
            min_competitive_similarities.push(collector.min_competitive_similarity());
        }

        let top_docs = collector.top_docs()?;
        let result_nodes: Vec<i32> = top_docs.score_docs.iter().map(|sd| sd.doc).collect();
        let result_scores: Vec<f32> = top_docs.score_docs.iter().map(|sd| sd.score).collect();
        // All nodes above resultSimilarity appear in order of collection
        assert_eq!(result_nodes, vec![4, 3, 2, 7, 9]);
        assert_eq_approx(&result_scores, &[0.5, 0.6, 0.9, 0.7, 0.8], 1e-3);
        // All nodes above resultSimilarity appear in order of collection
        let expected_min = [0.1, 0.2, 0.3, 0.3, 0.3, 0.3, 0.3, 0.3, 0.3, 0.3];
        assert_eq_approx(&min_competitive_similarities, &expected_min, 1e-3);
        Ok(())
    }

    fn assert_eq_approx(actual: &[f32], expected: &[f32], epsilon: f32) {
        assert_eq!(actual.len(), expected.len(), "length mismatch");
        for (i, (a, b)) in actual.iter().zip(expected.iter()).enumerate() {
            assert!(
                (a - b).abs() <= epsilon,
                "difference at index {}: actual={}, expected={}",
                i,
                a,
                b
            );
        }
    }
}
