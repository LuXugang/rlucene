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
use crate::core::geo::circle2d::Circle2D;
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::geo::geometry::Geometry;
use crate::core::geo::lat_lon_geometry::LatLonGeometry;
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
    self.lat.to_bits() == other.lat.to_bits()
      && self.lon.to_bits() == other.lon.to_bits()
      && self.radius_meters.to_bits() == other.radius_meters.to_bits()
  }
}

impl Eq for Circle {}

impl std::hash::Hash for Circle {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.lat.to_bits().hash(state);
    self.lon.to_bits().hash(state);
    self.radius_meters.to_bits().hash(state);
  }
}
impl Geometry for Circle {
  type Component2D = Circle2D;

  fn to_component2d(&self) -> Result<Self::Component2D> {
    todo!()
  }
}

impl LatLonGeometry for Circle {}

#[cfg(test)]
mod tests {
  use crate::core::geo::circle::Circle;
  use crate::core::util::error::lucene_error::LuceneError;
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::geo::geo_test_util::GeoTestUtil;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
  #[allow(dead_code)] // for quick serach
  struct TestCircle;
  #[test]
  fn test_invalid_lat() {
    let err = Circle::new(134.14, 45.23, 1000.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    let err = err.unwrap_err();
    assert!(
      err
        .to_string()
        .contains("invalid latitude 134.14; must be between -90 and 90")
    );
  }

  #[test]
  fn test_invalid_lon() {
    let err = Circle::new(43.5, 180.5, 1000.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    let err = err.unwrap_err();
    assert!(
      err
        .to_string()
        .contains("invalid longitude 180.5; must be between -180 and 180")
    );
  }

  #[test]
  fn test_negative_radius() {
    let err = Circle::new(43.5, 45.23, -1000.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    let err = err.unwrap_err();
    assert!(err.to_string().contains("radiusMeters: '-1000' is invalid"));
  }

  #[test]
  fn test_infinite_radius() {
    let err = Circle::new(43.5, 45.23, f64::INFINITY);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    let err = err.unwrap_err();
    assert!(err.to_string().contains("radiusMeters: 'inf' is invalid"));
  }

  #[test]
  fn test_equals_and_hash_code() -> Result<()> {
    let mut random = random();
    let circle = GeoTestUtil::next_circle(&mut random)?;
    let copy = Circle::new(circle.get_lat(), circle.get_lon(), circle.get_radius())?;
    assert_eq!(circle, copy);

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut h1 = DefaultHasher::new();
    circle.hash(&mut h1);
    let mut h2 = DefaultHasher::new();
    copy.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());

    let other_circle = GeoTestUtil::next_circle(&mut random)?;
    if circle.get_lon().to_bits() != other_circle.get_lon().to_bits()
      || circle.get_lat().to_bits() != other_circle.get_lat().to_bits()
      || circle.get_radius().to_bits() != other_circle.get_radius().to_bits()
    {
      assert_ne!(circle, other_circle);

      let mut h1 = DefaultHasher::new();
      circle.hash(&mut h1);
      let mut h2 = DefaultHasher::new();
      other_circle.hash(&mut h2);
      assert_ne!(h1.finish(), h2.finish());
    } else {
      assert_eq!(circle, other_circle);

      let mut h1 = DefaultHasher::new();
      circle.hash(&mut h1);
      let mut h2 = DefaultHasher::new();
      other_circle.hash(&mut h2);
      assert_eq!(h1.finish(), h2.finish());
    }

    Ok(())
  }
}
