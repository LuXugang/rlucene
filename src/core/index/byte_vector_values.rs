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
use crate::core::index::knn_vector_values::{BitsImpl1, KnnVectorValues};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::search::dummy::dummy_vector_scorer::DummyVectorScorer;
use crate::core::search::vector_scorer::VectorScorer;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// This class provides access to per-document floating point vector values indexed as KnnByteVectorField
pub trait ByteVectorValues: KnnVectorValues {
  /// Return the vector value for the given vector ordinal which must be in [0, size() - 1],
  /// otherwise IndexOutOfBoundsException is thrown. The returned array may be shared across calls.
  ///
  /// # Returns
  /// the vector value
  fn vector_value(&self, ord: usize) -> &[u8];

  type ByteVectorValues: ByteVectorValues;
  /// Creates a new copy of this [`KnnVectorValues`]. This is helpful when you
  /// need to access different values at once, to avoid overwriting the
  /// underlying vector returned.
  fn copy(&self) -> Result<&Self::ByteVectorValues>;

  type VectorScorer: VectorScorer;
  fn scorer(&self, _query: &[u8]) -> Result<Self::VectorScorer> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_encoding(&self) -> VectorEncoding {
    VectorEncoding::BYTE(1)
  }
}
/// Checks the Vector Encoding of a field
pub fn check_field<LR: LeafReader>(reader: &LR, field: &str) -> Result<()> {
  if let Some(fi) = reader.get_field_infos()?.field_info_by_name(field)
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
pub fn from_bytes(vectors: &[Vec<u8>], dim: usize) -> ByteVectorValuesImpl<'_> {
  ByteVectorValuesImpl::new(vectors, dim)
}

pub struct ByteVectorValuesImpl<'a> {
  vectors: &'a [Vec<u8>],
  dim: usize,
}
impl<'a> ByteVectorValuesImpl<'a> {
  pub(crate) fn new(vectors: &'a [Vec<u8>], dim: usize) -> Self {
    Self { vectors, dim }
  }
}

impl<'a> KnnVectorValues for ByteVectorValuesImpl<'a> {
  fn dimension(&self) -> usize {
    self.dim
  }

  fn size(&self) -> usize {
    self.vectors.len()
  }

  type KnnVectorValues = ByteVectorValuesImpl<'a>;

  fn copy(&self) -> Result<&Self::KnnVectorValues> {
    Ok(self)
  }

  fn get_encoding(&self) -> VectorEncoding {
    ByteVectorValues::get_encoding(self)
  }

  type Bits<B>
    = BitsImpl1<B>
  where
    B: Bits;

  fn get_accept_ords<B>(&self, accept_docs: Option<B>) -> Option<Self::Bits<B>>
  where
    B: Bits,
  {
    self.default_get_accept_ords(accept_docs)
  }

  type DocIndexIterator = DummyDocIndexIterator;
}

impl<'a> ByteVectorValues for ByteVectorValuesImpl<'a> {
  fn vector_value(&self, target_ord: usize) -> &[u8] {
    self.vectors[target_ord].as_slice()
  }

  type ByteVectorValues = ByteVectorValuesImpl<'a>;

  fn copy(&self) -> Result<&Self::ByteVectorValues> {
    Ok(self)
  }

  type VectorScorer = DummyVectorScorer;
}
