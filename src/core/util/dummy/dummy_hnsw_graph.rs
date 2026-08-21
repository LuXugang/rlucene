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
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::hnsw_graph::{ArrayNodesIterator, HnswGraph};

pub struct DummyHnswGraph;
impl HnswGraph for DummyHnswGraph {
  fn seek(&mut self, _level: usize, _target: usize) -> Result<()> {
    dummy_unreachable!()
  }

  fn size(&self) -> usize {
    dummy_unreachable!()
  }

  fn max_node_id(&self) -> Option<usize> {
    dummy_unreachable!()
  }

  fn next_neighbor(&mut self) -> Result<usize> {
    dummy_unreachable!()
  }

  fn num_levels(&self) -> Result<usize> {
    dummy_unreachable!()
  }

  fn entry_node(&self) -> Result<Option<usize>> {
    dummy_unreachable!()
  }

  type NodeIterator = ArrayNodesIterator;

  fn get_nodes_on_level(&mut self, _level: usize) -> Result<Self::NodeIterator> {
    dummy_unreachable!()
  }
}
