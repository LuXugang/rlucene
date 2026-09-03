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
use crate::core::document::double_range;
use crate::core::document::range_field_query::QueryType;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::core_helper::CoreHelper;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct DoubleRangeSlowRangeQuery {
  field: String,
  min: Vec<f64>,
  max: Vec<f64>,
}

impl DoubleRangeSlowRangeQuery {
  #[allow(clippy::new_ret_no_self)]
  pub(crate) fn new(
    field: String,
    min: Vec<f64>,
    max: Vec<f64>,
    query_type: QueryType,
  ) -> Result<BinaryRangeFieldRangeQuery> {
    let range = encode_ranges(&min, &max)?;
    let len = min.len();
    let sub = Self { field, min, max };
    BinaryRangeFieldRangeQuery::new(range, double_range::BYTES, len, query_type, sub)
  }

  pub(crate) fn field(&self) -> &str {
    &self.field
  }

  pub(crate) fn min(&self) -> &[f64] {
    &self.min
  }

  pub(crate) fn max(&self) -> &[f64] {
    &self.max
  }
}

impl PartialEq for DoubleRangeSlowRangeQuery {
  fn eq(&self, other: &Self) -> bool {
    self.field == other.field
      && CoreHelper::array_equals_f64(&self.min, &other.min)
      && CoreHelper::array_equals_f64(&self.max, &other.max)
  }
}

impl Eq for DoubleRangeSlowRangeQuery {}

impl Hash for DoubleRangeSlowRangeQuery {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.field.hash(state);
    for value in &self.min {
      (BitUtil::double_to_long_bits(*value) as u64).hash(state);
    }
    for value in &self.max {
      (BitUtil::double_to_long_bits(*value) as u64).hash(state);
    }
  }
}

impl Display for DoubleRangeSlowRangeQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}:{:?} TO {:?}", self.field, self.min, self.max)
  }
}
fn encode_ranges(min: &[f64], max: &[f64]) -> Result<Vec<u8>> {
  let mut result = vec![0u8; 2 * double_range::BYTES * min.len()];
  double_range::verify_and_encode(min, max, &mut result)?;
  Ok(result)
}
