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
use crate::core::index::vector_buffer::VectorElement;
use crate::core::util::accountable::Accountable;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::ram_usage_estimator::size_of_vec;
use crate::impl_from_for_enum;

/// Vectors’ writer for a field.
///
/// # Parameters
///
/// - `T`: an array type; the type of vectors to be written.
pub trait KnnFieldVectorsWriter: Accountable {
  /// Adds a new doc ID with its vector value to the given field for indexing.
  /// Doc IDs must be added in increasing order.
  fn add_value<F, V: VectorInput + ?Sized>(
    &mut self,
    _doc_id: i32,
    _vector_value: &V,
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
  fn copy_value<T: VectorElement, V: VectorInput + ?Sized>(
    &self,
    _vector_value: &V,
  ) -> Result<Vec<T>> {
    Err(LuceneError::unsupported_operation(""))
  }
}
/// Read-only vector input. Concrete slice types remain concrete through writer forwarding.
/// The enum implementation is retained only for callers at heterogeneous document boundaries.
pub trait VectorInput {
  fn as_floats(&self) -> Result<&[f32]> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn as_bytes(&self) -> Result<&[u8]> {
    Err(LuceneError::unsupported_operation(""))
  }
}
impl VectorInput for [f32] {
  fn as_floats(&self) -> Result<&[f32]> {
    Ok(self)
  }
}
impl VectorInput for [u8] {
  fn as_bytes(&self) -> Result<&[u8]> {
    Ok(self)
  }
}
impl<T> VectorInput for Vec<T>
where
  [T]: VectorInput,
{
  fn as_floats(&self) -> Result<&[f32]> {
    self.as_slice().as_floats()
  }
  fn as_bytes(&self) -> Result<&[u8]> {
    self.as_slice().as_bytes()
  }
}
impl VectorInput for VectorValueEnum {
  fn as_floats(&self) -> Result<&[f32]> {
    VectorValueEnum::as_floats(self)
  }
  fn as_bytes(&self) -> Result<&[u8]> {
    VectorValueEnum::as_bytes(self)
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
  pub fn copy_value(&self, offset: usize, dim: usize) -> VectorValueEnum {
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
  pub fn len(&self) -> usize {
    match self {
      Self::Byte(v) => v.len(),
      Self::Float(v) => v.len(),
    }
  }
  pub fn is_empty(&self) -> bool {
    match self {
      Self::Byte(v) => v.is_empty(),
      Self::Float(v) => v.is_empty(),
    }
  }
  pub fn as_bytes(&self) -> Result<&[u8]> {
    match self {
      Self::Byte(v) => Ok(v),
      Self::Float(_) => Err(LuceneError::unsupported_operation("")),
    }
  }
  pub fn as_floats(&self) -> Result<&[f32]> {
    match self {
      Self::Byte(_) => Err(LuceneError::unsupported_operation("")),
      Self::Float(v) => Ok(v),
    }
  }
  pub fn write_float(&self, chunk: &mut [u8]) -> Result<()> {
    match self {
      Self::Byte(_) => Err(LuceneError::unsupported_operation("")),
      Self::Float(v) => write_float_vector(v, chunk),
    }
  }
}

impl Accountable for VectorValueEnum {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(match self {
      Self::Byte(vector) => size_of_vec(vector),
      Self::Float(vector) => size_of_vec(vector),
    })
  }
}

pub(crate) fn write_float_vector(v: &[f32], chunk: &mut [u8]) -> Result<()> {
  let byte_len = v.len() * 4;
  debug_assert!(chunk.len() == byte_len);

  if v.len() > 16 && v.len() & 15 == 0 && chunk.len() >= byte_len {
    for (f, component) in v.iter().zip(chunk[..byte_len].as_chunks_mut::<4>().0) {
      let bytes = f.to_le_bytes();
      component.copy_from_slice(&bytes);
    }
  } else {
    let mut offset = 0;
    for f in v {
      let bytes = f.to_le_bytes();
      chunk[offset..offset + 4].copy_from_slice(&bytes);
      offset += 4;
    }
  }
  Ok(())
}
