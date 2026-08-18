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
use crate::core::document::binary_range_field_range_query::BinaryRangeFieldRangeQuery;
use crate::core::document::float_range;
use crate::core::document::range_field_query::QueryType;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct FloatRangeSlowRangeQuery {
  field: String,
  min: Vec<f32>,
  max: Vec<f32>,
}

impl FloatRangeSlowRangeQuery {
  #[allow(clippy::new_ret_no_self)]
  pub(crate) fn new(
    field: String,
    min: Vec<f32>,
    max: Vec<f32>,
    query_type: QueryType,
  ) -> Result<BinaryRangeFieldRangeQuery> {
    let range = encode_ranges(&min, &max)?;
    let len = min.len();
    let sub = Self { field, min, max };
    BinaryRangeFieldRangeQuery::new(range, float_range::BYTES, len, query_type, sub)
  }

  pub(crate) fn field(&self) -> &str {
    &self.field
  }

  pub(crate) fn min(&self) -> &[f32] {
    &self.min
  }

  pub(crate) fn max(&self) -> &[f32] {
    &self.max
  }
}

impl PartialEq for FloatRangeSlowRangeQuery {
  fn eq(&self, other: &Self) -> bool {
    self.field == other.field && self.min == other.min && self.max == other.max
  }
}

impl Eq for FloatRangeSlowRangeQuery {}

impl Hash for FloatRangeSlowRangeQuery {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.field.hash(state);
    for value in &self.min {
      value.to_bits().hash(state);
    }
    for value in &self.max {
      value.to_bits().hash(state);
    }
  }
}

impl Display for FloatRangeSlowRangeQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}:{:?} TO {:?}", self.field, self.min, self.max)
  }
}

fn encode_ranges(min: &[f32], max: &[f32]) -> Result<Vec<u8>> {
  let mut result = vec![0u8; 2 * float_range::BYTES * min.len()];
  float_range::verify_and_encode(min, max, &mut result)?;
  Ok(result)
}
