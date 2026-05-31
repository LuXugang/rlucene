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
use crate::core::index::point_values::Relation;
use crate::core::util::array_util::{ByteArrayComparator, ByteArrayComparatorEnum};
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;

pub trait RangeFieldQuery {}

/// Used by [`RangeFieldQuery`] to check how each internal or leaf node relates to the query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryType {
  /// Use this for intersects queries.
  Intersects,
  /// Use this for within queries.
  Within,
  /// Use this for contains queries.
  Contains,
  /// Use this for crosses queries.
  Crosses,
}

impl QueryType {
  #[allow(clippy::too_many_arguments)]
  fn compare_dim(
    &self,
    query_packed_value: &[u8],
    min_packed_value: &[u8],
    max_packed_value: &[u8],
    num_dims: usize,
    bytes_per_dim: usize,
    dim: usize,
    comparator: &ByteArrayComparatorEnum,
  ) -> Result<Relation> {
    let min_offset = dim * bytes_per_dim;
    let max_offset = min_offset + bytes_per_dim * num_dims;

    match self {
      QueryType::Intersects => {
        if comparator.compare(query_packed_value, max_offset, min_packed_value, min_offset) < 0
          || comparator.compare(query_packed_value, min_offset, max_packed_value, max_offset) > 0
        {
          return Ok(Relation::CellOutsideQuery);
        }

        if comparator.compare(query_packed_value, max_offset, max_packed_value, min_offset) >= 0
          && comparator.compare(query_packed_value, min_offset, min_packed_value, max_offset) <= 0
        {
          return Ok(Relation::CellInsideQuery);
        }

        Ok(Relation::CellCrossesQuery)
      },
      QueryType::Within => {
        if comparator.compare(query_packed_value, max_offset, min_packed_value, max_offset) < 0
          || comparator.compare(query_packed_value, min_offset, max_packed_value, min_offset) > 0
        {
          return Ok(Relation::CellOutsideQuery);
        }

        if comparator.compare(query_packed_value, max_offset, max_packed_value, max_offset) >= 0
          && comparator.compare(query_packed_value, min_offset, min_packed_value, min_offset) <= 0
        {
          return Ok(Relation::CellInsideQuery);
        }

        Ok(Relation::CellCrossesQuery)
      },
      QueryType::Contains => {
        if comparator.compare(query_packed_value, max_offset, max_packed_value, max_offset) > 0
          || comparator.compare(query_packed_value, min_offset, min_packed_value, min_offset) < 0
        {
          return Ok(Relation::CellOutsideQuery);
        }

        if comparator.compare(query_packed_value, max_offset, min_packed_value, max_offset) <= 0
          && comparator.compare(query_packed_value, min_offset, max_packed_value, min_offset) >= 0
        {
          return Ok(Relation::CellInsideQuery);
        }

        Ok(Relation::CellCrossesQuery)
      },
      QueryType::Crosses => Err(LuceneError::unsupported_operation("")),
    }
  }

  pub fn compare(
    &self,
    query_packed_value: &[u8],
    min_packed_value: &[u8],
    max_packed_value: &[u8],
    num_dims: usize,
    bytes_per_dim: usize,
    comparator: &ByteArrayComparatorEnum,
  ) -> Result<Relation> {
    let mut inside = true;
    for dim in 0..num_dims {
      let relation = self.compare_dim(
        query_packed_value,
        min_packed_value,
        max_packed_value,
        num_dims,
        bytes_per_dim,
        dim,
        comparator,
      )?;
      if relation == Relation::CellOutsideQuery {
        return Ok(Relation::CellOutsideQuery);
      } else if relation != Relation::CellInsideQuery {
        inside = false;
      }
    }
    if inside {
      Ok(Relation::CellInsideQuery)
    } else {
      Ok(Relation::CellCrossesQuery)
    }
  }

  fn matches_dim(
    &self,
    query_packed_value: &[u8],
    packed_value: &[u8],
    num_dims: usize,
    bytes_per_dim: usize,
    dim: usize,
    comparator: &ByteArrayComparatorEnum,
  ) -> Result<bool> {
    let min_offset = dim * bytes_per_dim;
    let max_offset = min_offset + bytes_per_dim * num_dims;

    match self {
      QueryType::Intersects => Ok(
        comparator.compare(query_packed_value, max_offset, packed_value, min_offset) >= 0
          && comparator.compare(query_packed_value, min_offset, packed_value, max_offset) <= 0,
      ),
      QueryType::Within => Ok(
        comparator.compare(query_packed_value, min_offset, packed_value, min_offset) <= 0
          && comparator.compare(query_packed_value, max_offset, packed_value, max_offset) >= 0,
      ),
      QueryType::Contains => Ok(
        comparator.compare(query_packed_value, min_offset, packed_value, min_offset) >= 0
          && comparator.compare(query_packed_value, max_offset, packed_value, max_offset) <= 0,
      ),
      QueryType::Crosses => Err(LuceneError::unsupported_operation("")),
    }
  }

  /// Compares every dim for 2 encoded ranges and returns true if all dims match.
  /// Matching implementation is based on the [`QueryType`].
  pub fn matches(
    &self,
    query_packed_value: &[u8],
    packed_value: &[u8],
    num_dims: usize,
    bytes_per_dim: usize,
    comparator: &ByteArrayComparatorEnum,
  ) -> Result<bool> {
    for dim in 0..num_dims {
      if !self.matches_dim(
        query_packed_value,
        packed_value,
        num_dims,
        bytes_per_dim,
        dim,
        comparator,
      )? {
        return Ok(false);
      }
    }
    Ok(true)
  }
}
