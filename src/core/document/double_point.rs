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
use crate::core::analysis::token_stream::AnalyzerTokenStreams;
use crate::core::document::field::{Field, FieldBase, FieldDataEnum};
use crate::core::document::field_type::FieldType;
use crate::core::document::invertable_field::InvertableType;
use crate::core::index::BytesRef;
use crate::core::index::indexable_field::{
  IndexableField, IndexingTokenStream, ReusedIndexingTokenStream,
};
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::search::point_range_query::{PointRangeBase, PointRangeQuery, check_args};
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
use crate::core::util::numeric_utils::NumericUtils;
use std::borrow::Cow;
use std::fmt;

pub struct DoublePoint {
  parent_field: Field,
}
impl DoublePoint {
  pub fn new<T, P>(name: T, point: P) -> Result<DoublePoint>
  where
    T: Into<String>,
    P: AsRef<[f64]>,
  {
    let point = point.as_ref();
    let value = Self::pack(point)?;
    let field_type = Self::get_type(point.len())?;
    let parent_field = Field::from_bytes_ref(name, value, field_type)?;
    Ok(DoublePoint { parent_field })
  }
  pub fn next_up(d: f64) -> f64 {
    // -0.0d
    if d.to_bits() == 0x8000_0000_0000_0000u64 {
      0.0
    } else {
      d.next_up()
    }
  }

  pub fn next_down(d: f64) -> f64 {
    if d.to_bits() == 0u64 {
      -0.0
    } else {
      d.next_down()
    }
  }

  fn get_type(num_dims: usize) -> Result<FieldType> {
    let mut field_type = FieldType::new();
    field_type.set_dimensions(num_dims, BitUtil::DOUBLE_BYTES)?;
    field_type.freeze();
    Ok(field_type)
  }
  /// Change the values of this field
  pub fn set_double_values(&mut self, point: &[f64]) -> Result<()> {
    if self.parent_field.field_type().point_dimension_count() != point.len() {
      return Err(LuceneError::illegal_argument(format!(
        "this field (name={}) uses {} dimensions; cannot change to (incoming) {} dimensions",
        self.parent_field.name(),
        self.parent_field.field_type().point_dimension_count(),
        point.len()
      )));
    }
    let value = Self::pack(point)?;
    self.parent_field.fields_data = value.into();
    Ok(())
  }
  fn pack(point: &[f64]) -> Result<BytesRef<Vec<u8>>> {
    if point.is_empty() {
      return Err(LuceneError::illegal_argument(
        "point must not be 0 dimensions".to_string(),
      ));
    }
    let mut packed = vec![0; point.len() * BitUtil::DOUBLE_BYTES];
    for (i, &dim) in point.iter().enumerate() {
      Self::encode_dimension(dim, &mut packed, i * BitUtil::DOUBLE_BYTES);
    }
    Ok(BytesRef::from_bytes(packed))
  }
  /// Encode a single double dimension into byte array
  pub fn encode_dimension(value: f64, dest: &mut [u8], offset: usize) {
    NumericUtils::long_to_sortable_bytes(
      NumericUtils::double_to_sortable_long(value),
      dest,
      offset,
    );
  }

  /// Decode a single double dimension from byte array
  pub fn decode_dimension(value: &[u8], offset: usize) -> f64 {
    NumericUtils::sortable_long_to_double(NumericUtils::sortable_bytes_to_long(value, offset))
  }
  pub fn new_exact_query<T>(field: T, value: f64) -> Result<PointRangeQuery>
  where
    T: Into<String>,
  {
    Self::new_range_query(field, value, value)
  }

  pub fn new_range_query<T>(field: T, lower_value: f64, upper_value: f64) -> Result<PointRangeQuery>
  where
    T: Into<String>,
  {
    Self::new_range_query_n(field, [lower_value], [upper_value])
  }

  pub fn new_range_query_n<T, V>(
    field: T,
    lower_value: V,
    upper_value: V,
  ) -> Result<PointRangeQuery>
  where
    T: Into<String>,
    V: AsRef<[f64]>,
  {
    let field = field.into();
    let len = lower_value.as_ref().len();
    let mut lower_point = DoublePoint::pack(lower_value.as_ref())?;
    let mut upper_point = DoublePoint::pack(upper_value.as_ref())?;
    check_args(&field, &lower_point.bytes, &upper_point.bytes)?;

    PointRangeQuery::new(
      field,
      lower_point.take_bytes(),
      upper_point.take_bytes(),
      len,
      DoublePointRangeQuery,
    )
  }
}
impl FieldBase for DoublePoint {
  fn set_bytes_value(&mut self, _value: BytesRef<Vec<u8>>) -> Result<()> {
    Err(LuceneError::illegal_argument(
      "cannot change value type from double to BytesRef",
    ))
  }

  fn set_double_value(&mut self, value: f64) -> Result<()> {
    self.set_double_values(&[value])
  }
}
impl IndexableField for DoublePoint {
  fn name(&self) -> &str {
    self.parent_field.name()
  }

  type FieldType = FieldType;

  fn field_type(&self) -> &Self::FieldType {
    self.parent_field.field_type()
  }
  fn token_stream<'a>(
    &'a mut self,
    token_stream: Option<&'a mut AnalyzerTokenStreams>,
    reuse_token_stream: &'a mut Option<ReusedIndexingTokenStream>,
  ) -> Result<IndexingTokenStream<'a>> {
    self
      .parent_field
      .token_stream(token_stream, reuse_token_stream)
  }

  fn binary_value(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    self.parent_field.binary_value()
  }

  fn take_binary_value(&mut self) -> Result<Option<BytesRef<Vec<u8>>>> {
    self.parent_field.take_binary_value()
  }

  fn string_value(&self) -> Result<Option<Cow<'_, String>>> {
    self.parent_field.string_value()
  }

  fn take_string_value(&mut self) -> Result<Option<String>> {
    self.parent_field.take_string_value()
  }

  fn take_reader_value(&mut self) -> Result<Option<ReaderEnum>> {
    self.parent_field.take_reader_value()
  }

  fn numeric_value(&self) -> Result<Option<Number>> {
    if self.parent_field.field_type().point_dimension_count() != 1 {
      return Err(LuceneError::illegal_state(format!(
        "this field (name={}) uses {} dimensions; cannot convert to a single numeric value",
        self.parent_field.name(),
        self.parent_field.field_type().point_dimension_count()
      )));
    }
    match &self.parent_field.fields_data {
      FieldDataEnum::Binary(bytes) => {
        debug_assert!(bytes.length == BitUtil::DOUBLE_BYTES);
        let value = Self::decode_dimension(&bytes.bytes, bytes.offset);
        Ok(Some(Number::F64(value)))
      },
      _ => {
        debug_assert!(false, "no possible here");
        Ok(None)
      },
    }
  }

  fn stored_value(&self) -> Option<&FieldDataEnum> {
    self.parent_field.stored_value()
  }

  fn take_stored_value(&mut self) -> Option<FieldDataEnum> {
    self.parent_field.take_stored_value()
  }

  fn invertable_type(&self) -> &InvertableType {
    self.parent_field.invertable_type()
  }

  fn init_token_stream<A>(&mut self, analyzer: &A) -> Result<()>
  where
    A: Analyzer,
  {
    self.parent_field.init_token_stream(analyzer)
  }
}
impl fmt::Display for DoublePoint {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{} <{}:",
      std::any::type_name::<Self>(),
      self.parent_field.name()
    )?;

    match &self.parent_field.fields_data {
      FieldDataEnum::Binary(bytes) => {
        let dim_count = self.parent_field.field_type().point_dimension_count();
        for dim in 0..dim_count {
          if dim > 0 {
            write!(f, ",")?;
          }
          let value =
            Self::decode_dimension(&bytes.bytes, bytes.offset + dim * BitUtil::DOUBLE_BYTES);
          write!(f, "{value}")?;
        }
      },
      _ => {
        debug_assert!(false, "no possible here");
        write!(f, "Unsupported FieldDataEnum variant")?;
      },
    }

    write!(f, ">")
  }
}

#[derive(Debug, Clone)]
pub struct DoublePointRangeQuery;

impl PointRangeBase for DoublePointRangeQuery {
  fn to_string(&self, _dimension: usize, value: &[u8]) -> Result<String> {
    Ok(DoublePoint::decode_dimension(value, 0).to_string())
  }
}

#[cfg(test)]
impl Clone for DoublePoint {
  fn clone(&self) -> Self {
    Self {
      parent_field: self.parent_field.clone(),
    }
  }
}
