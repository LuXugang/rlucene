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
use crate::core::index::knn_vector_values::KnnVectorValues;
use crate::core::index::merge_state::DocMap;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::on_heap_hnsw_graph::OnHeapHnswGraph;
use crate::core::util::info_stream::InfoStreamMT;

/// Abstraction of merging multiple graphs into one on-heap graph.
pub trait HnswGraphMerger {
  /// Adds a reader to the graph merger to record the state.
  ///
  /// Readers with deletes are skipped. Among the remaining readers, the merger
  /// keeps the largest graph as the initializer for the final merged graph.
  fn add_reader<R, B>(
    &mut self,
    reader_index: usize,
    reader: &R,
    doc_map_idx: usize,
    live_docs: Option<&B>,
  ) -> Result<()>
  where
    R: KnnVectorsReader,
    B: Bits;

  /// Merge and produce the on heap graph.
  fn merge<KV, R, D>(
    &mut self,
    merged_vector_values: KV,
    info_stream: InfoStreamMT,
    max_ord: i32,
    readers: &[Option<R>],
    doc_map: &[D],
  ) -> Result<OnHeapHnswGraph>
  where
    KV: KnnVectorValues,
    R: KnnVectorsReader,
    D: DocMap;
}
