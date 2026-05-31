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
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::numeric_utils::NumericUtils;

pub struct FloatRange;
pub const BYTES: usize = std::mem::size_of::<f32>();

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
