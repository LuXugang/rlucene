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
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use parking_lot::RwLock;

use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::util::accountable::Accountable;
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::hnsw::hnsw_graph::{
    ArrayNodesIterator, CollectionNodesIterator, HnswGraph, NodesIteratorEnums,
};
use crate::util::hnsw::neighbor_array::NeighborArray;
/// An [`HnswGraph`] where all nodes and connections are held in memory.
/// This struct is used to construct the HNSW graph before it's written to the
/// index.
pub struct OnHeapHnswGraph {
    entry_node: Arc<RwLock<EntryNode>>,
    // the internal graph representation where the first dimension is node id and second dimension
    // is level
    // e.g. graph[1][2] is all the neighbours of node 1 at level 2
    graph: Vec<Vec<Option<NeighborArray>>>,
    // essentially another 2d map which the first dimension is level and second dimension is node
    // id, this is only
    // generated on demand when there's someone calling getNodeOnLevel on a non-zero level
    level_to_nodes: Vec<Arc<Option<Vec<i32>>>>,
    last_freeze_size: usize,
    // remember the size we are at last time to freeze the graph and generate
    // levelToNodes
    size: AtomicUsize,
    non_zero_level_size: AtomicUsize,
    // total number of NeighborArrays created that is not on level 0, for now it
    // is only used to account memory usage
    max_node_id: AtomicI32,
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
        let entry_node = Arc::new(RwLock::new(EntryNode::new(-1, 1)));
        // Neighbours' size on upper levels (nsize) and level 0 (nsize0)
        // We allocate extra space for neighbours, but then prune them to keep allowed
        // maximum
        let nsize = m + 1;
        let nsize0 = m * 2 + 1;

        let no_growth = num_nodes != -1;
        if !no_growth {
            num_nodes = Self::INIT_SIZE as i32;
        }

        let graph = vec![vec![]; num_nodes as usize];

        Self {
            entry_node,
            graph,
            level_to_nodes: Vec::new(),
            last_freeze_size: 0,
            size: AtomicUsize::new(0),
            non_zero_level_size: AtomicUsize::new(0),
            max_node_id: AtomicI32::new(-1),
            nsize,
            nsize0,
            no_growth,
            upto: -1,
            cur_node: 0,
            cur_level: 0,
        }
    }
    /// Returns the [`NeighborQueue`] connected to the given node.
    ///
    /// # Arguments
    ///
    /// * `level` - The level of the graph.
    /// * `node` - The node whose neighbors are returned, represented as an
    ///   ordinal on level 0.
    pub fn get_neighbors(&mut self, level: usize, node: usize) -> &mut NeighborArray {
        debug_assert!(node < self.graph.len(),);

        debug_assert!(
            level < self.graph[node].len(),
            "level={} exceeds available levels ({}) for node={}",
            level,
            self.graph[node].len(),
            node
        );
        debug_assert!(self.graph[node][level].is_some());
        self.graph[node][level].as_mut().unwrap()
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
            ArrayUtil::grow_with_len(&mut self.graph, node + 1);
        }

        if self.graph[node].is_empty() {
            // assumption: we always call this function from top level
            self.graph[node].resize(level + 1, None);
            self.size.fetch_add(1, Ordering::AcqRel);
        } else {
            debug_assert!(
                self.graph[node].len() > level,
                "node must be inserted from the top level"
            );
        }

        let neighbor_array = if level == 0 {
            NeighborArray::new(self.nsize0, true)
        } else {
            self.non_zero_level_size.fetch_add(1, Ordering::Relaxed);
            NeighborArray::new(self.nsize, true)
        };

        self.graph[node][level] = Some(neighbor_array);

        self.max_node_id
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                Some(current.max(node as i32))
            })
            .ok();
        Ok(())
    }
    /// Try to set the entry node if the graph does not already have one.
    ///
    /// # Returns
    ///
    /// `true` if the entry node was successfully set to the provided node,  
    /// `false` if the entry node was already set.
    pub fn try_set_new_entry_node(&self, node: i32, level: usize) -> bool {
        let mut entry_node = self.entry_node.write();
        if entry_node.node == -1 {
            *entry_node = EntryNode::new(node, level);
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
        node: i32,
        level: usize,
        expect_old_level: usize,
    ) -> bool {
        debug_assert!(
            level > expect_old_level,
            "level must be greater than expect_old_level"
        );

        let mut entry = self.entry_node.write();
        if entry.level == expect_old_level {
            *entry = EntryNode::new(node, level);
            true
        } else {
            false
        }
    }
    fn generate_level_to_nodes(&mut self) -> Result<()> {
        if self.last_freeze_size == self.size() {
            return Ok(());
        }
        let mut level_to_nodes: Vec<Option<Vec<i32>>> = Vec::new();

        let max_levels = self.num_levels()?;
        level_to_nodes = vec![None; max_levels];
        for i in 1..max_levels {
            level_to_nodes[i] = Some(Vec::new());
        }

        let mut non_null_node = 0;
        for (node, levels) in self.graph.iter().enumerate() {
            if levels.is_empty() {
                continue;
            }
            non_null_node += 1;
            for i in 1..levels.len() {
                if let Some(ref mut vec) = level_to_nodes[i] {
                    vec.push(node as i32);
                }
            }
            if non_null_node == self.size() {
                break;
            }
        }
        self.level_to_nodes = level_to_nodes.into_iter().map(Arc::new).collect();

        self.last_freeze_size = self.size();
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
        self.size.load(std::sync::atomic::Ordering::Acquire)
    }

    /// When we initialize from another graph, the max node id is different from
    /// `size()` because we will add nodes out of order. Thus, we need two
    /// methods for each.
    ///
    /// # Returns
    ///
    /// The maximum node ID (inclusive).
    fn max_node_id(&self) -> i32 {
        if self.no_growth {
            debug_assert!(!self.graph.is_empty() && self.graph.len() <= i32::MAX as usize);
            // we know the eventual graph size and the graph can possibly
            // being concurrently modified
            self.graph.len() as i32 - 1
        } else {
            // The graph cannot be concurrently modified (and searched) if
            // we don't know the size beforehand, so it's safe to return the
            // actual maxNodeId
            self.max_node_id.load(std::sync::atomic::Ordering::Acquire)
        }
    }

    fn next_neighbor(&mut self) -> Result<i32> {
        let cur = self.graph[self.cur_node][self.cur_level].as_ref().unwrap();
        self.upto += 1;
        if (self.upto as usize) < cur.size() {
            Ok(cur.nodes()[self.upto as usize])
        } else {
            Ok(NO_MORE_DOCS)
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
    fn entry_node(&self) -> Result<i32> {
        let entry = self.entry_node.read();
        Ok(entry.node)
    }

    type NodeIterator = NodesIteratorEnums;
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
        let max_id = self.max_node_id();

        if size != (max_id + 1) as usize {
            return Err(LuceneError::illegal_state(format!(
                "graph build not complete: size={size}, maxNodeId={max_id}"
            )));
        }

        if level == 0 {
            Ok(NodesIteratorEnums::Array(ArrayNodesIterator::from_size(
                self.size(),
            )))
        } else {
            self.generate_level_to_nodes()?;
            Ok(NodesIteratorEnums::Collection(
                CollectionNodesIterator::new(self.level_to_nodes[level].clone()),
            ))
        }
    }
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
        // TODO
        todo!()
    }
}

#[derive(Debug)]
struct EntryNode {
    node: i32,
    level: usize,
}
impl EntryNode {
    pub fn new(node: i32, level: usize) -> Self {
        Self { node, level }
    }
}
#[cfg(test)]
mod tests {
    use rand::Rng;

    use crate::test::util::lucene_test_case::lucene_test_case_util::random;
    use crate::util::error::lucene_error::{LuceneError, Result};
    use crate::util::hnsw::hnsw_graph::{HnswGraph, NodesIterator};
    use crate::util::hnsw::on_heap_hnsw_graph::OnHeapHnswGraph;

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
        let mut level_to_nodes: Vec<Vec<i32>> = vec![Vec::new(); max_level];

        for i in 0..101i32 {
            let level = random.random_range(0..max_level);
            for l in (0..=level).rev() {
                graph.add_node(l, i as usize)?;
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
        let mut level_to_nodes: Vec<Vec<i32>> = vec![Vec::new(); max_level];

        let mut insertions: Vec<i32> = (0..num_nodes).collect();

        // Shuffle insertion order 40 times
        for _ in 0..40 {
            let pos1 = random.random_range(0..num_nodes);
            let pos2 = random.random_range(0..num_nodes);
            insertions.swap(pos1 as usize, pos2 as usize);
        }

        for &i in &insertions {
            let level = random.random_range(0..max_level);
            for l in (0..=level).rev() {
                graph.add_node(l, i as usize)?;
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

    fn assert_graph_equals(graph: &mut impl HnswGraph, level_to_nodes: &[Vec<i32>]) -> Result<()> {
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
}
