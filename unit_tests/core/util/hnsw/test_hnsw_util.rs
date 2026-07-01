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
use crate::test::support::core::util::lucene_test_case::{at_least, random};
use std::collections::VecDeque;

use rand::RngExt;

use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::hnsw::hnsw_graph::{HnswGraph, NodesIterator};
use crate::core::util::hnsw::hnsw_util::HnswUtil;

#[allow(dead_code)] // for quick search
struct TestHnswUtil;
#[test]
fn test_tree_with_cycle() -> Result<()> {
  let nodes: Vec<Vec<Option<Vec<usize>>>> = vec![vec![
    Some(vec![1, 2]),
    Some(vec![3, 4]),
    Some(vec![5, 6]),
    Some(vec![]),
    Some(vec![]),
    Some(vec![]),
    Some(vec![0]),
  ]];

  let mut graph = MockGraph::new(nodes);

  assert!(HnswUtil::is_rooted(&mut graph)?);
  assert_eq!(HnswUtil::component_sizes(&mut graph)?, vec![7]);

  Ok(())
}
#[test]
fn test_back_linking() -> Result<()> {
  let nodes: Vec<Vec<Option<Vec<usize>>>> = vec![vec![
    Some(vec![1, 2]),
    Some(vec![3, 4]),
    Some(vec![0]),
    Some(vec![1]),
    Some(vec![1]),
    Some(vec![1]),
    Some(vec![1]),
  ]];

  let mut graph = MockGraph::new(nodes);

  assert!(!HnswUtil::is_rooted(&mut graph)?);
  assert_eq!(HnswUtil::component_sizes(&mut graph)?, vec![5, 1, 1]);

  Ok(())
}
#[test]
fn test_chain() -> Result<()> {
  let nodes: Vec<Vec<Option<Vec<usize>>>> = vec![vec![
    Some(vec![1]),
    Some(vec![2]),
    Some(vec![3]),
    Some(vec![0]),
  ]];

  let mut graph = MockGraph::new(nodes);

  assert!(HnswUtil::is_rooted(&mut graph)?);
  assert_eq!(HnswUtil::component_sizes(&mut graph)?, vec![4]);

  Ok(())
}
#[test]
fn test_two_chains() -> Result<()> {
  let nodes: Vec<Vec<Option<Vec<usize>>>> = vec![vec![
    Some(vec![2]),
    Some(vec![3]),
    Some(vec![0]),
    Some(vec![1]),
  ]];

  let mut graph = MockGraph::new(nodes);

  assert!(!HnswUtil::is_rooted(&mut graph)?);
  assert_eq!(HnswUtil::component_sizes(&mut graph)?, vec![2, 2]);

  Ok(())
}
#[test]
fn test_levels() -> Result<()> {
  let nodes: Vec<Vec<Option<Vec<usize>>>> = vec![
    vec![
      Some(vec![1, 2]),
      Some(vec![3]),
      Some(vec![0]),
      Some(vec![0]),
    ],
    vec![Some(vec![2]), None, Some(vec![0]), None],
    vec![Some(vec![]), None, None, None],
  ];

  let mut graph = MockGraph::new(nodes);

  assert!(HnswUtil::is_rooted(&mut graph)?);
  assert_eq!(HnswUtil::component_sizes(&mut graph)?, vec![4]);

  Ok(())
}
#[test]
fn test_levels_not_rooted() -> Result<()> {
  let nodes: Vec<Vec<Option<Vec<usize>>>> = vec![
    vec![Some(vec![1]), Some(vec![0]), Some(vec![0])],
    vec![Some(vec![]), None, None],
  ];
  let mut graph = MockGraph::new(nodes);

  assert!(!HnswUtil::is_rooted(&mut graph)?);
  assert_eq!(HnswUtil::component_sizes(&mut graph)?, vec![2, 1]);

  Ok(())
}
#[test]
fn test_random_graph_rooted_check() -> Result<()> {
  let mut random = random();

  for _ in 0..at_least(&mut random, 10) {
    let num_nodes = random.random_range(1..100);
    let num_levels = (num_nodes as f64).ln().ceil() as usize;
    let mut nodes: Vec<Vec<Option<Vec<usize>>>> = vec![vec![None; num_nodes]; num_levels];

    for level in (0..num_levels).rev() {
      for node in 0..num_nodes {
        if level > 0 {
          let higher = level == num_levels - 1;
          let not_on_above = level < num_levels - 1 && nodes[level + 1][node].is_none();
          if ((higher && node > 0) || not_on_above)
            && random.random::<f32>() > (-(level as f32)).exp()
          {
            continue;
          }
        }

        let mut num_nbrs = random.random_range(0..num_nodes.div_ceil(8));
        if level == 0 {
          num_nbrs *= 2;
        }

        nodes[level][node] = Option::from(vec![0; num_nbrs]);
        for nbr in 0..num_nbrs {
          loop {
            let random_nbr = random.random_range(0..num_nodes);
            if nodes[level][random_nbr].is_some() {
              nodes[level][node].as_mut().unwrap()[nbr] = random_nbr;
              break;
            }
          }
        }
      }
    }

    let mut graph = MockGraph::new(nodes.clone());

    let expected = is_rooted(&nodes)?;
    let actual = HnswUtil::is_rooted(&mut graph)?;
    assert_eq!(expected, actual);
  }

  Ok(())
}

fn is_rooted(nodes: &[Vec<Option<Vec<usize>>>]) -> Result<bool> {
  for level in (0..nodes.len()).rev() {
    if !is_rooted_with_level(nodes, level)? {
      return Ok(false);
    }
  }
  Ok(true)
}

fn is_rooted_with_level(nodes: &[Vec<Option<Vec<usize>>>], level: usize) -> Result<bool> {
  let entry_points: Vec<usize> = if level == nodes.len() - 1 {
    vec![0]
  } else {
    nodes[level + 1]
      .iter()
      .enumerate()
      .filter_map(|(i, node)| node.as_ref().map(|_| i))
      .collect()
  };

  let mut connected = FixedBitSet::new(nodes[level].len());
  let mut count = 0;

  for &entry_point in &entry_points {
    if nodes[level]
      .get(entry_point)
      .and_then(|n| n.as_ref())
      .is_none()
    {
      continue;
    }

    let mut stack = VecDeque::new();
    stack.push_back(entry_point);

    while let Some(node) = stack.pop_back() {
      if connected.get(node)? {
        continue;
      }
      connected.set(node);
      count += 1;

      if let Some(neighbors) = nodes[level][node].as_ref() {
        for &nbr in neighbors {
          stack.push_back(nbr);
        }
      }
    }
  }

  Ok(count == level_size(&nodes[level]))
}

fn level_size(nodes: &[Option<Vec<usize>>]) -> usize {
  let mut count = 0;
  for node in nodes {
    if node.is_some() {
      count += 1;
    }
  }
  count
}

pub struct MockGraph {
  nodes: Vec<Vec<Option<Vec<usize>>>>,
  current_level: usize,
  current_node: usize,
  current_neighbor: usize,
}
impl MockGraph {
  pub fn new(nodes: Vec<Vec<Option<Vec<usize>>>>) -> Self {
    Self {
      nodes,
      current_level: 0,
      current_node: 0,
      current_neighbor: 0,
    }
  }
}
impl HnswGraph for MockGraph {
  fn seek(&mut self, level: usize, target: usize) -> Result<()> {
    assert!(
      level < self.nodes.len(),
      "level {} out of range, max level = {}",
      level,
      self.nodes.len()
    );
    assert!(
      target < self.nodes[level].len(),
      "target {} out of range for level {}, should be less than {}",
      target,
      level,
      self.nodes[level].len()
    );
    assert!(
      self.nodes[level][target].is_some(),
      "target {} not on level {}",
      target,
      level
    );
    self.current_level = level;
    self.current_node = target;
    self.current_neighbor = 0;
    Ok(())
  }

  fn size(&self) -> usize {
    self.nodes[0].len()
  }

  fn next_neighbor(&mut self) -> Result<usize> {
    let neighbors = self.nodes[self.current_level][self.current_node]
      .as_ref()
      .unwrap();
    if self.current_neighbor >= neighbors.len() {
      Ok(NO_MORE_DOCS as usize)
    } else {
      let result = neighbors[self.current_neighbor];
      self.current_neighbor += 1;
      Ok(result)
    }
  }

  fn num_levels(&self) -> Result<usize> {
    Ok(self.nodes.len())
  }

  fn entry_node(&self) -> Result<Option<usize>> {
    Ok(Some(0))
  }

  type NodeIterator = NodeIteratorImpl;

  fn get_nodes_on_level(&mut self, level: usize) -> Result<Self::NodeIterator> {
    let mut count = 0;
    for neighbors in &self.nodes[level] {
      if neighbors.is_some() {
        count += 1;
      }
    }

    let final_count = count;
    let v = NodeIteratorImpl::new(self.nodes.clone(), final_count, level);
    Ok(v)
  }
}
impl std::fmt::Display for MockGraph {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for level in (0..self.nodes.len()).rev() {
      writeln!(f, "\nLEVEL {}", level)?;
      for (node, neighbors) in self.nodes[level].iter().enumerate() {
        if !neighbors.is_some() {
          writeln!(f, "  {}: {:?}", node, neighbors)?;
        }
      }
    }
    Ok(())
  }
}

pub struct NodeIteratorImpl {
  cur: i32,
  cur_count: i32,
  final_count: i32,
  level: usize,
  nodes: Vec<Vec<Option<Vec<usize>>>>,
  size: usize,
}
impl NodeIteratorImpl {
  pub fn new(nodes: Vec<Vec<Option<Vec<usize>>>>, final_count: i32, level: usize) -> Self {
    NodeIteratorImpl {
      cur: -1,
      cur_count: 0,
      level,
      final_count,
      nodes,
      size: final_count as usize,
    }
  }
}

impl Iterator for NodeIteratorImpl {
  type Item = usize;

  fn next(&mut self) -> Option<Self::Item> {
    if !self.has_next() {
      return None;
    }
    while self.cur_count < self.final_count {
      self.cur += 1;
      if self.nodes[self.level][self.cur as usize].is_some() {
        self.cur_count += 1;
        return Some(self.cur as usize);
      }
    }
    unreachable!()
  }
}

impl NodesIterator for NodeIteratorImpl {
  fn size(&self) -> usize {
    self.size
  }

  fn consume(&mut self, _dest: &mut [usize]) -> Result<usize> {
    unreachable!()
  }

  fn has_next(&self) -> bool {
    self.cur_count < self.final_count
  }
}
