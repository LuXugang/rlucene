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
  fn byte_copy(&self) -> Result<Self::ByteVectorValues> {
    Err(LuceneError::unsupported_operation(""))
  }

  type VectorScorer: VectorScorer;
  fn scorer(&self, _query: Vec<u8>) -> Result<Self::VectorScorer> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_encoding(&self) -> VectorEncoding {
    VectorEncoding::BYTE(1)
  }

  fn get_vectors_mut(&mut self) -> Result<&mut Vec<Vec<u8>>> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn get_vectors(&self) -> Result<&[Vec<u8>]> {
    Err(LuceneError::unsupported_operation(""))
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
pub fn from_bytes(dim: usize) -> ByteVectorValuesImpl {
  ByteVectorValuesImpl::new(dim)
}

#[derive(Clone)]
pub struct ByteVectorValuesImpl {
  vectors: Vec<Vec<u8>>,
  dim: usize,
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

impl ByteVectorValues for ByteVectorValuesImpl {
  fn vector_value(&self, target_ord: usize) -> &[u8] {
    self.vectors[target_ord].as_slice()
  }

  type ByteVectorValues = Self;

  fn byte_copy(&self) -> Result<Self::ByteVectorValues> {
    todo!()
  }

  type VectorScorer = DummyVectorScorer;

  fn get_vectors_mut(&mut self) -> Result<&mut Vec<Vec<u8>>> {
    Ok(&mut self.vectors)
  }

  fn get_vectors(&self) -> Result<&[Vec<u8>]> {
    Ok(&self.vectors)
  }
}
