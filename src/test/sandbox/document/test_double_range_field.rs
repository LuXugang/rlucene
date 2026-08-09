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

use crate::core::document::double_range::DoubleRange;
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)] // for quick search
struct TestDoubleRangeField;

const FIELD_NAME: &str = "rangeField";

/// Test illegal NaN range values.
#[test]
fn test_illegal_na_n_values() -> Result<()> {
  let error = DoubleRange::new(FIELD_NAME, &[f64::NAN], &[5.0])
    .err()
    .expect("NaN minimum must fail");
  assert!(error.to_string().contains("invalid min value"));

  let error = DoubleRange::new(FIELD_NAME, &[5.0], &[f64::NAN])
    .err()
    .expect("NaN maximum must fail");
  assert!(error.to_string().contains("invalid max value"));
  Ok(())
}

/// Min/max slice sizes must agree.
#[test]
fn test_uneven_arrays() -> Result<()> {
  let error = DoubleRange::new(FIELD_NAME, &[5.0, 6.0][..], &[5.0][..])
    .err()
    .expect("uneven min/max slices must fail");
  assert!(error.to_string().contains("min/max ranges must agree"));
  Ok(())
}

/// Dimensions greater than 4 are not supported.
#[test]
fn test_oversize_dimensions() -> Result<()> {
  let error = DoubleRange::new(FIELD_NAME, &[1.0, 2.0, 3.0, 4.0, 5.0][..], &[5.0][..])
    .err()
    .expect("more than four dimensions must fail");
  assert!(
    error
      .to_string()
      .contains("does not support greater than 4 dimensions")
  );
  Ok(())
}

/// Min cannot be greater than max.
#[test]
fn test_min_greater_than_max() -> Result<()> {
  let error = DoubleRange::new(FIELD_NAME, &[3.0, 4.0], &[1.0, 2.0])
    .err()
    .expect("minimum greater than maximum must fail");
  assert!(error.to_string().contains("is greater than max value"));
  Ok(())
}
