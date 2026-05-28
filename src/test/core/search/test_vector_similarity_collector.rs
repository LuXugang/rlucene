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
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::vector_similarity_collector::VectorSimilarityCollector;
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)] // for quick search
struct TestVectorSimilarityCollector;
#[test]
fn test_result_collection() -> Result<()> {
  let traversal_similarity = 0.3f32;
  let result_similarity = 0.5f32;

  let mut collector =
    VectorSimilarityCollector::new(traversal_similarity, result_similarity, i32::MAX as usize)?;

  let nodes = [1, 5, 10, 4, 8, 3, 2, 6, 7, 9];
  let scores = [0.1, 0.2, 0.3, 0.5, 0.2, 0.6, 0.9, 0.3, 0.7, 0.8];

  let mut min_competitive_similarities = vec![];

  for (&node, &score) in nodes.iter().zip(scores.iter()) {
    collector.collect(node, score)?;
    min_competitive_similarities.push(collector.min_competitive_similarity()?);
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
