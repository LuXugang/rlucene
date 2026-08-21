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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use parking_lot::{RwLock, RwLockReadGuard};

use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::hnsw::hnsw_graph::{
  ArrayNodesIterator, CollectionNodesIterator, HnswGraph, NodesIteratorEnum2,
};
use crate::core::util::hnsw::neighbor_array::NeighborArray;
use crate::core::util::ram_usage_estimator::size_of_vec;
/// An [`HnswGraph`] where all nodes and connections are held in memory.
/// This struct is used to construct the HNSW graph before it's written to the
/// index.
pub struct OnHeapHnswGraph {
  entry_node: Arc<RwLock<EntryNode>>,
  // the internal graph representation where the first dimension is node id and second dimension
  // is level
  // e.g. graph[1][2] is all the neighbours of node 1 at level 2
  graph: Vec<OnceLock<Vec<RwLock<NeighborArray>>>>,
  // essentially another 2d map which the first dimension is level and second dimension is node
  // id, this is only
  // generated on demand when there's someone calling getNodeOnLevel on a non-zero level
  level_to_nodes: Vec<Arc<Option<Vec<usize>>>>,
  last_freeze_size: usize,
  // remember the size we are at last time to freeze the graph and generate
  // levelToNodes
  size: AtomicUsize,
  non_zero_level_size: AtomicUsize,
  // total number of NeighborArrays created that is not on level 0, for now it
  // is only used to account memory usage
  max_node_id: Option<AtomicUsize>,
  // neighbour array size at non-zero level
  nsize: usize,
  // neighbour array size at zero level
  nsize0: usize,
  // if an initial size is passed in, we don't expect the graph to grow itself
  no_growth: bool,
  // KnnGraphValues iterator members
  upto: i32,
  cur_node: usize,
  cur_level: usize,
}
impl OnHeapHnswGraph {
  const INIT_SIZE: usize = 128;
  /// Constructs a new instance.
  ///
  /// # Arguments
  ///
  /// * `num_nodes` - The number of nodes that will be added to this graph.
  ///   Passing `-1` means the graph is unbounded, while passing a
  ///   non-negative value locks the graph size,   disallowing any addition of
  ///   nodes with id ≥ `num_nodes`.
  pub fn new(m: usize, mut num_nodes: i32) -> Self {
    let entry_node = Arc::new(RwLock::new(EntryNode::new(None, 1)));
    // Neighbours' size on upper levels (nsize) and level 0 (nsize0)
    // We allocate extra space for neighbours, but then prune them to keep allowed
    // maximum
    let nsize = m + 1;
    let nsize0 = m * 2 + 1;

    let no_growth = num_nodes != -1;
    if !no_growth {
      num_nodes = Self::INIT_SIZE as i32;
    }

    let graph = std::iter::repeat_with(OnceLock::new)
      .take(num_nodes as usize)
      .collect();

    Self {
      entry_node,
      graph,
      level_to_nodes: Vec::new(),
      last_freeze_size: 0,
      size: AtomicUsize::new(0),
      non_zero_level_size: AtomicUsize::new(0),
      max_node_id: None,
      nsize,
      nsize0,
      no_growth,
      upto: -1,
      cur_node: 0,
      cur_level: 0,
    }
  }

  /// Add node on the given level. Nodes can be inserted out of order, but it
  /// requires that the nodes preceding the inserted out-of-order node are
  /// eventually added.
  ///
  /// **NOTE:** You must add a node starting from the node's top level.
  ///
  /// # Arguments
  ///
  /// * `level` - The level on which to add the node.
  /// * `node` - The node to add, represented as an ordinal on level 0.
  pub fn add_node(&mut self, level: usize, node: usize) -> Result<()> {
    if node >= self.graph.len() {
      if self.no_growth {
        return Err(LuceneError::illegal_state(
          "The graph does not expect to grow when an initial size is given",
        ));
      }
      self.graph.resize_with(node + 1, OnceLock::new);
    }

    self.add_node_without_growing(level, node)?;

    let atomic = self.max_node_id.get_or_insert_with(|| AtomicUsize::new(0));

    atomic.fetch_max(node, Ordering::SeqCst);
    Ok(())
  }

  pub(crate) fn get_neighbors_mut(
    &mut self,
    level: usize,
    node: usize,
  ) -> Result<&mut NeighborArray> {
    #[cfg(debug_assertions)]
    check_graph(&self.graph, level, node);
    Ok(self.graph[node].get_mut().unwrap()[level].get_mut())
  }

  pub(crate) fn get_neighbors(
    &self,
    level: usize,
    node: usize,
  ) -> Result<RwLockReadGuard<'_, NeighborArray>> {
    #[cfg(debug_assertions)]
    check_graph(&self.graph, level, node);
    Ok(self.graph[node].get().unwrap()[level].read())
  }

  pub(crate) fn with_neighbors_mut<T>(
    &self,
    level: usize,
    node: usize,
    action: impl FnOnce(&mut NeighborArray) -> Result<T>,
  ) -> Result<T> {
    #[cfg(debug_assertions)]
    check_graph(&self.graph, level, node);
    let mut neighbors = self.graph[node].get().unwrap()[level].write();
    action(&mut neighbors)
  }

  pub(crate) fn add_node_without_growing(&self, level: usize, node: usize) -> Result<()> {
    if node >= self.graph.len() {
      return Err(LuceneError::illegal_state(
        "The graph does not expect to grow when an initial size is given",
      ));
    }

    let mut added = false;
    let levels = self.graph[node].get_or_init(|| {
      added = true;
      (0..=level)
        .map(|current_level| {
          let max_size = if current_level == 0 {
            self.nsize0
          } else {
            self.nsize
          };
          RwLock::new(NeighborArray::new(max_size, true))
        })
        .collect()
    });
    debug_assert!(
      levels.len() > level,
      "node must be inserted from the top level"
    );
    if added {
      self.size.fetch_add(1, Ordering::SeqCst);
      self.non_zero_level_size.fetch_add(level, Ordering::SeqCst);
    }
    Ok(())
  }

  /// Try to set the entry node if the graph does not already have one.
  ///
  /// # Returns
  ///
  /// `true` if the entry node was successfully set to the provided node,  
  /// `false` if the entry node was already set.
  pub fn try_set_new_entry_node(&self, node: usize, level: usize) -> bool {
    let mut entry_node = self.entry_node.write();
    if entry_node.node.is_none() {
      *entry_node = EntryNode::new(Some(node), level);
      true
    } else {
      false
    }
  }
  /// Try to promote the provided node to be the new entry node.
  ///
  /// # Parameters
  /// - `level`: the level of the provided node, must be greater than
  ///   `expect_old_level`.
  /// - `expect_old_level`: the level the caller expects the current entry
  ///   node to be at; the actual graph level may differ due to concurrent
  ///   modification.
  ///
  /// # Returns
  /// `true` if the entry node was successfully promoted to the provided node.
  /// `false` if `expect_old_level` does not match the current entry node
  /// level. Even if `level` is higher than the current entry node level,
  /// this method will not update the entry node if the expected level
  /// check fails.
  pub fn try_promote_new_entry_node(
    &self,
    node: usize,
    level: usize,
    expect_old_level: usize,
  ) -> bool {
    debug_assert!(
      level > expect_old_level,
      "level must be greater than expect_old_level"
    );

    let mut entry = self.entry_node.write();
    if entry.level == expect_old_level {
      *entry = EntryNode::new(Some(node), level);
      true
    } else {
      false
    }
  }
  fn generate_level_to_nodes(&mut self) -> Result<()> {
    let size = self.size();
    if self.last_freeze_size == size {
      return Ok(());
    }

    let max_levels = self.num_levels()?;
    self.level_to_nodes.clear();
    self.level_to_nodes.reserve(max_levels);
    for level in 0..max_levels {
      let nodes = if level == 0 { None } else { Some(Vec::new()) };
      self.level_to_nodes.push(Arc::new(nodes));
    }

    let mut non_null_node = 0;
    for (node, levels) in self.graph.iter().enumerate() {
      let Some(levels) = levels.get() else {
        continue;
      };
      non_null_node += 1;
      for maybe_vec in self.level_to_nodes.iter_mut().take(levels.len()).skip(1) {
        if let Some(vec) = Arc::get_mut(maybe_vec).and_then(Option::as_mut) {
          vec.push(node);
        }
      }
      if non_null_node == size {
        break;
      }
    }

    self.last_freeze_size = size;
    Ok(())
  }
}
impl HnswGraph for OnHeapHnswGraph {
  fn seek(&mut self, level: usize, target_node: usize) -> Result<()> {
    self.cur_node = target_node;
    self.cur_level = level;
    self.upto = -1;
    Ok(())
  }

  fn size(&self) -> usize {
    self.size.load(Ordering::SeqCst)
  }

  /// When we initialize from another graph, the max node id is different from
  /// `size()` because we will add nodes out of order. Thus, we need two
  /// methods for each.
  ///
  /// # Returns
  ///
  /// The maximum node ID (inclusive).
  fn max_node_id(&self) -> Option<usize> {
    if self.no_growth {
      debug_assert!(!self.graph.is_empty() && self.graph.len() <= i32::MAX as usize);
      // we know the eventual graph size and the graph can possibly
      // being concurrently modified
      Some(self.graph.len() - 1)
    } else {
      // The graph cannot be concurrently modified (and searched) if
      // we don't know the size beforehand, so it's safe to return the
      // actual maxNodeId
      self.max_node_id.as_ref().map(|v| v.load(Ordering::SeqCst))
    }
  }

  fn next_neighbor(&mut self) -> Result<usize> {
    self.upto += 1;
    let cur = self.get_neighbors(self.cur_level, self.cur_node)?;
    if (self.upto as usize) < cur.size() {
      Ok(cur.nodes()[self.upto as usize])
    } else {
      Ok(NO_MORE_DOCS as usize)
    }
  }
  /// Returns the current number of levels in the graph.
  ///
  /// # Returns
  ///
  /// The current number of levels in the graph.
  fn num_levels(&self) -> Result<usize> {
    let entry = self.entry_node.read();
    Ok(entry.level + 1)
  }
  /// Returns the graph's current entry node on the top level,
  /// represented as an ordinal of the node on the 0th level.
  ///
  /// # Returns
  ///
  /// The graph's current entry node on the top level.
  fn entry_node(&self) -> Result<Option<usize>> {
    let entry = self.entry_node.read();
    Ok(entry.node)
  }

  type NodeIterator = NodesIteratorEnum2<ArrayNodesIterator, CollectionNodesIterator>;
  /// **WARN**: Calling this method will effectively iterate through all nodes
  /// at level 0, even if you're querying nodes at a different level.
  ///
  /// A caching mechanism is in place to ensure that only the *first* non-zero
  /// level call incurs the full cost, assuming the graph has not been
  /// modified.
  ///
  /// # NOTE
  /// Calling this method while the graph is still being built is
  /// **prohibited** and may result in incorrect behavior or performance
  /// degradation.
  fn get_nodes_on_level(&mut self, level: usize) -> Result<Self::NodeIterator> {
    let size = self.size();
    let max_id = match self.max_node_id() {
      Some(v) => v + 1,
      None => 0,
    };

    if size != (max_id) {
      return Err(LuceneError::illegal_state(format!(
        "graph build not complete: size={}, maxNodeId={}",
        size,
        max_id as i32 - 1
      )));
    }

    if level == 0 {
      Ok(NodesIteratorEnum2::A(ArrayNodesIterator::from_size(
        self.size(),
      )))
    } else {
      self.generate_level_to_nodes()?;
      Ok(NodesIteratorEnum2::B(CollectionNodesIterator::new(
        self.level_to_nodes[level].clone(),
      )))
    }
  }

  fn with_neighbors<T>(
    &self,
    level: usize,
    node: usize,
    action: impl FnOnce(&NeighborArray) -> Result<T>,
  ) -> Result<T> {
    let neighbors = OnHeapHnswGraph::get_neighbors(self, level, node)?;
    action(&neighbors)
  }
}
impl HnswGraph for &OnHeapHnswGraph {
  fn seek(&mut self, _level: usize, _target_node: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn size(&self) -> usize {
    (*self).size()
  }

  fn max_node_id(&self) -> Option<usize> {
    (*self).max_node_id()
  }

  fn next_neighbor(&mut self) -> Result<usize> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn num_levels(&self) -> Result<usize> {
    (*self).num_levels()
  }

  fn entry_node(&self) -> Result<Option<usize>> {
    (*self).entry_node()
  }

  type NodeIterator = <OnHeapHnswGraph as HnswGraph>::NodeIterator;

  fn get_nodes_on_level(&mut self, _level: usize) -> Result<Self::NodeIterator> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn with_neighbors<T>(
    &self,
    level: usize,
    node: usize,
    action: impl FnOnce(&NeighborArray) -> Result<T>,
  ) -> Result<T> {
    let neighbors = (*self).get_neighbors(level, node)?;
    action(&neighbors)
  }
}
#[cfg(debug_assertions)]
fn check_graph(graph: &[OnceLock<Vec<RwLock<NeighborArray>>>], level: usize, node: usize) {
  debug_assert!(node < graph.len(),);
  let levels = graph[node].get().unwrap();
  debug_assert!(
    level < levels.len(),
    "level={} exceeds available levels ({}) for node={}",
    level,
    levels.len(),
    node
  );
}
impl fmt::Display for OnHeapHnswGraph {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let size = self.size();
    let num_levels = self.num_levels().unwrap_or(0);
    let entry_node = self.entry_node.read();

    write!(
      f,
      "{}(size={size}, numLevels={num_levels}, entryNode={entry_node:?})",
      std::any::type_name::<Self>()
    )
  }
}

impl Accountable for OnHeapHnswGraph {
  fn ram_bytes_used(&self) -> Result<i64> {
    let mut size = (std::mem::size_of_val(self.entry_node.as_ref()) as i64)
      .saturating_add(size_of_vec(&self.graph));
    for levels in &self.graph {
      let Some(levels) = levels.get() else {
        continue;
      };
      size = size.saturating_add(size_of_vec(levels));
      for neighbors in levels {
        let neighbors = neighbors.read();
        size = size.saturating_add(neighbors.ram_bytes_used()?);
      }
    }

    size = size.saturating_add(size_of_vec(&self.level_to_nodes));
    for nodes in &self.level_to_nodes {
      size = size.saturating_add(std::mem::size_of_val(nodes.as_ref()) as i64);
      if let Some(nodes) = nodes.as_ref() {
        size = size.saturating_add(size_of_vec(nodes));
      }
    }
    Ok(size)
  }
}

#[derive(Debug)]
struct EntryNode {
  node: Option<usize>,
  level: usize,
}
impl EntryNode {
  pub fn new(node: Option<usize>, level: usize) -> Self {
    Self { node, level }
  }
}

impl HnswGraph for Arc<OnHeapHnswGraph> {
  type NodeIterator = <OnHeapHnswGraph as HnswGraph>::NodeIterator;

  fn seek(&mut self, level: usize, target: usize) -> Result<()> {
    Arc::get_mut(self)
      .ok_or_else(|| {
        LuceneError::unsupported_operation("concurrent graph traversal must use MergeSearcher")
      })?
      .seek(level, target)
  }

  fn size(&self) -> usize {
    self.as_ref().size()
  }

  fn max_node_id(&self) -> Option<usize> {
    self.as_ref().max_node_id()
  }

  fn next_neighbor(&mut self) -> Result<usize> {
    Arc::get_mut(self)
      .ok_or_else(|| {
        LuceneError::unsupported_operation("concurrent graph traversal must use MergeSearcher")
      })?
      .next_neighbor()
  }

  fn num_levels(&self) -> Result<usize> {
    self.as_ref().num_levels()
  }

  fn entry_node(&self) -> Result<Option<usize>> {
    self.as_ref().entry_node()
  }

  fn get_nodes_on_level(&mut self, level: usize) -> Result<Self::NodeIterator> {
    Arc::get_mut(self)
      .ok_or_else(|| {
        LuceneError::unsupported_operation(
          "nodes on a level are only available after concurrent graph construction",
        )
      })?
      .get_nodes_on_level(level)
  }

  fn with_neighbors<T>(
    &self,
    level: usize,
    node: usize,
    action: impl FnOnce(&NeighborArray) -> Result<T>,
  ) -> Result<T> {
    let neighbors = self.as_ref().get_neighbors(level, node)?;
    action(&neighbors)
  }
}
