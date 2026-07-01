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
use crate::core::search::top_knn_collector::TopKnnCollector;
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)] // for quick search
struct TestTopKnnResults;
#[test]
fn test_collect_and_provide_results() -> Result<()> {
  let mut results = TopKnnCollector::new(5, i32::MAX as usize)?;
  let nodes = [4, 1, 5, 7, 8, 10, 2];
  let scores = [1.0, 0.5, 0.6, 2.0, 2.0, 1.2, 4.0];

  for (node, score) in nodes.iter().zip(scores.iter()) {
    results.collect(*node, *score)?;
  }

  let top_docs = results.top_docs()?;
  let sorted_nodes: Vec<i32> = top_docs.score_docs.iter().map(|doc| doc.doc).collect();
  let sorted_scores: Vec<f32> = top_docs.score_docs.iter().map(|doc| doc.score).collect();

  assert_eq!(sorted_nodes, vec![2, 7, 8, 10, 4]);
  assert!(
    sorted_scores
      .iter()
      .zip([4.0, 2.0, 2.0, 1.2, 1.0].iter())
      .all(|(a, b)| (a - b).abs() < f32::EPSILON),
    "Scores do not match: {:?} vs expected",
    sorted_scores
  );
  Ok(())
}
