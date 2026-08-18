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
use crate::core::index::BytesRef;
use crate::core::index::binary_doc_values::BinaryDocValues;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;

/// A binary representation of a range that wraps a BinaryDocValues field
pub struct BinaryRangeDocValues<T> {
  in_: T,
  packed_value: Vec<u8>,
  num_dims: usize,
  num_bytes_per_dimension: usize,
  doc_id: i32,
}

impl<T: BinaryDocValues> BinaryRangeDocValues<T> {
  /// Creates BinaryRangeDocValues
  ///
  /// - `inner`: the binary doc values source field
  /// - `num_dims`: the number of dimensions in each doc values field
  /// - `num_bytes_per_dimension`: size of each dimension (2 * encoded value size)
  pub fn new(inner: T, num_dims: usize, num_bytes_per_dimension: usize) -> Self {
    let packed_value = vec![0u8; 2 * num_dims * num_bytes_per_dimension];
    Self {
      in_: inner,
      packed_value,
      num_dims,
      num_bytes_per_dimension,
      doc_id: -1,
    }
  }

  /// Gets the packed value that represents this range
  pub fn get_packed_value(&self) -> &[u8] {
    &self.packed_value
  }

  fn decode_ranges(&mut self) -> Result<()> {
    let bytes_ref = self.in_.binary_value()?;
    let len = 2 * self.num_dims * self.num_bytes_per_dimension;
    let src = &bytes_ref.bytes[bytes_ref.offset..bytes_ref.offset + len];
    self.packed_value[..len].copy_from_slice(src);
    Ok(())
  }
}

impl<T: BinaryDocValues> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for BinaryRangeDocValues<T>
{
}
impl<T: BinaryDocValues> DocIdSetIterator for BinaryRangeDocValues<T> {
  fn doc_id(&self) -> i32 {
    self.in_.doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.doc_id = self.in_.next_doc()?;
    if self.doc_id != NO_MORE_DOCS {
      self.decode_ranges()?;
    }
    Ok(self.doc_id)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    let res = self.in_.advance(target)?;
    if res != NO_MORE_DOCS {
      self.decode_ranges()?;
    }
    Ok(res)
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.in_.slow_advance(target)
  }

  fn cost(&self) -> Result<i64> {
    self.in_.cost()
  }
}

impl<T: BinaryDocValues> DocValuesIterator for BinaryRangeDocValues<T> {
  fn advance_exact(&mut self, target: i32) -> Result<bool> {
    let res = self.in_.advance_exact(target)?;
    if res {
      self.decode_ranges()?;
    }
    Ok(res)
  }
}

impl<T: BinaryDocValues> BinaryDocValues for BinaryRangeDocValues<T> {
  fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    self.in_.binary_value()
  }
}
