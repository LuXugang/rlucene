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
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::term::Term;
use crate::core::store::DataOutput;
use crate::core::util::error::lucene_error::Result;
use std::fmt::Display;
use std::sync::Arc;

/// An in-place update to a DocValues field.
pub struct DocValuesUpdate {
  pub(crate) doc_values_type: DocValuesType,
  pub term: Arc<Term>,
  pub field: String,
  // used in BufferedDeletes to apply this update only to a slice of docs.
  // It's initialized to BufferedUpdates.MAX_INT
  // since it's safe and most often used this way we save object creations.
  pub doc_id_upto: i32,
  pub has_value: bool,
  pub sub_update: DocValuesUpdateEnum,
}
impl DocValuesUpdate {
  #[allow(dead_code)] // Mirrors Java's retained sizeInBytes accounting path, which has no current callers.
  const RAW_SIZE_IN_BYTES: i32 = 0;
  pub fn new<T, F>(
    doc_values_type: DocValuesType,
    term: F,
    field: T,
    doc_id_upto: i32,
    sub_update: DocValuesUpdateEnum,
  ) -> Self
  where
    T: Into<String>,
    F: Into<Arc<Term>>,
  {
    let field = field.into();
    let term = term.into();
    debug_assert!(doc_id_upto >= 0, "{doc_id_upto} must be >= 0");
    let has_value = sub_update.has_value();
    DocValuesUpdate {
      doc_values_type,
      term,
      field,
      doc_id_upto,
      has_value,
      sub_update,
    }
  }

  pub(crate) fn has_value(&self) -> bool {
    self.has_value
  }
  #[allow(dead_code)] // Mirrors Java's retained sizeInBytes method, which has no current callers.
  fn size_in_bytes(&self) -> i32 {
    unimplemented!("Retained for Java parity, but there is no current caller")
  }
  #[cfg(test)]
  pub fn prepare_for_apply(&mut self, doc_id_upto: i32) -> Option<DocValuesUpdate> {
    if doc_id_upto == self.doc_id_upto {
      return None;
    }
    let sub_update = self.sub_update.prepare_for_apply();
    Some(DocValuesUpdate::new(
      self.doc_values_type,
      self.term.clone(),
      self.field.clone(),
      doc_id_upto,
      sub_update,
    ))
  }
}
impl Display for DocValuesUpdate {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "term={}, field={}, value={}, docIDUpTo={}",
      self.term,
      self.field,
      self.sub_update.value_to_string(),
      self.doc_id_upto
    )
  }
}
pub trait DocValuesUpdateBase {
  #[allow(dead_code)] // Mirrors Java's retained valueSizeInBytes path, which is only called by the unused sizeInBytes method.
  fn value_size_in_bytes(&self) -> i64 {
    unimplemented!("Retained for Java parity, but there is no current caller")
  }
  fn value_to_string(&self) -> String;
  #[allow(dead_code)] // Mirrors Java's retained writeTo method, which has no current callers.
  fn write_to<D>(&self, _bytes: &mut BytesRef<Vec<u8>>) -> Result<()>
  where
    D: DataOutput,
  {
    unimplemented!("Retained for Java parity, but there is no current caller")
  }
  fn has_value(&self) -> bool;
  #[cfg(test)]
  fn prepare_for_apply(&mut self) -> DocValuesUpdateEnum;
}
/// An in-place update to a binary DocValues field.
pub struct BinaryDocValuesUpdate {
  value: Option<BytesRef<Vec<u8>>>,
}
impl BinaryDocValuesUpdate {
  #[allow(dead_code)] // Mirrors Java's retained valueSizeInBytes accounting path, which has no current callers.
  const RAW_VALUE_SIZE_IN_BYTES: i32 = 0;
  pub fn new(value: Option<BytesRef<Vec<u8>>>) -> Self {
    BinaryDocValuesUpdate { value }
  }
  pub fn get_value(&self) -> &BytesRef<Vec<u8>> {
    debug_assert!(self.value.is_some());
    self.value.as_ref().unwrap()
  }
}
impl DocValuesUpdateBase for BinaryDocValuesUpdate {
  fn value_to_string(&self) -> String {
    match &self.value {
      Some(v) => v.to_string(),
      None => "null".to_string(),
    }
  }

  fn has_value(&self) -> bool {
    self.value.is_some()
  }

  #[cfg(test)]
  fn prepare_for_apply(&mut self) -> DocValuesUpdateEnum {
    DocValuesUpdateEnum::Binary(BinaryDocValuesUpdate::new(self.value.clone()))
  }
}
#[derive(Clone)]
pub struct NumericDocValuesUpdate {
  value: Option<i64>,
}
impl NumericDocValuesUpdate {
  pub fn new(value: Option<i64>) -> Self {
    NumericDocValuesUpdate { value }
  }
  pub fn get_value(&self) -> i64 {
    debug_assert!(
      self.value.is_some(),
      "getValue should only be called if this update has a value"
    );
    *self.value.as_ref().unwrap()
  }
}
impl DocValuesUpdateBase for NumericDocValuesUpdate {
  fn value_to_string(&self) -> String {
    match self.value {
      Some(v) => v.to_string(),
      None => "null".to_string(),
    }
  }

  fn has_value(&self) -> bool {
    self.value.is_some()
  }

  #[cfg(test)]
  fn prepare_for_apply(&mut self) -> DocValuesUpdateEnum {
    DocValuesUpdateEnum::Numeric(NumericDocValuesUpdate::new(self.value))
  }
}

pub enum DocValuesUpdateEnum {
  Binary(BinaryDocValuesUpdate),
  Numeric(NumericDocValuesUpdate),
}
impl DocValuesUpdateEnum {
  pub fn get_binary(&self) -> Option<&BinaryDocValuesUpdate> {
    debug_assert!(matches!(self, DocValuesUpdateEnum::Binary(_)));
    match self {
      DocValuesUpdateEnum::Binary(b) => Some(b),
      _ => None,
    }
  }

  pub fn get_numeric(&self) -> Option<&NumericDocValuesUpdate> {
    debug_assert!(matches!(self, DocValuesUpdateEnum::Numeric(_)));
    match self {
      DocValuesUpdateEnum::Numeric(n) => Some(n),
      _ => None,
    }
  }
}
impl DocValuesUpdateBase for DocValuesUpdateEnum {
  fn value_to_string(&self) -> String {
    match self {
      DocValuesUpdateEnum::Binary(b) => b.value_to_string(),
      DocValuesUpdateEnum::Numeric(n) => n.value_to_string(),
    }
  }

  fn has_value(&self) -> bool {
    match self {
      DocValuesUpdateEnum::Binary(b) => b.has_value(),
      DocValuesUpdateEnum::Numeric(n) => n.has_value(),
    }
  }

  #[cfg(test)]
  fn prepare_for_apply(&mut self) -> DocValuesUpdateEnum {
    match self {
      DocValuesUpdateEnum::Binary(b) => b.prepare_for_apply(),
      DocValuesUpdateEnum::Numeric(n) => n.prepare_for_apply(),
    }
  }
}
