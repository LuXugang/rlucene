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
use crate::core::codecs::CodecUtil;
use crate::core::codecs::hnsw::flat_field_vectors_writer::FlatFieldVectorsWriter;
use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
use crate::core::codecs::hnsw::flat_vectors_writer::{FlatVectorsWriter, FlatVectorsWriterSs};
use crate::core::codecs::knn_field_vectors_writer::KnnFieldVectorsWriter;
use crate::core::codecs::knn_vectors_writer::{KnnVectorsWriter, map_old_ord_to_new_ord};
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_format::{
  DIRECT_MONOTONIC_BLOCK_SHIFT, META_CODEC_NAME, META_EXTENSION, VECTOR_INDEX_CODEC_NAME,
  VECTOR_INDEX_EXTENSION, VERSION_CURRENT,
};
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_reader::SIMILARITY_FUNCTIONS;
use crate::core::index::IndexFileNames;
use crate::core::index::byte_vector_values::{ByteVectorValuesImpl, from_bytes};
use crate::core::index::docs_with_field_set::DocsWithFieldSet;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::float_vector_values::{FloatVectorValuesImpl, from_floats};
use crate::core::index::knn_vector_values::KnnVectorValuesEnum;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::index::sorter::DocMap;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::store::IndexOutput;
use crate::core::store::directory::Directory;
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
use std::marker::PhantomData;
use std::sync::Arc;

//TODO: memory calculation not implement
const SHALLOW_RAM_BYTES_USED: i64 = 0;
/// Writes vector values and knn graphs to index segments.
pub struct Lucene99HnswVectorsWriter<F, O, V>
where
  F: FlatVectorsWriter,
  O: IndexOutput,
  V: Clone,
{
  meta: O,
  vector_index: O,
  m: usize,
  beam_width: usize,
  flat_vector_writer: F,
  num_merge_workers: usize,
  // TODO IMPORTANT 多线程未实现
  finished: bool,
  info_stream: InfoStreamMT,
  fields: Vec<FieldWriterType<DefaultRandomVectorScorerSupplier<F>, V>>,
}
pub type DefaultRandomVectorScorerSupplier<F> =
  FlatVectorsWriterSs<F, ByteVectorValuesImpl, FloatVectorValuesImpl>;
impl<F, O, V> Lucene99HnswVectorsWriter<F, O, V>
where
  F: FlatVectorsWriter,
  O: IndexOutput,
  V: Clone,
{
  pub fn new<D1, D2>(
    state: &SegmentWriteState<D1>,
    m: usize,
    beam_width: usize,
    flat_vector_writer: F,
    num_merge_workers: usize,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self>
  where
    D1: Directory<IndexOutput = O>,
    D2: Directory,
  {
    let meta_file_name =
      IndexFileNames::segment_file_name(&segment_info.name, &state.segment_suffix, META_EXTENSION);

    let index_data_file_name = IndexFileNames::segment_file_name(
      &segment_info.name,
      &state.segment_suffix,
      VECTOR_INDEX_EXTENSION,
    );
    let mut meta = state
      .directory
      .create_output(&meta_file_name, state.context)?;

    let mut vector_index = state
      .directory
      .create_output(&index_data_file_name, state.context)?;
    let result = (|| -> Result<()> {
      CodecUtil::write_index_header(
        &mut meta,
        META_CODEC_NAME,
        VERSION_CURRENT,
        segment_info.get_id(),
        &state.segment_suffix,
      )?;

      CodecUtil::write_index_header(
        &mut vector_index,
        VECTOR_INDEX_CODEC_NAME,
        VERSION_CURRENT,
        segment_info.get_id(),
        &state.segment_suffix,
      )?;
      Ok(())
    })();

    result?;

    Ok(Self {
      meta,
      vector_index,
      m,
      beam_width,
      flat_vector_writer,
      num_merge_workers,
      finished: false,
      info_stream: state.info_stream.clone(),
      fields: Vec::new(),
    })
  }
  fn write_field(&mut self, field_data: usize) -> Result<()> {
    let field_data = self.fields.get_mut(field_data).unwrap();
    let vector_index_offset = self.vector_index.get_file_pointer();

    let flat_field_vectors_writers = self.flat_vector_writer.get_fields_mut();
    let cardinality = field_data
      .get_docs_with_field_set(flat_field_vectors_writers)?
      .cardinality();
    let field_info = field_data.field_info.clone();
    let graph_level_node_offsets = {
      let graph = field_data.get_graph(flat_field_vectors_writers)?;
      Self::write_graph(&mut self.vector_index, graph)?
    };

    let vector_index_length = self.vector_index.get_file_pointer() - vector_index_offset;
    let graph = field_data.get_graph(flat_field_vectors_writers)?;
    Self::write_meta(
      &mut self.vector_index,
      &mut self.meta,
      self.m,
      &field_info,
      vector_index_offset.try_convert()?,
      vector_index_length.try_convert()?,
      cardinality,
      graph,
      &graph_level_node_offsets,
    )?;

    Ok(())
  }
  fn write_sorting_field<DM>(&mut self, field_data_idx: usize, sort_map: &DM) -> Result<()>
  where
    DM: DocMap,
  {
    let flat_field_vectors_writers = self.flat_vector_writer.get_fields_mut();
    let field_data = self.fields.get_mut(field_data_idx).unwrap();
    let cardinality = field_data
      .get_docs_with_field_set(flat_field_vectors_writers)?
      .cardinality() as usize;

    let mut ord_map = vec![0; cardinality];
    let mut old_ord_map = vec![0; cardinality];

    map_old_ord_to_new_ord(
      field_data.get_docs_with_field_set(flat_field_vectors_writers)?,
      sort_map,
      Some(&mut old_ord_map),
      Some(&mut ord_map),
      None,
    )?;

    let vector_index_offset = self.vector_index.get_file_pointer();

    let field_info = field_data.field_info.clone();

    let count = field_data
      .get_docs_with_field_set(flat_field_vectors_writers)?
      .cardinality();
    let graph = field_data.get_graph(flat_field_vectors_writers)?;

    let mut graph_level_node_offsets = if let Some(ref g) = graph {
      vec![Vec::new(); g.num_levels()?]
    } else {
      Vec::new()
    };

    let mut mock_graph = Self::reconstruct_and_write_graph(
      &mut self.vector_index,
      graph,
      ord_map.as_ref(),
      old_ord_map.as_ref(),
      &mut graph_level_node_offsets,
    )?;

    let vector_index_length = self.vector_index.get_file_pointer() - vector_index_offset;

    Self::write_meta(
      &mut self.vector_index,
      &mut self.meta,
      self.m,
      &field_info,
      vector_index_offset.try_convert()?,
      vector_index_length.try_convert()?,
      count,
      mock_graph.as_mut(),
      &graph_level_node_offsets,
    )?;

    Ok(())
  }
  /// Reconstructs the graph given the old and new node ids.
  ///
  /// Additionally, the graph node connections are written to the vectorIndex.
  ///
  /// # Arguments
  /// * `graph` - The current on heap graph
  /// * `new_to_old_map` - the new node ids indexed to the old node ids
  /// * `old_to_new_map` - the old node ids indexed to the new node ids
  /// * `level_node_offsets` - where to place the new offsets for the nodes in the vector index.
  ///
  /// # Returns
  /// The graph
  ///
  /// # Errors
  /// if writing to vectorIndex fails
  fn reconstruct_and_write_graph<'a>(
    vector_index: &mut O,
    graph: Option<&'a mut OnHeapHnswGraph>,
    new_to_old_map: &[usize],
    old_to_new_map: &[usize],
    level_node_offsets: &mut [Vec<i32>],
  ) -> Result<Option<HnswGraphImpl<'a>>> {
    let Some(graph) = graph else {
      return Ok(None);
    };
    let num_levels = graph.num_levels()?;
    let mut nodes_by_level = Vec::with_capacity(num_levels);
    nodes_by_level.push(Arc::new(Vec::new()));

    let max_ord = graph.size();
    let mut nodes_on_level0 = graph.get_nodes_on_level(0)?;
    level_node_offsets[0] = vec![0i32; nodes_on_level0.size()];

    while nodes_on_level0.has_next() {
      let node = nodes_on_level0
        .next()
        .ok_or_else(|| LuceneError::illegal_state("Expected more nodes on level 0"))?;
      let neighbors = graph.get_neighbors_mut(0, new_to_old_map[node])?;

      let offset = vector_index.get_file_pointer();

      Self::reconstruct_and_write_neighbours(vector_index, neighbors, old_to_new_map, max_ord)?;

      let delta = (vector_index.get_file_pointer() - offset).try_convert()?;

      level_node_offsets[0][node] = delta;
    }

    for (level, level_offsets) in level_node_offsets
      .iter_mut()
      .enumerate()
      .take(num_levels)
      .skip(1)
    {
      let mut nodes_on_level = graph.get_nodes_on_level(level)?;
      let mut new_nodes = vec![0usize; nodes_on_level.size()];

      let mut n = 0;
      while nodes_on_level.has_next() {
        new_nodes[n] = old_to_new_map[nodes_on_level
          .next()
          .ok_or_else(|| LuceneError::illegal_state("Expected more nodes on level"))?];
        n += 1;
      }

      new_nodes.sort();

      *level_offsets = vec![0i32; new_nodes.len()];

      for (node_offset_index, &node) in new_nodes.iter().enumerate() {
        let neighbors = graph.get_neighbors_mut(level, new_to_old_map[node])?;

        let offset = vector_index.get_file_pointer();

        Self::reconstruct_and_write_neighbours(vector_index, neighbors, old_to_new_map, max_ord)?;

        let delta = (vector_index.get_file_pointer() - offset).try_convert()?;

        level_offsets[node_offset_index] = delta;
      }
      nodes_by_level.push(Arc::new(new_nodes));
    }

    Ok(Some(HnswGraphImpl::new(graph, nodes_by_level)))
  }
  fn reconstruct_and_write_neighbours(
    vector_index: &mut O,
    neighbors: &mut NeighborArray,
    old_to_new_map: &[usize],
    max_ord: usize,
  ) -> Result<()> {
    let size = neighbors.size();
    vector_index.write_vint(size as i32)?;

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
      vector_index.write_vint(node as i32)?;
    }

    Ok(())
  }

  /// @param graph Write the graph in a compressed format
  /// @return The non-cumulative offsets for the nodes. Should be used to create cumulative offsets.
  /// @throws IOException if writing to vectorIndex fails
  fn write_graph(
    vector_index: &mut O,
    graph: Option<&mut OnHeapHnswGraph>,
  ) -> Result<Vec<Vec<i32>>> {
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
        let neighbors = graph.get_neighbors_mut(level, node)?;
        let size = neighbors.size();

        let offset_start = vector_index.get_file_pointer();

        vector_index.write_vint(size as i32)?;

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
          vector_index.write_vint(n as i32)?;
        }

        let offset = (vector_index.get_file_pointer() - offset_start).try_convert()?;

        current_level_offsets[node_offset_id] = offset;
      }

      *level_offsets = current_level_offsets;
    }

    Ok(offsets)
  }
  #[allow(clippy::too_many_arguments)]
  fn write_meta<H>(
    meta: &mut O,
    vector_index: &mut O,
    m: usize,
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
    meta.write_int(field.number)?;
    meta.write_int(field.get_vector_encoding().ordinal())?;
    meta.write_int(dist_func_to_ord(field.get_vector_similarity_function())? as i32)?;
    meta.write_vlong(vector_index_offset)?;
    meta.write_vlong(vector_index_length)?;
    meta.write_vint(field.get_vector_dimension())?;
    meta.write_int(count)?;
    meta.write_vint(m as i32)?;

    let Some(graph) = graph else {
      meta.write_vint(0)?;
      return Ok(());
    };

    meta.write_vint(graph.num_levels()? as i32)?;
    let mut value_count: i64 = 0;

    for level in 0..graph.num_levels()? {
      let mut nodes_on_level = graph.get_nodes_on_level(level)?;
      value_count += nodes_on_level.size() as i64;

      if level > 0 {
        let mut nol = vec![0usize; nodes_on_level.size()];
        let number_consumed = nodes_on_level.consume(nol.as_mut())?;
        nol.sort();

        debug_assert_eq!(number_consumed, nodes_on_level.size());

        meta.write_vint(nol.len() as i32)?;

        for i in (1..nol.len()).rev() {
          nol[i] -= nol[i - 1];
        }

        for &n in &nol {
          meta.write_vint(n as i32)?;
        }
      } else {
        debug_assert_eq!(
          nodes_on_level.size(),
          count as usize,
          "Level 0 expects to have all nodes"
        );
      }
    }

    let start = vector_index.get_file_pointer();
    meta.write_long(start as i64)?;

    meta.write_vint(DIRECT_MONOTONIC_BLOCK_SHIFT)?;

    let mut memory_offsets_writer = DirectMonotonicWriter::get_instance(
      meta,
      vector_index,
      value_count,
      DIRECT_MONOTONIC_BLOCK_SHIFT,
    )?;

    let mut cumulative_offset_sum: i64 = 0;

    for level_offsets in graph_level_node_offsets {
      for &v in level_offsets {
        memory_offsets_writer.add(cumulative_offset_sum)?;
        cumulative_offset_sum += v as i64;
      }
    }

    memory_offsets_writer.finish()?;

    let end = vector_index.get_file_pointer();
    meta.write_long((end - start) as i64)?;

    Ok(())
  }
  fn create_graph_merger(&self) -> HnswGraphMergerEnum {
    todo!()
  }
  fn flush<DM>(&mut self, max_doc: i32, sort_map: Option<&DM>) -> Result<()>
  where
    DM: DocMap,
  {
    self
      .flat_vector_writer
      .flat_flush::<DM, F, V>(max_doc, sort_map, &self.fields)?;

    for field_idx in 0..self.fields.len() {
      if let Some(sm) = sort_map {
        self.write_sorting_field(field_idx, sm)?;
      } else {
        self.write_field(field_idx)?;
      }
    }

    Ok(())
  }
  fn finish(&mut self) -> Result<()> {
    if self.finished {
      return Err(LuceneError::illegal_state("already finished"));
    }
    self.finished = true;

    self.flat_vector_writer.finish()?;
    // write end of fields marker
    self.meta.write_int(-1)?;
    CodecUtil::write_footer(&mut self.meta)?;

    CodecUtil::write_footer(&mut self.vector_index)?;

    Ok(())
  }
}

impl<F, O, V> Accountable for Lucene99HnswVectorsWriter<F, O, V>
where
  F: FlatVectorsWriter,
  O: IndexOutput,
  V: Clone,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    // TODO: memory calculation not implement
    Ok(0)
  }
}
impl<F, O> KnnVectorsWriter for Lucene99HnswVectorsWriter<F, O, u8>
where
  F: FlatVectorsWriter,
  O: IndexOutput,
{
  fn add_field(&mut self, field_info: Arc<FieldInfo>) -> Result<usize> {
    let flat_field_vectors_writer =
      FlatVectorsWriter::flat_add_field(&mut self.flat_vector_writer, field_info.clone())?;
    let scorer = self.flat_vector_writer.get_flat_vector_scorer();
    let v = create_field_writer_byte(
      scorer,
      flat_field_vectors_writer,
      field_info,
      self.m,
      self.beam_width,
      self.info_stream.clone(),
    )?;
    self.fields.push(v);
    Ok(self.fields.len() - 1)
  }

  fn flush<DM>(&mut self, max_doc: i32, sort_map: Option<&DM>) -> Result<()>
  where
    DM: DocMap,
  {
    self.flush(max_doc, sort_map)
  }

  fn finish(&mut self) -> Result<()> {
    self.finish()
  }
}
impl<F, O> KnnVectorsWriter for Lucene99HnswVectorsWriter<F, O, f32>
where
  F: FlatVectorsWriter,
  O: IndexOutput,
{
  fn add_field(&mut self, field_info: Arc<FieldInfo>) -> Result<usize> {
    let flat_field_vectors_writer =
      FlatVectorsWriter::flat_add_field(&mut self.flat_vector_writer, field_info.clone())?;
    let scorer = self.flat_vector_writer.get_flat_vector_scorer();
    let v = create_field_writer_float(
      scorer,
      flat_field_vectors_writer,
      field_info,
      self.m,
      self.beam_width,
      self.info_stream.clone(),
    )?;
    self.fields.push(v);
    Ok(self.fields.len() - 1)
  }

  fn flush<DM>(&mut self, max_doc: i32, sort_map: Option<&DM>) -> Result<()>
  where
    DM: DocMap,
  {
    self.flush(max_doc, sort_map)
  }

  fn finish(&mut self) -> Result<()> {
    self.finish()
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

pub(crate) fn create_field_writer_byte<S>(
  scorer: &S,
  flat_field_vectors_writer_idx: usize,
  field_info: Arc<FieldInfo>,
  m: usize,
  beam_width: usize,
  info_stream: InfoStreamMT,
) -> Result<
  FieldWriterType<S::RandomVectorScorerSupplier<ByteVectorValuesImpl, FloatVectorValuesImpl>, u8>,
>
where
  S: FlatVectorsScorer,
{
  FieldWriter::from_byte(
    scorer,
    flat_field_vectors_writer_idx,
    field_info,
    m,
    beam_width,
    info_stream,
  )
}
pub(crate) fn create_field_writer_float<S>(
  scorer: &S,
  flat_field_vectors_writer_idx: usize,
  field_info: Arc<FieldInfo>,
  m: usize,
  beam_width: usize,
  info_stream: InfoStreamMT,
) -> Result<
  FieldWriterType<S::RandomVectorScorerSupplier<ByteVectorValuesImpl, FloatVectorValuesImpl>, f32>,
>
where
  S: FlatVectorsScorer,
{
  FieldWriter::from_float(
    scorer,
    flat_field_vectors_writer_idx,
    field_info,
    m,
    beam_width,
    info_stream,
  )
}
pub type FieldWriterType<S, V> = FieldWriter<S, FixedBitSet, HnswGraphSearcherBaseDefault, V>;
pub struct FieldWriter<S, B, H, V>
where
  S: RandomVectorScorerSupplier,
  B: BitSet,
  H: HnswGraphSearcherBase,
  V: Clone,
{
  field_info: Arc<FieldInfo>,
  pub(crate) hnsw_graph_builder: HnswGraphBuilder<S, B, H>,
  last_doc_id: i32,
  node: usize,
  flat_field_vectors_writer_idx: usize,
  _marker: PhantomData<V>,
}
impl<S, V> FieldWriterType<S, V>
where
  S: RandomVectorScorerSupplier,
  V: Clone,
{
  fn from_byte(
    scorer: &impl FlatVectorsScorer<
      RandomVectorScorerSupplier<ByteVectorValuesImpl, FloatVectorValuesImpl> = S,
    >,
    flat_field_vectors_writer_idx: usize,
    field_info: Arc<FieldInfo>,
    m: usize,
    beam_width: usize,
    info_stream: InfoStreamMT,
  ) -> Result<Self> {
    let random_vector_scorer_supplier = from_bytes(field_info.get_vector_dimension() as usize);
    let scorer_supplier = scorer.get_random_vector_scorer_supplier(
      *field_info.get_vector_similarity_function(),
      KnnVectorValuesEnum::<ByteVectorValuesImpl, FloatVectorValuesImpl>::Byte(
        random_vector_scorer_supplier,
      ),
    )?;
    Self::new(
      scorer_supplier,
      flat_field_vectors_writer_idx,
      field_info,
      m,
      beam_width,
      info_stream,
    )
  }
  fn from_float(
    scorer: &impl FlatVectorsScorer<
      RandomVectorScorerSupplier<ByteVectorValuesImpl, FloatVectorValuesImpl> = S,
    >,
    flat_field_vectors_writer_idx: usize,
    field_info: Arc<FieldInfo>,
    m: usize,
    beam_width: usize,
    info_stream: InfoStreamMT,
  ) -> Result<Self> {
    let random_vector_scorer_supplier = from_floats(field_info.get_vector_dimension() as usize);
    let scorer_supplier = scorer.get_random_vector_scorer_supplier(
      *field_info.get_vector_similarity_function(),
      KnnVectorValuesEnum::<ByteVectorValuesImpl, FloatVectorValuesImpl>::Float(
        random_vector_scorer_supplier,
      ),
    )?;
    Self::new(
      scorer_supplier,
      flat_field_vectors_writer_idx,
      field_info,
      m,
      beam_width,
      info_stream,
    )
  }
}
impl<S, V> FieldWriterType<S, V>
where
  S: RandomVectorScorerSupplier,
  V: Clone,
{
  fn new(
    scorer_supplier: S,
    flat_field_vectors_writer_idx: usize,
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
      flat_field_vectors_writer_idx,
      _marker: PhantomData,
    })
  }
}
impl<S, B, H, V> FieldWriter<S, B, H, V>
where
  B: BitSet,
  H: HnswGraphSearcherBase,
  S: RandomVectorScorerSupplier,
  V: Clone,
{
  pub fn get_docs_with_field_set<'a, F>(
    &self,
    flat_field_vectors_writers: &'a mut [F],
  ) -> Result<&'a DocsWithFieldSet>
  where
    F: FlatFieldVectorsWriter,
  {
    let v = flat_field_vectors_writers
      .get(self.flat_field_vectors_writer_idx)
      .ok_or_else(|| LuceneError::illegal_state("Invalid flat field vectors writer index"))?;
    Ok(v.get_docs_with_field_set())
  }
  pub(crate) fn get_graph<F>(
    &mut self,
    flat_field_vectors_writers: &mut [F],
  ) -> Result<Option<&mut OnHeapHnswGraph>>
  where
    F: FlatFieldVectorsWriter,
  {
    debug_assert!({
      let v = flat_field_vectors_writers
        .get(self.flat_field_vectors_writer_idx)
        .ok_or_else(|| LuceneError::illegal_state("Invalid flat field vectors writer index"))?;
      v.is_finished()
    });

    if self.node > 0 {
      Ok(Some(self.hnsw_graph_builder.get_completed_graph()?))
    } else {
      Ok(None)
    }
  }
}
impl<S, B, H, V> Accountable for FieldWriter<S, B, H, V>
where
  B: BitSet,
  H: HnswGraphSearcherBase,
  S: RandomVectorScorerSupplier,
  V: Clone,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    //TODO: memory calculation not implement
    Ok(0)
  }
}

impl<S, B, H> KnnFieldVectorsWriter for FieldWriter<S, B, H, u8>
where
  S: RandomVectorScorerSupplier,
  B: BitSet,
  H: HnswGraphSearcherBase,
{
  type V = Vec<u8>;

  fn add_value<F>(
    &mut self,
    doc_id: i32,
    vector_value: Self::V,
    flat_field_vectors_writers: &mut [F],
  ) -> Result<()>
  where
    F: FlatFieldVectorsWriter<V = Self::V>,
  {
    if doc_id == self.last_doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "VectorValuesField \"{}\" appears more than once in this document (only one value is allowed per field)",
        self.field_info.name
      )));
    }
    let flat_field_vectors_writer = flat_field_vectors_writers
      .get_mut(self.flat_field_vectors_writer_idx)
      .ok_or_else(|| LuceneError::illegal_state("Invalid flat field vectors writer index"))?;
    let ss = self.hnsw_graph_builder.get_scorer_supplier_mut();
    let vectors = ss.get_vector_byte_mut()?;
    FlatFieldVectorsWriter::flat_add_value::<F>(
      flat_field_vectors_writer,
      doc_id,
      vector_value,
      vectors,
    )?;
    self.hnsw_graph_builder.add_graph_node(self.node)?;
    self.node += 1;
    self.last_doc_id = doc_id;
    Ok(())
  }
}
impl<S, B, H> KnnFieldVectorsWriter for FieldWriter<S, B, H, f32>
where
  S: RandomVectorScorerSupplier,
  B: BitSet,
  H: HnswGraphSearcherBase,
{
  type V = Vec<f32>;

  fn add_value<F>(
    &mut self,
    doc_id: i32,
    vector_value: Self::V,
    flat_field_vectors_writers: &mut [F],
  ) -> Result<()>
  where
    F: FlatFieldVectorsWriter<V = Self::V>,
  {
    if doc_id == self.last_doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "VectorValuesField \"{}\" appears more than once in this document (only one value is allowed per field)",
        self.field_info.name
      )));
    }
    let flat_field_vectors_writer = flat_field_vectors_writers
      .get_mut(self.flat_field_vectors_writer_idx)
      .ok_or_else(|| LuceneError::illegal_state("Invalid flat field vectors writer index"))?;
    let ss = self.hnsw_graph_builder.get_scorer_supplier_mut();
    let vectors = ss.get_vector_float_mut()?;
    FlatFieldVectorsWriter::flat_add_value::<F>(
      flat_field_vectors_writer,
      doc_id,
      vector_value,
      vectors,
    )?;
    self.hnsw_graph_builder.add_graph_node(self.node)?;
    self.node += 1;
    self.last_doc_id = doc_id;
    Ok(())
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
