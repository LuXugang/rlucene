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
use crate::core::geo::point2d;
use crate::core::geo::point2d::Point2D;
use crate::core::util::error::lucene_error::Result;
/// Represents a point on the earth's surface. You can construct the point directly
/// with `f64` coordinates.
///
/// NOTES:
///
/// 1. latitude/longitude values must be in decimal degrees.
/// 2. For more advanced GeoSpatial indexing and query operations see the
///    `spatial-extras` module
#[derive(Clone, Copy, Debug)]
pub struct Point {
  /// latitude coordinate
  lat: f64,

  /// longitude coordinate
  lon: f64,
}

impl Point {
  /// Creates a new Point from the supplied latitude/longitude.
  pub fn new(lat: f64, lon: f64) -> Result<Self> {
    GeoUtils::check_latitude(lat)?;
    GeoUtils::check_longitude(lon)?;
    Ok(Self { lat, lon })
  }

  /// Returns latitude value at given index
  pub fn get_lat(&self) -> f64 {
    self.lat
  }

  /// Returns longitude value at given index
  pub fn get_lon(&self) -> f64 {
    self.lon
  }
}

impl Geometry for Point {
  type Component2D = Point2D;

  fn to_component2d(&self) -> Result<Self::Component2D> {
    point2d::create_from_point(self)
  }
}

impl LatLonGeometry for Point {}

impl PartialEq for Point {
  fn eq(&self, other: &Self) -> bool {
    self.lat == other.lat && self.lon == other.lon
  }
}

impl Eq for Point {}

impl std::hash::Hash for Point {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.lat.to_bits().hash(state);
    self.lon.to_bits().hash(state);
  }
}

impl std::fmt::Display for Point {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Point({},{})", self.lon, self.lat)
  }
}
