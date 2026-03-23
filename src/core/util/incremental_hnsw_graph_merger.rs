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
use crate::core::codecs::knn_vectors_reader::KnnVectorsReaderEnum;
use crate::core::index::knn_vector_values::KnnVectorValues;
use crate::core::index::merge_state::DocMap;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::hnsw_graph_merger::HnswGraphMerger;
use crate::core::util::hnsw::on_heap_hnsw_graph::OnHeapHnswGraph;
use crate::core::util::info_stream::InfoStreamMT;

pub struct IncrementalHnswGraphMerger;
impl HnswGraphMerger for IncrementalHnswGraphMerger {
  fn add_reader<D, B>(
    &mut self,
    _reader: KnnVectorsReaderEnum,
    _doc_map: D,
    _live_docs: Option<B>,
  ) -> Result<()>
  where
    D: DocMap,
    B: Bits,
  {
    todo!()
  }

  fn merge<KV, IS>(
    &mut self,
    _merged_vector_values: KV,
    _info_stream: Option<InfoStreamMT>,
    _max_ord: i32,
  ) -> Result<OnHeapHnswGraph>
  where
    KV: KnnVectorValues,
  {
    todo!()
  }
}
