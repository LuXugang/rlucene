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
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::geo::geometry::Geometry;
use crate::core::geo::lat_lon_geometry::LatLonGeometry;
use crate::core::geo::line2d::{Line2D, create_from_line};
use crate::core::util::bit_util::BitUtil;
use crate::core::util::core_helper::CoreHelper;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
/// Represents a line on the earth's surface. You can construct the [`Line`] directly with
/// `Vec<f64>` coordinates.
///
/// NOTES:
///
/// 1. All latitude/longitude values must be in decimal degrees.
/// 2. For more advanced GeoSpatial indexing and query operations see the `spatial-extras`
///    module.
#[derive(Debug, Clone)]
pub struct Line {
  /// Array of latitude coordinates.
  lats: Vec<f64>,

  /// Array of longitude coordinates.
  lons: Vec<f64>,

  /// Minimum latitude of this line's bounding box.
  pub min_lat: f64,

  /// Maximum latitude of this line's bounding box.
  pub max_lat: f64,

  /// Minimum longitude of this line's bounding box.
  pub min_lon: f64,

  /// Maximum longitude of this line's bounding box.
  pub max_lon: f64,
}

impl Line {
  /// Creates a new [`Line`] from the supplied latitude/longitude array.
  pub fn new(lats: Vec<f64>, lons: Vec<f64>) -> Result<Self> {
    if lats.len() != lons.len() {
      return Err(LuceneError::illegal_argument(
        "lats and lons must be equal length",
      ));
    }
    if lats.len() < 2 {
      return Err(LuceneError::illegal_argument(
        "at least 2 line points required",
      ));
    }

    let mut min_lat = lats[0];
    let mut min_lon = lons[0];
    let mut max_lat = lats[0];
    let mut max_lon = lons[0];
    for i in 0..lats.len() {
      GeoUtils::check_latitude(lats[i])?;
      GeoUtils::check_longitude(lons[i])?;
      min_lat = lats[i].min(min_lat);
      min_lon = lons[i].min(min_lon);
      max_lat = lats[i].max(max_lat);
      max_lon = lons[i].max(max_lon);
    }

    Ok(Self {
      lats,
      lons,
      min_lat,
      max_lat,
      min_lon,
      max_lon,
    })
  }

  /// Returns the number of vertex points.
  pub fn num_points(&self) -> usize {
    self.lats.len()
  }

  /// Returns latitude value at given index.
  pub fn get_lat(&self, vertex: usize) -> f64 {
    self.lats[vertex]
  }

  /// Returns longitude value at given index.
  pub fn get_lon(&self, vertex: usize) -> f64 {
    self.lons[vertex]
  }

  /// Returns a copy of the internal latitude array.
  pub fn get_lats(&self) -> &[f64] {
    self.lats.as_slice()
  }

  /// Returns a copy of the internal longitude array.
  pub fn get_lons(&self) -> &[f64] {
    self.lons.as_slice()
  }
}
impl PartialEq for Line {
  fn eq(&self, other: &Self) -> bool {
    CoreHelper::array_equals_f64(&self.lats, &other.lats)
      && CoreHelper::array_equals_f64(&self.lons, &other.lons)
  }
}

impl Eq for Line {}

impl Hash for Line {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for lat in &self.lats {
      (BitUtil::double_to_long_bits(*lat) as u64).hash(state);
    }
    for lon in &self.lons {
      (BitUtil::double_to_long_bits(*lon) as u64).hash(state);
    }
  }
}

impl Display for Line {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "Line(")?;
    for i in 0..self.lats.len() {
      write!(f, "[{}, {}]", self.lons[i], self.lats[i])?;
    }
    write!(f, ")")
  }
}
impl Geometry for Line {
  type Component2D = Line2D;

  fn to_component2d(&self) -> Result<Self::Component2D> {
    create_from_line(self)
  }
}

impl LatLonGeometry for Line {}
