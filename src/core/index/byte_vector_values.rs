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

/// A read-only byte slice that keeps its backing buffer borrowed until dropped.
/// Use [`ByteVectorValue::into_owned`] only when an independent snapshot is needed.
#[derive(Debug)]
pub struct ByteVectorValue<'a>(pub(crate) ByteVectorValueStorage<'a>);

#[derive(Debug)]
pub(crate) enum ByteVectorValueStorage<'a> {
  Borrowed(&'a [u8]),
  Owned(Vec<u8>),
  Buffer(parking_lot::MappedMutexGuard<'a, [u8]>),
}

impl std::ops::Deref for ByteVectorValue<'_> {
  type Target = [u8];

  fn deref(&self) -> &Self::Target {
    match &self.0 {
      ByteVectorValueStorage::Borrowed(value) => value,
      ByteVectorValueStorage::Owned(value) => value.as_slice(),
      ByteVectorValueStorage::Buffer(value) => value,
    }
  }
}

impl AsRef<[u8]> for ByteVectorValue<'_> {
  fn as_ref(&self) -> &[u8] {
    self
  }
}

impl<'a> From<&'a [u8]> for ByteVectorValue<'a> {
  fn from(value: &'a [u8]) -> Self {
    Self(ByteVectorValueStorage::Borrowed(value))
  }
}

impl From<Vec<u8>> for ByteVectorValue<'_> {
  fn from(value: Vec<u8>) -> Self {
    Self(ByteVectorValueStorage::Owned(value))
  }
}

impl<'a> From<parking_lot::MappedMutexGuard<'a, [u8]>> for ByteVectorValue<'a> {
  fn from(value: parking_lot::MappedMutexGuard<'a, [u8]>) -> Self {
    Self(ByteVectorValueStorage::Buffer(value))
  }
}

impl<'a> From<Cow<'a, [u8]>> for ByteVectorValue<'a> {
  fn from(value: Cow<'a, [u8]>) -> Self {
    Self(match value {
      Cow::Borrowed(value) => ByteVectorValueStorage::Borrowed(value),
      Cow::Owned(value) => ByteVectorValueStorage::Owned(value),
    })
  }
}

impl<'a> TryFrom<Cow<'a, VectorValueEnum>> for ByteVectorValue<'a> {
  type Error = LuceneError;

  fn try_from(value: Cow<'a, VectorValueEnum>) -> Result<Self> {
    match value {
      Cow::Borrowed(VectorValueEnum::Byte(value)) => {
        Ok(Self(ByteVectorValueStorage::Borrowed(value.as_slice())))
      },
      Cow::<VectorValueEnum>::Owned(VectorValueEnum::Byte(value)) => {
        Ok(Self(ByteVectorValueStorage::Owned(value)))
      },
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }
}

impl<'a> From<ByteVectorValue<'a>> for Cow<'a, [u8]> {
  fn from(value: ByteVectorValue<'a>) -> Self {
    match value.0 {
      ByteVectorValueStorage::Borrowed(value) => Cow::Borrowed(value),
      ByteVectorValueStorage::Owned(value) => Cow::Owned(value),
      ByteVectorValueStorage::Buffer(value) => Cow::Owned(value.to_vec()),
    }
  }
}

impl ByteVectorValue<'_> {
  /// Returns an independent snapshot, moving an already-owned vector without copying.
  pub fn into_owned(self) -> Vec<u8> {
    match self.0 {
      ByteVectorValueStorage::Borrowed(value) => value.to_vec(),
      ByteVectorValueStorage::Owned(value) => value,
      ByteVectorValueStorage::Buffer(value) => value.to_vec(),
    }
  }
}

impl From<ByteVectorValue<'_>> for VectorValueEnum {
  fn from(value: ByteVectorValue<'_>) -> Self {
    Self::Byte(value.into_owned())
  }
}

/// This trait provides access to per-document byte vector values indexed as [`KnnByteVectorField`](crate::core::document::knn_byte_vector_field::KnnByteVectorField).
pub trait ByteVectorValues: KnnVectorValues {
  /// The concrete slice carrier for this provider. Borrowed heap values can return
  /// `&[u8]`; buffered values keep their guard or independent snapshot.
  type VectorValue<'v>: std::ops::Deref<Target = [u8]> + Into<ByteVectorValue<'v>>
  where
    Self: 'v;

  /// Returns the vector value for an ordinal in `0..size()`.
  /// Returns an out-of-bounds error for an invalid ordinal. The returned array may be shared across calls.
  ///
  /// # Returns
  /// A byte slice view; vector encoding is already resolved by this interface.
  ///
  /// The returned view may hold the reader's buffer lock. Drop it before reading,
  /// copying or otherwise accessing the same instance again. Use independent
  /// [`Self::byte_copy`] instances for simultaneous reads of reusable buffers.
  fn vector_value(&self, ord: usize) -> Result<Self::VectorValue<'_>>;

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
  /// continue using `self` directly. In this case the instance must support
  /// simultaneous vector views without locking a shared reusable buffer.
  fn byte_copy(&self) -> Result<Option<Self::ByteVectorValues>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type VectorScorer: VectorScorer;
  fn scorer<TV>(&self, _query: TV) -> Result<Option<Self::VectorScorer>>
  where
    TV: Into<Vec<u8>>,
  {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_encoding(&self) -> VectorEncoding {
    VectorEncoding::BYTE(1)
  }

  fn get_vectors_mut(&mut self) -> Result<&mut Vec<Box<[u8]>>> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn get_vectors(&self) -> Result<&[Box<[u8]>]> {
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
  type VectorValue<'v>
    = T::VectorValue<'v>
  where
    Self: 'v;

  fn vector_value(&self, ord: usize) -> Result<Self::VectorValue<'_>> {
    (**self).vector_value(ord)
  }

  type ByteVectorValues = T::ByteVectorValues;

  fn byte_copy(&self) -> Result<Option<Self::ByteVectorValues>> {
    (**self).byte_copy()
  }

  type VectorScorer = T::VectorScorer;

  fn scorer<TV>(&self, query: TV) -> Result<Option<Self::VectorScorer>>
  where
    TV: Into<Vec<u8>>,
  {
    let query = query.into();
    (**self).scorer(query)
  }

  fn get_encoding(&self) -> VectorEncoding {
    <T as ByteVectorValues>::get_encoding(*self)
  }

  fn get_vectors_mut(&mut self) -> Result<&mut Vec<Box<[u8]>>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_vectors(&self) -> Result<&[Box<[u8]>]> {
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
  vectors: Vec<Box<[u8]>>,
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
  type VectorValue<'v>
    = &'v [u8]
  where
    Self: 'v;

  #[inline]
  fn vector_value(&self, target_ord: usize) -> Result<Self::VectorValue<'_>> {
    Ok(self.vectors[target_ord].as_ref())
  }

  type ByteVectorValues = Self;

  fn byte_copy(&self) -> Result<Option<Self::ByteVectorValues>> {
    Ok(None)
  }

  type VectorScorer = DummyVectorScorer;

  fn get_vectors_mut(&mut self) -> Result<&mut Vec<Box<[u8]>>> {
    Ok(&mut self.vectors)
  }

  fn get_vectors(&self) -> Result<&[Box<[u8]>]> {
    Ok(&self.vectors)
  }

  fn get_vectors_capacity(&self) -> Result<usize> {
    Ok(self.vectors.capacity())
  }
}

use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
