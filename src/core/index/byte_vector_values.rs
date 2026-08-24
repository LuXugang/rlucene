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
use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
use crate::core::index::dummy::dummy_doc_index_iterator::DummyDocIndexIterator;
use crate::core::index::knn_vector_values::{BitsImpl, KnnVectorValues};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::search::dummy::dummy_vector_scorer::DummyVectorScorer;
use crate::core::search::vector_scorer::VectorScorer;
use crate::core::util::bits::Bits;
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;

/// This trait provides access to per-document byte vector values indexed as `KnnByteVectorField`.
pub trait ByteVectorValues: KnnVectorValues {
  /// Returns the vector value for an ordinal in `0..size()`.
  /// Returns an out-of-bounds error for an invalid ordinal. The returned array may be shared across calls.
  ///
  /// # Returns
  /// the vector value
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>>;

  type ByteVectorValues: ByteVectorValues;
  /// Creates a copy of this [`KnnVectorValues`] when an independent instance is
  /// needed.
  ///
  /// This is useful when multiple vector values need to be accessed at the same
  /// time, in order to avoid overwriting the underlying vector buffer returned by
  /// a single instance.
  ///
  /// Returning `Some(...)` means that a new independent object was created.
  ///
  /// Returning `None` means that no new object was created, and callers should
  /// continue using `self` directly.
  fn byte_copy(&self) -> Result<Option<Self::ByteVectorValues>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type VectorScorer: VectorScorer;
  fn scorer(&self, _query: Vec<u8>) -> Result<Option<Self::VectorScorer>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_encoding(&self) -> VectorEncoding {
    VectorEncoding::BYTE(1)
  }

  fn get_vectors_mut(&mut self) -> Result<&mut Vec<VectorValueEnum>> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn get_vectors(&self) -> Result<&[VectorValueEnum]> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_vectors_capacity(&self) -> Result<usize> {
    Ok(self.get_vectors()?.len())
  }
}

impl<T> ByteVectorValues for &T
where
  T: ByteVectorValues,
{
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    (**self).vector_value(ord)
  }

  type ByteVectorValues = T::ByteVectorValues;

  fn byte_copy(&self) -> Result<Option<Self::ByteVectorValues>> {
    (**self).byte_copy()
  }

  type VectorScorer = T::VectorScorer;

  fn scorer(&self, query: Vec<u8>) -> Result<Option<Self::VectorScorer>> {
    (**self).scorer(query)
  }

  fn get_encoding(&self) -> VectorEncoding {
    <T as ByteVectorValues>::get_encoding(*self)
  }

  fn get_vectors_mut(&mut self) -> Result<&mut Vec<VectorValueEnum>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_vectors(&self) -> Result<&[VectorValueEnum]> {
    (**self).get_vectors()
  }

  fn get_vectors_capacity(&self) -> Result<usize> {
    (**self).get_vectors_capacity()
  }
}

/// Checks the Vector Encoding of a field
pub fn check_field<LR>(reader: &LR, field: &str) -> Result<()>
where
  LR: LeafReader,
{
  if let Some(fi) = reader.get_field_infos()?.field_info_by_name(field)?
    && fi.has_vector_values()
    && *fi.get_vector_encoding() != VectorEncoding::BYTE(1)
  {
    return Err(LuceneError::illegal_state(format!(
      "Unexpected vector encoding ({:?}) for field {}(expected={:?})",
      fi.get_vector_encoding(),
      field,
      VectorEncoding::BYTE(1)
    )));
  }
  Ok(())
}
/// Creates a [`ByteVectorValues`] from a list of byte arrays.
///
/// # Arguments
/// * `vectors` - the list of byte arrays
/// * `dim` - the dimension of the vectors
///
/// # Returns
/// a [`ByteVectorValues`] instance
pub fn from_bytes(dim: usize) -> ByteVectorValuesImpl {
  ByteVectorValuesImpl::new(dim)
}

pub struct ByteVectorValuesImpl {
  vectors: Vec<VectorValueEnum>,
  dim: usize,
}
// TODO IMPORTANT avoid CLone ?
impl TryClone for ByteVectorValuesImpl {
  fn try_clone(&self) -> Result<Self>
  where
    Self: Sized,
  {
    Ok(Self {
      vectors: self.vectors.clone(),
      dim: self.dim,
    })
  }
}
impl ByteVectorValuesImpl {
  pub(crate) fn new(dim: usize) -> Self {
    Self {
      vectors: Vec::new(),
      dim,
    }
  }
}

impl KnnVectorValues for ByteVectorValuesImpl {
  fn dimension(&self) -> usize {
    self.dim
  }

  fn size(&self) -> usize {
    self.vectors.len()
  }

  type KnnVectorValues = ByteVectorValuesImpl;

  fn get_encoding(&self) -> VectorEncoding {
    ByteVectorValues::get_encoding(self)
  }

  type Bits<'a, B>
    = BitsImpl<B, &'a Self>
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    self.default_get_accept_ords(accept_docs)
  }

  type DocIndexIterator = DummyDocIndexIterator;
}

impl ByteVectorValues for ByteVectorValuesImpl {
  fn vector_value(&self, target_ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    Ok(Cow::Borrowed(&self.vectors[target_ord]))
  }

  type ByteVectorValues = Self;

  fn byte_copy(&self) -> Result<Option<Self::ByteVectorValues>> {
    Ok(None)
  }

  type VectorScorer = DummyVectorScorer;

  fn get_vectors_mut(&mut self) -> Result<&mut Vec<VectorValueEnum>> {
    Ok(&mut self.vectors)
  }

  fn get_vectors(&self) -> Result<&[VectorValueEnum]> {
    Ok(&self.vectors)
  }

  fn get_vectors_capacity(&self) -> Result<usize> {
    Ok(self.vectors.capacity())
  }
}
