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
use crate::core::search::point_in_set_query::{PointInSetBase, PointInSetBaseEnum};
use crate::core::search::point_range_query::PointRangeBase;
use crate::core::util::error::lucene_error::Result;

#[derive(Debug, Clone)]
pub struct PointRangeQueryBaseImpl;

impl PointRangeBase for PointRangeQueryBaseImpl {
  fn to_string(&self, _dimension: usize, _value: &[u8]) -> Result<String> {
    Ok("foo".to_string())
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MultiDimIntPointInSetQuery {
  num_dims: usize,
}

impl MultiDimIntPointInSetQuery {
  pub fn new(num_dims: usize) -> Self {
    Self { num_dims }
  }
}

impl From<MultiDimIntPointInSetQuery> for PointInSetBaseEnum {
  fn from(value: MultiDimIntPointInSetQuery) -> Self {
    Self::MultiDimInt(value)
  }
}

impl PointInSetBase for MultiDimIntPointInSetQuery {
  fn to_string(&self, value: &[u8]) -> Result<String> {
    let mut sb = String::new();
    for dim in 0..self.num_dims {
      if dim > 0 {
        sb.push(',');
      }
      sb.push_str(
        &crate::core::util::numeric_utils::NumericUtils::sortable_bytes_to_int(
          value,
          dim * std::mem::size_of::<i32>(),
        )
        .to_string(),
      );
    }
    Ok(sb)
  }
}
