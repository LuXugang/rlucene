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
use crate::core::geo::circle2d::{Circle2D, HaversinDistance, create_from_circle};
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::geo::geometry::Geometry;
use crate::core::geo::lat_lon_geometry::LatLonGeometry;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::core_helper::CoreHelper;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};

/// Represents a circle on the earth's surface.
///
/// NOTES:
///
/// 1. Latitude/longitude values must be in decimal degrees.
/// 2. Radius must be in meters.
/// 3. For more advanced GeoSpatial indexing and query operations see the `spatial-extras`
///    module
///

#[derive(Clone, Debug)]
pub struct Circle {
  /// Center latitude
  lat: f64,

  /// Center longitude
  lon: f64,

  /// radius in meters
  radius_meters: f64,
}

impl Circle {
  /// Creates a new circle from the supplied latitude/longitude center and a radius in meters..
  pub fn new(lat: f64, lon: f64, radius_meters: f64) -> Result<Self> {
    GeoUtils::check_latitude(lat)?;
    GeoUtils::check_longitude(lon)?;
    if !radius_meters.is_finite() || radius_meters < 0.0 {
      return Err(LuceneError::illegal_argument(format!(
        "radiusMeters: '{radius_meters}' is invalid"
      )));
    }
    Ok(Self {
      lat,
      lon,
      radius_meters,
    })
  }

  /// Returns the center's latitude
  pub fn get_lat(&self) -> f64 {
    self.lat
  }

  /// Returns the center's longitude
  pub fn get_lon(&self) -> f64 {
    self.lon
  }

  /// Returns the radius in meters
  pub fn get_radius(&self) -> f64 {
    self.radius_meters
  }
}
impl Display for Circle {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "Circle([{},{}] radius = {} meters)",
      self.lat, self.lon, self.radius_meters
    )
  }
}
impl PartialEq for Circle {
  fn eq(&self, other: &Self) -> bool {
    CoreHelper::compare_f64(self.lat, other.lat).is_eq()
      && CoreHelper::compare_f64(self.lon, other.lon).is_eq()
      && CoreHelper::compare_f64(self.radius_meters, other.radius_meters).is_eq()
  }
}

impl Eq for Circle {}

impl std::hash::Hash for Circle {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    (BitUtil::double_to_long_bits(self.lat) as u64).hash(state);
    (BitUtil::double_to_long_bits(self.lon) as u64).hash(state);
    (BitUtil::double_to_long_bits(self.radius_meters) as u64).hash(state);
  }
}
impl Geometry for Circle {
  type Component2D = Circle2D<HaversinDistance>;

  fn to_component2d(&self) -> Result<Self::Component2D> {
    create_from_circle(self)
  }
}

impl LatLonGeometry for Circle {}
