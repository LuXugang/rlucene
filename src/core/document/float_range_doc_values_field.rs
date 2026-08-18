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
use crate::core::analysis::analyzer::Analyzer;
use crate::core::analysis::reader::ReaderEnum;
use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
use crate::core::document::binary_range_field_range_query::BinaryRangeFieldRangeQuery;
use crate::core::document::field::{FieldBase, FieldDataEnum};
use crate::core::document::field_type::FieldType;
use crate::core::document::float_range;
use crate::core::document::float_range_slow_range_query::FloatRangeSlowRangeQuery;
use crate::core::document::invertable_field::InvertableType;
use crate::core::document::range_field_query::QueryType;
use crate::core::index::BytesRef;
use crate::core::index::indexable_field::{
  IndexableField, IndexingTokenStream, ReusedIndexingTokenStream,
};
use crate::core::util::CoreHelper;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};

/// DocValues field for FloatRange. This is a single valued field per document due to wrapping a
/// BinaryDocValuesField.
pub struct FloatRangeDocValuesField {
  pub(crate) base: BinaryDocValuesField,
  #[allow(dead_code)] // Mirrors Java's package-private field for source and test API fidelity.
  pub(crate) field: String,
  pub(crate) min: Vec<f32>,
  pub(crate) max: Vec<f32>,
}

impl FloatRangeDocValuesField {
  /// Creates a new instance.
  pub fn new<T, P>(field: T, min: P, max: P) -> Result<Self>
  where
    T: Into<String>,
    P: AsRef<[f32]>,
  {
    let field = field.into();
    let min = min.as_ref();
    let max = max.as_ref();
    Self::check_args(min, max)?;
    let encoded = float_range::encode_range(min, max)?;
    let base = BinaryDocValuesField::new(field.clone(), BytesRef::from_bytes(encoded));
    Ok(Self {
      base,
      field,
      min: min.to_vec(),
      max: max.to_vec(),
    })
  }

  /// Get the minimum value for the given dimension.
  pub fn get_min(&self, dimension: usize) -> Result<f32> {
    CoreHelper::check_index(dimension, self.min.len())?;
    Ok(self.min[dimension])
  }

  /// Get the maximum value for the given dimension.
  pub fn get_max(&self, dimension: usize) -> Result<f32> {
    CoreHelper::check_index(dimension, self.max.len())?;
    Ok(self.max[dimension])
  }

  fn new_slow_range_query<T, P>(
    field: T,
    min: P,
    max: P,
    query_type: QueryType,
  ) -> Result<BinaryRangeFieldRangeQuery>
  where
    T: Into<String>,
    P: AsRef<[f32]>,
  {
    let min = min.as_ref();
    let max = max.as_ref();
    Self::check_args(min, max)?;
    FloatRangeSlowRangeQuery::new(field.into(), min.to_vec(), max.to_vec(), query_type)
  }

  /// Create a new range query that finds all ranges that intersect using doc values. NOTE: This
  /// doesn't leverage indexing and may be slow.
  pub fn new_slow_intersects_query<T, P>(
    field: T,
    min: P,
    max: P,
  ) -> Result<BinaryRangeFieldRangeQuery>
  where
    T: Into<String>,
    P: AsRef<[f32]>,
  {
    Self::new_slow_range_query(field, min, max, QueryType::Intersects)
  }

  /// Validate the arguments.
  fn check_args(min: &[f32], max: &[f32]) -> Result<()> {
    if min.is_empty() || max.is_empty() {
      return Err(LuceneError::illegal_argument(
        "min/max range values cannot be null or empty",
      ));
    }
    if min.len() != max.len() {
      return Err(LuceneError::illegal_argument("min/max ranges must agree"));
    }
    for i in 0..min.len() {
      if min[i] > max[i] {
        return Err(LuceneError::illegal_argument("min should be less than max"));
      }
    }
    Ok(())
  }
}

impl FieldBase for FloatRangeDocValuesField {
  fn set_bytes_value(&mut self, value: BytesRef<Vec<u8>>) -> Result<()> {
    self.base.set_bytes_value(value)
  }
}

impl IndexableField for FloatRangeDocValuesField {
  fn name(&self) -> &str {
    self.base.name()
  }

  type FieldType<'a>
    = &'a FieldType
  where
    Self: 'a;

  fn field_type(&self) -> Self::FieldType<'_> {
    self.base.field_type()
  }

  fn token_stream<'a, A>(
    &'a mut self,
    analyzer: &'a A,
    reuse_token_stream: &'a mut Option<ReusedIndexingTokenStream>,
  ) -> Result<IndexingTokenStream<'a>>
  where
    A: Analyzer,
  {
    self.base.token_stream(analyzer, reuse_token_stream)
  }

  fn binary_value(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    self.base.binary_value()
  }

  fn take_binary_value(&mut self) -> Result<Option<BytesRef<Vec<u8>>>> {
    self.base.take_binary_value()
  }

  fn string_value(&self) -> Result<Option<Cow<'_, String>>> {
    self.base.string_value()
  }

  fn take_string_value(&mut self) -> Result<Option<String>> {
    self.base.take_string_value()
  }

  fn take_reader_value(&mut self) -> Result<Option<ReaderEnum>> {
    self.base.take_reader_value()
  }

  fn numeric_value(&self) -> Result<Option<Number>> {
    self.base.numeric_value()
  }

  fn stored_value(&self) -> Option<FieldDataEnum> {
    self.base.stored_value()
  }

  fn invertable_type(&self) -> &InvertableType {
    self.base.invertable_type()
  }
}

impl Display for FloatRangeDocValuesField {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    self.base.fmt(f)
  }
}

#[cfg(test)]
impl Clone for FloatRangeDocValuesField {
  fn clone(&self) -> Self {
    Self {
      base: self.base.clone(),
      field: self.field.clone(),
      min: self.min.clone(),
      max: self.max.clone(),
    }
  }
}
