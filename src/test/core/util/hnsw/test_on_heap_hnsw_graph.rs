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
use rand::RngExt;

use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::hnsw::hnsw_graph::{HnswGraph, NodesIterator};
use crate::core::util::hnsw::on_heap_hnsw_graph::OnHeapHnswGraph;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;

#[allow(dead_code)] // for quick search
struct TestOnHeapHnswGraph;
#[test]
fn test_no_growth() {
  let mut graph = OnHeapHnswGraph::new(10, 100);

  let result = graph.add_node(1, 100);

  assert!(
    matches!(result, Err(LuceneError::IllegalState(msg)) if msg.message.contains("does not expect to grow")),
  );
}
#[test]
fn test_add_level_out_of_order() {
  let mut graph = OnHeapHnswGraph::new(10, -1);

  graph.add_node(0, 0).unwrap();

  let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    graph.add_node(1, 0).unwrap();
  }));

  assert!(
    result.is_err(),
    "Expected panic when adding level out of order"
  );
}
#[test]
fn test_incomplete_graph_throw() {
  let mut graph = OnHeapHnswGraph::new(10, -1);

  graph.add_node(1, 0).unwrap();
  graph.add_node(0, 0).unwrap();

  let level1 = graph.get_nodes_on_level(1).unwrap();
  assert_eq!(level1.size(), 1);

  graph.add_node(0, 5).unwrap();

  let result = graph.get_nodes_on_level(0);
  assert!(
    matches!(result, Err(LuceneError::IllegalState(msg)) if msg.message.contains("graph build not complete")),
  );
}
#[test]
fn test_graph_growth() -> Result<()> {
  let mut random = random();
  let mut graph = OnHeapHnswGraph::new(10, -1);

  let max_level = 5;
  let mut level_to_nodes: Vec<Vec<usize>> = vec![Vec::new(); max_level];

  for i in 0..101 {
    let level = random.random_range(0..max_level);
    for l in (0..=level).rev() {
      graph.add_node(l, i)?;
      graph.try_set_new_entry_node(i, l);
      if l > graph.num_levels()? - 1 {
        graph.try_promote_new_entry_node(i, l, graph.num_levels()? - 1);
      }
      level_to_nodes[l].push(i);
    }
  }

  assert_graph_equals(&mut graph, &level_to_nodes)?;

  Ok(())
}
#[test]
fn test_graph_build_out_of_order() -> Result<()> {
  let mut random = random();
  let mut graph = OnHeapHnswGraph::new(10, -1);

  let max_level = 5;
  let num_nodes = 100;
  let mut level_to_nodes: Vec<Vec<usize>> = vec![Vec::new(); max_level];

  let mut insertions: Vec<usize> = (0..num_nodes).collect();

  // Shuffle insertion order 40 times
  for _ in 0..40 {
    let pos1 = random.random_range(0..num_nodes);
    let pos2 = random.random_range(0..num_nodes);
    insertions.swap(pos1, pos2);
  }

  for &i in &insertions {
    let level = random.random_range(0..max_level);
    for l in (0..=level).rev() {
      graph.add_node(l, i)?;
      graph.try_set_new_entry_node(i, l);
      if l > graph.num_levels()? - 1 {
        graph.try_promote_new_entry_node(i, l, graph.num_levels()? - 1);
      }
      level_to_nodes[l].push(i);
    }
  }

  // Sort nodes per level for order-insensitive comparison
  for nodes in &mut level_to_nodes {
    nodes.sort_unstable();
  }

  assert_graph_equals(&mut graph, &level_to_nodes)?;

  Ok(())
}

fn assert_graph_equals(graph: &mut impl HnswGraph, level_to_nodes: &[Vec<usize>]) -> Result<()> {
  let num_levels = graph.num_levels()?;

  for (level, expected) in level_to_nodes.iter().enumerate().take(num_levels) {
    let mut nodes_iterator = graph.get_nodes_on_level(level)?;

    assert_eq!(
      expected.len(),
      nodes_iterator.size(),
      "Mismatch at level {}",
      level
    );

    for (idx, expected_val) in expected.iter().enumerate() {
      let actual = nodes_iterator.next().unwrap();
      assert_eq!(
        *expected_val, actual,
        "Mismatch at level {}, index {}",
        level, idx
      );
    }

    assert!(
      nodes_iterator.next().is_none(),
      "Extra elements in iterator at level {}",
      level
    );
  }

  Ok(())
}
