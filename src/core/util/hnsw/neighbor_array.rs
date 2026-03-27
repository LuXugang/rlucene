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
use std::fmt;

use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::hnsw::dummy::dummy_random_vector_scorer::DummyRandomVectorScorer;
use crate::core::util::hnsw::hnsw_graph::HnswGraph;
use crate::core::util::hnsw::on_heap_hnsw_graph::OnHeapHnswGraph;
use crate::core::util::hnsw::random_vector_scorer::RandomVectorScorer;
use crate::core::util::hnsw::random_vector_scorer_supplier::RandomVectorScorerSupplier;
/// `NeighborArray` encodes the neighbors of a node and their mutual scores in
/// the HNSW graph as a pair of growable arrays. Nodes are arranged in the
/// sorted order of their scores in:
///
/// - descending order if `scores_desc_order` is `true`
/// - ascending order if `scores_desc_order` is `false`
#[derive(Clone, Default)]
pub struct NeighborArray {
  scores_desc_order: bool,
  size: usize,
  scores: Vec<f32>,
  nodes: Vec<usize>,
  sorted_node_size: usize,
}

impl NeighborArray {
  /// Creates a new NeighborArray with the given capacity and sort order.
  pub fn new(max_size: usize, desc_order: bool) -> Self {
    Self {
      scores_desc_order: desc_order,
      size: 0,
      sorted_node_size: 0,
      scores: vec![0.0; max_size],
      nodes: vec![0; max_size],
    }
  }
  /// Add a new node to the `NeighborArray`.
  /// The new node must be worse than all previously stored nodes.
  /// This cannot be called after [`add_out_of_order(int,
  /// float)`](Self::add_out_of_order).
  pub fn add_in_order(&mut self, new_node: usize, new_score: f32) -> Result<()> {
    debug_assert!(
      self.size == self.sorted_node_size,
      "cannot call add_in_order after add_out_of_order"
    );

    if self.size == self.nodes.len() {
      return Err(LuceneError::illegal_state("No growth is allowed"));
    }

    if self.size > 0 {
      debug_assert!(
        {
          let previous_score = self.scores[self.size - 1];
          if self.scores_desc_order {
            previous_score >= new_score
          } else {
            previous_score <= new_score
          }
        },
        "Nodes are added in the incorrect order! Comparing {} to {:?}",
        new_score,
        &self.scores[0..self.size]
      );
    }

    self.nodes[self.size] = new_node;
    self.scores[self.size] = new_score;
    self.size += 1;
    self.sorted_node_size += 1;

    Ok(())
  }
  /// Add node and newScore but do not insert as sorted
  pub fn add_out_of_order(&mut self, new_node: usize, new_score: f32) -> Result<()> {
    if self.size == self.nodes.len() {
      return Err(LuceneError::illegal_state("No growth is allowed"));
    }

    self.scores[self.size] = new_score;
    self.nodes[self.size] = new_node;
    self.size += 1;

    Ok(())
  }
  /// In addition to [`add_out_of_order(int, float)`](Self::add_out_of_order),
  /// this function will also remove the least-diverse node if the node
  /// array is full after insertion.
  ///
  /// In multi-threading environment, this method needs to be locked as it
  /// will be called by multiple threads, while the other add method is
  /// only supposed to be called by one thread.
  ///
  /// # Arguments
  ///
  /// * `node_id` - Node ID of the owner of this `NeighborArray`.
  pub fn add_and_ensure_diversity(
    hnsw: &mut OnHeapHnswGraph,
    level: usize,
    new_node: usize,
    new_score: f32,
    node_id: usize,
    scorer_supplier: &mut impl RandomVectorScorerSupplier,
  ) -> Result<()> {
    let neighbor_array = hnsw.get_neighbors_mut(level, node_id)?;
    neighbor_array.add_out_of_order(new_node, new_score)?;

    if neighbor_array.size < neighbor_array.nodes.len() {
      return Ok(());
    }

    // We're oversize, need to drop the least diverse neighbor
    let worst_idx = neighbor_array.find_worst_non_diverse(node_id, scorer_supplier)?;
    neighbor_array.remove_index(worst_idx);

    debug_assert!(neighbor_array.size == neighbor_array.nodes.len() - 1);

    Ok(())
  }
  /// Sorts the array according to scores, and returns the sorted indexes of
  /// previously unsorted nodes (unchecked nodes).
  ///
  /// # Returns
  ///
  /// Indexes of newly sorted (unchecked) nodes, in ascending order, or `None`
  /// if the array is already fully sorted.
  pub(crate) fn sort<S>(&mut self, scorer: &mut S) -> Result<Vec<usize>>
  where
    S: RandomVectorScorer,
  {
    if self.size == self.sorted_node_size {
      // all nodes checked and sorted
      return Ok(vec![]);
    }

    debug_assert!(self.sorted_node_size < self.size);

    let mut unchecked_indexes = vec![0usize; self.size - self.sorted_node_size];
    let mut count = 0;

    while self.sorted_node_size != self.size {
      let inserted_index = self.insert_sorted_internal(scorer)?;
      unchecked_indexes[count] = inserted_index;

      for idx in &mut unchecked_indexes[..count] {
        if *idx >= inserted_index {
          *idx += 1;
        }
      }

      count += 1;
    }

    unchecked_indexes[..count].sort_unstable();
    Ok(unchecked_indexes)
  }
  /// insert the first unsorted node into its sorted position
  fn insert_sorted_internal<S>(&mut self, scorer: &mut S) -> Result<usize>
  where
    S: RandomVectorScorer,
  {
    debug_assert!(
      self.sorted_node_size < self.size,
      "Call this method only when there's an unsorted node"
    );

    let tmp_node = self.nodes[self.sorted_node_size];
    let mut tmp_score = self.scores[self.sorted_node_size];

    if tmp_score.is_nan() {
      tmp_score = scorer.score(tmp_node)?;
    }

    let insertion_point = if self.scores_desc_order {
      self.desc_sort_find_rightmost_insertion_point(tmp_score, self.sorted_node_size)
    } else {
      self.asc_sort_find_rightmost_insertion_point(tmp_score, self.sorted_node_size)
    };

    // Move [insertion_point..sorted_node_size) one position to the right
    self
      .nodes
      .copy_within(insertion_point..self.sorted_node_size, insertion_point + 1);
    self
      .scores
      .copy_within(insertion_point..self.sorted_node_size, insertion_point + 1);

    self.nodes[insertion_point] = tmp_node;
    self.scores[insertion_point] = tmp_score;

    self.sorted_node_size += 1;
    Ok(insertion_point)
  }
  /// This method is for test only.
  #[cfg(debug_assertions)]
  pub(crate) fn insert_sorted(&mut self, new_node: usize, new_score: f32) -> Result<()> {
    self.add_out_of_order(new_node, new_score)?;
    let mut v = DummyRandomVectorScorer;
    self.insert_sorted_internal(&mut v)?;
    Ok(())
  }
  pub fn size(&self) -> usize {
    self.size
  }
  /// irect access to the internal list of node ids; provided for efficient
  /// writing of the graph
  pub fn nodes(&self) -> &[usize] {
    &self.nodes
  }
  pub fn nodes_mut(&mut self) -> &mut [usize] {
    &mut self.nodes
  }

  pub fn scores(&self) -> &[f32] {
    &self.scores
  }

  pub fn clear(&mut self) {
    self.size = 0;
    self.sorted_node_size = 0;
  }

  pub(crate) fn remove_last(&mut self) {
    debug_assert!(self.size > 0);
    self.size -= 1;
    self.sorted_node_size = self.sorted_node_size.min(self.size);
  }

  pub(crate) fn remove_index(&mut self, idx: usize) {
    if idx == self.size - 1 {
      self.remove_last();
      return;
    }
    self.nodes.copy_within(idx + 1..self.size, idx);
    self.scores.copy_within(idx + 1..self.size, idx);

    if idx < self.sorted_node_size {
      self.sorted_node_size -= 1;
    }

    self.size -= 1;
  }
  fn asc_sort_find_rightmost_insertion_point(&self, new_score: f32, bound: usize) -> usize {
    match self.scores[0..bound].binary_search_by(|&s| s.partial_cmp(&new_score).unwrap()) {
      Ok(mut insertion_point) => {
        // move right over equal values
        while insertion_point < bound - 1
          && self.scores[insertion_point + 1] == self.scores[insertion_point]
        {
          insertion_point += 1;
        }
        insertion_point + 1
      },
      Err(pos) => pos,
    }
  }

  /// Finds the rightmost insertion point in descending order (stable insert).
  pub fn desc_sort_find_rightmost_insertion_point(&self, new_score: f32, bound: usize) -> usize {
    let mut start = 0;
    let mut end = bound as isize - 1;
    while start as isize <= end {
      let mid = (start + end as usize) / 2;
      if self.scores[mid] < new_score {
        end = mid as isize - 1;
      } else {
        start = mid + 1;
      }
    }
    start
  }
  /// Find first non-diverse neighbour among the list of neighbors starting
  /// from the most distant neighbours
  fn find_worst_non_diverse<S>(&mut self, node_ord: usize, scorer_supplier: &mut S) -> Result<usize>
  where
    S: RandomVectorScorerSupplier,
  {
    let unchecked_indexes = {
      let mut scorer = scorer_supplier.scorer(node_ord)?;
      self.sort(&mut scorer)?
    };

    debug_assert!(
      !unchecked_indexes.is_empty(),
      "We will always have something unchecked"
    );

    let mut unchecked_cursor = unchecked_indexes.len() as isize - 1;

    for i in (1..self.size).rev() {
      if unchecked_cursor < 0 {
        break; // no unchecked node left
      }

      let worst = self.is_worst_non_diverse(
        i,
        &unchecked_indexes,
        unchecked_cursor as usize,
        scorer_supplier,
      )?;
      if worst {
        return Ok(i);
      }

      if i == unchecked_indexes[unchecked_cursor as usize] {
        unchecked_cursor -= 1;
      }
    }

    Ok(self.size - 1)
  }
  fn is_worst_non_diverse<S>(
    &self,
    candidate_index: usize,
    unchecked_indexes: &[usize],
    unchecked_cursor: usize,
    scorer_supplier: &mut S,
  ) -> Result<bool>
  where
    S: RandomVectorScorerSupplier,
  {
    let min_accepted_similarity = self.scores[candidate_index];
    let candidate_node = self.nodes[candidate_index];
    let mut scorer = scorer_supplier.scorer(candidate_node)?;

    if candidate_index == unchecked_indexes[unchecked_cursor] {
      // the candidate itself is unchecked
      for i in (0..candidate_index).rev() {
        let neighbor_node = self.nodes[i];
        let neighbor_similarity = scorer.score(neighbor_node)?;
        if neighbor_similarity >= min_accepted_similarity {
          return Ok(true);
        }
      }
    } else {
      // else we just need to make sure candidate does not violate diversity with the
      // (newly inserted) unchecked nodes
      debug_assert!(candidate_index > unchecked_indexes[unchecked_cursor],);
      for &unchecked_idx in unchecked_indexes.iter().take(unchecked_cursor + 1).rev() {
        let neighbor_node = self.nodes[unchecked_idx];
        let neighbor_similarity = scorer.score(neighbor_node)?;
        if neighbor_similarity >= min_accepted_similarity {
          return Ok(true);
        }
      }
    }

    Ok(false)
  }
}
impl fmt::Display for NeighborArray {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}[{}]", std::any::type_name::<Self>(), self.size)
  }
}

#[cfg(test)]
mod tests {
  use std::panic::{AssertUnwindSafe, catch_unwind};

  use crate::core::util::bits::Bits;
  use crate::core::util::dummy::dummy_bits::DummyBits;
  use crate::core::util::error::lucene_error::Result;
  use crate::core::util::hnsw::dummy::dummy_random_vector_scorer::DummyRandomVectorScorer;
  use crate::core::util::hnsw::neighbor_array::NeighborArray;
  use crate::core::util::hnsw::random_vector_scorer::RandomVectorScorer;

  #[allow(dead_code)] // for quick search
  struct TestNeighborArray;
  #[test]
  fn test_scores_desc_order() -> Result<()> {
    let mut neighbors = NeighborArray::new(10, true);

    neighbors.add_in_order(0, 1.0)?;
    neighbors.add_in_order(1, 0.8)?;

    let result = catch_unwind(AssertUnwindSafe(|| {
      neighbors.add_in_order(2, 0.9).unwrap();
    }));
    assert!(result.is_err());
    if let Err(err) = result {
      if let Some(s) = err.downcast_ref::<String>() {
        assert!(
          s.contains("Nodes are added in the incorrect order!"),
          "{}",
          s
        );
      } else {
        unreachable!();
      }
    }

    neighbors.insert_sorted(3, 0.9)?;
    assert_scores_equal(&[1.0, 0.9, 0.8], &neighbors);
    assert_nodes_equal(&[0, 3, 1], &neighbors);

    neighbors.insert_sorted(4, 1.0)?;
    assert_scores_equal(&[1.0, 1.0, 0.9, 0.8], &neighbors);
    assert_nodes_equal(&[0, 4, 3, 1], &neighbors);

    neighbors.insert_sorted(5, 1.1)?;
    assert_scores_equal(&[1.1, 1.0, 1.0, 0.9, 0.8], &neighbors);
    assert_nodes_equal(&[5, 0, 4, 3, 1], &neighbors);

    neighbors.insert_sorted(6, 0.8)?;
    assert_scores_equal(&[1.1, 1.0, 1.0, 0.9, 0.8, 0.8], &neighbors);
    assert_nodes_equal(&[5, 0, 4, 3, 1, 6], &neighbors);

    neighbors.insert_sorted(7, 0.8)?;
    assert_scores_equal(&[1.1, 1.0, 1.0, 0.9, 0.8, 0.8, 0.8], &neighbors);
    assert_nodes_equal(&[5, 0, 4, 3, 1, 6, 7], &neighbors);

    neighbors.remove_index(2);
    assert_scores_equal(&[1.1, 1.0, 0.9, 0.8, 0.8, 0.8], &neighbors);
    assert_nodes_equal(&[5, 0, 3, 1, 6, 7], &neighbors);

    neighbors.remove_index(0);
    assert_scores_equal(&[1.0, 0.9, 0.8, 0.8, 0.8], &neighbors);
    assert_nodes_equal(&[0, 3, 1, 6, 7], &neighbors);

    neighbors.remove_index(4);
    assert_scores_equal(&[1.0, 0.9, 0.8, 0.8], &neighbors);
    assert_nodes_equal(&[0, 3, 1, 6], &neighbors);

    neighbors.remove_last();
    assert_scores_equal(&[1.0, 0.9, 0.8], &neighbors);
    assert_nodes_equal(&[0, 3, 1], &neighbors);

    neighbors.insert_sorted(8, 0.9)?;
    assert_scores_equal(&[1.0, 0.9, 0.9, 0.8], &neighbors);
    assert_nodes_equal(&[0, 3, 8, 1], &neighbors);

    Ok(())
  }
  #[test]
  fn test_scores_asc_order() -> Result<()> {
    let mut neighbors = NeighborArray::new(10, false);

    neighbors.add_in_order(0, 0.1)?;
    neighbors.add_in_order(1, 0.3)?;

    let result = catch_unwind(AssertUnwindSafe(|| {
      neighbors.add_in_order(2, 0.15).unwrap();
    }));
    assert!(result.is_err());
    if let Err(err) = result {
      if let Some(s) = err.downcast_ref::<String>() {
        assert!(
          s.contains("Nodes are added in the incorrect order!"),
          "{}",
          s
        );
      } else {
        unreachable!("panic payload is not String");
      }
    }

    neighbors.insert_sorted(3, 0.3)?;
    assert_scores_equal(&[0.1, 0.3, 0.3], &neighbors);
    assert_nodes_equal(&[0, 1, 3], &neighbors);

    neighbors.insert_sorted(4, 0.2)?;
    assert_scores_equal(&[0.1, 0.2, 0.3, 0.3], &neighbors);
    assert_nodes_equal(&[0, 4, 1, 3], &neighbors);

    neighbors.insert_sorted(5, 0.05)?;
    assert_scores_equal(&[0.05, 0.1, 0.2, 0.3, 0.3], &neighbors);
    assert_nodes_equal(&[5, 0, 4, 1, 3], &neighbors);

    neighbors.insert_sorted(6, 0.2)?;
    assert_scores_equal(&[0.05, 0.1, 0.2, 0.2, 0.3, 0.3], &neighbors);
    assert_nodes_equal(&[5, 0, 4, 6, 1, 3], &neighbors);

    neighbors.insert_sorted(7, 0.2)?;
    assert_scores_equal(&[0.05, 0.1, 0.2, 0.2, 0.2, 0.3, 0.3], &neighbors);
    assert_nodes_equal(&[5, 0, 4, 6, 7, 1, 3], &neighbors);

    neighbors.remove_index(2);
    assert_scores_equal(&[0.05, 0.1, 0.2, 0.2, 0.3, 0.3], &neighbors);
    assert_nodes_equal(&[5, 0, 6, 7, 1, 3], &neighbors);

    neighbors.remove_index(0);
    assert_scores_equal(&[0.1, 0.2, 0.2, 0.3, 0.3], &neighbors);
    assert_nodes_equal(&[0, 6, 7, 1, 3], &neighbors);

    neighbors.remove_index(4);
    assert_scores_equal(&[0.1, 0.2, 0.2, 0.3], &neighbors);
    assert_nodes_equal(&[0, 6, 7, 1], &neighbors);

    neighbors.remove_last();
    assert_scores_equal(&[0.1, 0.2, 0.2], &neighbors);
    assert_nodes_equal(&[0, 6, 7], &neighbors);

    neighbors.insert_sorted(8, 0.01)?;
    assert_scores_equal(&[0.01, 0.1, 0.2, 0.2], &neighbors);
    assert_nodes_equal(&[8, 0, 6, 7], &neighbors);

    Ok(())
  }
  #[test]
  fn test_sort_asc() -> Result<()> {
    let mut neighbors = NeighborArray::new(10, false);

    neighbors.add_out_of_order(1, 2.0)?;
    let result = catch_unwind(AssertUnwindSafe(|| {
      neighbors.add_in_order(1, 2.0).unwrap();
    }));
    assert!(result.is_err());

    neighbors.add_out_of_order(2, 3.0)?;
    neighbors.add_out_of_order(5, 6.0)?;
    neighbors.add_out_of_order(3, 4.0)?;
    neighbors.add_out_of_order(7, 8.0)?;
    neighbors.add_out_of_order(6, 7.0)?;
    neighbors.add_out_of_order(4, 5.0)?;

    let unchecked = neighbors.sort(&mut DummyRandomVectorScorer)?;
    assert_eq!(unchecked, vec![0, 1, 2, 3, 4, 5, 6]);
    assert_nodes_equal(&[1, 2, 3, 4, 5, 6, 7], &neighbors);
    assert_scores_equal(&[2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], &neighbors);

    let mut neighbors2 = NeighborArray::new(10, false);
    neighbors2.add_in_order(0, 1.0)?;
    neighbors2.add_in_order(1, 2.0)?;
    neighbors2.add_in_order(4, 5.0)?;
    neighbors2.add_out_of_order(2, 3.0)?;
    neighbors2.add_out_of_order(6, 7.0)?;
    neighbors2.add_out_of_order(5, 6.0)?;
    neighbors2.add_out_of_order(3, 4.0)?;

    let unchecked = neighbors2.sort(&mut DummyRandomVectorScorer)?;
    assert_eq!(unchecked, vec![2, 3, 5, 6]);
    assert_nodes_equal(&[0, 1, 2, 3, 4, 5, 6], &neighbors2);
    assert_scores_equal(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0], &neighbors2);

    Ok(())
  }
  #[test]
  fn test_sort_desc() -> Result<()> {
    let mut neighbors = NeighborArray::new(10, true);

    neighbors.add_out_of_order(1, 7.0)?;
    let result = catch_unwind(AssertUnwindSafe(|| {
      neighbors.add_in_order(1, 2.0).unwrap();
    }));
    assert!(result.is_err());

    neighbors.add_out_of_order(2, 6.0)?;
    neighbors.add_out_of_order(5, 3.0)?;
    neighbors.add_out_of_order(3, 5.0)?;
    neighbors.add_out_of_order(7, 1.0)?;
    neighbors.add_out_of_order(6, 2.0)?;
    neighbors.add_out_of_order(4, 4.0)?;

    let unchecked = neighbors.sort(&mut DummyRandomVectorScorer)?;
    assert_eq!(unchecked, vec![0, 1, 2, 3, 4, 5, 6]);
    assert_nodes_equal(&[1, 2, 3, 4, 5, 6, 7], &neighbors);
    assert_scores_equal(&[7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0], &neighbors);

    let mut neighbors2 = NeighborArray::new(10, true);
    neighbors2.add_in_order(1, 7.0)?;
    neighbors2.add_in_order(2, 6.0)?;
    neighbors2.add_in_order(5, 3.0)?;
    neighbors2.add_out_of_order(3, 5.0)?;
    neighbors2.add_out_of_order(7, 1.0)?;
    neighbors2.add_out_of_order(6, 2.0)?;
    neighbors2.add_out_of_order(4, 4.0)?;

    let unchecked = neighbors2.sort(&mut DummyRandomVectorScorer)?;
    assert_eq!(unchecked, vec![2, 3, 5, 6]);
    assert_nodes_equal(&[1, 2, 3, 4, 5, 6, 7], &neighbors2);
    assert_scores_equal(&[7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0], &neighbors2);

    Ok(())
  }
  #[test]
  fn test_add_with_scoring_function() -> Result<()> {
    let mut neighbors = NeighborArray::new(10, true);
    neighbors.add_out_of_order(1, f32::NAN)?;

    let result = catch_unwind(AssertUnwindSafe(|| {
      neighbors.add_in_order(1, 2.0).unwrap();
    }));
    assert!(result.is_err());

    neighbors.add_out_of_order(2, f32::NAN)?;
    neighbors.add_out_of_order(5, f32::NAN)?;
    neighbors.add_out_of_order(3, f32::NAN)?;
    neighbors.add_out_of_order(7, f32::NAN)?;
    neighbors.add_out_of_order(6, f32::NAN)?;
    neighbors.add_out_of_order(4, f32::NAN)?;

    let mut scorer = TestRandomVectorScorer;
    let unchecked = neighbors.sort(&mut scorer)?;
    assert_eq!(unchecked, vec![0, 1, 2, 3, 4, 5, 6]);
    assert_nodes_equal(&[1, 2, 3, 4, 5, 6, 7], &neighbors);
    assert_scores_equal(&[7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0], &neighbors);

    Ok(())
  }
  #[test]
  fn test_add_with_scoring_function_large_ord() -> Result<()> {
    let mut neighbors = NeighborArray::new(10, true);
    neighbors.add_out_of_order(11, f32::NAN)?;

    let result = catch_unwind(AssertUnwindSafe(|| {
      neighbors.add_in_order(1, 2.0).unwrap();
    }));
    assert!(result.is_err());

    neighbors.add_out_of_order(12, f32::NAN)?;
    neighbors.add_out_of_order(15, f32::NAN)?;
    neighbors.add_out_of_order(13, f32::NAN)?;
    neighbors.add_out_of_order(17, f32::NAN)?;
    neighbors.add_out_of_order(16, f32::NAN)?;
    neighbors.add_out_of_order(14, f32::NAN)?;

    let mut scorer = TestRandomVectorScorer1;
    let unchecked = neighbors.sort(&mut scorer)?;
    assert_eq!(unchecked, vec![0, 1, 2, 3, 4, 5, 6]);
    assert_nodes_equal(&[11, 12, 13, 14, 15, 16, 17], &neighbors);
    assert_scores_equal(&[7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0], &neighbors);

    Ok(())
  }

  fn assert_scores_equal(expected: &[f32], neighbors: &NeighborArray) {
    for (i, &score) in expected.iter().enumerate() {
      assert!(
        (score - neighbors.scores()[i]).abs() <= 0.01,
        "Mismatch at index {}: expected {}, got {}",
        i,
        score,
        neighbors.scores()[i]
      );
    }
  }

  fn assert_nodes_equal(expected: &[usize], neighbors: &NeighborArray) {
    for (i, &node) in expected.iter().enumerate() {
      assert_eq!(
        node,
        neighbors.nodes()[i],
        "Mismatch at index {}: expected {}, got {}",
        i,
        node,
        neighbors.nodes()[i]
      );
    }
  }

  #[derive(Default)]
  struct TestRandomVectorScorer;
  impl RandomVectorScorer for TestRandomVectorScorer {
    fn score(&mut self, node: usize) -> Result<f32> {
      Ok((7 - node + 1) as f32)
    }

    fn max_ord(&self) -> usize {
      0
    }

    type Bits<B>
      = DummyBits
    where
      B: Bits;

    fn get_accept_ords<B>(&self, _accept_docs: Option<B>) -> Result<Option<Self::Bits<B>>>
    where
      B: Bits,
    {
      unreachable!("Dummy implementation: this method should never be called in real usage")
    }
  }
  #[derive(Default)]
  struct TestRandomVectorScorer1;
  impl RandomVectorScorer for TestRandomVectorScorer1 {
    fn score(&mut self, node: usize) -> Result<f32> {
      Ok((7 - node + 11) as f32)
    }

    fn max_ord(&self) -> usize {
      0
    }

    type Bits<B>
      = DummyBits
    where
      B: Bits;

    fn get_accept_ords<B>(&self, _accept_docs: Option<B>) -> Result<Option<Self::Bits<B>>>
    where
      B: Bits,
    {
      unreachable!("Dummy implementation: this method should never be called in real usage")
    }
  }
}
