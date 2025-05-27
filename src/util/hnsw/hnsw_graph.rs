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
use std::vec::IntoIter;

use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::util::error::lucene_error::Result;
use crate::util::SliceCopyOps;
/// Hierarchical Navigable Small World (HNSW) graph.
///
/// Provides efficient approximate nearest neighbor search for high-dimensional
/// vectors. See the paper:
/// ["Efficient and robust approximate nearest neighbor search using Hierarchical Navigable Small World graphs (2018)"](https://arxiv.org/abs/1603.09320)
/// for algorithmic details.
///
/// The nomenclature here differs slightly from the paper:
///
/// ## Hyperparameters
///
/// - `beam_width` in [`HnswGraphBuilder`] corresponds to `efConst` in the
///   paper: it is the number of nearest neighbor candidates tracked while
///   searching the graph for each newly inserted node.
///
/// - `max_conn` corresponds to `M` in the paper: it controls how many of the
///   `efConst` neighbors are connected to the new node.
///
/// Note: The graph may be searched by multiple threads concurrently, but
/// **updates are not thread-safe**. The search method optionally takes a set of
/// *accepted nodes*, which can be used to exclude deleted documents.
pub trait HnswGraph {
    /// Move the pointer to exactly the given `level`'s `target`.
    ///
    /// After this method returns, call [`next_neighbor()`](Self::next_neighbor)
    /// to return successive (ordered) connected node ordinals.
    ///
    /// - `level`: the level of the graph.
    /// - `target`: the ordinal of a node in the graph; must be `>= 0` and `<
    ///   [FloatVectorValues::size()]`.
    fn seek(&mut self, level: usize, target: usize) -> Result<()>;
    /// Returns the number of nodes in the graph
    fn size(&self) -> usize;
    /// Returns max node id, inclusive. Normally this value will be size - 1.
    fn max_node_id(&self) -> usize {
        self.size().saturating_sub(1)
    }
    /// Iterates over the neighbor list.
    ///
    /// It is illegal to call this method after it returns `NO_MORE_DOCS`
    /// without calling [`seek(level, target)`](Self::seek), which resets the
    /// iterator.
    ///
    /// # Returns
    ///
    /// A node ordinal in the graph, or `NO_MORE_DOCS` if the iteration is
    /// complete.
    fn next_neighbor(&mut self) -> Result<Option<usize>>;
    /// Returns the number of levels of the graph
    fn num_levels(&self) -> Result<usize>;
    /// Returns graph's entry point on the top level *
    fn entry_node(&self) -> Result<usize>;
    type NodeIterator: NodesIterator;
    /// Get all nodes on a given level as node 0th ordinals.
    ///
    /// The nodes are **NOT** guaranteed to be presented in any particular
    /// order.
    ///
    /// # Arguments
    ///
    /// * `level` - Level for which to get all nodes.
    ///
    /// # Returns
    ///
    /// An iterator over nodes where `next_int()` returns the next node on the
    /// level.
    fn get_nodes_on_level(&self, _level: usize) -> Result<Self::NodeIterator>;
}
pub struct EmptyHnswGraph;
impl HnswGraph for EmptyHnswGraph {
    fn seek(&mut self, _level: usize, _target: usize) -> Result<()> {
        Ok(())
    }

    fn size(&self) -> usize {
        0
    }

    fn next_neighbor(&mut self) -> Result<Option<usize>> {
        Ok(Some(NO_MORE_DOCS as usize))
    }

    fn num_levels(&self) -> Result<usize> {
        Ok(0)
    }

    fn entry_node(&self) -> Result<usize> {
        Ok(0)
    }

    type NodeIterator = ArrayNodesIterator;

    fn get_nodes_on_level(&self, _level: usize) -> Result<Self::NodeIterator> {
        Ok(ArrayNodesIterator::empty())
    }
}
/// Iterator over the graph nodes on a certain level. Iterator also provides the
/// size – the total number of nodes to be iterated over. The nodes are NOT
/// guaranteed to be presented in any  particular order.
pub trait NodesIterator: Iterator<Item = i32> {
    ///  The number of elements in this iterator
    fn size(&self) -> usize;
    /// Consume integers from the iterator and place them into the `dest` array.
    ///
    /// # Arguments
    ///
    /// * `dest` - Where to put the integers.
    ///
    /// # Returns
    ///
    /// The number of integers written to `dest`.
    fn consume(&mut self, dest: &mut [i32]) -> Option<i32>;

    fn get_sorted_nodes<I: NodesIterator>(nodes: &mut I) -> Vec<i32> {
        let mut sorted = Vec::with_capacity(nodes.size());
        for v in nodes.by_ref() {
            sorted.push(v);
        }
        sorted.sort_unstable();
        sorted
    }
    fn has_next(&self) -> bool;
}
/// NodesIterator that accepts nodes as an integer array.
pub struct ArrayNodesIterator {
    nodes: Option<Vec<i32>>,
    cur: usize,
    size: usize,
}

impl ArrayNodesIterator {
    /// Constructor for explicit node list (with partial length).
    pub fn from_nodes(nodes: Vec<i32>, size: usize) -> Self {
        debug_assert!(size <= nodes.len());
        Self {
            nodes: Some(nodes),
            cur: 0,
            size,
        }
    }

    /// Constructor for implicit index-based iteration (0..size).
    pub fn from_size(size: usize) -> Self {
        Self {
            nodes: None,
            cur: 0,
            size,
        }
    }

    /// Shared empty singleton.
    pub fn empty() -> Self {
        Self::from_size(0)
    }
}

impl Iterator for ArrayNodesIterator {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.has_next() {
            None
        } else {
            let value = match &self.nodes {
                Some(vec) => vec[self.cur],
                None => self.cur as i32,
            };
            self.cur += 1;
            Some(value)
        }
    }
}

impl NodesIterator for ArrayNodesIterator {
    fn size(&self) -> usize {
        self.size
    }

    fn consume(&mut self, dest: &mut [i32]) -> Option<i32> {
        if !self.has_next() {
            return None;
        }
        let num_to_copy = std::cmp::min(dest.len(), self.size - self.cur);
        match &self.nodes {
            Some(vec) => {
                dest.copy_from(&vec[self.cur..self.cur + num_to_copy], 0);
                self.cur += num_to_copy;
                Some(num_to_copy as i32)
            },
            None => {
                for i in 0..num_to_copy {
                    dest[i] = (self.cur + i) as i32;
                }
                Some(num_to_copy as i32)
            },
        }
    }

    fn has_next(&self) -> bool {
        self.cur < self.size
    }
}

/// Nodes iterator based on set representation of nodes.
pub struct CollectionNodesIterator {
    nodes: IntoIter<i32>,
    size: usize,
}

impl CollectionNodesIterator {
    /// Constructs a new iterator from a collection of nodes.
    pub fn new(mut nodes: Vec<i32>) -> Self {
        let size = nodes.len();
        let nodes = std::mem::take(&mut nodes).into_iter();
        Self { nodes, size }
    }
}

impl Iterator for CollectionNodesIterator {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        self.nodes.next()
    }
}

impl NodesIterator for CollectionNodesIterator {
    fn size(&self) -> usize {
        self.size
    }

    fn has_next(&self) -> bool {
        self.nodes.len() > 0
    }

    fn consume(&mut self, dest: &mut [i32]) -> Option<i32> {
        let mut count = 0;
        for d in dest.iter_mut() {
            match self.next() {
                Some(v) => {
                    *d = v;
                    count += 1;
                },
                None => break,
            }
        }
        if count == 0 {
            None
        } else {
            Some(count)
        }
    }
}
pub enum NodesIteratorEnums {
    Array(ArrayNodesIterator),
    Collection(CollectionNodesIterator),
}

impl Iterator for NodesIteratorEnums {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            NodesIteratorEnums::Array(iter) => iter.next(),
            NodesIteratorEnums::Collection(iter) => iter.next(),
        }
    }
}

impl NodesIterator for NodesIteratorEnums {
    fn size(&self) -> usize {
        match self {
            NodesIteratorEnums::Array(iter) => iter.size(),
            NodesIteratorEnums::Collection(iter) => iter.size(),
        }
    }

    fn consume(&mut self, dest: &mut [i32]) -> Option<i32> {
        match self {
            NodesIteratorEnums::Array(iter) => iter.consume(dest),
            NodesIteratorEnums::Collection(iter) => iter.consume(dest),
        }
    }

    fn has_next(&self) -> bool {
        match self {
            NodesIteratorEnums::Array(iter) => iter.has_next(),
            NodesIteratorEnums::Collection(iter) => iter.has_next(),
        }
    }
}
