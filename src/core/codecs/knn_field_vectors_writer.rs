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
use crate::core::util::accountable::Accountable;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::impl_from_for_enum;

/// Vectors’ writer for a field.
///
/// # Parameters
///
/// - `T`: an array type; the type of vectors to be written.
pub trait KnnFieldVectorsWriter: Accountable {
  /// Adds a new doc ID with its vector value to the given field for indexing.
  /// Doc IDs must be added in increasing order.
  fn add_value<F>(
    &mut self,
    _doc_id: i32,
    _vector_value: &VectorValueEnum,
    _flat_field_vectors_writers: &mut [F],
  ) -> Result<()>
  where
    F: FlatFieldVectorsWriter,
  {
    Err(LuceneError::unsupported_operation(""))
  }
  /// Used to copy values being indexed to internal storage.
  ///
  /// # Arguments
  ///
  /// - `vector_value`: an array containing the vector value to add.
  ///
  /// # Returns
  ///
  /// A copy of the value; a new array.
  fn copy_value(&self, _vector_value: &VectorValueEnum) -> Result<VectorValueEnum> {
    Err(LuceneError::unsupported_operation(""))
  }
}
#[derive(Clone, Debug)]
pub enum VectorValueEnum {
  Byte(Vec<u8>),
  Float(Vec<f32>),
}
impl_from_for_enum!(
    VectorValueEnum,
    Vec<u8> => Byte,
    Vec<f32> => Float,
);
impl VectorValueEnum {
  pub(crate) fn copy_value(&self, offset: usize, dim: usize) -> VectorValueEnum {
    match self {
      Self::Byte(v) => {
        let v = ArrayUtil::copy_of_sub_array(v, offset, offset + dim);
        VectorValueEnum::Byte(v)
      },
      Self::Float(v) => {
        let v = ArrayUtil::copy_of_sub_array(v, offset, offset + dim);
        VectorValueEnum::Float(v)
      },
    }
  }
  pub(crate) fn len(&self) -> usize {
    match self {
      Self::Byte(v) => v.len(),
      Self::Float(v) => v.len(),
    }
  }
  pub(crate) fn as_bytes(&self) -> Result<&[u8]> {
    match self {
      Self::Byte(v) => Ok(v),
      Self::Float(_) => Err(LuceneError::unsupported_operation("")),
    }
  }
  pub(crate) fn as_floats(&self) -> Result<&[f32]> {
    match self {
      Self::Byte(_) => Err(LuceneError::unsupported_operation("")),
      Self::Float(v) => Ok(v),
    }
  }
  pub(crate) fn write_float(&self, chunk: &mut [u8]) -> Result<()> {
    match self {
      Self::Byte(_) => Err(LuceneError::unsupported_operation("")),
      Self::Float(v) => {
        let byte_len = v.len() * 4;
        debug_assert!(chunk.len() == byte_len);

        let mut offset = 0;
        for f in v {
          let bytes = f.to_le_bytes();
          chunk[offset..offset + 4].copy_from_slice(&bytes);
          offset += 4;
        }
        Ok(())
      },
    }
  }
}
