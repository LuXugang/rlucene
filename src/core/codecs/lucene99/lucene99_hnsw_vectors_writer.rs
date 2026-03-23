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
use crate::core::codecs::hnsw::flat_field_vectors_writer::FlatFieldVectorsWriter;
use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
use crate::core::codecs::hnsw::flat_vectors_writer::FlatVectorsWriter;
use crate::core::codecs::knn_field_vectors_writer::KnnFieldVectorsWriter;
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_format::Lucene99HnswVectorsFormat;
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_reader::SIMILARITY_FUNCTIONS;
use crate::core::index::byte_vector_values::from_bytes;
use crate::core::index::docs_with_field_set::DocsWithFieldSet;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::float_vector_values::from_floats;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::store::IndexOutput;
use crate::core::util::TryIntoInt;
use crate::core::util::accountable::Accountable;
use crate::core::util::bit_set::BitSet;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::hnsw::hnsw_builder::HnswBuilder;
use crate::core::util::hnsw::hnsw_graph::{
  ArrayNodesIterator, HnswGraph, NodesIterator, NodesIteratorEnums, get_sorted_nodes,
};
use crate::core::util::hnsw::hnsw_graph_builder::{HnswGraphBuilder, RAND_SEED, create};
use crate::core::util::hnsw::hnsw_graph_merger::HnswGraphMergerEnum;
use crate::core::util::hnsw::hnsw_graph_searcher::{
  HnswGraphSearcherBase, HnswGraphSearcherBaseDefault,
};
use crate::core::util::hnsw::neighbor_array::NeighborArray;
use crate::core::util::hnsw::on_heap_hnsw_graph::OnHeapHnswGraph;
use crate::core::util::hnsw::random_vector_scorer_supplier::RandomVectorScorerSupplier;
use crate::core::util::info_stream::InfoStreamMT;
use crate::core::util::packed::direct_monotonic_writer::DirectMonotonicWriter;
use std::sync::Arc;

//TODO: memory calculation not implement
const SHALLOW_RAM_BYTES_USED: i64 = 0;
pub struct Lucene99HnswVectorsWriter<F, O>
where
  F: FlatVectorsWriter,
  O: IndexOutput,
{
  meta: O,
  vector_index: O,
  m: usize,
  beam_width: usize,
  flat_vector_writer: F,
  num_merge_workers: usize,
  // TODO IMPORTANT 多线程未实现
  finished: bool,
}

impl<F, O> Lucene99HnswVectorsWriter<F, O>
where
  F: FlatVectorsWriter,
  O: IndexOutput,
{
  fn reconstruct_and_write_neighbours(
    &mut self,
    neighbors: &mut NeighborArray,
    old_to_new_map: &[usize],
    max_ord: usize,
  ) -> Result<()> {
    let size = neighbors.size();
    self.vector_index.write_vint(size as i32)?;

    let nnodes = neighbors.nodes_mut();

    for node in nnodes.iter_mut().take(size) {
      *node = old_to_new_map[*node];
    }

    nnodes[..size].sort();

    for i in (1..size).rev() {
      debug_assert!(
        nnodes[i] < max_ord,
        "node too large: {} >= {}",
        nnodes[i],
        max_ord
      );
      nnodes[i] -= nnodes[i - 1];
    }

    for &node in nnodes.iter().take(size) {
      self.vector_index.write_vint(node as i32)?;
    }

    Ok(())
  }

  /// @param graph Write the graph in a compressed format
  /// @return The non-cumulative offsets for the nodes. Should be used to create cumulative offsets.
  /// @throws IOException if writing to vectorIndex fails
  fn write_graph(&mut self, graph: Option<&mut OnHeapHnswGraph>) -> Result<Vec<Vec<i32>>> {
    let Some(graph) = graph else {
      return Ok(Vec::new());
    };

    let count_on_level0 = graph.size();
    let num_levels = graph.num_levels()?;
    let mut offsets = vec![Vec::new(); num_levels];

    for (level, level_offsets) in offsets.iter_mut().enumerate().take(num_levels) {
      let mut nodes = graph.get_nodes_on_level(level)?;
      let sorted_nodes = get_sorted_nodes(&mut nodes);

      let mut current_level_offsets = vec![0i32; sorted_nodes.len()];

      for (node_offset_id, &node) in sorted_nodes.iter().enumerate() {
        let neighbors = graph.get_neighbors(level, node);
        let size = neighbors.size();

        let offset_start = self.vector_index.get_file_pointer();

        self.vector_index.write_vint(size as i32)?;

        let nnodes = neighbors.nodes();
        let mut nnodes = nnodes[..size].to_vec();
        nnodes.sort();

        for i in (1..size).rev() {
          debug_assert!(
            nnodes[i] < count_on_level0,
            "node too large: {} >= {}",
            nnodes[i],
            count_on_level0
          );
          nnodes[i] -= nnodes[i - 1];
        }

        for &n in &nnodes {
          self.vector_index.write_vint(n as i32)?;
        }

        let offset = (self.vector_index.get_file_pointer() - offset_start).try_convert()?;

        current_level_offsets[node_offset_id] = offset;
      }

      *level_offsets = current_level_offsets;
    }

    Ok(offsets)
  }

  fn write_meta<H>(
    &mut self,
    field: &FieldInfo,
    vector_index_offset: i64,
    vector_index_length: i64,
    count: i32,
    graph: Option<&mut H>,
    graph_level_node_offsets: &[Vec<i32>],
  ) -> Result<()>
  where
    H: HnswGraph,
  {
    self.meta.write_int(field.number)?;
    self.meta.write_int(field.get_vector_encoding().ordinal())?;
    self
      .meta
      .write_int(dist_func_to_ord(field.get_vector_similarity_function())? as i32)?;
    self.meta.write_vlong(vector_index_offset)?;
    self.meta.write_vlong(vector_index_length)?;
    self.meta.write_vint(field.get_vector_dimension())?;
    self.meta.write_int(count)?;
    self.meta.write_vint(self.m as i32)?;

    let Some(graph) = graph else {
      self.meta.write_vint(0)?;
      return Ok(());
    };

    self.meta.write_vint(graph.num_levels()? as i32)?;
    let mut value_count: i64 = 0;

    for level in 0..graph.num_levels()? {
      let mut nodes_on_level = graph.get_nodes_on_level(level)?;
      value_count += nodes_on_level.size() as i64;

      if level > 0 {
        let mut nol = vec![0usize; nodes_on_level.size()];
        let number_consumed = nodes_on_level.consume(nol.as_mut())?;
        nol.sort();

        debug_assert_eq!(number_consumed, nodes_on_level.size());

        self.meta.write_vint(nol.len() as i32)?;

        for i in (1..nol.len()).rev() {
          nol[i] -= nol[i - 1];
        }

        for &n in &nol {
          self.meta.write_vint(n as i32)?;
        }
      } else {
        debug_assert_eq!(
          nodes_on_level.size(),
          count as usize,
          "Level 0 expects to have all nodes"
        );
      }
    }

    let start = self.vector_index.get_file_pointer();
    self.meta.write_long(start as i64)?;

    self
      .meta
      .write_vint(Lucene99HnswVectorsFormat::DIRECT_MONOTONIC_BLOCK_SHIFT)?;

    let mut memory_offsets_writer = DirectMonotonicWriter::get_instance(
      &mut self.meta,
      &mut self.vector_index,
      value_count,
      Lucene99HnswVectorsFormat::DIRECT_MONOTONIC_BLOCK_SHIFT,
    )?;

    let mut cumulative_offset_sum: i64 = 0;

    for level_offsets in graph_level_node_offsets {
      for &v in level_offsets {
        memory_offsets_writer.add(cumulative_offset_sum)?;
        cumulative_offset_sum += v as i64;
      }
    }

    memory_offsets_writer.finish()?;

    let end = self.vector_index.get_file_pointer();
    self.meta.write_long((end - start) as i64)?;

    Ok(())
  }
  fn create_graph_merger(&self) -> HnswGraphMergerEnum {
    todo!()
  }
}

pub(crate) fn dist_func_to_ord(func: &VectorSimilarityFunction) -> Result<u8> {
  for (i, f) in SIMILARITY_FUNCTIONS.iter().enumerate() {
    if f == func {
      return Ok(i as u8);
    }
  }
  Err(LuceneError::illegal_argument(format!(
    "invalid distance function: {:?}",
    func
  )))
}

pub(crate) fn create_field_writer_byte<F, S>(
  scorer: &S,
  flat_field_vectors_writer: F,
  field_info: Arc<FieldInfo>,
  m: usize,
  beam_width: usize,
  info_stream: InfoStreamMT,
) -> Result<FieldWriter<S::RandomVectorScorerSupplier, FixedBitSet, HnswGraphSearcherBaseDefault, F>>
where
  F: FlatFieldVectorsWriter<V = Vec<u8>>,
  S: FlatVectorsScorer,
{
  FieldWriter::from_byte(
    scorer,
    flat_field_vectors_writer,
    field_info,
    m,
    beam_width,
    info_stream,
  )
}
pub(crate) fn create_field_writer_float<F, S>(
  scorer: &S,
  flat_field_vectors_writer: F,
  field_info: Arc<FieldInfo>,
  m: usize,
  beam_width: usize,
  info_stream: InfoStreamMT,
) -> Result<FieldWriter<S::RandomVectorScorerSupplier, FixedBitSet, HnswGraphSearcherBaseDefault, F>>
where
  F: FlatFieldVectorsWriter<V = Vec<f32>>,
  S: FlatVectorsScorer,
{
  FieldWriter::from_float(
    scorer,
    flat_field_vectors_writer,
    field_info,
    m,
    beam_width,
    info_stream,
  )
}

pub(crate) struct FieldWriter<S, B, H, F>
where
  S: RandomVectorScorerSupplier,
  B: BitSet,
  H: HnswGraphSearcherBase,
  F: FlatFieldVectorsWriter,
{
  field_info: Arc<FieldInfo>,
  hnsw_graph_builder: HnswGraphBuilder<S, B, H>,
  last_doc_id: i32,
  node: usize,
  flat_field_vectors_writer: F,
}
impl<S, F> FieldWriter<S, FixedBitSet, HnswGraphSearcherBaseDefault, F>
where
  S: RandomVectorScorerSupplier,
  F: FlatFieldVectorsWriter<V = Vec<u8>>,
{
  fn from_byte(
    scorer: &impl FlatVectorsScorer<RandomVectorScorerSupplier = S>,
    flat_field_vectors_writer: F,
    field_info: Arc<FieldInfo>,
    m: usize,
    beam_width: usize,
    info_stream: InfoStreamMT,
  ) -> Result<Self> {
    let random_vector_scorer_supplier = from_bytes(
      flat_field_vectors_writer.get_vectors().as_slice(),
      field_info.get_vector_dimension() as usize,
    );
    let scorer_supplier = scorer.get_random_vector_scorer_supplier(
      *field_info.get_vector_similarity_function(),
      &random_vector_scorer_supplier,
    )?;
    Self::new(
      scorer_supplier,
      flat_field_vectors_writer,
      field_info,
      m,
      beam_width,
      info_stream,
    )
  }
}
impl<S, F> FieldWriter<S, FixedBitSet, HnswGraphSearcherBaseDefault, F>
where
  S: RandomVectorScorerSupplier,
  F: FlatFieldVectorsWriter<V = Vec<f32>>,
{
  fn from_float(
    scorer: &impl FlatVectorsScorer<RandomVectorScorerSupplier = S>,
    flat_field_vectors_writer: F,
    field_info: Arc<FieldInfo>,
    m: usize,
    beam_width: usize,
    info_stream: InfoStreamMT,
  ) -> Result<Self> {
    let random_vector_scorer_supplier = from_floats(
      flat_field_vectors_writer.get_vectors().as_slice(),
      field_info.get_vector_dimension() as usize,
    );
    let scorer_supplier = scorer.get_random_vector_scorer_supplier(
      *field_info.get_vector_similarity_function(),
      &random_vector_scorer_supplier,
    )?;
    Self::new(
      scorer_supplier,
      flat_field_vectors_writer,
      field_info,
      m,
      beam_width,
      info_stream,
    )
  }
}
impl<S, F> FieldWriter<S, FixedBitSet, HnswGraphSearcherBaseDefault, F>
where
  S: RandomVectorScorerSupplier,
  F: FlatFieldVectorsWriter,
{
  fn new(
    scorer_supplier: S,
    flat_field_vectors_writer: F,
    field_info: Arc<FieldInfo>,
    m: usize,
    beam_width: usize,
    info_stream: InfoStreamMT,
  ) -> Result<Self> {
    let mut hnsw_graph_builder = create(scorer_supplier, m, beam_width, RAND_SEED)?;

    hnsw_graph_builder.set_info_stream(info_stream);

    Ok(Self {
      field_info,
      hnsw_graph_builder,
      last_doc_id: 0,
      node: 0,
      flat_field_vectors_writer,
    })
  }
}
impl<S, B, H, F> FieldWriter<S, B, H, F>
where
  B: BitSet,
  F: FlatFieldVectorsWriter,
  H: HnswGraphSearcherBase,
  S: RandomVectorScorerSupplier,
{
  pub fn get_docs_with_field_set(&self) -> &DocsWithFieldSet {
    self.flat_field_vectors_writer.get_docs_with_field_set()
  }
  pub(crate) fn get_graph(&mut self) -> Result<Option<&mut OnHeapHnswGraph>> {
    debug_assert!(self.flat_field_vectors_writer.is_finished());

    if self.node > 0 {
      Ok(Some(self.hnsw_graph_builder.get_completed_graph()?))
    } else {
      Ok(None)
    }
  }
}
impl<S, B, H, F> Accountable for FieldWriter<S, B, H, F>
where
  B: BitSet,
  F: FlatFieldVectorsWriter,
  H: HnswGraphSearcherBase,
  S: RandomVectorScorerSupplier,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    //TODO: memory calculation not implement
    Ok(0)
  }
}

impl<S, B, H, F> KnnFieldVectorsWriter for FieldWriter<S, B, H, F>
where
  S: RandomVectorScorerSupplier,
  B: BitSet,
  H: HnswGraphSearcherBase,
  F: FlatFieldVectorsWriter,
{
  type V = F::V;

  fn add_value(&mut self, doc_id: i32, vector_value: Self::V) -> Result<()> {
    if doc_id == self.last_doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "VectorValuesField \"{}\" appears more than once in this document (only one value is allowed per field)",
        self.field_info.name
      )));
    }

    self
      .flat_field_vectors_writer
      .add_value(doc_id, vector_value)?;
    self.hnsw_graph_builder.add_graph_node(self.node)?;
    self.node += 1;
    self.last_doc_id = doc_id;
    Ok(())
  }

  fn copy_value(&self, _vector_value: Self::V) -> Result<Self::V> {
    Err(LuceneError::unsupported_operation(""))
  }
}

struct HnswGraphImpl<'a> {
  graph: &'a mut OnHeapHnswGraph,
  nodes_by_level: Vec<Arc<Vec<usize>>>,
}
impl<'a> HnswGraphImpl<'a> {
  fn new(graph: &'a mut OnHeapHnswGraph, nodes_by_level: Vec<Arc<Vec<usize>>>) -> Self {
    Self {
      graph,
      nodes_by_level,
    }
  }
}
impl<'a> HnswGraph for HnswGraphImpl<'a> {
  fn seek(&mut self, _level: usize, _target: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "Not supported on a mock graph",
    ))
  }

  fn size(&self) -> usize {
    self.graph.size()
  }

  fn next_neighbor(&mut self) -> Result<usize> {
    Err(LuceneError::unsupported_operation(
      "Not supported on a mock graph",
    ))
  }

  fn num_levels(&self) -> Result<usize> {
    self.graph.num_levels()
  }

  fn entry_node(&self) -> Result<Option<usize>> {
    Err(LuceneError::unsupported_operation(
      "Not supported on a mock graph",
    ))
  }

  type NodeIterator = NodesIteratorEnums;

  fn get_nodes_on_level(&mut self, level: usize) -> Result<Self::NodeIterator> {
    if level == 0 {
      self.graph.get_nodes_on_level(0)
    } else {
      let nodes = self
        .nodes_by_level
        .get(level)
        .ok_or_else(|| LuceneError::illegal_argument(format!("Invalid level: {}", level)))?;
      Ok(NodesIteratorEnums::Array(ArrayNodesIterator::from_nodes(
        Option::from(nodes.clone()),
        nodes.len(),
      )))
    }
  }
}
