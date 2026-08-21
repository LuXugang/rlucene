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
use crate::core::codecs::knn_vectors_reader::KnnVectorsReader;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::knn_vector_values::{
  DocIndexIterator, DocIndexIteratorEnum2, KnnVectorValues,
};
use crate::core::index::merge_state::DocMap;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::concurrent_hnsw_merger::ConcurrentHnswMerger;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::hnsw::hnsw_builder::HnswBuilder;
use crate::core::util::hnsw::hnsw_graph_builder::{create_with_graph_size, rand_seed};
use crate::core::util::hnsw::hnsw_graph_merger::HnswGraphMerger;
use crate::core::util::hnsw::initialized_hnsw_graph_builder::from_graph;
use crate::core::util::hnsw::on_heap_hnsw_graph::OnHeapHnswGraph;
use crate::core::util::hnsw::random_vector_scorer_supplier::RandomVectorScorerSupplier;
use crate::core::util::info_stream::InfoStreamMT;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
pub(crate) enum HnswGraphMergerHook {
  #[default]
  Default,
  Concurrent(ConcurrentHnswMerger),
}

pub(crate) struct IncrementalHnswGraphMergerDefaults;

pub(crate) trait IncrementalHnswGraphMergerBase<S>
where
  S: RandomVectorScorerSupplier,
{
  #[allow(clippy::too_many_arguments)]
  fn create_builder<KV, R, D>(
    &self,
    field_info: &FieldInfo,
    scorer_supplier: S,
    m: usize,
    beam_width: usize,
    init_reader: Option<usize>,
    init_doc_map: Option<usize>,
    init_graph_size: usize,
    merged_vector_values: &mut KV,
    max_ord: i32,
    readers: &[Option<R>],
    doc_maps: &[D],
    info_stream: InfoStreamMT,
  ) -> Result<OnHeapHnswGraph>
  where
    KV: KnnVectorValues,
    R: KnnVectorsReader,
    D: DocMap,
  {
    IncrementalHnswGraphMergerDefaults::create_builder(
      field_info,
      scorer_supplier,
      m,
      beam_width,
      init_reader,
      init_doc_map,
      init_graph_size,
      merged_vector_values,
      max_ord,
      readers,
      doc_maps,
      info_stream,
    )
  }
}

impl<S> IncrementalHnswGraphMergerBase<S> for HnswGraphMergerHook
where
  S: RandomVectorScorerSupplier,
{
  fn create_builder<KV, R, D>(
    &self,
    field_info: &FieldInfo,
    scorer_supplier: S,
    m: usize,
    beam_width: usize,
    init_reader: Option<usize>,
    init_doc_map: Option<usize>,
    init_graph_size: usize,
    merged_vector_values: &mut KV,
    max_ord: i32,
    readers: &[Option<R>],
    doc_maps: &[D],
    info_stream: InfoStreamMT,
  ) -> Result<OnHeapHnswGraph>
  where
    KV: KnnVectorValues,
    R: KnnVectorsReader,
    D: DocMap,
  {
    match self {
      Self::Default => IncrementalHnswGraphMergerDefaults::create_builder(
        field_info,
        scorer_supplier,
        m,
        beam_width,
        init_reader,
        init_doc_map,
        init_graph_size,
        merged_vector_values,
        max_ord,
        readers,
        doc_maps,
        info_stream,
      ),
      Self::Concurrent(hook) => hook.create_builder(
        field_info,
        scorer_supplier,
        m,
        beam_width,
        init_reader,
        init_doc_map,
        init_graph_size,
        merged_vector_values,
        max_ord,
        readers,
        doc_maps,
        info_stream,
      ),
    }
  }
}

/// This selects the biggest Hnsw graph from the provided merge state and initializes a new
/// HnswGraphBuilder with that graph as a starting point.
pub struct IncrementalHnswGraphMerger<S> {
  field_info: Arc<FieldInfo>,
  scorer_supplier: Option<S>,
  m: usize,
  beam_width: usize,
  init_reader: Option<usize>,
  init_doc_map: Option<usize>,
  init_graph_size: usize,
  hook: HnswGraphMergerHook,
}

impl<S> IncrementalHnswGraphMerger<S> {
  pub fn new(field_info: Arc<FieldInfo>, scorer_supplier: S, m: usize, beam_width: usize) -> Self {
    Self {
      field_info,
      scorer_supplier: Some(scorer_supplier),
      m,
      beam_width,
      init_reader: None,
      init_doc_map: None,
      init_graph_size: 0,
      hook: HnswGraphMergerHook::Default,
    }
  }
}

impl<S> IncrementalHnswGraphMerger<S> {
  pub(crate) fn new_with_hook(
    field_info: Arc<FieldInfo>,
    scorer_supplier: S,
    m: usize,
    beam_width: usize,
    hook: HnswGraphMergerHook,
  ) -> Self {
    Self {
      field_info,
      scorer_supplier: Some(scorer_supplier),
      m,
      beam_width,
      init_reader: None,
      init_doc_map: None,
      init_graph_size: 0,
      hook,
    }
  }
}

impl<S> IncrementalHnswGraphMerger<S>
where
  S: RandomVectorScorerSupplier,
{
  /// Builds a new HnswGraphBuilder using the biggest graph from the merge state as a starting point.
  /// If no valid readers were added to the merge state, a new graph is created.
  ///
  /// # Arguments
  ///
  /// * `merged_vector_values` - vector values in the merged segment
  /// * `max_ord` - max num of vectors that will be merged into the graph
  ///
  /// # Returns
  ///
  /// HnswGraphBuilder
  ///
  /// # Errors
  ///
  /// Returns an error if reading from the merge state fails.
  fn create_builder<KV, R, D>(
    &mut self,
    merged_vector_values: &mut KV,
    max_ord: i32,
    readers: &[Option<R>],
    doc_maps: &[D],
    info_stream: InfoStreamMT,
  ) -> Result<OnHeapHnswGraph>
  where
    KV: KnnVectorValues,
    R: KnnVectorsReader,
    D: DocMap,
  {
    let scorer_supplier = self
      .scorer_supplier
      .take()
      .ok_or_else(|| LuceneError::illegal_state("scorer supplier has already been consumed"))?;
    self.hook.create_builder(
      self.field_info.as_ref(),
      scorer_supplier,
      self.m,
      self.beam_width,
      self.init_reader,
      self.init_doc_map,
      self.init_graph_size,
      merged_vector_values,
      max_ord,
      readers,
      doc_maps,
      info_stream,
    )
  }
}

impl IncrementalHnswGraphMergerDefaults {
  #[allow(clippy::too_many_arguments)]
  fn create_builder<KV, R, D>(
    field_info: &FieldInfo,
    scorer_supplier: impl RandomVectorScorerSupplier,
    m: usize,
    beam_width: usize,
    init_reader: Option<usize>,
    init_doc_map: Option<usize>,
    init_graph_size: usize,
    merged_vector_values: &mut KV,
    max_ord: i32,
    readers: &[Option<R>],
    doc_maps: &[D],
    info_stream: InfoStreamMT,
  ) -> Result<OnHeapHnswGraph>
  where
    KV: KnnVectorValues,
    R: KnnVectorsReader,
    D: DocMap,
  {
    match init_reader {
      Some(init_reader_idx) => {
        let init_reader = readers[init_reader_idx].as_ref().ok_or_else(|| {
          LuceneError::illegal_state(format!(
            "Reader at index {init_reader_idx} is not available"
          ))
        })?;
        let mut initializer_graph = init_reader.get_graph(field_info.name.as_str())?;
        let mut initialized_nodes = FixedBitSet::new(max_ord as usize);
        let init_doc_map_idx = init_doc_map
          .ok_or_else(|| LuceneError::illegal_state("initializer reader has no document map"))?;
        let doc_map = doc_maps.get(init_doc_map_idx).ok_or_else(|| {
          LuceneError::illegal_state(format!(
            "DocMap at index {init_doc_map_idx} is not available"
          ))
        })?;
        let old_to_new_ordinal_map = get_new_ord_mapping(
          field_info,
          init_graph_size,
          merged_vector_values,
          &mut initialized_nodes,
          init_reader,
          doc_map,
        )?;
        let mut builder = from_graph(
          scorer_supplier,
          m,
          beam_width,
          rand_seed(),
          &mut initializer_graph,
          &old_to_new_ordinal_map,
          initialized_nodes,
          max_ord,
        )?;
        builder.set_info_stream(info_stream);
        Ok(std::mem::replace(
          builder.build(max_ord as usize)?,
          OnHeapHnswGraph::new(m, 0),
        ))
      },
      None => {
        let mut builder =
          create_with_graph_size(scorer_supplier, m, beam_width, rand_seed(), max_ord)?;
        builder.set_info_stream(info_stream);
        Ok(std::mem::replace(
          builder.build(max_ord as usize)?,
          OnHeapHnswGraph::new(m, 0),
        ))
      },
    }
  }
}

/// Creates a new mapping from old ordinals to new ordinals and returns the total number of vectors
/// in the newly merged segment.
#[allow(clippy::too_many_arguments)]
pub(crate) fn get_new_ord_mapping<KV, R, D>(
  field_info: &FieldInfo,
  init_graph_size: usize,
  merged_vector_values: &mut KV,
  initialized_nodes: &mut FixedBitSet,
  reader: &R,
  init_doc_map: &D,
) -> Result<Vec<usize>>
where
  KV: KnnVectorValues,
  R: KnnVectorsReader,
  D: DocMap,
{
  let mut initializer_iterator = match field_info.get_vector_encoding() {
    VectorEncoding::BYTE(_) => DocIndexIteratorEnum2::A(
      reader
        .get_byte_vector_values(&field_info.name)?
        .iterator()?,
    ),
    VectorEncoding::FLOAT32(_) => DocIndexIteratorEnum2::B(
      reader
        .get_float_vector_values(&field_info.name)?
        .iterator()?,
    ),
  };

  let mut new_id_to_old_ordinal = HashMap::with_capacity(init_graph_size);
  let mut max_new_doc_id = -1;
  let mut doc_id = initializer_iterator.next_doc()?;
  while doc_id != NO_MORE_DOCS {
    let new_id = init_doc_map.get(doc_id)?;
    max_new_doc_id = max_new_doc_id.max(new_id);
    new_id_to_old_ordinal.insert(new_id, initializer_iterator.index()? as usize);
    doc_id = initializer_iterator.next_doc()?;
  }

  if max_new_doc_id == -1 {
    return Ok(Vec::new());
  }

  let mut old_to_new_ordinal_map = vec![0; init_graph_size];
  let mut merged_vector_iterator = merged_vector_values.iterator()?;
  let mut new_doc_id = merged_vector_iterator.next_doc()?;
  while new_doc_id != NO_MORE_DOCS && new_doc_id <= max_new_doc_id {
    if let Some(&old_ordinal) = new_id_to_old_ordinal.get(&new_doc_id) {
      let new_ord = merged_vector_iterator.index()? as usize;
      initialized_nodes.set(new_ord);
      old_to_new_ordinal_map[old_ordinal] = new_ord;
    }
    new_doc_id = merged_vector_iterator.next_doc()?;
  }

  Ok(old_to_new_ordinal_map)
}

impl<S> HnswGraphMerger for IncrementalHnswGraphMerger<S>
where
  S: RandomVectorScorerSupplier,
{
  /// Adds a reader to the graph merger if it meets the following criteria: 1. Does not contain any
  /// deleted docs 2. Is a HnswGraphProvider/PerFieldKnnVectorReader 3. Has the most docs of any
  /// previous reader that met the above criteria
  fn add_reader<R, B>(
    &mut self,
    reader_index: usize,
    reader: &R,
    doc_map_idx: usize,
    live_docs: Option<&B>,
  ) -> Result<()>
  where
    R: KnnVectorsReader,
    B: Bits,
  {
    if !reader.is_hnsw_graph_provider(&self.field_info.name) || !no_deletes(live_docs)? {
      return Ok(());
    }

    let candidate_vector_count = match self.field_info.get_vector_encoding() {
      VectorEncoding::BYTE(_) => reader.get_byte_vector_values(&self.field_info.name)?.size(),
      VectorEncoding::FLOAT32(_) => reader
        .get_float_vector_values(&self.field_info.name)?
        .size(),
    };

    if candidate_vector_count <= self.init_graph_size {
      return Ok(());
    }
    self.init_reader = Some(reader_index);
    self.init_doc_map = Some(doc_map_idx);
    self.init_graph_size = candidate_vector_count;
    Ok(())
  }

  fn merge<KV, R, D>(
    &mut self,
    mut merged_vector_values: KV,
    info_stream: InfoStreamMT,
    max_ord: i32,
    readers: &[Option<R>],
    doc_map: &[D],
  ) -> Result<OnHeapHnswGraph>
  where
    KV: KnnVectorValues,
    R: KnnVectorsReader,
    D: DocMap,
  {
    self.create_builder(
      &mut merged_vector_values,
      max_ord,
      readers,
      doc_map,
      info_stream,
    )
  }
}

fn no_deletes<B>(live_docs: Option<&B>) -> Result<bool>
where
  B: Bits,
{
  let Some(live_docs) = live_docs else {
    return Ok(true);
  };

  for i in 0..live_docs.length() {
    if !live_docs.get(i)? {
      return Ok(false);
    }
  }
  Ok(true)
}
