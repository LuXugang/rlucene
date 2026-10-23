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

use crate::core::codecs::knn_field_vectors_writer::VectorInput;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::util::{
  array_util::ArrayUtil,
  error::lucene_error::{LuceneError, Result},
  ram_usage_estimator::size_of_vec,
};
use std::sync::Arc;

/// Scalar type fixed by a buffered vector field's encoding.
pub trait VectorElement: Clone + Send + Sync + 'static {
  fn copy_value<V: VectorInput + ?Sized>(value: &V, dimension: usize) -> Result<Box<[Self]>>;
}
impl VectorElement for f32 {
  fn copy_value<V: VectorInput + ?Sized>(value: &V, dimension: usize) -> Result<Box<[Self]>> {
    Ok(ArrayUtil::copy_of_sub_array(value.as_floats()?, 0, dimension).into_boxed_slice())
  }
}
impl VectorElement for u8 {
  fn copy_value<V: VectorInput + ?Sized>(value: &V, dimension: usize) -> Result<Box<[Self]>> {
    Ok(ArrayUtil::copy_of_sub_array(value.as_bytes()?, 0, dimension).into_boxed_slice())
  }
}

/// Borrowed field buffers. Select once before iterating the concrete rows.
#[derive(Clone, Copy)]
pub enum VectorSlices<'a> {
  Byte(&'a [Box<[u8]>]),
  Float(&'a [Box<[f32]>]),
}
impl<'a> VectorSlices<'a> {
  pub fn as_floats(self) -> Result<&'a [Box<[f32]>]> {
    match self {
      Self::Float(v) => Ok(v),
      Self::Byte(_) => Err(LuceneError::unsupported_operation("")),
    }
  }
}

/// Whole-field snapshots retain the existing copy-on-write ownership contract.
#[derive(Clone)]
pub enum VectorBuffers {
  Byte(Arc<Vec<Box<[u8]>>>),
  Float(Arc<Vec<Box<[f32]>>>),
}
impl VectorBuffers {
  pub fn new(encoding: VectorEncoding) -> Self {
    match encoding {
      VectorEncoding::BYTE(_) => Self::Byte(Arc::new(Vec::new())),
      VectorEncoding::FLOAT32(_) => Self::Float(Arc::new(Vec::new())),
    }
  }
  pub fn as_slices(&self) -> VectorSlices<'_> {
    match self {
      Self::Byte(v) => VectorSlices::Byte(v),
      Self::Float(v) => VectorSlices::Float(v),
    }
  }
  pub fn shallow_buffer_bytes(&self) -> i64 {
    match self {
      Self::Byte(v) => std::mem::size_of_val(v.as_ref()) as i64 + size_of_vec(v.as_ref()),
      Self::Float(v) => std::mem::size_of_val(v.as_ref()) as i64 + size_of_vec(v.as_ref()),
    }
  }
}
