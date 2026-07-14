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
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::point_values::PointValues;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::io::Write;
use std::sync::Arc;

pub struct CheckIndex;

#[derive(Clone, Debug, Default)]
pub struct Status {
  pub segment_infos: Vec<SegmentInfoStatus>,
}

#[derive(Clone, Debug, Default)]
pub struct SegmentInfoStatus {
  pub points_status: PointsStatus,
}

#[derive(Clone, Debug, Default)]
pub struct PointsStatus {
  pub total_value_points: usize,
  pub total_value_fields: usize,
}

impl CheckIndex {
  pub fn check_index<D, W>(
    directory: Arc<D>,
    level: i32,
    _cross_check_term_vectors: bool,
    _fail_fast: bool,
    output: &mut W,
  ) -> Result<Status>
  where
    D: Directory + 'static,
    W: Write,
  {
    Level::check_if_level_in_bounds(level)?;

    let reader = directory_reader::open(directory)?;
    let context = (&reader).get_context()?;
    let mut status = Status::default();

    for leaf in context.leaves()? {
      let reader = leaf.reader();
      let field_infos = reader.get_field_infos()?;
      let mut segment_status = SegmentInfoStatus::default();

      if field_infos.has_point_values() {
        writeln!(output, "test: points...")?;
      }

      for field_info in field_infos.iter() {
        if field_info.get_point_dimension_count() == 0 {
          continue;
        }
        if let Some(points) = reader.get_point_values(&field_info.name)? {
          segment_status.points_status.total_value_points += points.size()?;
          segment_status.points_status.total_value_fields += 1;
        }
      }

      status.segment_infos.push(segment_status);
    }

    Ok(status)
  }
}

pub struct Level;

impl Level {
  /// Minimum valid level.
  pub const MIN_VALUE: i32 = 1;

  /// Maximum valid level.
  pub const MAX_VALUE: i32 = 3;

  /// The default level if none is specified.
  pub const DEFAULT_VALUE: i32 = Self::MIN_VALUE;

  /// Minimum level required to run checksum checks.
  pub const MIN_LEVEL_FOR_CHECKSUM_CHECKS: i32 = 1;

  /// Minimum level required to run integrity checks.
  pub const MIN_LEVEL_FOR_INTEGRITY_CHECKS: i32 = 2;

  /// Minimum level required to run slow checks.
  pub const MIN_LEVEL_FOR_SLOW_CHECKS: i32 = 3;

  /// Checks if given level value is within the allowed bounds else it returns an error.
  pub fn check_if_level_in_bounds(level_val: i32) -> Result<()> {
    if !(Self::MIN_VALUE..=Self::MAX_VALUE).contains(&level_val) {
      return Err(LuceneError::illegal_argument(format!(
        "ERROR: given value: '{}' for -level option is out of bounds. Please use a value from '{}'->'{}'",
        level_val,
        Self::MIN_VALUE,
        Self::MAX_VALUE
      )));
    }

    Ok(())
  }
}
