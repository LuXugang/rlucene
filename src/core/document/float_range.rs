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
use crate::core::document::range_field_query::{QueryType, RangeFieldQuery, RangeFieldQueryBase};
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::numeric_utils::NumericUtils;

pub struct FloatRange;
pub const BYTES: usize = std::mem::size_of::<f32>();

impl FloatRange {
  /// Create a query for matching indexed ranges that intersect the defined range.
  ///
  /// # Parameters
  /// - `field`: field name.
  /// - `min`: array of min values. Accepts `f32::NEG_INFINITY`.
  /// - `max`: array of max values. Accepts `f32::MAX`.
  ///
  /// # Returns
  /// Query for matching intersecting ranges (overlap, within, or contains).
  ///
  /// # Errors
  /// Returns an error if `min` or `max` is invalid.
  pub fn new_intersects_query<T, P>(field: T, min: P, max: P) -> Result<RangeFieldQuery>
  where
    T: Into<String>,
    P: AsRef<[f32]>,
  {
    Self::new_relation_query(field, min, max, QueryType::Intersects)
  }

  /// Create a query for matching indexed float ranges that contain the defined range.
  ///
  /// # Parameters
  /// - `field`: field name.
  /// - `min`: array of min values. Accepts `f32::NEG_INFINITY`.
  /// - `max`: array of max values. Accepts `f32::INFINITY`.
  ///
  /// # Returns
  /// Query for matching ranges that contain the defined range.
  ///
  /// # Errors
  /// Returns an error if `min` or `max` is invalid.
  pub fn new_contains_query<T, P>(field: T, min: P, max: P) -> Result<RangeFieldQuery>
  where
    T: Into<String>,
    P: AsRef<[f32]>,
  {
    Self::new_relation_query(field, min, max, QueryType::Contains)
  }

  /// Create a query for matching indexed ranges that are within the defined range.
  ///
  /// # Parameters
  /// - `field`: field name.
  /// - `min`: array of min values. Accepts `f32::NEG_INFINITY`.
  /// - `max`: array of max values. Accepts `f32::INFINITY`.
  ///
  /// # Returns
  /// Query for matching ranges within the defined range.
  ///
  /// # Errors
  /// Returns an error if `min` or `max` is invalid.
  pub fn new_within_query<T, P>(field: T, min: P, max: P) -> Result<RangeFieldQuery>
  where
    T: Into<String>,
    P: AsRef<[f32]>,
  {
    Self::new_relation_query(field, min, max, QueryType::Within)
  }

  /// Create a query for matching indexed ranges that cross the defined range. A cross relation is
  /// any set of ranges that are not disjoint and not wholly contained by the query. Effectively,
  /// it is the complement of union(WITHIN, DISJOINT).
  ///
  /// # Parameters
  /// - `field`: field name.
  /// - `min`: array of min values. Accepts `f32::NEG_INFINITY`.
  /// - `max`: array of max values. Accepts `f32::INFINITY`.
  ///
  /// # Returns
  /// Query for matching ranges that cross the defined range.
  ///
  /// # Errors
  /// Returns an error if `min` or `max` is invalid.
  pub fn new_crosses_query<T, P>(field: T, min: P, max: P) -> Result<RangeFieldQuery>
  where
    T: Into<String>,
    P: AsRef<[f32]>,
  {
    Self::new_relation_query(field, min, max, QueryType::Crosses)
  }

  /// Helper method for creating the desired relational query.
  fn new_relation_query<T, P>(
    field: T,
    min: P,
    max: P,
    relation: QueryType,
  ) -> Result<RangeFieldQuery>
  where
    T: Into<String>,
    P: AsRef<[f32]>,
  {
    let min = min.as_ref();
    let max = max.as_ref();
    check_args(min, max)?;
    RangeFieldQuery::new(
      field.into(),
      encode_range(min, max)?,
      min.len(),
      relation,
      FloatRangeFieldQuery,
    )
  }
}

/// Validates the arguments.
fn check_args(min: &[f32], max: &[f32]) -> Result<()> {
  if min.is_empty() || max.is_empty() {
    return Err(LuceneError::illegal_argument(
      "min/max range values cannot be null or empty",
    ));
  }
  if min.len() != max.len() {
    return Err(LuceneError::illegal_argument("min/max ranges must agree"));
  }
  if min.len() > 4 {
    return Err(LuceneError::illegal_argument(
      "FloatRange does not support greater than 4 dimensions",
    ));
  }
  Ok(())
}

/// Encodes the min, max ranges into a byte array.
pub(crate) fn encode_range(min: &[f32], max: &[f32]) -> Result<Vec<u8>> {
  check_args(min, max)?;
  let mut bytes = vec![0u8; BYTES * 2 * min.len()];
  verify_and_encode(min, max, &mut bytes)?;
  Ok(bytes)
}

/// Encodes the ranges into a sortable byte array (`f32::NAN` not allowed).
///
/// Example for 4 dimensions (4 bytes per dimension value):
/// minD1 ... minD4 | maxD1 ... maxD4
pub fn verify_and_encode(min: &[f32], max: &[f32], bytes: &mut [u8]) -> Result<()> {
  for d in 0..min.len() {
    let i = d * BYTES;
    let j = min.len() * BYTES + d * BYTES;

    if min[d].is_nan() {
      return Err(LuceneError::illegal_argument(
        "invalid min value (NaN) in FloatRange",
      ));
    }
    if max[d].is_nan() {
      return Err(LuceneError::illegal_argument(
        "invalid max value (NaN) in FloatRange",
      ));
    }
    if min[d] > max[d] {
      return Err(LuceneError::illegal_argument(format!(
        "min value ({}) is greater than max value ({})",
        min[d], max[d]
      )));
    }

    encode(min[d], bytes, i);
    encode(max[d], bytes, j);
  }

  Ok(())
}

/// Encodes the given value into the byte array at the defined offset.
fn encode(val: f32, bytes: &mut [u8], offset: usize) {
  NumericUtils::int_to_sortable_bytes(NumericUtils::float_to_sortable_int(val), bytes, offset);
}

fn decode_min(bytes: &[u8], dimension: usize) -> f32 {
  let offset = dimension * BYTES;
  NumericUtils::sortable_int_to_float(NumericUtils::sortable_bytes_to_int(bytes, offset))
}

fn decode_max(bytes: &[u8], dimension: usize) -> f32 {
  let offset = bytes.len() / 2 + dimension * BYTES;
  NumericUtils::sortable_int_to_float(NumericUtils::sortable_bytes_to_int(bytes, offset))
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FloatRangeFieldQuery;

impl RangeFieldQueryBase for FloatRangeFieldQuery {
  fn to_string(&self, value: &[u8], dimension: usize) -> Result<String> {
    Ok(to_string(value, dimension))
  }
}

fn to_string(ranges: &[u8], dimension: usize) -> String {
  format!(
    "[{} : {}]",
    decode_min(ranges, dimension),
    decode_max(ranges, dimension)
  )
}
