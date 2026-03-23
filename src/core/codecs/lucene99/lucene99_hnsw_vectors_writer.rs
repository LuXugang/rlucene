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
use crate::core::index::byte_vector_values::from_bytes;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::float_vector_values::from_floats;
use crate::core::util::bit_set::BitSet;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::hnsw::hnsw_builder::HnswBuilder;
use crate::core::util::hnsw::hnsw_graph_builder::{HnswGraphBuilder, RAND_SEED, create};
use crate::core::util::hnsw::hnsw_graph_searcher::{
  HnswGraphSearcherBase, HnswGraphSearcherBaseDefault,
};
use crate::core::util::hnsw::random_vector_scorer_supplier::RandomVectorScorerSupplier;
use crate::core::util::info_stream::InfoStreamMT;
use std::sync::Arc;

//TODO: memory calculation not implement
const SHALLOW_RAM_BYTES_USED: i64 = 0;
pub struct Lucene99HnswVectorsWriter<F>
where
  F: FlatVectorsWriter,
{
  m: usize,
  beam_width: usize,
  flat_vector_writer: F,
  num_merge_workers: usize,
  // TODO IMPORTANT 多线程未实现
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
