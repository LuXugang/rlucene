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
use crate::core::geo::polygon::Polygon;
use crate::core::geo::rectangle2d::{Rectangle2DType, create_from_rectangle};
use crate::core::util::SloppyMath;
use crate::core::util::error::lucene_error::Result;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

/// Represents a lat/lon rectangle.
#[derive(Clone)]
pub struct Rectangle {
  /// maximum longitude value (in degrees)
  pub min_lat: f64,

  /// minimum longitude value (in degrees)
  pub min_lon: f64,

  /// maximum latitude value (in degrees)
  pub max_lat: f64,

  /// minimum latitude value (in degrees)
  pub max_lon: f64,
}

impl Rectangle {
  pub const AXISLAT_ERROR: f64 =
    (0.1f64 / GeoUtils::EARTH_MEAN_RADIUS_METERS) * 180.0 / std::f64::consts::PI;
  /// Constructs a bounding box by first validating the provided latitude and longitude coordinates
  pub fn new(min_lat: f64, max_lat: f64, min_lon: f64, max_lon: f64) -> Result<Self> {
    GeoUtils::check_latitude(min_lat)?;
    GeoUtils::check_latitude(max_lat)?;
    GeoUtils::check_longitude(min_lon)?;
    GeoUtils::check_longitude(max_lon)?;

    debug_assert!(max_lat >= min_lat);

    // NOTE: cannot assert max_lon >= min_lon since this rect could cross the dateline
    Ok(Self {
      min_lon,
      max_lon,
      min_lat,
      max_lat,
    })
  }
  pub fn crosses_dateline(&self) -> bool {
    self.max_lon < self.min_lon
  }
  /// returns true if rectangle (defined by minLat, maxLat, minLon, maxLon) contains the lat lon
  /// point
  pub fn contains_point(
    lat: f64,
    lon: f64,
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
  ) -> bool {
    lat >= min_lat && lat <= max_lat && lon >= min_lon && lon <= max_lon
  }
  /// Compute Bounding Box for a circle using WGS-84 parameters
  pub fn from_point_distance(center_lat: f64, center_lon: f64, radius_meters: f64) -> Result<Self> {
    GeoUtils::check_latitude(center_lat)?;
    GeoUtils::check_longitude(center_lon)?;

    let rad_lat = center_lat.to_radians();
    let rad_lon = center_lon.to_radians();

    let rad_distance = (radius_meters + 7E-2) / GeoUtils::EARTH_MEAN_RADIUS_METERS;
    let mut min_lat = rad_lat - rad_distance;
    let mut max_lat = rad_lat + rad_distance;
    let min_lon;
    let max_lon;

    if min_lat > GeoUtils::MIN_LAT_RADIANS && max_lat < GeoUtils::MAX_LAT_RADIANS {
      let delta_lon =
        SloppyMath::asin(GeoUtils::sloppy_sin(rad_distance) / SloppyMath::cos(rad_lat));

      let mut min_lon_ = rad_lon - delta_lon;
      if min_lon_ < GeoUtils::MIN_LON_RADIANS {
        min_lon_ += 2.0 * std::f64::consts::PI;
      }

      let mut max_lon_ = rad_lon + delta_lon;
      if max_lon_ > GeoUtils::MAX_LON_RADIANS {
        max_lon_ -= 2.0 * std::f64::consts::PI;
      }

      min_lon = min_lon_;
      max_lon = max_lon_;
    } else {
      // a pole is within the distance
      min_lat = min_lat.max(GeoUtils::MIN_LAT_RADIANS);
      max_lat = max_lat.min(GeoUtils::MAX_LAT_RADIANS);
      min_lon = GeoUtils::MIN_LON_RADIANS;
      max_lon = GeoUtils::MAX_LON_RADIANS;
    }

    Self::new(
      min_lat.to_degrees(),
      max_lat.to_degrees(),
      min_lon.to_degrees(),
      max_lon.to_degrees(),
    )
  }
  /// Calculate the latitude of a circle's intersections with its bbox meridians.
  ///
  /// **NOTE:** the returned value will be +/- `AXISLAT_ERROR` of the actual value.
  ///
  /// # Arguments
  ///
  /// * `center_lat` - The latitude of the circle center
  /// * `radius_meters` - The radius of the circle in meters
  ///
  /// # Returns
  ///
  /// A latitude
  pub fn axis_lat(center_lat: f64, radius_meters: f64) -> f64 {
    // A spherical triangle with:
    // r is the radius of the circle in radians
    // l1 is the latitude of the circle center
    // l2 is the latitude of the point at which the circle intersect's its bbox longitudes
    // We know r is tangent to the bbox meridians at l2, therefore it is a right angle.
    // So from the law of cosines, with the angle of l1 being 90, we have:
    // cos(l1) = cos(r) * cos(l2) + sin(r) * sin(l2) * cos(90)
    // The second part cancels out because cos(90) == 0, so we have:
    // cos(l1) = cos(r) * cos(l2)
    // Solving for l2, we get:
    // l2 = acos( cos(l1) / cos(r) )
    // We ensure r is in the range (0, PI/2) and l1 in the range (0, PI/2]. This means we
    // cannot divide by 0, and we will always get a positive value in the range [0, 1) as
    // the argument to arc cosine, resulting in a range (0, PI/2].
    let pio2 = std::f64::consts::FRAC_PI_2;
    let mut l1 = center_lat.to_radians();
    let r = (radius_meters + 7E-2) / GeoUtils::EARTH_MEAN_RADIUS_METERS;

    // if we are within radius range of a pole, the lat is the pole itself
    if l1.abs() + r >= GeoUtils::MAX_LAT_RADIANS {
      return if center_lat >= 0.0 {
        GeoUtils::MAX_LAT_INCL
      } else {
        GeoUtils::MIN_LAT_INCL
      };
    }

    // adjust l1 as distance from closest pole, to form a right triangle with bbox meridians
    // and ensure it is in the range (0, PI/2]
    l1 = if center_lat >= 0.0 {
      pio2 - l1
    } else {
      l1 + pio2
    };

    let mut l2 = (l1.cos() / r.cos()).acos();
    debug_assert!(!l2.is_nan());

    // now adjust back to range [-pi/2, pi/2], ie latitude in radians
    l2 = if center_lat >= 0.0 {
      pio2 - l2
    } else {
      l2 - pio2
    };

    l2.to_degrees()
  }
  /// Returns the bounding box over an array of polygons
  pub fn from_polygon(polygons: &[Polygon]) -> Result<Rectangle> {
    let mut min_lat = f64::INFINITY;
    let mut max_lat = f64::NEG_INFINITY;
    let mut min_lon = f64::INFINITY;
    let mut max_lon = f64::NEG_INFINITY;

    for polygon in polygons {
      min_lat = f64::min(polygon.min_lat, min_lat);
      max_lat = f64::max(polygon.max_lat, max_lat);
      min_lon = f64::min(polygon.min_lon, min_lon);
      max_lon = f64::max(polygon.max_lon, max_lon);
    }

    Rectangle::new(min_lat, max_lat, min_lon, max_lon)
  }
}

impl Geometry for Rectangle {
  type Component2D = Rectangle2DType;

  fn to_component2d(&self) -> Result<Self::Component2D> {
    create_from_rectangle(self)
  }
}

impl LatLonGeometry for Rectangle {}
impl Display for Rectangle {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Rectangle(lat={} TO {} lon={} TO {}",
      self.min_lat, self.max_lat, self.min_lon, self.max_lon
    )?;
    if self.max_lon < self.min_lon {
      write!(f, " [crosses dateline!]")?;
    }
    write!(f, ")")
  }
}
impl PartialEq for Rectangle {
  fn eq(&self, other: &Self) -> bool {
    self.min_lat == other.min_lat
      && self.min_lon == other.min_lon
      && self.max_lat == other.max_lat
      && self.max_lon == other.max_lon
  }
}

impl Eq for Rectangle {}

impl Hash for Rectangle {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.min_lat.to_bits().hash(state);
    self.min_lon.to_bits().hash(state);
    self.max_lat.to_bits().hash(state);
    self.max_lon.to_bits().hash(state);
  }
}
