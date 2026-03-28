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
use crate::core::index::byte_vector_values::ByteVectorValues;
use crate::core::index::dummy::dummy_byte_vector_values::DummyByteVectorValues;
use crate::core::index::dummy::dummy_knn_vector_values::DummyKnnVectorsWriter;
use crate::core::index::index_reader::Identity;
use crate::core::index::knn_vector_values::{
  DenseDocIndexIterator, DocIndexIterator, KnnVectorValues, create_dense_iterator,
};
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum2};
use crate::core::search::dummy::dummy_vector_scorer::DummyVectorScorer;
use crate::core::search::vector_scorer::VectorScorer;
use crate::core::store::IndexInput;
use crate::core::store::dummy::dummy_index_input::DummyIndexInput;
use crate::core::store::random_access_input::RandomAccessInput;
use crate::core::util::bits::Bits;
use crate::core::util::clone::TryClone;
use crate::core::util::dummy::dummy_bits::DummyBits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::hnsw::random_vector_scorer::RandomVectorScorer;
use crate::core::util::long_values::LongValues;
use crate::core::util::packed::direct_monotonic_reader::DirectMonotonicReader;
use crate::core::util::{HasIdentity, TryIntoInt};
use std::sync::Arc;

struct OffHeapByteVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  pub(crate) dimension: usize,
  pub(crate) size: usize,
  pub(crate) slice: I,
  pub(crate) last_ord: Option<usize>,
  pub(crate) binary_value: Vec<u8>,
  pub(crate) byte_size: usize,
  pub(crate) similarity_function: VectorSimilarityFunction,
  pub(crate) flat_vectors_scorer: F,
}

impl<I, F> OffHeapByteVectorValues<I, F>
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
      last_ord: None,
      binary_value: vec![0; byte_size],
      byte_size,
      similarity_function,
      flat_vectors_scorer,
    }
  }

  fn read_value(&mut self, target_ord: usize) -> Result<()> {
    let pos = target_ord
      .checked_mul(self.byte_size)
      .ok_or_else(|| LuceneError::illegal_state("seek overflow"))?;
    self.slice.seek(pos)?;
    self
      .slice
      .read_bytes(&mut self.binary_value, 0, self.byte_size)?;
    Ok(())
  }
}

impl<I, F> HasIndexSlice for OffHeapByteVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  type Input = I;

  fn get_slice(&mut self) -> Option<&mut Self::Input> {
    Some(&mut self.slice)
  }
}

impl<I, F> KnnVectorValues for OffHeapByteVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  fn dimension(&self) -> usize {
    self.dimension
  }

  fn size(&self) -> usize {
    self.size
  }

  type KnnVectorValues = DummyKnnVectorsWriter;

  fn get_encoding(&self) -> VectorEncoding {
    ByteVectorValues::get_encoding(self)
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
      "should never call get_accept_ords on OffHeapByteVectorValues, should be called on DenseOffHeapVectorValues or SparseOffHeapVectorValues"
    );
    None
  }

  type DocIndexIterator = DenseDocIndexIterator;
}

impl<I, F> ByteVectorValues for OffHeapByteVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  fn vector_value(&mut self, target_ord: usize) -> Result<&[u8]> {
    let same_ord = matches!(self.last_ord, Some(last_ord) if last_ord == target_ord);
    if !same_ord {
      self.read_value(target_ord)?;
      self.last_ord = Some(target_ord);
    }
    Ok(self.binary_value.as_slice())
  }

  type ByteVectorValues = DummyByteVectorValues;
  type VectorScorer = DummyVectorScorer;
}

pub struct DenseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  base: OffHeapByteVectorValues<I, F>,
  #[cfg(debug_assertions)]
  iter_called: bool,
}

impl<I, F> DenseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  pub fn new(
    dimension: usize,
    size: usize,
    slice: I,
    byte_size: usize,
    flat_vectors_scorer: F,
    similarity_function: VectorSimilarityFunction,
  ) -> Self {
    Self {
      base: OffHeapByteVectorValues::new(
        dimension,
        size,
        slice,
        byte_size,
        flat_vectors_scorer,
        similarity_function,
      ),
      iter_called: false,
    }
  }
}

impl<I, F> HasIndexSlice for DenseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer,
{
  type Input = I;

  fn get_slice(&mut self) -> Option<&mut Self::Input> {
    self.base.get_slice()
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

  type KnnVectorValues = DummyKnnVectorsWriter;

  fn get_encoding(&self) -> VectorEncoding {
    ByteVectorValues::get_encoding(self)
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

impl<I, F> ByteVectorValues for DenseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer + Clone,
{
  fn vector_value(&mut self, ord: usize) -> Result<&[u8]> {
    self.base.vector_value(ord)
  }

  type ByteVectorValues = Self;

  fn byte_copy(&self) -> Result<Self::ByteVectorValues> {
    Ok(Self::new(
      self.base.dimension,
      self.base.size,
      self.base.slice.try_clone()?,
      self.base.byte_size,
      self.base.flat_vectors_scorer.clone(),
      self.base.similarity_function,
    ))
  }

  type VectorScorer = DenseVectorScorer<
    <F as FlatVectorsScorer>::RandomVectorScorerU8<DenseOffHeapVectorValues<I, F>>,
  >;

  fn scorer(&self, query: Vec<u8>) -> Result<Self::VectorScorer> {
    let mut copy = self.byte_copy()?;
    let iterator = copy.iterator()?;
    let sf = copy.base.similarity_function;
    let random_vector_scorer = self
      .base
      .flat_vectors_scorer
      .get_random_vector_scorer_u8(sf, copy, query)?;
    Ok(DenseVectorScorer::new(iterator, random_vector_scorer))
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
  F: FlatVectorsScorer,
  I: Clone,
{
  base: OffHeapByteVectorValues<I::IndexInput, F>,
  ord_to_doc: Arc<DirectMonotonicReader<I::RandomAccessSlice>>,
  data_in: I,
  configuration: Arc<OrdToDocDISIReaderConfiguration>,
  disi: Option<DocIndexIteratorImpl<I>>,
}

impl<I, F> SparseOffHeapVectorValues<I, F>
where
  I: IndexInput + Clone,
  F: FlatVectorsScorer,
{
  pub fn new(
    configuration: Arc<OrdToDocDISIReaderConfiguration>,
    data_in: I,
    slice: I::IndexInput,
    dimension: usize,
    byte_size: usize,
    flat_vectors_scorer: F,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<Self> {
    let base = OffHeapByteVectorValues::new(
      dimension,
      configuration.size as usize,
      slice,
      byte_size,
      flat_vectors_scorer,
      similarity_function,
    );
    let addresses_data = data_in.random_access_slice(
      configuration.addresses_offset,
      configuration.addresses_length,
    )?;

    let ord_to_doc = match configuration.meta {
      Some(ref meta) => DirectMonotonicReader::get_instance(meta, addresses_data)?,
      None => return Err(LuceneError::illegal_state("meta is None")),
    };

    let disi = IndexedDISI::new(
      &data_in,
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
  I: IndexInput + Clone,
  F: FlatVectorsScorer,
{
  type Input = I::IndexInput;

  fn get_slice(&mut self) -> Option<&mut Self::Input> {
    self.base.get_slice()
  }
}

impl<I, F> KnnVectorValues for SparseOffHeapVectorValues<I, F>
where
  I: IndexInput + Clone,
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

  type KnnVectorValues = DummyKnnVectorsWriter;

  fn get_encoding(&self) -> VectorEncoding {
    ByteVectorValues::get_encoding(self)
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

impl<I, F> ByteVectorValues for SparseOffHeapVectorValues<I, F>
where
  I: IndexInput + Clone,
  F: FlatVectorsScorer + Clone,
{
  fn vector_value(&mut self, ord: usize) -> Result<&[u8]> {
    self.base.vector_value(ord)
  }

  type ByteVectorValues = Self;

  fn byte_copy(&self) -> Result<Self::ByteVectorValues> {
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
    <F as FlatVectorsScorer>::RandomVectorScorerU8<SparseOffHeapVectorValues<I, F>>,
  >;

  fn scorer(&self, query: Vec<u8>) -> Result<Self::VectorScorer> {
    let mut copy = self.byte_copy()?;
    let iterator = copy.iterator()?;
    let sf = copy.base.similarity_function;
    let random_vector_scorer = self
      .base
      .flat_vectors_scorer
      .get_random_vector_scorer_u8(sf, copy, query)?;
    Ok(SparseVectorScorer::new(iterator, random_vector_scorer))
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
  binary_value: Vec<u8>,
  #[cfg(debug_assertions)]
  iter_called: bool,
}

impl EmptyOffHeapVectorValues {
  fn new(dimension: usize) -> Self {
    Self {
      dimension,
      binary_value: Vec::new(),
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
    ByteVectorValues::get_encoding(self)
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

impl ByteVectorValues for EmptyOffHeapVectorValues {
  fn vector_value(&mut self, _ord: usize) -> Result<&[u8]> {
    Err(LuceneError::unsupported_operation(""))
  }

  type ByteVectorValues = Self;

  fn byte_copy(&self) -> Result<Self::ByteVectorValues> {
    Err(LuceneError::unsupported_operation(""))
  }

  type VectorScorer = DummyVectorScorer;

  fn scorer(&self, _query: Vec<u8>) -> Result<Self::VectorScorer> {
    Err(LuceneError::unsupported_operation(""))
  }
}

pub enum OffHeapByteVectorValuesEnum<I, F>
where
  I: IndexInput + Clone,
  F: FlatVectorsScorer,
{
  Empty(EmptyOffHeapVectorValues),
  Dense(DenseOffHeapVectorValues<I::IndexInput, F>),
  Sparse(SparseOffHeapVectorValues<I, F>),
}

impl<I, F> KnnVectorValues for OffHeapByteVectorValuesEnum<I, F>
where
  I: IndexInput + Clone,
  F: FlatVectorsScorer + Clone,
{
  fn dimension(&self) -> usize {
    match self {
      Self::Empty(e) => e.dimension(),
      Self::Dense(e) => e.dimension(),
      Self::Sparse(e) => e.dimension(),
    }
  }

  fn size(&self) -> usize {
    match self {
      Self::Empty(e) => e.size(),
      Self::Dense(e) => e.size(),
      Self::Sparse(e) => e.size(),
    }
  }

  type KnnVectorValues = DummyKnnVectorsWriter;

  fn get_encoding(&self) -> VectorEncoding {
    match self {
      Self::Empty(e) => ByteVectorValues::get_encoding(e),
      Self::Dense(e) => ByteVectorValues::get_encoding(e),
      Self::Sparse(e) => ByteVectorValues::get_encoding(e),
    }
  }

  type Bits<B>
    = OffHeapByteVectorValueBitsEnum<I::RandomAccessSlice, B>
  where
    B: Bits;

  fn get_accept_ords<B>(&self, accept_docs: Option<B>) -> Option<Self::Bits<B>>
  where
    B: Bits,
  {
    match self {
      Self::Empty(_) => None,
      Self::Dense(e) => e
        .get_accept_ords(accept_docs)
        .map(OffHeapByteVectorValueBitsEnum::Dense),
      Self::Sparse(e) => e
        .get_accept_ords(accept_docs)
        .map(OffHeapByteVectorValueBitsEnum::Sparse),
    }
  }

  type DocIndexIterator = IterEnum<I>;

  fn iterator(&mut self) -> Result<Self::DocIndexIterator> {
    match self {
      Self::Empty(e) => e.iterator().map(IterEnum::Empty),
      Self::Dense(e) => e.iterator().map(IterEnum::Dense),
      Self::Sparse(e) => e.iterator().map(IterEnum::Sparse),
    }
  }
}

impl<I, F> ByteVectorValues for OffHeapByteVectorValuesEnum<I, F>
where
  I: IndexInput + Clone,
  F: FlatVectorsScorer + Clone,
{
  fn vector_value(&mut self, ord: usize) -> Result<&[u8]> {
    match self {
      Self::Empty(e) => e.vector_value(ord),
      Self::Dense(e) => e.vector_value(ord),
      Self::Sparse(e) => e.vector_value(ord),
    }
  }

  type ByteVectorValues = Self;

  fn byte_copy(&self) -> Result<Self::ByteVectorValues> {
    match self {
      Self::Empty(e) => e.byte_copy().map(Self::Empty),
      Self::Dense(e) => e.byte_copy().map(Self::Dense),
      Self::Sparse(e) => e.byte_copy().map(Self::Sparse),
    }
  }

  type VectorScorer = VectorScorerEnum<
    I,
    <F as FlatVectorsScorer>::RandomVectorScorerU8<DenseOffHeapVectorValues<I::IndexInput, F>>,
    <F as FlatVectorsScorer>::RandomVectorScorerU8<SparseOffHeapVectorValues<I, F>>,
  >;

  fn scorer(&self, target: Vec<u8>) -> Result<Self::VectorScorer> {
    match self {
      Self::Empty(_) => Err(LuceneError::unsupported_operation("")),
      Self::Dense(e) => e
        .scorer(target)
        .map(|scorer| VectorScorerEnum::new_dense(scorer.iterator, scorer.random_vector_scorer)),
      Self::Sparse(e) => e
        .scorer(target)
        .map(|scorer| VectorScorerEnum::new_sparse(scorer.iterator, scorer.random_vector_scorer)),
    }
  }

  fn get_encoding(&self) -> VectorEncoding {
    match self {
      Self::Empty(e) => ByteVectorValues::get_encoding(e),
      Self::Dense(e) => ByteVectorValues::get_encoding(e),
      Self::Sparse(e) => ByteVectorValues::get_encoding(e),
    }
  }
}

pub enum OffHeapByteVectorValueBitsEnum<R, B>
where
  R: RandomAccessInput,
  B: Bits,
{
  Dense(B),
  Sparse(SparseBits<B, R>),
}

impl<R, B> HasIdentity for OffHeapByteVectorValueBitsEnum<R, B>
where
  R: RandomAccessInput,
  B: Bits,
{
  fn identity(&self) -> &Identity {
    match self {
      Self::Dense(e) => e.identity(),
      Self::Sparse(e) => e.identity(),
    }
  }
}

impl<R, B> Bits for OffHeapByteVectorValueBitsEnum<R, B>
where
  R: RandomAccessInput,
  B: Bits,
{
  fn get(&self, index: usize) -> Result<bool> {
    match self {
      Self::Dense(e) => e.get(index),
      Self::Sparse(e) => e.get(index),
    }
  }

  fn length(&self) -> usize {
    match self {
      Self::Dense(e) => e.length(),
      Self::Sparse(e) => e.length(),
    }
  }

  fn copy_of(&self) -> Result<FixedBitSet> {
    match self {
      Self::Dense(e) => e.copy_of(),
      Self::Sparse(e) => e.copy_of(),
    }
  }

  fn as_string(&self) -> String {
    match self {
      Self::Dense(e) => e.as_string(),
      Self::Sparse(e) => e.as_string(),
    }
  }
}

pub enum IterEnum<I>
where
  I: IndexInput,
{
  Empty(DenseDocIndexIterator),
  Dense(DenseDocIndexIterator),
  Sparse(DocIndexIteratorImpl<I>),
}

impl<I> DocIdSetIterator for IterEnum<I>
where
  I: IndexInput,
{
  fn doc_id(&self) -> i32 {
    match self {
      Self::Empty(e) => e.doc_id(),
      Self::Dense(e) => e.doc_id(),
      Self::Sparse(e) => e.doc_id(),
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    match self {
      Self::Empty(e) => e.next_doc(),
      Self::Dense(e) => e.next_doc(),
      Self::Sparse(e) => e.next_doc(),
    }
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    match self {
      Self::Empty(e) => e.advance(target),
      Self::Dense(e) => e.advance(target),
      Self::Sparse(e) => e.advance(target),
    }
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    match self {
      Self::Empty(e) => e.slow_advance(target),
      Self::Dense(e) => e.slow_advance(target),
      Self::Sparse(e) => e.slow_advance(target),
    }
  }

  fn cost(&self) -> Result<i64> {
    match self {
      Self::Empty(e) => e.cost(),
      Self::Dense(e) => e.cost(),
      Self::Sparse(e) => e.cost(),
    }
  }
}

impl<I> DocIndexIterator for IterEnum<I>
where
  I: IndexInput,
{
  fn index(&self) -> Result<i32> {
    match self {
      Self::Empty(e) => e.index(),
      Self::Dense(e) => e.index(),
      Self::Sparse(e) => e.index(),
    }
  }
}

pub enum VectorScorerEnum<I, R1, R2>
where
  I: IndexInput,
  R1: RandomVectorScorer,
  R2: RandomVectorScorer,
{
  Dense {
    iterator: DocIdSetIteratorEnum2<DenseDocIndexIterator, DocIndexIteratorImpl<I>>,
    random_vector_scorer: R1,
  },
  Sparse {
    iterator: DocIdSetIteratorEnum2<DenseDocIndexIterator, DocIndexIteratorImpl<I>>,
    random_vector_scorer: R2,
  },
}

impl<I, R1, R2> VectorScorerEnum<I, R1, R2>
where
  I: IndexInput,
  R1: RandomVectorScorer,
  R2: RandomVectorScorer,
{
  fn new_dense(iterator: DenseDocIndexIterator, random_vector_scorer: R1) -> Self {
    Self::Dense {
      iterator: DocIdSetIteratorEnum2::A(iterator),
      random_vector_scorer,
    }
  }

  fn new_sparse(iterator: DocIndexIteratorImpl<I>, random_vector_scorer: R2) -> Self {
    Self::Sparse {
      iterator: DocIdSetIteratorEnum2::B(iterator),
      random_vector_scorer,
    }
  }
}

impl<I, R1, R2> VectorScorer for VectorScorerEnum<I, R1, R2>
where
  I: IndexInput,
  R1: RandomVectorScorer,
  R2: RandomVectorScorer,
{
  fn score(&mut self) -> Result<f32> {
    match self {
      Self::Dense {
        iterator,
        random_vector_scorer,
      } => {
        let doc_id = iterator.doc_id().try_convert()?;
        random_vector_scorer.score(doc_id)
      },
      Self::Sparse {
        iterator,
        random_vector_scorer,
      } => {
        let index = match iterator {
          DocIdSetIteratorEnum2::B(iterator) => iterator.index()?,
          DocIdSetIteratorEnum2::A(_) => {
            unreachable!("sparse vector scorer must use sparse iterator")
          },
        };
        random_vector_scorer.score(index as usize)
      },
    }
  }

  type DocIdSetIterator = DocIdSetIteratorEnum2<DenseDocIndexIterator, DocIndexIteratorImpl<I>>;

  fn iterator(&self) -> &Self::DocIdSetIterator {
    match self {
      Self::Dense { iterator, .. } => iterator,
      Self::Sparse { iterator, .. } => iterator,
    }
  }

  fn iterator_mut(&mut self) -> &mut Self::DocIdSetIterator {
    match self {
      Self::Dense { iterator, .. } => iterator,
      Self::Sparse { iterator, .. } => iterator,
    }
  }
}

impl<I, F> OffHeapByteVectorValuesEnum<I, F>
where
  I: IndexInput + Clone,
  F: FlatVectorsScorer,
{
  #[allow(clippy::too_many_arguments)]
  pub fn load(
    vector_similarity_function: VectorSimilarityFunction,
    flat_vectors_scorer: F,
    configuration: Arc<OrdToDocDISIReaderConfiguration>,
    vector_encoding: VectorEncoding,
    dimension: usize,
    vector_data_offset: usize,
    vector_data_length: usize,
    vector_data: I,
  ) -> Result<Self> {
    if configuration.is_empty() || vector_encoding != VectorEncoding::BYTE(1) {
      return Ok(Self::Empty(EmptyOffHeapVectorValues::new(dimension)));
    }

    let bytes_slice = vector_data.slice("vector-data", vector_data_offset, vector_data_length)?;

    if configuration.is_dense() {
      Ok(Self::Dense(DenseOffHeapVectorValues::new(
        dimension,
        configuration.size.try_convert()?,
        bytes_slice,
        dimension,
        flat_vectors_scorer,
        vector_similarity_function,
      )))
    } else {
      Ok(Self::Sparse(SparseOffHeapVectorValues::new(
        configuration,
        vector_data,
        bytes_slice,
        dimension,
        dimension,
        flat_vectors_scorer,
        vector_similarity_function,
      )?))
    }
  }
}
