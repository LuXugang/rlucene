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
use crate::core::util::bkd::heap_point_write::HeapPointValue;
use crate::core::util::bkd::offline_point_reader::OfflinePointValue;

/// Represents a dimensional point value written in the BKD tree.
pub(crate) trait PointValue {
  /// Returns the packed values for the dimensions.
  fn packed_value(&self) -> BytesRef<&[u8]>;

  /// Returns the docID.
  fn doc_id(&self) -> i32;

  /// Returns the byte representation of the packed value together with the
  /// docID.
  fn packed_value_doc_id_bytes(&self) -> BytesRef<&[u8]>;
}

/// Borrowed view used only by the heterogeneous reader adapter.
pub(crate) enum PointValueRef<'a> {
  Heap(&'a HeapPointValue),
  Offline(&'a OfflinePointValue),
}

impl<'a> PointValueRef<'a> {
  pub fn packed_value(&self) -> BytesRef<&'a [u8]> {
    match *self {
      Self::Heap(value) => value.packed_value(),
      Self::Offline(value) => value.packed_value(),
    }
  }
  pub fn doc_id(&self) -> i32 {
    match *self {
      Self::Heap(value) => value.doc_id(),
      Self::Offline(value) => value.doc_id(),
    }
  }
  pub fn packed_value_doc_id_bytes(&self) -> BytesRef<&'a [u8]> {
    match *self {
      Self::Heap(value) => value.packed_value_doc_id_bytes(),
      Self::Offline(value) => value.packed_value_doc_id_bytes(),
    }
  }
}

impl PointValue for PointValueRef<'_> {
  fn packed_value(&self) -> BytesRef<&[u8]> {
    self.packed_value()
  }
  fn doc_id(&self) -> i32 {
    self.doc_id()
  }
  fn packed_value_doc_id_bytes(&self) -> BytesRef<&[u8]> {
    self.packed_value_doc_id_bytes()
  }
}

impl<V: PointValue + ?Sized> PointValue for &V {
  fn packed_value(&self) -> BytesRef<&[u8]> {
    (**self).packed_value()
  }
  fn doc_id(&self) -> i32 {
    (**self).doc_id()
  }
  fn packed_value_doc_id_bytes(&self) -> BytesRef<&[u8]> {
    (**self).packed_value_doc_id_bytes()
  }
}
