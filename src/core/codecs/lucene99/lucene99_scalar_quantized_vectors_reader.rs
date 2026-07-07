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
use crate::core::codecs::hnsw::flat_vectors_reader::FlatVectorsReader;
use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
use crate::core::codecs::hnsw::hnsw_graph_provider::HnswGraphProvider;
use crate::core::codecs::knn_vectors_reader::KnnVectorsReader;
use crate::core::index::dummy::dummy_byte_vector_values::DummyByteVectorValues;
use crate::core::index::dummy::dummy_float_vector_values::DummyFloatVectorValues;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::store::IndexInput;
use crate::core::util::accountable::Accountable;
use crate::core::util::bits::Bits;
use crate::core::util::close::Closeable;
use crate::core::util::dummy::dummy_hnsw_graph::DummyHnswGraph;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::dummy::dummy_random_vector_scorer::DummyRandomVectorScorer;
use std::marker::PhantomData;

/// Reads scalar quantized vectors from the index segments.
pub struct Lucene99ScalarQuantizedVectorsReader<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  flat_vector_scorer: F,
  _input: PhantomData<I>,
}

impl<I, F> Lucene99ScalarQuantizedVectorsReader<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  #[allow(dead_code)]
  pub(crate) fn new(flat_vector_scorer: F) -> Self {
    Self {
      flat_vector_scorer,
      _input: PhantomData,
    }
  }
}

impl<I, F> Closeable for Lucene99ScalarQuantizedVectorsReader<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
}

impl<I, F> HnswGraphProvider for Lucene99ScalarQuantizedVectorsReader<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  type HnswGraph = DummyHnswGraph;
}

impl<I, F> KnnVectorsReader for Lucene99ScalarQuantizedVectorsReader<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  fn check_integrity(&self) -> Result<()> {
    todo!("Lucene99ScalarQuantizedVectorsReader is not implemented yet")
  }

  type FloatVectorValues = DummyFloatVectorValues;

  fn get_float_vector_values(&self, _field: &str) -> Result<Self::FloatVectorValues> {
    todo!("Lucene99ScalarQuantizedVectorsReader is not implemented yet")
  }

  type ByteVectorValues = DummyByteVectorValues;

  fn get_byte_vector_values(&self, _field: &str) -> Result<Self::ByteVectorValues> {
    todo!("Lucene99ScalarQuantizedVectorsReader is not implemented yet")
  }

  fn search_f32<B, K>(
    &self,
    _field: &str,
    _target: Vec<f32>,
    _knn_collector: &mut K,
    _accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    todo!("Lucene99ScalarQuantizedVectorsReader is not implemented yet")
  }

  fn search_u8<B, K>(
    &self,
    _field: &str,
    _target: Vec<u8>,
    _knn_collector: &mut K,
    _accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    todo!("Lucene99ScalarQuantizedVectorsReader is not implemented yet")
  }
}

impl<I, F> Accountable for Lucene99ScalarQuantizedVectorsReader<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    todo!("Lucene99ScalarQuantizedVectorsReader is not implemented yet")
  }
}

impl<I, F> FlatVectorsReader for Lucene99ScalarQuantizedVectorsReader<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  type FlatVectorsScorer = F;

  fn get_flat_vector_scorer(&self) -> &Self::FlatVectorsScorer {
    &self.flat_vector_scorer
  }

  type RandomVectorScorerF32 = DummyRandomVectorScorer;

  fn get_random_vector_scorer_f32(
    &self,
    _field: &str,
    _target: Vec<f32>,
  ) -> Result<Self::RandomVectorScorerF32> {
    todo!("Lucene99ScalarQuantizedVectorsReader is not implemented yet")
  }

  type RandomVectorScorerU8 = DummyRandomVectorScorer;

  fn get_random_vector_scorer_u8(
    &self,
    _field: &str,
    _target: Vec<u8>,
  ) -> Result<Self::RandomVectorScorerU8> {
    todo!("Lucene99ScalarQuantizedVectorsReader is not implemented yet")
  }
}
