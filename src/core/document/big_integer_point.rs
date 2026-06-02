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
#[cfg(debug_assertions)]
use crate::core::search::point_range_query::check_args;
use crate::core::search::point_range_query::{PointRangeBase, PointRangeQuery};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
use crate::core::util::numeric_utils::NumericUtils;
use num_bigint::BigInt;
use std::borrow::Cow;
use std::fmt;

/// An indexed 128-bit `BigInt` field.
///
/// Finding all documents within an N-dimensional shape or range at search time is efficient.
/// Multiple values for the same field in one document is allowed.
///
/// This field defines static factory methods for creating common queries:
///
/// - [`new_exact_query`](Self::new_exact_query) for matching an exact 1D point.
/// - [`new_range_query`](Self::new_range_query) for matching a 1D range.
/// - [`new_range_query_n`](Self::new_range_query_n) for matching points/ranges in
///   n-dimensional space.
pub struct BigIntegerPoint {
  parent_field: Field,
}

impl BigIntegerPoint {
  /// The number of bytes per dimension: 128 bits.
  pub const BYTES: usize = 16;

  /// A value holding the minimum value a BigIntegerPoint can have, -2^127.
  pub fn min_value() -> BigInt {
    -(BigInt::from(1) << (Self::BYTES * 8 - 1))
  }

  /// A value holding the maximum value a BigIntegerPoint can have, 2^127-1.
  pub fn max_value() -> BigInt {
    (BigInt::from(1) << (Self::BYTES * 8 - 1)) - BigInt::from(1)
  }

  fn get_type(num_dims: usize) -> Result<FieldType> {
    let mut ty = FieldType::new();
    ty.set_dimensions(num_dims, Self::BYTES)?;
    ty.freeze();
    Ok(ty)
  }

  /// Change the values of this field.
  pub fn set_big_integer_values<P>(&mut self, point: P) -> Result<()>
  where
    P: AsRef<[BigInt]>,
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
    self.parent_field.fields_data = FieldDataEnum::Binary(Self::pack(point)?);
    Ok(())
  }

  fn pack<P>(point: P) -> Result<BytesRef<Vec<u8>>>
  where
    P: AsRef<[BigInt]>,
  {
    let point = point.as_ref();
    if point.is_empty() {
      return Err(LuceneError::illegal_argument(
        "point must not be 0 dimensions".to_string(),
      ));
    }
    let mut packed = vec![0u8; point.len() * Self::BYTES];
    for (dim, value) in point.iter().enumerate() {
      Self::encode_dimension(value, &mut packed, dim * Self::BYTES)?;
    }
    Ok(BytesRef::from_bytes(packed))
  }

  /// Creates a new BigIntegerPoint, indexing the provided N-dimensional big integer point.
  ///
  /// # Arguments
  ///
  /// - `name` - Field name.
  /// - `point` - BigInteger values.
  ///
  /// # Errors
  ///
  /// Returns an error if the field name or value is invalid.
  pub fn new<T, P>(name: T, point: P) -> Result<Self>
  where
    T: Into<String>,
    P: AsRef<[BigInt]>,
  {
    let point = point.as_ref();
    let packed = Self::pack(point)?;
    let parent_field = Field::from_bytes_ref(name, packed, Self::get_type(point.len())?)?;
    Ok(Self { parent_field })
  }

  /// Encode single BigInteger dimension.
  pub fn encode_dimension(value: &BigInt, dest: &mut [u8], offset: usize) -> Result<()> {
    NumericUtils::big_int_to_sortable_bytes(value, Self::BYTES, dest, offset)
  }

  /// Decode single BigInteger dimension.
  pub fn decode_dimension(value: &[u8], offset: usize) -> Result<BigInt> {
    NumericUtils::sortable_bytes_to_big_int(value, offset, Self::BYTES)
  }

  /// Create a query for matching an exact big integer value.
  ///
  /// This is for simple one-dimension points, for multidimensional points use
  /// [`new_range_query_n`](Self::new_range_query_n) instead.
  pub fn new_exact_query<T>(field: T, value: BigInt) -> Result<PointRangeQuery>
  where
    T: Into<String>,
  {
    Self::new_range_query(field, value.clone(), value)
  }

  /// Create a range query for big integer values.
  ///
  /// This is for simple one-dimension ranges, for multidimensional ranges use
  /// [`new_range_query_n`](Self::new_range_query_n) instead.
  ///
  /// You can have half-open ranges (which are in fact `<`/`<=` or `>`/`>=` queries) by setting
  /// `lower_value = BigIntegerPoint::min_value()` or
  /// `upper_value = BigIntegerPoint::max_value()`.
  ///
  /// Ranges are inclusive. For exclusive ranges, pass `lower_value + 1` or `upper_value - 1`.
  pub fn new_range_query<T>(
    field: T,
    lower_value: BigInt,
    upper_value: BigInt,
  ) -> Result<PointRangeQuery>
  where
    T: Into<String>,
  {
    Self::new_range_query_n(field, [lower_value], [upper_value])
  }

  /// Create a range query for n-dimensional big integer values.
  ///
  /// You can have half-open ranges (which are in fact `<`/`<=` or `>`/`>=` queries) by setting
  /// `lower_value[i] = BigIntegerPoint::min_value()` or
  /// `upper_value[i] = BigIntegerPoint::max_value()`.
  ///
  /// Ranges are inclusive. For exclusive ranges, pass `lower_value[i] + 1` or
  /// `upper_value[i] - 1`.
  pub fn new_range_query_n<T, LV, UV>(
    field: T,
    lower_value: LV,
    upper_value: UV,
  ) -> Result<PointRangeQuery>
  where
    T: Into<String>,
    LV: AsRef<[BigInt]>,
    UV: AsRef<[BigInt]>,
  {
    let field = field.into();
    let lower_value = lower_value.as_ref();
    let upper_value = upper_value.as_ref();
    if lower_value.len() != upper_value.len() {
      return Err(LuceneError::illegal_argument(
        "lowerValue.length != upperValue.length".to_string(),
      ));
    }

    let mut lower_point = Self::pack(lower_value)?;
    let mut upper_point = Self::pack(upper_value)?;
    #[cfg(debug_assertions)]
    check_args(&field, &lower_point.bytes, &upper_point.bytes)?;
    PointRangeQuery::new(
      field,
      lower_point.take_bytes(),
      upper_point.take_bytes(),
      lower_value.len(),
      BigIntegerPointRangeQuery,
    )
  }
}

impl FieldBase for BigIntegerPoint {
  fn set_bytes_value(&mut self, _value: BytesRef<Vec<u8>>) -> Result<()> {
    Err(LuceneError::illegal_argument(
      "cannot change value type from BigInteger to BytesRef",
    ))
  }
}

impl IndexableField for BigIntegerPoint {
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
        debug_assert!(bytes.length == Self::BYTES);
        Ok(Some(Number::BigInt(Self::decode_dimension(
          &bytes.bytes,
          bytes.offset,
        )?)))
      },
      _ => Err(LuceneError::illegal_argument(
        "Unsupported FieldDataEnum variant",
      )),
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

impl fmt::Display for BigIntegerPoint {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "BigIntegerPoint <{}:", self.parent_field.name())?;
    let bytes = match &self.parent_field.fields_data {
      FieldDataEnum::Binary(bytes) => bytes,
      _ => {
        debug_assert!(false, "BigIntegerPoint fieldsData must be BytesRef");
        return Err(fmt::Error);
      },
    };
    for dim in 0..self.parent_field.field_type().point_dimension_count() {
      if dim > 0 {
        write!(f, ",")?;
      }
      let value =
        Self::decode_dimension(&bytes.bytes, bytes.offset + dim * Self::BYTES).map_err(|_| {
          debug_assert!(false, "BigIntegerPoint fieldsData must decode");
          fmt::Error
        })?;
      write!(f, "{value}")?;
    }
    write!(f, ">")
  }
}

#[derive(Debug, Clone)]
pub struct BigIntegerPointRangeQuery;

impl PointRangeBase for BigIntegerPointRangeQuery {
  fn to_string(&self, _dimension: usize, value: &[u8]) -> Result<String> {
    Ok(BigIntegerPoint::decode_dimension(value, 0)?.to_string())
  }
}

#[cfg(test)]
impl Clone for BigIntegerPoint {
  fn clone(&self) -> Self {
    Self {
      parent_field: self.parent_field.clone(),
    }
  }
}
