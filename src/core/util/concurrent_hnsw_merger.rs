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
use crate::core::index::knn_vector_values::KnnVectorValues;
use crate::core::index::merge_state::DocMap;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::hnsw::hnsw_builder::HnswBuilder;
use crate::core::util::hnsw::hnsw_concurrent_merge_builder::HnswConcurrentMergeBuilder;
use crate::core::util::hnsw::initialized_hnsw_graph_builder::init_graph;
use crate::core::util::hnsw::on_heap_hnsw_graph::OnHeapHnswGraph;
use crate::core::util::hnsw::random_vector_scorer_supplier::RandomVectorScorerSupplier;
use crate::core::util::incremental_hnsw_graph_merger::{
  IncrementalHnswGraphMergerBase, get_new_ord_mapping,
};
use crate::core::util::info_stream::InfoStreamMT;

/// This merger merges graphs in a concurrent manner by using [`HnswConcurrentMergeBuilder`].
pub(crate) struct ConcurrentHnswMerger {
  num_workers: usize,
}

impl ConcurrentHnswMerger {
  pub(crate) fn new(num_workers: usize) -> Self {
    Self { num_workers }
  }
}

impl<S> IncrementalHnswGraphMergerBase<S> for ConcurrentHnswMerger
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
    let (hnsw, initialized_nodes) = match init_reader {
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
        let init_doc_map = doc_maps.get(init_doc_map_idx).ok_or_else(|| {
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
          init_doc_map,
        )?;
        (
          init_graph(m, &mut initializer_graph, &old_to_new_ordinal_map, max_ord)?,
          Some(initialized_nodes),
        )
      },
      None => (OnHeapHnswGraph::new(m, max_ord), None),
    };

    let mut builder = HnswConcurrentMergeBuilder::new(
      self.num_workers,
      scorer_supplier,
      m,
      beam_width,
      hnsw,
      initialized_nodes,
    )?;
    builder.set_info_stream(info_stream);
    Ok(std::mem::replace(
      builder.build(max_ord as usize)?,
      OnHeapHnswGraph::new(m, 0),
    ))
  }
}
