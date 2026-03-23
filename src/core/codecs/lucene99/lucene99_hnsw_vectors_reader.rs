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
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::store::IndexInput;
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::hnsw::hnsw_graph::{ArrayNodesIterator, HnswGraph};
use crate::core::util::long_values::LongValues;
use crate::core::util::packed::direct_monotonic_reader::direct_monotonic::Meta;
use crate::core::util::packed::direct_monotonic_reader::{DirectMonotonicReader, load_meta};
use std::sync::Arc;

pub struct Lucene99HnswVectorsReader;

pub const SIMILARITY_FUNCTIONS: &[VectorSimilarityFunction] = &[
  VectorSimilarityFunction::Euclidean,
  VectorSimilarityFunction::DotProduct,
  VectorSimilarityFunction::Cosine,
  VectorSimilarityFunction::MaximumInnerProduct,
];
pub struct FieldEntry {
  similarity_function: VectorSimilarityFunction,
  vector_encoding: VectorEncoding,
  vector_index_offset: usize,
  vector_index_length: usize,
  m: usize,
  num_levels: usize,
  dimension: i32,
  size: usize,
  nodes_by_level: Arc<Vec<Arc<Vec<usize>>>>,
  // for each level the start offsets in vectorIndex file from where to read neighbours
  offsets_meta: Option<Meta>,
  offsets_offset: usize,
  offsets_block_shift: i32,
  offsets_length: usize,
}

impl FieldEntry {
  pub fn create<I: IndexInput>(
    input: &mut I,
    vector_encoding: VectorEncoding,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<Self> {
    let vector_index_offset = input.read_vlong()?.try_convert()?;
    let vector_index_length = input.read_vlong()?.try_convert()?;
    let dimension = input.read_vint()?;
    let size = input.read_int()?.try_convert()?;

    let m = input.read_vint()?.try_convert()?;
    let num_levels = input.read_vint()?.try_convert()?;

    let mut nodes_by_level = Vec::with_capacity(num_levels);

    let mut number_of_offsets: i64 = 0;

    for level in 0..num_levels {
      if level > 0 {
        let num_nodes_on_level = input.read_vint()?.try_convert()?;
        number_of_offsets += num_nodes_on_level as i64;

        let mut level_nodes = vec![0usize; num_nodes_on_level];
        if num_nodes_on_level > 0 {
          level_nodes[0] = input.read_vint()?.try_convert()?;
          for i in 1..num_nodes_on_level {
            level_nodes[i] = level_nodes[i - 1] + input.read_vint()?.try_convert()?;
          }
        }
        nodes_by_level.push(Arc::new(level_nodes));
      } else {
        number_of_offsets += size as i64;
        nodes_by_level.push(Arc::new(Vec::new()));
      }
    }

    let (offsets_offset, offsets_block_shift, offsets_meta, offsets_length) =
      if number_of_offsets > 0 {
        let offsets_offset = input.read_long()?.try_convert()?;
        let offsets_block_shift = input.read_vint()?;
        let offsets_meta = Some(load_meta(input, number_of_offsets, offsets_block_shift)?);
        let offsets_length = input.read_long()?.try_convert()?;
        (
          offsets_offset,
          offsets_block_shift,
          offsets_meta,
          offsets_length,
        )
      } else {
        (0, 0, None, 0)
      };
    let nodes_by_level = Arc::new(nodes_by_level);
    Ok(Self {
      similarity_function,
      vector_encoding,
      vector_index_offset,
      vector_index_length,
      m,
      num_levels,
      dimension,
      size,
      nodes_by_level,
      offsets_meta,
      offsets_offset,
      offsets_block_shift,
      offsets_length,
    })
  }

  pub fn size(&self) -> usize {
    self.size
  }
}

pub struct OffHeapHnswGraph<I>
where
  I: IndexInput,
{
  data_in: I,
  nodes_by_level: Arc<Vec<Arc<Vec<usize>>>>,
  num_levels: usize,
  entry_node: usize,
  size: usize,
  arc_count: usize,
  arc_up_to: usize,
  arc: usize,
  graph_level_node_offsets: DirectMonotonicReader<I::RandomAccessSlice>,
  graph_level_node_index_offsets: Vec<usize>,
  // Allocated to be M*2 to track the current neighbors being explored
  current_neighbors_buffer: Vec<usize>,
}
impl<I> OffHeapHnswGraph<I>
where
  I: IndexInput<IndexInput = I>,
{
  pub fn new(entry: &FieldEntry, vector_index: &I) -> Result<Self> {
    let data_in = vector_index.slice(
      "graph-data",
      entry.vector_index_offset,
      entry.vector_index_length,
    )?;

    let nodes_by_level = entry.nodes_by_level.clone();
    let num_levels = entry.num_levels;
    let entry_node = if num_levels > 1 {
      nodes_by_level[num_levels - 1][0]
    } else {
      0
    };

    let size = entry.size();

    let addresses_data =
      vector_index.random_access_slice(entry.offsets_offset, entry.offsets_length)?;

    let graph_level_node_offsets = DirectMonotonicReader::get_instance(
      entry
        .offsets_meta
        .as_ref()
        .ok_or_else(|| LuceneError::illegal_state("meta is None"))?,
      addresses_data,
    )?;

    let current_neighbors_buffer = vec![0usize; entry.m * 2];

    let mut graph_level_node_index_offsets = vec![0usize; num_levels];
    graph_level_node_index_offsets[0] = 0;

    for i in 1..num_levels {
      let node_count = if nodes_by_level[i - 1].is_empty() {
        size
      } else {
        nodes_by_level[i - 1].len()
      };
      graph_level_node_index_offsets[i] = graph_level_node_index_offsets[i - 1] + node_count;
    }

    Ok(Self {
      data_in,
      nodes_by_level,
      num_levels,
      entry_node,
      size,
      arc_count: 0,
      arc_up_to: 0,
      arc: 0,
      graph_level_node_offsets,
      graph_level_node_index_offsets,
      current_neighbors_buffer,
    })
  }
}
impl<I> HnswGraph for OffHeapHnswGraph<I>
where
  I: IndexInput,
{
  fn seek(&mut self, level: usize, target_ord: usize) -> Result<()> {
    let target_index = if level == 0 {
      target_ord
    } else {
      let nodes = &self.nodes_by_level[level];
      match nodes.binary_search(&target_ord) {
        Ok(idx) => idx,
        Err(_) => {
          debug_assert!(false, "target_ord not found in level");
          return Err(LuceneError::illegal_state("target_ord not found"));
        },
      }
    };

    let offset = self
      .graph_level_node_offsets
      .get(target_index + self.graph_level_node_index_offsets[level])?;

    self.data_in.seek(offset as usize)?;

    self.arc_count = self.data_in.read_vint()?.try_convert()?;

    debug_assert!(
      self.arc_count <= self.current_neighbors_buffer.len(),
      "too many neighbors: {}",
      self.arc_count
    );

    if self.arc_count > 0 {
      self.current_neighbors_buffer[0] = self.data_in.read_vint()?.try_convert()?;
      for i in 1..self.arc_count {
        let delta = self.data_in.read_vint()?.try_convert()?;
        self.current_neighbors_buffer[i] = self.current_neighbors_buffer[i - 1] + delta;
      }
    }
    self.arc_up_to = 0;

    Ok(())
  }

  fn size(&self) -> usize {
    self.size
  }

  fn next_neighbor(&mut self) -> Result<usize> {
    if self.arc_up_to >= self.arc_count {
      return Ok(NO_MORE_DOCS as usize);
    }
    self.arc = self.current_neighbors_buffer[self.arc_up_to];
    self.arc_up_to += 1;
    Ok(self.arc)
  }

  fn num_levels(&self) -> Result<usize> {
    Ok(self.num_levels)
  }

  fn entry_node(&self) -> Result<Option<usize>> {
    Ok(Some(self.entry_node))
  }

  type NodeIterator = ArrayNodesIterator;

  fn get_nodes_on_level(&mut self, level: usize) -> Result<Self::NodeIterator> {
    if level == 0 {
      Ok(ArrayNodesIterator::from_size(self.size()))
    } else {
      let nodes = self.nodes_by_level[level].clone();
      let len = nodes.len();
      Ok(ArrayNodesIterator::from_nodes(Some(nodes), len))
    }
  }
}
