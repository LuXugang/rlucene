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

use crate::core::index::index_reader::Identity;
use crate::core::store::random_access_input::RandomAccessInput;
use crate::core::util::HasIdentity;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::long_values::LongValues;
use crate::core::util::packed::direct_monotonic_reader::DirectMonotonicReader;
use std::cell::RefCell;
use std::rc::Rc;

pub struct SparseOffHeapVectorValueBits<B, R> {
  accept_docs: B,
  size: usize,
  map: Rc<RefCell<DirectMonotonicReader<R>>>,
  id: Identity,
}

impl<B, R> SparseOffHeapVectorValueBits<B, R> {
  pub(crate) fn new(
    accept_docs: B,
    size: usize,
    map: Rc<RefCell<DirectMonotonicReader<R>>>,
  ) -> Self {
    Self {
      accept_docs,
      size,
      map,
      id: Identity::new(),
    }
  }
}

impl<B, R> HasIdentity for SparseOffHeapVectorValueBits<B, R> {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl<B, R> Bits for SparseOffHeapVectorValueBits<B, R>
where
  B: Bits,
  R: RandomAccessInput,
{
  fn get(&self, index: usize) -> Result<bool> {
    let index = self.map.borrow_mut().get_mut(index)? as usize;
    self.accept_docs.get(index)
  }

  fn length(&self) -> usize {
    self.size
  }
}

pub enum OffHeapVectorValueBits<R, B> {
  Dense(B),
  Sparse(SparseOffHeapVectorValueBits<B, R>),
}

impl<R, B> HasIdentity for OffHeapVectorValueBits<R, B>
where
  B: HasIdentity,
{
  fn identity(&self) -> &Identity {
    match self {
      Self::Dense(bits) => bits.identity(),
      Self::Sparse(bits) => bits.identity(),
    }
  }
}

impl<R, B> Bits for OffHeapVectorValueBits<R, B>
where
  R: RandomAccessInput,
  B: Bits,
{
  fn get(&self, index: usize) -> Result<bool> {
    match self {
      Self::Dense(bits) => bits.get(index),
      Self::Sparse(bits) => bits.get(index),
    }
  }

  fn length(&self) -> usize {
    match self {
      Self::Dense(bits) => bits.length(),
      Self::Sparse(bits) => bits.length(),
    }
  }

  fn copy_of(&self) -> Result<FixedBitSet> {
    match self {
      Self::Dense(bits) => bits.copy_of(),
      Self::Sparse(bits) => bits.copy_of(),
    }
  }

  fn to_string(&self) -> String {
    match self {
      Self::Dense(bits) => bits.to_string(),
      Self::Sparse(bits) => bits.to_string(),
    }
  }
}
