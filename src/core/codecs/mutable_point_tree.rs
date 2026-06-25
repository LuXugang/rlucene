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
use crate::core::index::point_values::{IntersectVisitor, PointTree};
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error;
use std::borrow::Cow;

/// One leaf [PointTree] whose order of points can be changed.
/// This trait is useful for codecs to optimize flush.
pub trait MutablePointTree: PointTree {
  /// Set `packed_value` with a reference to the packed bytes of the i-th
  /// value.
  fn get_value(&self, i: usize, packed_value: &mut BytesRef<Vec<u8>>) -> lucene_error::Result<()>;

  /// Get the k-th byte of the i-th value.
  fn get_byte_at(&self, i: usize, k: usize) -> u8;

  /// Return the doc ID of the i-th value.
  fn get_doc_id(&self, i: usize) -> lucene_error::Result<i32>;

  /// Swap the i-th and j-th values.
  fn swap(&mut self, i: usize, j: usize);

  /// Save the i-th value into the j-th position in temporary storage.
  fn save(&mut self, i: usize, j: usize);

  /// Restore values between i-th and j-th (excluding) in temporary storage
  /// into original storage.
  fn restore(&mut self, i: usize, j: usize);
}

// MutablePointTree
pub enum MutablePointTreeEnum2<A, B> {
  A(A),
  B(B),
}

impl<A, B> PointTree for MutablePointTreeEnum2<A, B>
where
  A: MutablePointTree,
  B: MutablePointTree,
{
  fn move_to_child(&mut self) -> lucene_error::Result<bool> {
    match self {
      MutablePointTreeEnum2::A(t) => t.move_to_child(),
      MutablePointTreeEnum2::B(s) => s.move_to_child(),
    }
  }

  fn move_to_sibling(&mut self) -> lucene_error::Result<bool> {
    match self {
      MutablePointTreeEnum2::A(t) => t.move_to_sibling(),
      MutablePointTreeEnum2::B(s) => s.move_to_sibling(),
    }
  }

  fn move_to_parent(&mut self) -> lucene_error::Result<bool> {
    match self {
      MutablePointTreeEnum2::A(t) => t.move_to_parent(),
      MutablePointTreeEnum2::B(s) => s.move_to_parent(),
    }
  }

  fn get_min_packed_value(&self) -> lucene_error::Result<Cow<'_, [u8]>> {
    match self {
      MutablePointTreeEnum2::A(t) => t.get_min_packed_value(),
      MutablePointTreeEnum2::B(s) => s.get_min_packed_value(),
    }
  }

  fn get_max_packed_value(&self) -> lucene_error::Result<Cow<'_, [u8]>> {
    match self {
      MutablePointTreeEnum2::A(t) => t.get_max_packed_value(),
      MutablePointTreeEnum2::B(s) => s.get_max_packed_value(),
    }
  }

  fn size(&self) -> lucene_error::Result<usize> {
    match self {
      MutablePointTreeEnum2::A(t) => t.size(),
      MutablePointTreeEnum2::B(s) => s.size(),
    }
  }

  fn visit_doc_ids<IV>(&mut self, visitor: &mut IV) -> lucene_error::Result<()>
  where
    IV: IntersectVisitor,
  {
    match self {
      MutablePointTreeEnum2::A(t) => t.visit_doc_ids(visitor),
      MutablePointTreeEnum2::B(s) => s.visit_doc_ids(visitor),
    }
  }

  fn visit_doc_values<IV>(&mut self, visitor: &mut IV) -> lucene_error::Result<()>
  where
    IV: IntersectVisitor,
  {
    match self {
      MutablePointTreeEnum2::A(t) => t.visit_doc_values(visitor),
      MutablePointTreeEnum2::B(s) => s.visit_doc_values(visitor),
    }
  }
}

impl<A, B> TryClone for MutablePointTreeEnum2<A, B>
where
  A: MutablePointTree,
  B: MutablePointTree,
{
  fn try_clone(&self) -> lucene_error::Result<Self>
  where
    Self: Sized,
  {
    match self {
      MutablePointTreeEnum2::A(t) => Ok(MutablePointTreeEnum2::A(t.try_clone()?)),
      MutablePointTreeEnum2::B(s) => Ok(MutablePointTreeEnum2::B(s.try_clone()?)),
    }
  }
}

impl<A, B> MutablePointTree for MutablePointTreeEnum2<A, B>
where
  A: MutablePointTree,
  B: MutablePointTree,
{
  fn get_value(&self, i: usize, packed_value: &mut BytesRef<Vec<u8>>) -> lucene_error::Result<()> {
    match self {
      MutablePointTreeEnum2::A(t) => t.get_value(i, packed_value),
      MutablePointTreeEnum2::B(s) => s.get_value(i, packed_value),
    }
  }

  fn get_byte_at(&self, i: usize, k: usize) -> u8 {
    match self {
      MutablePointTreeEnum2::A(t) => t.get_byte_at(i, k),
      MutablePointTreeEnum2::B(s) => s.get_byte_at(i, k),
    }
  }

  fn get_doc_id(&self, i: usize) -> lucene_error::Result<i32> {
    match self {
      MutablePointTreeEnum2::A(t) => t.get_doc_id(i),
      MutablePointTreeEnum2::B(s) => s.get_doc_id(i),
    }
  }

  fn swap(&mut self, i: usize, j: usize) {
    match self {
      MutablePointTreeEnum2::A(t) => t.swap(i, j),
      MutablePointTreeEnum2::B(s) => s.swap(i, j),
    }
  }

  fn save(&mut self, i: usize, j: usize) {
    match self {
      MutablePointTreeEnum2::A(t) => t.save(i, j),
      MutablePointTreeEnum2::B(s) => s.save(i, j),
    }
  }

  fn restore(&mut self, i: usize, j: usize) {
    match self {
      MutablePointTreeEnum2::A(t) => t.restore(i, j),
      MutablePointTreeEnum2::B(s) => s.restore(i, j),
    }
  }
}
