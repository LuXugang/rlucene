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
use crate::core::document::field::{Field, FieldBase, FieldDataEnum};
use crate::core::document::field_type::FieldType;
use crate::core::document::invertable_field::InvertableType;
use crate::core::index::BytesRef;
use crate::core::index::indexable_field::{
  IndexableField, IndexingTokenStream, ReusedIndexingTokenStream,
};
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::search::point_in_set_query::{PointInSetBase, PointInSetQuery};
#[cfg(debug_assertions)]
use crate::core::search::point_range_query::check_args;
use crate::core::search::point_range_query::{PointRangeBase, PointRangeQuery};
use crate::core::util::bit_util::BitUtil;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
use crate::core::util::numeric_utils::NumericUtils;
use std::borrow::Cow;
use std::fmt;

pub struct LongPoint {
  parent_field: Field,
}

impl LongPoint {
  /// Create a new LongPoint with the given name and long values
  pub fn new<T, P>(name: T, point: P) -> Result<LongPoint>
  where
    T: Into<String>,
    P: AsRef<[i64]>,
  {
    let point = point.as_ref();
    let value = Self::pack(point)?;
    let field_type = Self::get_type(point.len())?;
    let parent_field = Field::from_bytes_ref(name, value, field_type)?;
    Ok(LongPoint { parent_field })
  }

  fn get_type(num_dims: usize) -> Result<FieldType> {
    let mut field_type = FieldType::new();
    field_type.set_dimensions(num_dims, BitUtil::LONG_BYTES)?;
    field_type.freeze();
    Ok(field_type)
  }

  /// Change the values of this field
  pub fn set_long_values<V>(&mut self, point: V) -> Result<()>
  where
    V: AsRef<[i64]>,
  {
    let point = point.as_ref();
    if self.parent_field.field_type().point_dimension_count() != point.len() {
      return Err(LuceneError::illegal_argument(format!(
        "this field (name={}) uses {} dimensions; cannot change to (incoming) {} dimensions",
        self.parent_field.name(),
        self.parent_field.field_type().point_dimension_count(),
        point.len()
      )));
    }
    let value = Self::pack(point)?;
    self.parent_field.fields_data = FieldDataEnum::Binary(value);
    Ok(())
  }

  /// Pack a long array into bytes
  pub fn pack<P>(point: P) -> Result<BytesRef<Vec<u8>>>
  where
    P: AsRef<[i64]>,
  {
    let point = point.as_ref();
    if point.is_empty() {
      return Err(LuceneError::illegal_argument(
        "point must not be 0 dimensions".to_string(),
      ));
    }
    let mut packed = vec![0u8; point.len() * BitUtil::LONG_BYTES];
    for (i, &dim) in point.iter().enumerate() {
      Self::encode_dimension(dim, &mut packed, i * BitUtil::LONG_BYTES);
    }
    Ok(BytesRef::from_bytes(packed))
  }

  /// Unpack bytes into a long array
  pub fn unpack(bytes_ref: &BytesRef<Vec<u8>>, start: usize, buf: &mut [i64]) {
    for (i, val) in buf.iter_mut().enumerate() {
      *val = Self::decode_dimension(&bytes_ref.bytes, start + i * BitUtil::LONG_BYTES);
    }
  }

  /// Encode single long dimension
  pub fn encode_dimension(value: i64, dest: &mut [u8], offset: usize) {
    NumericUtils::long_to_sortable_bytes(value, dest, offset);
  }

  /// Decode single long dimension
  pub fn decode_dimension(value: &[u8], offset: usize) -> i64 {
    NumericUtils::sortable_bytes_to_long(value, offset)
  }
  pub fn new_exact_query<T>(field: T, value: i64) -> Result<PointRangeQuery>
  where
    T: Into<String>,
  {
    Self::new_range_query(field, value, value)
  }
  pub fn new_range_query<T>(field: T, lower_value: i64, upper_value: i64) -> Result<PointRangeQuery>
  where
    T: Into<String>,
  {
    Self::new_range_query_n(field, [lower_value], [upper_value])
  }

  /// Create a query matching any of the specified 1D values. This is the points equivalent of
  /// `TermsQuery`.
  ///
  /// # Arguments
  ///
  /// * `field` - Field name. must not be `null`.
  /// * `values` - All values to match.
  pub fn new_set_query<T, V>(field: T, values: V) -> Result<PointInSetQuery>
  where
    T: Into<String>,
    V: AsRef<[i64]>,
  {
    let mut sorted_values = values.as_ref().to_vec();
    sorted_values.sort();

    PointInSetQuery::new(
      field.into(),
      1,
      BitUtil::LONG_BYTES,
      LongPointSetBytesRefIterator::new(sorted_values),
      LongPointInSetQuery,
    )
  }

  pub fn new_range_query_n<T, V>(
    field: T,
    lower_value: V,
    upper_value: V,
  ) -> Result<PointRangeQuery>
  where
    T: Into<String>,
    V: AsRef<[i64]>,
  {
    let field = field.into();
    let len = lower_value.as_ref().len();
    let mut lower_point = LongPoint::pack(lower_value.as_ref())?;
    let mut upper_point = LongPoint::pack(upper_value.as_ref())?;

    #[cfg(debug_assertions)]
    check_args(&field, &lower_point.bytes, &upper_point.bytes)?;

    PointRangeQuery::new(
      field,
      lower_point.take_bytes(),
      upper_point.take_bytes(),
      len, // numDims
      LongPointRangeQuery,
    )
  }
}

struct LongPointSetBytesRefIterator {
  sorted_values: Vec<i64>,
  upto: usize,
  encoded: BytesRef<Vec<u8>>,
}

impl LongPointSetBytesRefIterator {
  fn new(sorted_values: Vec<i64>) -> Self {
    Self {
      sorted_values,
      upto: 0,
      encoded: BytesRef::from_bytes(vec![0u8; BitUtil::LONG_BYTES]),
    }
  }
}

impl BytesRefIterator for LongPointSetBytesRefIterator {
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    if self.upto == self.sorted_values.len() {
      Ok(None)
    } else {
      LongPoint::encode_dimension(self.sorted_values[self.upto], &mut self.encoded.bytes, 0);
      self.upto += 1;
      Ok(Some(Cow::Borrowed(&self.encoded)))
    }
  }
}

impl FieldBase for LongPoint {
  fn set_bytes_value(&mut self, _value: BytesRef<Vec<u8>>) -> Result<()> {
    Err(LuceneError::illegal_argument(
      "cannot change value type from long to BytesRef",
    ))
  }

  fn set_long_value(&mut self, value: i64) -> Result<()> {
    self.set_long_values([value])
  }
}

impl IndexableField for LongPoint {
  fn name(&self) -> &str {
    self.parent_field.name()
  }

  type FieldType = FieldType;

  fn field_type(&self) -> &Self::FieldType {
    self.parent_field.field_type()
  }
  fn token_stream<'a, A>(
    &'a mut self,
    analyzer: &'a A,
    reuse_token_stream: &'a mut Option<ReusedIndexingTokenStream>,
  ) -> Result<IndexingTokenStream<'a>>
  where
    A: Analyzer,
  {
    self.parent_field.token_stream(analyzer, reuse_token_stream)
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
    todo!()
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
        debug_assert!(bytes.length == BitUtil::LONG_BYTES);
        let value = Self::decode_dimension(&bytes.bytes, bytes.offset);
        Ok(Some(Number::I64(value)))
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

impl fmt::Display for LongPoint {
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
            Self::decode_dimension(&bytes.bytes, bytes.offset + dim * BitUtil::LONG_BYTES);
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
pub struct LongPointRangeQuery;

impl PointRangeBase for LongPointRangeQuery {
  fn to_string(&self, _dimension: usize, value: &[u8]) -> Result<String> {
    Ok(LongPoint::decode_dimension(value, 0).to_string())
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LongPointInSetQuery;

impl PointInSetBase for LongPointInSetQuery {
  fn to_string(&self, value: &[u8]) -> Result<String> {
    debug_assert!(value.len() == BitUtil::LONG_BYTES);
    Ok(LongPoint::decode_dimension(value, 0).to_string())
  }
}

#[cfg(test)]
impl Clone for LongPoint {
  fn clone(&self) -> Self {
    Self {
      parent_field: self.parent_field.clone(),
    }
  }
}
