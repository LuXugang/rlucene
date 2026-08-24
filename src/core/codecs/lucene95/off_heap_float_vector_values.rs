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
  DocIndexIteratorImpl, IndexedDISIDocIndexIterator, IndexedDISIImpl, get_doc_index_iterator,
};
use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
use crate::core::codecs::lucene95::has_index_slice::HasIndexSlice;
use crate::core::codecs::lucene95::ord_to_doc_disi_reader_configuration::OrdToDocDISIReaderConfiguration;
use crate::core::codecs::off_heap_vector_values::{
  OffHeapVectorValueBits, SparseOffHeapVectorValueBits,
};
use crate::core::index::dummy::dummy_float_vector_values::DummyFloatVectorValues;
use crate::core::index::dummy::dummy_knn_vector_values::DummyKnnVectorsWriter;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::knn_vector_values::{
  DenseDocIndexIterator, DocIndexIterator, KnnVectorValues, create_dense_iterator,
};
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum2};
use crate::core::search::dummy::dummy_vector_scorer::DummyVectorScorer;
use crate::core::search::vector_scorer::VectorScorer;
use crate::core::store::IndexInput;
use crate::core::util::TryIntoInt;
use crate::core::util::bits::Bits;
use crate::core::util::clone::TryClone;
use crate::core::util::dummy::dummy_bits::DummyBits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::hnsw::random_vector_scorer::RandomVectorScorer;
use crate::core::util::long_values::LongValues;
use crate::core::util::packed::direct_monotonic_reader::DirectMonotonicReader;
use parking_lot::Mutex;
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// Read the vector values froGm the index input. This supports both iterated and random access.
struct OffHeapFloatVectorValues<I, F> {
  pub(crate) dimension: usize,
  pub(crate) size: usize,
  pub(crate) byte_size: usize,
  pub(crate) similarity_function: VectorSimilarityFunction,
  pub(crate) flat_vectors_scorer: F,
  pub(crate) inner: Mutex<Inner<I>>,
}
pub struct Inner<I> {
  pub(crate) slice: I,
  pub(crate) last_ord: Option<usize>,
  pub(crate) value: Vec<f32>,
}
impl<I, F> OffHeapFloatVectorValues<I, F> {
  fn new(
    dimension: usize,
    size: usize,
    slice: I,
    byte_size: usize,
    flat_vectors_scorer: F,
    similarity_function: VectorSimilarityFunction,
  ) -> Self {
    let inner = Inner {
      slice,
      last_ord: None,
      value: vec![0.0; dimension],
    };
    Self {
      dimension,
      size,
      byte_size,
      similarity_function,
      flat_vectors_scorer,
      inner: Mutex::new(inner),
    }
  }
}

impl<I, F> OffHeapFloatVectorValues<I, F>
where
  I: IndexInput,
{
  fn read_value(&self, target_ord: usize, inner: &mut Inner<I>) -> Result<()> {
    let pos = target_ord
      .checked_mul(self.byte_size)
      .ok_or_else(|| LuceneError::illegal_state("seek overflow"))?;
    inner.slice.seek(pos)?;
    let len = inner.value.len();
    inner.slice.read_floats(&mut inner.value, 0, len)?;

    inner.last_ord = Some(target_ord);
    Ok(())
  }
}
impl<I, F> HasIndexSlice for OffHeapFloatVectorValues<I, F>
where
  I: IndexInput,
{
  fn seek(&self, pos: usize) -> Result<()> {
    self.inner.lock().slice.seek(pos)
  }

  fn read_bytes(&self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    self.inner.lock().slice.read_bytes(b, offset, len)
  }
}

impl<I, F> KnnVectorValues for OffHeapFloatVectorValues<I, F>
where
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

  type Bits<'a, B>
    = DummyBits
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, _accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
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
{
  fn vector_value(&self, target_ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    let mut inner = self.inner.lock();
    let same_ord = matches!(inner.last_ord, Some(last_ord) if last_ord == target_ord);
    if !same_ord {
      self.read_value(target_ord, &mut inner)?;
      inner.last_ord = Some(target_ord);
    }

    Ok(Cow::Owned(VectorValueEnum::Float(inner.value.clone())))
  }

  type FloatVectorValues = DummyFloatVectorValues;
  type VectorScorer = DummyVectorScorer;
}

pub struct DenseOffHeapVectorValues<I, F> {
  base: OffHeapFloatVectorValues<I, F>,
}

impl<I, F> DenseOffHeapVectorValues<I, F> {
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
    Ok(Self { base })
  }
}

impl<I, F> TryClone for DenseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer + Clone,
{
  fn try_clone(&self) -> Result<Self>
  where
    Self: Sized,
  {
    let inner = self.base.inner.lock();
    Self::new(
      self.base.dimension,
      self.base.size,
      inner.slice.try_clone()?,
      self.base.byte_size,
      self.base.flat_vectors_scorer.clone(),
      self.base.similarity_function,
    )
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

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    Ok(ord)
  }

  type KnnVectorValues = Self;

  fn copy(&self) -> Result<Self::KnnVectorValues> {
    let inner = self.base.inner.lock();
    Self::new(
      self.base.dimension,
      self.base.size,
      inner.slice.try_clone()?,
      self.base.byte_size,
      self.base.flat_vectors_scorer.clone(),
      self.base.similarity_function,
    )
  }

  fn get_encoding(&self) -> VectorEncoding {
    FloatVectorValues::get_encoding(self)
  }

  type Bits<'a, B>
    = B
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    accept_docs
  }

  type DocIndexIterator = DenseDocIndexIterator;

  fn iterator(&self) -> Result<Self::DocIndexIterator> {
    Ok(create_dense_iterator(self.size() as i32))
  }
}
impl<I, F> HasIndexSlice for DenseOffHeapVectorValues<I, F>
where
  I: IndexInput,
{
  fn seek(&self, pos: usize) -> Result<()> {
    self.base.seek(pos)
  }

  fn read_bytes(&self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    self.base.read_bytes(b, offset, len)
  }
}

impl<I, F> FloatVectorValues for DenseOffHeapVectorValues<I, F>
where
  I: IndexInput,
  F: FlatVectorsScorer + Clone,
{
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    self.base.vector_value(ord)
  }

  type FloatVectorValues = Self;

  fn float_copy(&self) -> Result<Option<Self::FloatVectorValues>> {
    Ok(Some(Self::new(
      self.base.dimension,
      self.base.size,
      self.base.inner.lock().slice.try_clone()?,
      self.base.byte_size,
      self.base.flat_vectors_scorer.clone(),
      self.base.similarity_function,
    )?))
  }

  type VectorScorer = DenseVectorScorer<
    <F as FlatVectorsScorer>::RandomVectorScorerF32<DenseOffHeapVectorValues<I, F>>,
  >;

  fn scorer(&self, query: Vec<f32>) -> Result<Option<Self::VectorScorer>> {
    let copy = self.float_copy()?.ok_or_else(|| {
      LuceneError::illegal_state("DenseOffHeapVectorValues should support float_copy()")
    })?;
    let iterator = copy.iterator()?;

    let sf = copy.base.similarity_function;
    let random_vector_scorer = self
      .base
      .flat_vectors_scorer
      .get_random_vector_scorer_f32(sf, copy, query)?;
    Ok(Some(DenseVectorScorer::new(iterator, random_vector_scorer)))
  }
}

pub struct DenseVectorScorer<R> {
  iterator: DenseDocIndexIterator,
  random_vector_scorer: R,
}
impl<R> DenseVectorScorer<R> {
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
  fn score(&self) -> Result<f32> {
    let doc_id = self.iterator.doc_id().try_convert()?;
    self.random_vector_scorer.score(doc_id)
  }

  type DocIdSetIteratorRef<'a>
    = &'a DenseDocIndexIterator
  where
    Self: 'a;

  fn iterator(&self) -> Self::DocIdSetIteratorRef<'_> {
    &self.iterator
  }

  type DocIdSetIteratorMut<'a>
    = &'a mut DenseDocIndexIterator
  where
    Self: 'a;

  fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_> {
    &mut self.iterator
  }
}

pub struct SparseOffHeapVectorValues<I, F>
where
  I: IndexInput,
{
  base: OffHeapFloatVectorValues<I::IndexInput, F>,
  ord_to_doc: Rc<RefCell<DirectMonotonicReader<I::RandomAccessSlice>>>,
  data_in: I,
  configuration: Arc<OrdToDocDISIReaderConfiguration>,
  disi: RefCell<Option<DocIndexIteratorImpl<I>>>,
}

impl<I, F> SparseOffHeapVectorValues<I, F>
where
  I: IndexInput + Clone,
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
    let base = OffHeapFloatVectorValues::new(
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

    let disi = IndexedDISIImpl::new(
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
      ord_to_doc: Rc::new(RefCell::new(ord_to_doc)),
      data_in,
      configuration,
      disi: RefCell::new(disi),
    })
  }
}
impl<I, F> HasIndexSlice for SparseOffHeapVectorValues<I, F>
where
  I: IndexInput,
{
  fn seek(&self, pos: usize) -> Result<()> {
    self.base.seek(pos)
  }

  fn read_bytes(&self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    self.base.read_bytes(b, offset, len)
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
    Ok(self.ord_to_doc.borrow_mut().get_mut(ord)? as usize)
  }

  type KnnVectorValues = Self;

  fn copy(&self) -> Result<Self::KnnVectorValues> {
    Self::new(
      self.configuration.clone(),
      self.data_in.clone(),
      self.base.inner.lock().slice.try_clone()?,
      self.base.dimension,
      self.base.byte_size,
      self.base.flat_vectors_scorer.clone(),
      self.base.similarity_function,
    )
  }

  fn get_encoding(&self) -> VectorEncoding {
    FloatVectorValues::get_encoding(self)
  }

  type Bits<'a, B>
    = SparseOffHeapVectorValueBits<B, I::RandomAccessSlice>
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    accept_docs
      .map(|bits| SparseOffHeapVectorValueBits::new(bits, self.base.size, self.ord_to_doc.clone()))
  }

  type DocIndexIterator = DocIndexIteratorImpl<I>;

  fn iterator(&self) -> Result<Self::DocIndexIterator> {
    match self.disi.borrow_mut().take() {
      Some(disi) => Ok(disi),
      None => Err(LuceneError::illegal_state("iterator only called once")),
    }
  }
}

impl<I, F> FloatVectorValues for SparseOffHeapVectorValues<I, F>
where
  I: IndexInput + Clone,
  F: FlatVectorsScorer + Clone,
{
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    self.base.vector_value(ord)
  }

  type FloatVectorValues = Self;

  fn float_copy(&self) -> Result<Option<Self::FloatVectorValues>> {
    Ok(Some(Self::new(
      self.configuration.clone(),
      self.data_in.clone(),
      self.base.inner.lock().slice.try_clone()?,
      self.base.dimension,
      self.base.byte_size,
      self.base.flat_vectors_scorer.clone(),
      self.base.similarity_function,
    )?))
  }

  type VectorScorer = SparseVectorScorer<
    I,
    <F as FlatVectorsScorer>::RandomVectorScorerF32<SparseOffHeapVectorValues<I, F>>,
  >;

  fn scorer(&self, query: Vec<f32>) -> Result<Option<Self::VectorScorer>> {
    let copy = self.float_copy()?.ok_or_else(|| {
      LuceneError::illegal_state("DenseOffHeapVectorValues should support float_copy()")
    })?;
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
{
  iterator: DocIndexIteratorImpl<I>,
  random_vector_scorer: R,
}

impl<I, R> SparseVectorScorer<I, R>
where
  I: IndexInput,
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
  fn score(&self) -> Result<f32> {
    let index = self.iterator.index()?;
    self.random_vector_scorer.score(index as usize)
  }

  type DocIdSetIteratorRef<'a>
    = &'a DocIndexIteratorImpl<I>
  where
    Self: 'a;

  fn iterator(&self) -> Self::DocIdSetIteratorRef<'_> {
    &self.iterator
  }

  type DocIdSetIteratorMut<'a>
    = &'a mut DocIndexIteratorImpl<I>
  where
    Self: 'a;

  fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_> {
    &mut self.iterator
  }
}
pub struct EmptyOffHeapVectorValues {
  dimension: usize,
  vectors: Vec<f32>,
}
impl EmptyOffHeapVectorValues {
  fn new(dimension: usize) -> Self {
    let vectors = Vec::new();

    Self { dimension, vectors }
  }
}
impl HasIndexSlice for EmptyOffHeapVectorValues {}
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

  type Bits<'a, B>
    = DummyBits
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, _accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    None
  }

  type DocIndexIterator = DenseDocIndexIterator;

  fn iterator(&self) -> Result<Self::DocIndexIterator> {
    Ok(create_dense_iterator(0))
  }
}

impl FloatVectorValues for EmptyOffHeapVectorValues {
  fn vector_value(&self, _ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    debug_assert!(self.vectors.is_empty());
    Err(LuceneError::unsupported_operation(""))
  }

  type FloatVectorValues = Self;

  fn float_copy(&self) -> Result<Option<Self::FloatVectorValues>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type VectorScorer = DummyVectorScorer;

  fn scorer(&self, _target: Vec<f32>) -> Result<Option<Self::VectorScorer>> {
    Ok(None)
  }
}

pub enum OffHeapFloatVectorValuesEnum<I, F>
where
  I: IndexInput,
{
  Empty(EmptyOffHeapVectorValues),
  Dense(DenseOffHeapVectorValues<I::IndexInput, F>),
  Sparse(SparseOffHeapVectorValues<I, F>),
}

pub enum OffHeapFloatVectorValuesCopy<I, F>
where
  I: IndexInput,
{
  Dense(DenseOffHeapVectorValues<I::IndexInput, F>),
  Sparse(SparseOffHeapVectorValues<I, F>),
}

impl<I, F> KnnVectorValues for OffHeapFloatVectorValuesCopy<I, F>
where
  F: FlatVectorsScorer + Clone,
  I: IndexInput + Clone,
{
  fn dimension(&self) -> usize {
    match self {
      Self::Dense(values) => values.dimension(),
      Self::Sparse(values) => values.dimension(),
    }
  }

  fn size(&self) -> usize {
    match self {
      Self::Dense(values) => values.size(),
      Self::Sparse(values) => values.size(),
    }
  }

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    match self {
      Self::Dense(values) => values.ord_to_doc(ord),
      Self::Sparse(values) => values.ord_to_doc(ord),
    }
  }

  type KnnVectorValues = Self;

  fn copy(&self) -> Result<Self::KnnVectorValues> {
    match self {
      Self::Dense(values) => KnnVectorValues::copy(values).map(Self::Dense),
      Self::Sparse(values) => KnnVectorValues::copy(values).map(Self::Sparse),
    }
  }

  fn get_vector_byte_length(&self) -> usize {
    match self {
      Self::Dense(values) => values.get_vector_byte_length(),
      Self::Sparse(values) => values.get_vector_byte_length(),
    }
  }

  fn get_encoding(&self) -> VectorEncoding {
    match self {
      Self::Dense(values) => KnnVectorValues::get_encoding(values),
      Self::Sparse(values) => KnnVectorValues::get_encoding(values),
    }
  }

  type Bits<'a, B>
    = OffHeapVectorValueBits<I::RandomAccessSlice, B>
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    match self {
      Self::Dense(values) => values
        .get_accept_ords(accept_docs)
        .map(OffHeapVectorValueBits::Dense),
      Self::Sparse(values) => values
        .get_accept_ords(accept_docs)
        .map(OffHeapVectorValueBits::Sparse),
    }
  }

  type DocIndexIterator = IndexedDISIDocIndexIterator<I>;

  fn iterator(&self) -> Result<Self::DocIndexIterator> {
    match self {
      Self::Dense(values) => values.iterator().map(IndexedDISIDocIndexIterator::Dense),
      Self::Sparse(values) => values.iterator().map(IndexedDISIDocIndexIterator::Sparse),
    }
  }
}

impl<I, F> FloatVectorValues for OffHeapFloatVectorValuesCopy<I, F>
where
  I: IndexInput + Clone,
  F: FlatVectorsScorer + Clone,
{
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    match self {
      Self::Dense(values) => values.vector_value(ord),
      Self::Sparse(values) => values.vector_value(ord),
    }
  }

  type FloatVectorValues = Self;

  fn float_copy(&self) -> Result<Option<Self::FloatVectorValues>> {
    match self {
      Self::Dense(values) => Ok(values.float_copy()?.map(Self::Dense)),
      Self::Sparse(values) => Ok(values.float_copy()?.map(Self::Sparse)),
    }
  }

  type VectorScorer = VectorScorerEnum<I, F>;

  fn scorer(&self, target: Vec<f32>) -> Result<Option<Self::VectorScorer>> {
    match self {
      Self::Dense(values) => Ok(values.scorer(target)?.map(VectorScorerEnum::Dense)),
      Self::Sparse(values) => Ok(values.scorer(target)?.map(VectorScorerEnum::Sparse)),
    }
  }

  fn get_encoding(&self) -> VectorEncoding {
    match self {
      Self::Dense(values) => FloatVectorValues::get_encoding(values),
      Self::Sparse(values) => FloatVectorValues::get_encoding(values),
    }
  }

  fn get_vectors_mut(&mut self) -> Result<&mut Vec<VectorValueEnum>> {
    match self {
      Self::Dense(values) => values.get_vectors_mut(),
      Self::Sparse(values) => values.get_vectors_mut(),
    }
  }

  fn get_vectors(&self) -> Result<&[VectorValueEnum]> {
    match self {
      Self::Dense(values) => values.get_vectors(),
      Self::Sparse(values) => values.get_vectors(),
    }
  }
}

impl<I, F> KnnVectorValues for OffHeapFloatVectorValuesEnum<I, F>
where
  F: FlatVectorsScorer + Clone,
  I: IndexInput + Clone,
{
  fn dimension(&self) -> usize {
    match self {
      OffHeapFloatVectorValuesEnum::Empty(e) => e.dimension(),
      OffHeapFloatVectorValuesEnum::Dense(e) => e.dimension(),
      OffHeapFloatVectorValuesEnum::Sparse(e) => e.dimension(),
    }
  }

  fn size(&self) -> usize {
    match self {
      OffHeapFloatVectorValuesEnum::Empty(e) => e.size(),
      OffHeapFloatVectorValuesEnum::Dense(e) => e.size(),
      OffHeapFloatVectorValuesEnum::Sparse(e) => e.size(),
    }
  }

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    match self {
      OffHeapFloatVectorValuesEnum::Empty(e) => e.ord_to_doc(ord),
      OffHeapFloatVectorValuesEnum::Dense(e) => e.ord_to_doc(ord),
      OffHeapFloatVectorValuesEnum::Sparse(e) => e.ord_to_doc(ord),
    }
  }

  type KnnVectorValues = OffHeapFloatVectorValuesCopy<I, F>;

  fn copy(&self) -> Result<Self::KnnVectorValues> {
    match self {
      OffHeapFloatVectorValuesEnum::Empty(_) => Err(LuceneError::unsupported_operation("")),
      OffHeapFloatVectorValuesEnum::Dense(e) => {
        KnnVectorValues::copy(e).map(OffHeapFloatVectorValuesCopy::Dense)
      },
      OffHeapFloatVectorValuesEnum::Sparse(e) => {
        KnnVectorValues::copy(e).map(OffHeapFloatVectorValuesCopy::Sparse)
      },
    }
  }

  fn get_vector_byte_length(&self) -> usize {
    match self {
      OffHeapFloatVectorValuesEnum::Empty(e) => e.get_vector_byte_length(),
      OffHeapFloatVectorValuesEnum::Dense(e) => e.get_vector_byte_length(),
      OffHeapFloatVectorValuesEnum::Sparse(e) => e.get_vector_byte_length(),
    }
  }

  fn get_encoding(&self) -> VectorEncoding {
    match self {
      OffHeapFloatVectorValuesEnum::Empty(e) => FloatVectorValues::get_encoding(e),
      OffHeapFloatVectorValuesEnum::Dense(e) => FloatVectorValues::get_encoding(e),
      OffHeapFloatVectorValuesEnum::Sparse(e) => FloatVectorValues::get_encoding(e),
    }
  }

  type Bits<'a, B>
    = OffHeapVectorValueBits<I::RandomAccessSlice, B>
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    match self {
      OffHeapFloatVectorValuesEnum::Empty(_) => None,
      OffHeapFloatVectorValuesEnum::Dense(e) => e
        .get_accept_ords(accept_docs)
        .map(OffHeapVectorValueBits::Dense),
      OffHeapFloatVectorValuesEnum::Sparse(e) => e
        .get_accept_ords(accept_docs)
        .map(OffHeapVectorValueBits::Sparse),
    }
  }

  type DocIndexIterator = IndexedDISIDocIndexIterator<I>;

  fn iterator(&self) -> Result<Self::DocIndexIterator> {
    match self {
      OffHeapFloatVectorValuesEnum::Empty(e) => {
        e.iterator().map(IndexedDISIDocIndexIterator::Dense)
      },
      OffHeapFloatVectorValuesEnum::Dense(e) => {
        e.iterator().map(IndexedDISIDocIndexIterator::Dense)
      },
      OffHeapFloatVectorValuesEnum::Sparse(e) => {
        e.iterator().map(IndexedDISIDocIndexIterator::Sparse)
      },
    }
  }
}

impl<I, F> FloatVectorValues for OffHeapFloatVectorValuesEnum<I, F>
where
  I: IndexInput + Clone,
  F: FlatVectorsScorer + Clone,
{
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    match self {
      OffHeapFloatVectorValuesEnum::Empty(e) => e.vector_value(ord),
      OffHeapFloatVectorValuesEnum::Dense(e) => e.vector_value(ord),
      OffHeapFloatVectorValuesEnum::Sparse(e) => e.vector_value(ord),
    }
  }

  type FloatVectorValues = OffHeapFloatVectorValuesCopy<I, F>;

  fn float_copy(&self) -> Result<Option<Self::FloatVectorValues>> {
    match self {
      OffHeapFloatVectorValuesEnum::Empty(_) => Err(LuceneError::unsupported_operation("")),
      OffHeapFloatVectorValuesEnum::Dense(e) => {
        Ok(e.float_copy()?.map(OffHeapFloatVectorValuesCopy::Dense))
      },
      OffHeapFloatVectorValuesEnum::Sparse(e) => {
        Ok(e.float_copy()?.map(OffHeapFloatVectorValuesCopy::Sparse))
      },
    }
  }

  type VectorScorer = VectorScorerEnum<I, F>;

  fn scorer(&self, target: Vec<f32>) -> Result<Option<Self::VectorScorer>> {
    match self {
      OffHeapFloatVectorValuesEnum::Empty(_) => Ok(None),

      OffHeapFloatVectorValuesEnum::Dense(e) => Ok(e.scorer(target)?.map(VectorScorerEnum::Dense)),

      OffHeapFloatVectorValuesEnum::Sparse(e) => {
        Ok(e.scorer(target)?.map(VectorScorerEnum::Sparse))
      },
    }
  }

  fn get_encoding(&self) -> VectorEncoding {
    match self {
      OffHeapFloatVectorValuesEnum::Empty(e) => FloatVectorValues::get_encoding(e),
      OffHeapFloatVectorValuesEnum::Dense(e) => FloatVectorValues::get_encoding(e),
      OffHeapFloatVectorValuesEnum::Sparse(e) => FloatVectorValues::get_encoding(e),
    }
  }

  fn get_vectors_mut(&mut self) -> Result<&mut Vec<VectorValueEnum>> {
    match self {
      OffHeapFloatVectorValuesEnum::Empty(e) => e.get_vectors_mut(),
      OffHeapFloatVectorValuesEnum::Dense(e) => e.get_vectors_mut(),
      OffHeapFloatVectorValuesEnum::Sparse(e) => e.get_vectors_mut(),
    }
  }

  fn get_vectors(&self) -> Result<&[VectorValueEnum]> {
    match self {
      OffHeapFloatVectorValuesEnum::Empty(e) => e.get_vectors(),
      OffHeapFloatVectorValuesEnum::Dense(e) => e.get_vectors(),
      OffHeapFloatVectorValuesEnum::Sparse(e) => e.get_vectors(),
    }
  }
}

pub enum VectorScorerEnum<I, F>
where
  I: IndexInput + Clone,
  F: FlatVectorsScorer + Clone,
{
  Dense(DenseVectorScorer<F::RandomVectorScorerF32<DenseOffHeapVectorValues<I::IndexInput, F>>>),
  Sparse(SparseVectorScorer<I, F::RandomVectorScorerF32<SparseOffHeapVectorValues<I, F>>>),
}
impl<I, F> VectorScorer for VectorScorerEnum<I, F>
where
  I: IndexInput + Clone,
  F: FlatVectorsScorer + Clone,
{
  fn score(&self) -> Result<f32> {
    match self {
      Self::Dense(scorer) => scorer.score(),
      Self::Sparse(scorer) => scorer.score(),
    }
  }

  type DocIdSetIteratorRef<'a>
    = DocIdSetIteratorEnum2<&'a DenseDocIndexIterator, &'a DocIndexIteratorImpl<I>>
  where
    Self: 'a;

  fn iterator(&self) -> Self::DocIdSetIteratorRef<'_> {
    match self {
      Self::Dense(scorer) => DocIdSetIteratorEnum2::A(scorer.iterator()),
      Self::Sparse(scorer) => DocIdSetIteratorEnum2::B(scorer.iterator()),
    }
  }

  type DocIdSetIteratorMut<'a>
    = DocIdSetIteratorEnum2<&'a mut DenseDocIndexIterator, &'a mut DocIndexIteratorImpl<I>>
  where
    Self: 'a;

  fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_> {
    match self {
      Self::Dense(scorer) => DocIdSetIteratorEnum2::A(scorer.iterator_mut()),
      Self::Sparse(scorer) => DocIdSetIteratorEnum2::B(scorer.iterator_mut()),
    }
  }
}

impl<I, F> OffHeapFloatVectorValuesEnum<I, F>
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
    if configuration.docs_with_field_offset == -2 || vector_encoding != VectorEncoding::FLOAT32(4) {
      return Ok(Self::Empty(EmptyOffHeapVectorValues::new(dimension)));
    }

    let bytes_slice = vector_data.slice("vector-data", vector_data_offset, vector_data_length)?;
    let byte_size = dimension
      .checked_mul(vector_encoding.byte_size())
      .ok_or_else(|| LuceneError::illegal_state("vector byte size overflow"))?;

    if configuration.docs_with_field_offset == -1 {
      Ok(Self::Dense(DenseOffHeapVectorValues::new(
        dimension,
        configuration.size.try_convert()?,
        bytes_slice,
        byte_size,
        flat_vectors_scorer,
        vector_similarity_function,
      )?))
    } else {
      Ok(Self::Sparse(SparseOffHeapVectorValues::new(
        configuration,
        vector_data,
        bytes_slice,
        dimension,
        byte_size,
        flat_vectors_scorer,
        vector_similarity_function,
      )?))
    }
  }
}
