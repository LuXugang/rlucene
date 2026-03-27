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
use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
use crate::core::codecs::indexed_disi::{
  DocIndexIteratorImpl, IndexedDISI, get_doc_index_iterator,
};
use crate::core::codecs::lucene95::has_index_slice::HasIndexSlice;
use crate::core::codecs::lucene95::ord_to_doc_disi_reader_configuration::OrdToDocDISIReaderConfiguration;
use crate::core::index::dummy::dummy_float_vector_values::DummyFloatVectorValues;
use crate::core::index::dummy::dummy_knn_vector_values::DummyKnnVectorsWriter;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::index_reader::Identity;
use crate::core::index::knn_vector_values::{
  DenseDocIndexIterator, DocIndexIterator, KnnVectorValues, create_dense_iterator,
};
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::dummy::dummy_vector_scorer::DummyVectorScorer;
use crate::core::search::vector_scorer::VectorScorer;
use crate::core::store::IndexInput;
use crate::core::store::dummy::dummy_index_input::DummyIndexInput;
use crate::core::store::random_access_input::RandomAccessInput;
use crate::core::util::bits::Bits;
use crate::core::util::dummy::dummy_bits::DummyBits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::hnsw::random_vector_scorer::RandomVectorScorer;
use crate::core::util::long_values::LongValues;
use crate::core::util::packed::direct_monotonic_reader::DirectMonotonicReader;
use crate::core::util::{HasIdentity, TryIntoInt};
use std::sync::Arc;

/// Read the vector values from the index input. This supports both iterated and random access.
struct OffHeapFloatVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  pub(crate) dimension: usize,
  pub(crate) size: usize,
  pub(crate) slice: I,
  pub(crate) byte_size: usize,
  pub(crate) last_ord: Option<usize>,
  pub(crate) value: Vec<f32>,
  pub(crate) similarity_function: VectorSimilarityFunction,
  pub(crate) flat_vectors_scorer: F,
}
impl<I, F> OffHeapFloatVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  fn new(
    dimension: usize,
    size: usize,
    slice: I,
    byte_size: usize,
    flat_vectors_scorer: F,
    similarity_function: VectorSimilarityFunction,
  ) -> Self {
    Self {
      dimension,
      size,
      slice,
      byte_size,
      last_ord: None,
      value: vec![0.0; dimension],
      similarity_function,
      flat_vectors_scorer,
    }
  }
}
impl<I, F> HasIndexSlice for OffHeapFloatVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  type Input = I;

  fn get_slice(&mut self) -> Option<&mut Self::Input> {
    Some(&mut self.slice)
  }
}

impl<I, F> KnnVectorValues for OffHeapFloatVectorValues<I, F>
where
  F: FlatVectorsScorer,
  I: IndexInput,
{
  fn dimension(&self) -> usize {
    self.dimension
  }

  fn size(&self) -> usize {
    self.size
  }

  type KnnVectorValues = DummyKnnVectorsWriter;

  fn get_encoding(&self) -> VectorEncoding {
    FloatVectorValues::get_encoding(self)
  }

  type Bits<B>
    = DummyBits
  where
    B: Bits;

  fn get_accept_ords<B>(&self, _accept_docs: Option<B>) -> Option<Self::Bits<B>>
  where
    B: Bits,
  {
    debug_assert!(
      false,
      "should nerver call get_accept_ords on OffHeapFloatVectorValues, should be called on DenseOffHeapVectorValues or SparseOffHeapVectorValues"
    );
    None
  }

  type DocIndexIterator = DenseDocIndexIterator;
}

impl<I, F> FloatVectorValues for OffHeapFloatVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  fn vector_value(&mut self, target_ord: usize) -> Result<&[f32]> {
    let same_ord = match self.last_ord {
      Some(last_ord) => last_ord == target_ord,
      None => false,
    };
    if same_ord {
      return Ok(self.value.as_slice());
    }

    let pos = (target_ord)
      .checked_mul(self.byte_size)
      .ok_or_else(|| LuceneError::illegal_state("seek overflow"))?;

    self.slice.seek(pos)?;
    let len = self.value.len();
    self.slice.read_floats(&mut self.value, 0, len)?;

    self.last_ord = Some(target_ord);

    Ok(self.value.as_slice())
  }

  type FloatVectorValues = DummyFloatVectorValues;
  type VectorScorer = DummyVectorScorer;
}

pub struct DenseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer + Clone,
{
  base: OffHeapFloatVectorValues<I, F>,
  #[cfg(debug_assertions)]
  iter_called: bool,
}

impl<I, F> DenseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer + Clone,
{
  pub fn new(
    dimension: usize,
    size: usize,
    slice: I,
    byte_size: usize,
    flat_vectors_scorer: F,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<Self> {
    let base = OffHeapFloatVectorValues::new(
      dimension,
      size,
      slice,
      byte_size,
      flat_vectors_scorer,
      similarity_function,
    );
    Ok(Self {
      base,
      iter_called: false,
    })
  }
  pub fn ord_to_doc(&self, ord: i32) -> i32 {
    ord
  }
}

impl<I, F> KnnVectorValues for DenseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer + Clone,
{
  fn dimension(&self) -> usize {
    self.base.dimension
  }

  fn size(&self) -> usize {
    self.base.size
  }

  type KnnVectorValues = Self;

  fn get_encoding(&self) -> VectorEncoding {
    FloatVectorValues::get_encoding(self)
  }

  type Bits<B>
    = B
  where
    B: Bits;

  fn get_accept_ords<B>(&self, accept_docs: Option<B>) -> Option<Self::Bits<B>>
  where
    B: Bits,
  {
    accept_docs
  }

  type DocIndexIterator = DenseDocIndexIterator;

  fn iterator(&mut self) -> Result<Self::DocIndexIterator> {
    #[cfg(debug_assertions)]
    if self.iter_called {
      unreachable!("iterator should only be called once, otherwise iter will be reset?")
    } else {
      self.iter_called = true;
    }
    Ok(create_dense_iterator(self.size() as i32))
  }
}
impl<I, F> HasIndexSlice for DenseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer + Clone,
{
  type Input = I;

  fn get_slice(&mut self) -> Option<&mut Self::Input> {
    self.base.get_slice()
  }
}

impl<I, F> FloatVectorValues for DenseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer + Clone,
{
  fn vector_value(&mut self, ord: usize) -> Result<&[f32]> {
    self.base.vector_value(ord)
  }

  type FloatVectorValues = Self;

  fn float_copy(&self) -> Result<Self::FloatVectorValues> {
    Self::new(
      self.base.dimension,
      self.base.size,
      self.base.slice.try_clone()?,
      self.base.byte_size,
      self.base.flat_vectors_scorer.clone(),
      self.base.similarity_function,
    )
  }

  type VectorScorer = DenseVectorScorer<
    <F as FlatVectorsScorer>::RandomVectorScorerF32<DenseOffHeapVectorValues<I, F>>,
  >;

  fn scorer(&self, query: Vec<f32>) -> Result<Option<Self::VectorScorer>> {
    let mut copy = self.float_copy()?;
    let iterator = copy.iterator()?;

    let sf = copy.base.similarity_function;
    let random_vector_scorer = self
      .base
      .flat_vectors_scorer
      .get_random_vector_scorer_f32(sf, copy, query)?;
    Ok(Some(DenseVectorScorer::new(iterator, random_vector_scorer)))
  }
}

pub struct DenseVectorScorer<R>
where
  R: RandomVectorScorer,
{
  iterator: DenseDocIndexIterator,
  random_vector_scorer: R,
}
impl<R> DenseVectorScorer<R>
where
  R: RandomVectorScorer,
{
  fn new(iterator: DenseDocIndexIterator, random_vector_scorer: R) -> Self {
    Self {
      iterator,
      random_vector_scorer,
    }
  }
}
impl<R> VectorScorer for DenseVectorScorer<R>
where
  R: RandomVectorScorer,
{
  fn score(&mut self) -> Result<f32> {
    let doc_id = self.iterator.doc_id().try_convert()?;
    self.random_vector_scorer.score(doc_id)
  }

  type DocIdSetIterator = DenseDocIndexIterator;

  fn iterator(&self) -> &Self::DocIdSetIterator {
    &self.iterator
  }

  fn iterator_mut(&mut self) -> &mut Self::DocIdSetIterator {
    &mut self.iterator
  }
}

pub struct SparseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer + Clone,
{
  base: OffHeapFloatVectorValues<I, F>,
  ord_to_doc: Arc<DirectMonotonicReader<I::RandomAccessSlice>>,
  data_in: Arc<I>,
  configuration: OrdToDocDISIReaderConfiguration,
  disi: Option<DocIndexIteratorImpl<I>>,
}

impl<I, F> SparseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer + Clone,
{
  pub fn new<T>(
    configuration: OrdToDocDISIReaderConfiguration,
    data_in: T,
    slice: I,
    dimension: usize,
    byte_size: usize,
    flat_vectors_scorer: F,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<Self>
  where
    T: Into<Arc<I>>,
  {
    let base = OffHeapFloatVectorValues::new(
      dimension,
      configuration.size as usize,
      slice,
      byte_size,
      flat_vectors_scorer,
      similarity_function,
    );
    let data_in = data_in.into();
    let addresses_data = data_in.random_access_slice(
      configuration.addresses_offset,
      configuration.addresses_length,
    )?;

    let ord_to_doc = match configuration.meta {
      Some(ref meta) => DirectMonotonicReader::get_instance(meta, addresses_data)?,
      None => return Err(LuceneError::illegal_state("meta is None")),
    };

    let disi = IndexedDISI::new(
      data_in.as_ref(),
      configuration.docs_with_field_offset.try_convert()?,
      configuration.docs_with_field_length,
      configuration.jump_table_entry_count as i32,
      configuration.dense_rank_power,
      configuration.size as i64,
    )?;
    let disi = Some(get_doc_index_iterator(disi));

    Ok(Self {
      base,
      ord_to_doc: Arc::new(ord_to_doc),
      data_in,
      configuration,
      disi,
    })
  }
}
impl<I, F> HasIndexSlice for SparseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer + Clone,
{
  type Input = I;

  fn get_slice(&mut self) -> Option<&mut Self::Input> {
    self.base.get_slice()
  }
}
impl<I, F> KnnVectorValues for SparseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer + Clone,
{
  fn dimension(&self) -> usize {
    self.base.dimension
  }

  fn size(&self) -> usize {
    self.base.size
  }

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    Ok(self.ord_to_doc.get(ord)? as usize)
  }

  type KnnVectorValues = Self;

  fn get_encoding(&self) -> VectorEncoding {
    FloatVectorValues::get_encoding(self)
  }

  type Bits<B>
    = SparseBits<B, I::RandomAccessSlice>
  where
    B: Bits;

  fn get_accept_ords<B>(&self, accept_docs: Option<B>) -> Option<Self::Bits<B>>
  where
    B: Bits,
  {
    accept_docs.map(|bits| SparseBits::new(bits, self.base.size, self.ord_to_doc.clone()))
  }

  type DocIndexIterator = DocIndexIteratorImpl<I>;

  fn iterator(&mut self) -> Result<Self::DocIndexIterator> {
    match self.disi.take() {
      Some(disi) => Ok(disi),
      None => Err(LuceneError::illegal_state("iterator only called once")),
    }
  }
}

impl<I, F> FloatVectorValues for SparseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer + Clone,
{
  fn vector_value(&mut self, ord: usize) -> Result<&[f32]> {
    self.base.vector_value(ord)
  }

  type FloatVectorValues = Self;

  fn float_copy(&self) -> Result<Self::FloatVectorValues> {
    Self::new(
      self.configuration.clone(),
      self.data_in.clone(),
      self.base.slice.try_clone()?,
      self.base.dimension,
      self.base.byte_size,
      self.base.flat_vectors_scorer.clone(),
      self.base.similarity_function,
    )
  }

  type VectorScorer = SparseVectorScorer<
    I,
    <F as FlatVectorsScorer>::RandomVectorScorerF32<SparseOffHeapVectorValues<I, F>>,
  >;

  fn scorer(&self, query: Vec<f32>) -> Result<Option<Self::VectorScorer>> {
    let mut copy = self.float_copy()?;
    let iterator = copy.iterator()?;

    let sf = copy.base.similarity_function;

    let random_vector_scorer = self
      .base
      .flat_vectors_scorer
      .get_random_vector_scorer_f32(sf, copy, query)?;

    Ok(Some(SparseVectorScorer::new(
      iterator,
      random_vector_scorer,
    )))
  }
}
pub struct SparseVectorScorer<I, R>
where
  I: IndexInput,
  R: RandomVectorScorer,
{
  iterator: DocIndexIteratorImpl<I>,
  random_vector_scorer: R,
}
impl<I, R> SparseVectorScorer<I, R>
where
  I: IndexInput,
  R: RandomVectorScorer,
{
  fn new(iterator: DocIndexIteratorImpl<I>, random_vector_scorer: R) -> Self {
    Self {
      iterator,
      random_vector_scorer,
    }
  }
}
impl<I, R> VectorScorer for SparseVectorScorer<I, R>
where
  I: IndexInput,
  R: RandomVectorScorer,
{
  fn score(&mut self) -> Result<f32> {
    let index = self.iterator.index()?;
    self.random_vector_scorer.score(index as usize)
  }

  type DocIdSetIterator = DocIndexIteratorImpl<I>;

  fn iterator(&self) -> &Self::DocIdSetIterator {
    &self.iterator
  }

  fn iterator_mut(&mut self) -> &mut Self::DocIdSetIterator {
    &mut self.iterator
  }
}
pub struct SparseBits<B, R>
where
  B: Bits,
  R: RandomAccessInput,
{
  accept_docs: B,
  size: usize,
  map: Arc<DirectMonotonicReader<R>>,
  id: Identity,
}
impl<B, R> SparseBits<B, R>
where
  B: Bits,
  R: RandomAccessInput,
{
  fn new(accept_docs: B, size: usize, map: Arc<DirectMonotonicReader<R>>) -> Self {
    Self {
      accept_docs,
      size,
      map,
      id: Identity::new(),
    }
  }
}

impl<B, R> HasIdentity for SparseBits<B, R>
where
  B: Bits,
  R: RandomAccessInput,
{
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl<B, R> Bits for SparseBits<B, R>
where
  B: Bits,
  R: RandomAccessInput,
{
  fn get(&self, index: usize) -> Result<bool> {
    let index = self.map.get(index)? as usize;
    self.accept_docs.get(index)
  }

  fn length(&self) -> usize {
    self.size
  }
}

pub struct EmptyOffHeapVectorValues {
  dimension: usize,
  vectors: Vec<f32>,
  #[cfg(debug_assertions)]
  iter_called: bool,
}
impl EmptyOffHeapVectorValues {
  fn new(dimension: usize) -> Self {
    let vectors = Vec::new();

    Self {
      dimension,
      vectors,
      iter_called: false,
    }
  }
}
impl HasIndexSlice for EmptyOffHeapVectorValues {
  type Input = DummyIndexInput;

  fn get_slice(&mut self) -> Option<&mut Self::Input> {
    None
  }
}
impl KnnVectorValues for EmptyOffHeapVectorValues {
  fn dimension(&self) -> usize {
    self.dimension
  }

  fn size(&self) -> usize {
    0
  }

  type KnnVectorValues = DummyKnnVectorsWriter;

  fn get_encoding(&self) -> VectorEncoding {
    FloatVectorValues::get_encoding(self)
  }

  type Bits<B>
    = DummyBits
  where
    B: Bits;

  fn get_accept_ords<B>(&self, _accept_docs: Option<B>) -> Option<Self::Bits<B>>
  where
    B: Bits,
  {
    None
  }

  type DocIndexIterator = DenseDocIndexIterator;

  fn iterator(&mut self) -> Result<Self::DocIndexIterator> {
    #[cfg(debug_assertions)]
    if self.iter_called {
      unreachable!("iterator should only be called once, otherwise iter will be reset?")
    } else {
      self.iter_called = true;
    }
    Ok(create_dense_iterator(0))
  }
}

impl FloatVectorValues for EmptyOffHeapVectorValues {
  fn vector_value(&mut self, _ord: usize) -> Result<&[f32]> {
    debug_assert!(self.vectors.is_empty());
    Ok(self.vectors.as_slice())
  }

  type FloatVectorValues = DummyFloatVectorValues;

  fn float_copy(&self) -> Result<Self::FloatVectorValues> {
    Err(LuceneError::unsupported_operation(""))
  }

  type VectorScorer = DummyVectorScorer;

  fn scorer(&self, _target: Vec<f32>) -> Result<Option<Self::VectorScorer>> {
    Ok(None)
  }
}
