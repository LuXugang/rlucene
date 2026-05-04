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
use crate::core::geo::rectangle::Rectangle;
use crate::core::index::point_values::Relation;
use crate::core::util::SloppyMath;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::f64::consts::{FRAC_PI_2, PI};

/// Basic reusable geo-spatial utility methods
pub struct GeoUtils;

impl GeoUtils {
  /// Minimum longitude value.
  pub const MIN_LON_INCL: f64 = -180.0;
  /// Maximum longitude value.
  pub const MAX_LON_INCL: f64 = 180.0;
  /// Minimum latitude value.
  pub const MIN_LAT_INCL: f64 = -90.0;
  /// Maximum latitude value.
  pub const MAX_LAT_INCL: f64 = 90.0;

  /// Minimum longitude value in radians.
  pub const MIN_LON_RADIANS: f64 = Self::MIN_LON_INCL * PI / 180.0;
  /// Minimum latitude value in radians.
  pub const MIN_LAT_RADIANS: f64 = Self::MIN_LAT_INCL * PI / 180.0;
  /// Maximum longitude value in radians.
  pub const MAX_LON_RADIANS: f64 = Self::MAX_LON_INCL * PI / 180.0;
  /// Maximum latitude value in radians.
  pub const MAX_LAT_RADIANS: f64 = Self::MAX_LAT_INCL * PI / 180.0;

  /// Mean earth axis in meters (WGS84).
  pub const EARTH_MEAN_RADIUS_METERS: f64 = 6_371_008.771_4;

  const PIO2: f64 = FRAC_PI_2;

  /// Validates latitude value is within standard +/-90 coordinate bounds.
  pub fn check_latitude(latitude: f64) -> Result<()> {
    if latitude.is_nan() || !(Self::MIN_LAT_INCL..=Self::MAX_LAT_INCL).contains(&latitude) {
      return Err(LuceneError::illegal_argument(format!(
        "invalid latitude {}; must be between {} and {}",
        latitude,
        Self::MIN_LAT_INCL,
        Self::MAX_LAT_INCL
      )));
    }

    Ok(())
  }

  /// Validates longitude value is within standard +/-180 coordinate bounds.
  pub fn check_longitude(longitude: f64) -> Result<()> {
    if longitude.is_nan() || !(Self::MIN_LON_INCL..=Self::MAX_LON_INCL).contains(&longitude) {
      return Err(LuceneError::illegal_argument(format!(
        "invalid longitude {}; must be between {} and {}",
        longitude,
        Self::MIN_LON_INCL,
        Self::MAX_LON_INCL
      )));
    }

    Ok(())
  }

  /// Returns the trigonometric sine of an angle converted as a cosine operation.
  ///
  /// This intentionally mirrors Lucene's `sloppySin`, including its approximation
  /// behavior.
  pub fn sloppy_sin(a: f64) -> f64 {
    (a - Self::PIO2).cos()
  }

  /// Placeholder for Lucene's `distanceQuerySortKey`.
  ///
  /// This depends on the haversine helpers that have not been ported yet.
  pub fn distance_query_sort_key(radius: f64) -> f64 {
    let max_sort_key = f64::MAX;
    let max_haversin = SloppyMath::haversin_meters_from_sort_key(max_sort_key);

    if radius >= max_haversin {
      return max_haversin;
    }

    let mut lo: u64 = 0;
    let mut hi: u64 = f64::MAX.to_bits();

    while lo <= hi {
      let mid = lo + ((hi - lo) >> 1);
      let sort_key = f64::from_bits(mid);
      let mid_radius = SloppyMath::haversin_meters_from_sort_key(sort_key);

      if mid_radius == radius {
        return sort_key;
      } else if mid_radius > radius {
        if mid == 0 {
          break;
        }
        hi = mid - 1;
      } else {
        lo = mid + 1;
      }
    }

    let ceil = f64::from_bits(lo);
    debug_assert!(SloppyMath::haversin_meters_from_sort_key(ceil) > radius);
    ceil
  }

  /// Placeholder for Lucene's `relate`.
  ///
  /// This depends on `SloppyMath` and `Rectangle::AXISLAT_ERROR`, which are
  /// not available in the Rust port yet.
  #[allow(clippy::too_many_arguments)]
  pub fn relate(
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
    lat: f64,
    lon: f64,
    distance_sort_key: f64,
    axis_lat: f64,
  ) -> Result<Relation> {
    if min_lon > max_lon {
      return Err(LuceneError::illegal_argument("Box crosses the dateline"));
    }

    if (lon < min_lon || lon > max_lon)
      && (axis_lat + Rectangle::AXISLAT_ERROR < min_lat
        || axis_lat - Rectangle::AXISLAT_ERROR > max_lat)
    {
      // circle not fully inside / crossing axis
      if SloppyMath::haversin_sort_key(lat, lon, min_lat, min_lon) > distance_sort_key
        && SloppyMath::haversin_sort_key(lat, lon, min_lat, max_lon) > distance_sort_key
        && SloppyMath::haversin_sort_key(lat, lon, max_lat, min_lon) > distance_sort_key
        && SloppyMath::haversin_sort_key(lat, lon, max_lat, max_lon) > distance_sort_key
      {
        // no points inside
        return Ok(Relation::CellOutsideQuery);
      }
    }

    if Self::within_90_lon_degrees(lon, min_lon, max_lon)
      && SloppyMath::haversin_sort_key(lat, lon, min_lat, min_lon) <= distance_sort_key
      && SloppyMath::haversin_sort_key(lat, lon, min_lat, max_lon) <= distance_sort_key
      && SloppyMath::haversin_sort_key(lat, lon, max_lat, min_lon) <= distance_sort_key
      && SloppyMath::haversin_sort_key(lat, lon, max_lat, max_lon) <= distance_sort_key
    {
      // we are fully enclosed, collect everything within this subtree
      return Ok(Relation::CellInsideQuery);
    }

    Ok(Relation::CellCrossesQuery)
  }

  /// Return whether all points of `[min_lon, max_lon]` are within 90 degrees of `lon`.
  pub fn within_90_lon_degrees(mut lon: f64, min_lon: f64, max_lon: f64) -> bool {
    if max_lon <= lon - 180.0 {
      lon -= 360.0;
    } else if min_lon >= lon + 180.0 {
      lon += 360.0;
    }

    max_lon - lon < 90.0 && lon - min_lon < 90.0
  }

  /// Returns a positive value if points a, b, and c are arranged in counter-clockwise order,
  /// negative if clockwise, zero if collinear.
  pub fn orient(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> i32 {
    let v1 = (bx - ax) * (cy - ay);
    let v2 = (cx - ax) * (by - ay);
    if v1 > v2 {
      1
    } else if v1 < v2 {
      -1
    } else {
      0
    }
  }

  /// Uses `orient` to compute whether two line segments cross.
  #[allow(clippy::too_many_arguments)]
  pub fn line_crosses_line(
    a1x: f64,
    a1y: f64,
    b1x: f64,
    b1y: f64,
    a2x: f64,
    a2y: f64,
    b2x: f64,
    b2y: f64,
  ) -> bool {
    Self::orient(a2x, a2y, b2x, b2y, a1x, a1y) * Self::orient(a2x, a2y, b2x, b2y, b1x, b1y) < 0
      && Self::orient(a1x, a1y, b1x, b1y, a2x, a2y) * Self::orient(a1x, a1y, b1x, b1y, b2x, b2y) < 0
  }

  /// Uses `orient` to compute whether two lines overlap each other.
  #[allow(clippy::too_many_arguments)]
  pub fn line_overlap_line(
    a1x: f64,
    a1y: f64,
    b1x: f64,
    b1y: f64,
    a2x: f64,
    a2y: f64,
    b2x: f64,
    b2y: f64,
  ) -> bool {
    Self::orient(a2x, a2y, b2x, b2y, a1x, a1y) == 0
      && Self::orient(a2x, a2y, b2x, b2y, b1x, b1y) == 0
      && Self::orient(a1x, a1y, b1x, b1y, a2x, a2y) == 0
      && Self::orient(a1x, a1y, b1x, b1y, b2x, b2y) == 0
  }

  /// uses orient method to compute whether two line segments cross; boundaries included - returning
  /// true for lines that terminate on each other.
  ///
  /// e.g., (plus sign) + == true, and (capital 't') T == true
  ///
  /// Use `line_crosses_line` to exclude lines that terminate on each other from the truth table
  #[allow(clippy::too_many_arguments)]
  pub fn line_crosses_line_with_boundary(
    a1x: f64,
    a1y: f64,
    b1x: f64,
    b1y: f64,
    a2x: f64,
    a2y: f64,
    b2x: f64,
    b2y: f64,
  ) -> bool {
    Self::orient(a2x, a2y, b2x, b2y, a1x, a1y) * Self::orient(a2x, a2y, b2x, b2y, b1x, b1y) <= 0
      && Self::orient(a1x, a1y, b1x, b1y, a2x, a2y) * Self::orient(a1x, a1y, b1x, b1y, b2x, b2y)
        <= 0
  }
}

/// Used to define the orientation of 3 points:
/// `-1 = Clockwise`, `0 = Collinear`, `1 = Counter-clockwise`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindingOrder {
  Cw = -1,
  Colinear = 0,
  Ccw = 1,
}

impl WindingOrder {
  pub fn sign(self) -> i32 {
    self as i32
  }

  pub fn from_sign(sign: i32) -> Result<Self> {
    match sign {
      -1 => Ok(Self::Cw),
      0 => Ok(Self::Colinear),
      1 => Ok(Self::Ccw),
      _ => Err(LuceneError::illegal_argument(format!(
        "Invalid WindingOrder sign: {}",
        sign
      ))),
    }
  }
}
#[cfg(test)]
pub mod tests {
  use rand::{Rng, RngExt};

  use super::GeoUtils;
  use crate::core::geo::rectangle::Rectangle;
  use crate::core::util::SloppyMath;
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::geo::earth_debugger::EarthDebugger;
  use crate::test::core::geo::geo_test_util::GeoTestUtil;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{at_least, random};

  struct TestGeoUtils;

  #[test]
  fn test_random_circle_to_bbox() -> Result<()> {
    let mut random = random();
    let iters = at_least(&mut random, 100);

    for _ in 0..iters {
      let center_lat = GeoTestUtil::next_latitude(&mut random);
      let center_lon = GeoTestUtil::next_longitude(&mut random);

      let radius_meters = if random.random_bool(0.5) {
        random.random::<f64>() * 444000.0
      } else {
        random.random::<f64>() * 50000000.0
      };

      let bbox = Rectangle::from_point_distance(center_lat, center_lon, radius_meters)?;

      let num_points_to_try = 1000;
      for _ in 0..num_points_to_try {
        let point = GeoTestUtil::next_point_near(&mut random, &bbox)?;
        let lat = point[0];
        let lon = point[1];

        let distance_meters = SloppyMath::haversin_meters(center_lat, center_lon, lat, lon);

        let haversin_says = distance_meters <= radius_meters;

        let bbox_says = if bbox.crosses_dateline() {
          if lat >= bbox.min_lat && lat <= bbox.max_lat {
            lon <= bbox.max_lon || lon >= bbox.min_lon
          } else {
            false
          }
        } else {
          lat >= bbox.min_lat && lat <= bbox.max_lat && lon >= bbox.min_lon && lon <= bbox.max_lon
        };

        if haversin_says && !bbox_says {
          println!(
            "centerLat={} centerLon={} radiusMeters={}",
            center_lat, center_lon, radius_meters
          );
          println!(
            "  bbox: lat={} to {} lon={} to {}",
            bbox.min_lat, bbox.max_lat, bbox.min_lon, bbox.max_lon
          );
          println!("  point: lat={} lon={}", lat, lon);
          println!("  haversin: {}", distance_meters);
          unreachable!(
            "point was within the distance according to haversin, but the bbox doesn't contain it"
          );
        }
      }
    }
    Ok(())
  }
  #[test]
  fn test_bounding_box_opto() -> Result<()> {
    let mut random = random();
    let iters = at_least(&mut random, 100);

    for _ in 0..iters {
      let lat = GeoTestUtil::next_latitude(&mut random);
      let lon = GeoTestUtil::next_longitude(&mut random);
      let radius = 50000000.0 * random.random::<f64>();
      let box_ = Rectangle::from_point_distance(lat, lon, radius)?;

      let (box1, box2) = if box_.crosses_dateline() {
        (
          Rectangle::new(box_.min_lat, box_.max_lat, -180.0, box_.max_lon)?,
          Some(Rectangle::new(
            box_.min_lat,
            box_.max_lat,
            box_.min_lon,
            180.0,
          )?),
        )
      } else {
        (box_.clone(), None)
      };

      for _ in 0..1000 {
        let point = GeoTestUtil::next_point_near(&mut random, &box_)?;
        let lat2 = point[0];
        let lon2 = point[1];

        if SloppyMath::haversin_meters(lat, lon, lat2, lon2) <= radius {
          assert!(lat >= box_.min_lat && lat <= box_.max_lat);
          assert!(
            (lon >= box1.min_lon && lon <= box1.max_lon)
              || box2
                .as_ref()
                .is_some_and(|b| lon >= b.min_lon && lon <= b.max_lon)
          );
        }
      }
    }

    Ok(())
  }

  #[test]
  fn test_haversin_opto() -> Result<()> {
    let mut random = random();
    let iters = at_least(&mut random, 100);

    for _iter in 0..iters {
      let lat = GeoTestUtil::next_latitude(&mut random);
      let lon = GeoTestUtil::next_longitude(&mut random);
      let radius = 50000000.0 * random.random::<f64>();
      let box_ = Rectangle::from_point_distance(lat, lon, radius)?;

      if box_.max_lon - lon < 90.0 && lon - box_.min_lon < 90.0 {
        let min_partial_distance = f64::max(
          SloppyMath::haversin_sort_key(lat, lon, lat, box_.max_lon),
          SloppyMath::haversin_sort_key(lat, lon, box_.max_lat, lon),
        );

        for _ in 0..10000 {
          let point = GeoTestUtil::next_point_near(&mut random, &box_)?;
          let lat2 = point[0];
          let lon2 = point[1];
          if SloppyMath::haversin_meters(lat, lon, lat2, lon2) <= radius {
            assert!(SloppyMath::haversin_sort_key(lat, lon, lat2, lon2) <= min_partial_distance);
          }
        }
      }
    }

    Ok(())
  }

  #[test]
  fn test_infinite_rect() -> Result<()> {
    let mut random = random();
    for _ in 0..1000 {
      let center_lat = GeoTestUtil::next_latitude(&mut random);
      let center_lon = GeoTestUtil::next_longitude(&mut random);
      let rect = Rectangle::from_point_distance(center_lat, center_lon, f64::INFINITY)?;
      assert_eq!(-180.0, rect.min_lon);
      assert_eq!(180.0, rect.max_lon);
      assert_eq!(-90.0, rect.min_lat);
      assert_eq!(90.0, rect.max_lat);
      assert!(!rect.crosses_dateline());
    }
    Ok(())
  }

  #[test]
  fn test_axis_lat() -> Result<()> {
    let earth_circumference = 2.0 * std::f64::consts::PI * GeoUtils::EARTH_MEAN_RADIUS_METERS;
    assert_eq!(90.0, Rectangle::axis_lat(0.0, earth_circumference / 4.0));

    let mut random = random();
    for _ in 0..100 {
      let really_big = random.random_range(0..10) == 0;
      let max_radius = if really_big {
        1.1 * earth_circumference
      } else {
        earth_circumference / 8.0
      };
      let radius = max_radius * random.random::<f64>();
      let mut prev_axis_lat = Rectangle::axis_lat(0.0, radius);
      let mut lat = 0.1f64;
      while lat < 90.0 {
        let next_axis_lat = Rectangle::axis_lat(lat, radius);
        let bbox = Rectangle::from_point_distance(lat, 180.0, radius)?;
        let dist = SloppyMath::haversin_meters(lat, 180.0, next_axis_lat, bbox.max_lon);
        if next_axis_lat < GeoUtils::MAX_LAT_INCL {
          assert!(
            (dist - radius).abs() <= 0.1,
            "lat = {lat}, dist = {dist}, radius = {radius}"
          );
        }
        assert!(prev_axis_lat <= next_axis_lat, "lat = {lat}");
        prev_axis_lat = next_axis_lat;
        lat += 0.1;
      }

      prev_axis_lat = Rectangle::axis_lat(-0.0, radius);
      let mut lat = -0.1f64;
      while lat > -90.0 {
        let next_axis_lat = Rectangle::axis_lat(lat, radius);
        let bbox = Rectangle::from_point_distance(lat, 180.0, radius)?;
        let dist = SloppyMath::haversin_meters(lat, 180.0, next_axis_lat, bbox.max_lon);
        if next_axis_lat > GeoUtils::MIN_LAT_INCL {
          assert!(
            (dist - radius).abs() <= 0.1,
            "lat = {lat}, dist = {dist}, radius = {radius}"
          );
        }
        assert!(prev_axis_lat >= next_axis_lat, "lat = {lat}");
        prev_axis_lat = next_axis_lat;
        lat -= 0.1;
      }
    }

    Ok(())
  }

  #[test]
  fn test_circle_opto() -> Result<()> {
    let mut random = random();
    let iters = at_least(&mut random, 3);

    let mut i = 0;
    while i < iters {
      let center_lat = -90.0 + 180.0 * random.random::<f64>();
      let center_lon = -180.0 + 360.0 * random.random::<f64>();
      let radius = 50_000_000.0 * random.random::<f64>();
      let box_ = Rectangle::from_point_distance(center_lat, center_lon, radius)?;
      if box_.crosses_dateline() {
        continue;
      }
      let axis_lat = Rectangle::axis_lat(center_lat, radius);

      let inner_iters = at_least(&mut random, 100);
      for _ in 0..inner_iters {
        let lat_bounds = [-90.0, box_.min_lat, axis_lat, box_.max_lat, 90.0];
        let lon_bounds = [-180.0, box_.min_lon, center_lon, box_.max_lon, 180.0];

        let max_lat_row = random.random_range(0..4);
        let lat_max = random_in_range(
          &mut random,
          lat_bounds[max_lat_row],
          lat_bounds[max_lat_row + 1],
        );

        let min_lon_col = random.random_range(0..4);
        let lon_min = random_in_range(
          &mut random,
          lon_bounds[min_lon_col],
          lon_bounds[min_lon_col + 1],
        );

        let min_lat_max_row = if max_lat_row == 3 { 3 } else { max_lat_row + 1 };
        let min_lat_row = random.random_range(0..min_lat_max_row);
        let lat_min = random_in_range(
          &mut random,
          lat_bounds[min_lat_row],
          f64::min(lat_bounds[min_lat_row + 1], lat_max),
        );

        let max_lon_min_col = usize::max(min_lon_col, 1);
        let max_lon_col = max_lon_min_col + random.random_range(0..(4 - max_lon_min_col));
        let lon_max = random_in_range(
          &mut random,
          f64::max(lon_bounds[max_lon_col], lon_min),
          lon_bounds[max_lon_col + 1],
        );

        debug_assert!(lat_max >= lat_min);
        debug_assert!(lon_max >= lon_min);

        if is_disjoint(
          center_lat, center_lon, radius, axis_lat, lat_min, lat_max, lon_min, lon_max,
        ) {
          for _ in 0..200 {
            let mut lat = lat_min + (lat_max - lat_min) * random.random::<f64>();
            let mut lon = lon_min + (lon_max - lon_min) * random.random::<f64>();

            if random.random_bool(0.5) {
              let edge = random.random_range(0..4);
              if edge == 0 {
                lat = lat_min;
              } else if edge == 1 {
                lat = lat_max;
              } else if edge == 2 {
                lon = lon_min;
              } else {
                lon = lon_max;
              }
            }

            let distance = SloppyMath::haversin_meters(center_lat, center_lon, lat, lon);

            let ok = distance > radius;
            if !ok {
              let _ed = {
                let mut ed = EarthDebugger::new();
                ed.add_rect(lat_min, lat_max, lon_min, lon_max);
                ed.add_circle(center_lat, center_lon, radius, true)?;
                println!("{}", ed.finish()?);
                ed
              };
              panic!(
                "\nisDisjoint(\ncenterLat={}\ncenterLon={}\nradius={}\nlatMin={}\nlatMax={}\nlonMin={}\nlonMax={}) == false BUT\nhaversin({}, {}, {}, {}) = {}\nbbox={}",
                center_lat,
                center_lon,
                radius,
                lat_min,
                lat_max,
                lon_min,
                lon_max,
                center_lat,
                center_lon,
                lat,
                lon,
                distance,
                Rectangle::from_point_distance(center_lat, center_lon, radius)?
              );
            }
          }
        }
      }

      i += 1;
    }

    Ok(())
  }

  fn random_in_range<R>(random: &mut R, min: f64, max: f64) -> f64
  where
    R: Rng + ?Sized,
  {
    min + (max - min) * random.random::<f64>()
  }
  #[allow(clippy::too_many_arguments)]
  fn is_disjoint(
    center_lat: f64,
    center_lon: f64,
    radius: f64,
    axis_lat: f64,
    lat_min: f64,
    lat_max: f64,
    lon_min: f64,
    lon_max: f64,
  ) -> bool {
    if (center_lon < lon_min || center_lon > lon_max)
      && (axis_lat + Rectangle::AXISLAT_ERROR < lat_min
        || axis_lat - Rectangle::AXISLAT_ERROR > lat_max)
      && SloppyMath::haversin_meters(center_lat, center_lon, lat_min, lon_min) > radius
      && SloppyMath::haversin_meters(center_lat, center_lon, lat_min, lon_max) > radius
      && SloppyMath::haversin_meters(center_lat, center_lon, lat_max, lon_min) > radius
      && SloppyMath::haversin_meters(center_lat, center_lon, lat_max, lon_max) > radius
    {
      return true;
    }

    false
  }
  #[test]
  fn test_within_90_lon_degrees() {
    assert!(GeoUtils::within_90_lon_degrees(0.0, -80.0, 80.0));
    assert!(!GeoUtils::within_90_lon_degrees(0.0, -100.0, 80.0));
    assert!(!GeoUtils::within_90_lon_degrees(0.0, -80.0, 100.0));

    assert!(GeoUtils::within_90_lon_degrees(-150.0, 140.0, 170.0));
    assert!(!GeoUtils::within_90_lon_degrees(-150.0, 120.0, 150.0));

    assert!(GeoUtils::within_90_lon_degrees(150.0, -170.0, -140.0));
    assert!(!GeoUtils::within_90_lon_degrees(150.0, -150.0, -120.0));
  }
}
