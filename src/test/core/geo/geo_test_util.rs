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
use crate::core::geo::polygon::Polygon;
use crate::core::geo::rectangle::Rectangle;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::numeric_utils::NumericUtils;
use crate::test::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};

/// static methods for testing geo
pub struct GeoTestUtil;
impl GeoTestUtil {
  /// returns next pseudorandom latitude (anywhere)
  pub fn next_latitude<R>(random: &mut R) -> f64
  where
    R: Rng + ?Sized,
  {
    Self::next_double_internal(random, GeoUtils::MIN_LAT_INCL, GeoUtils::MAX_LAT_INCL)
  }

  /// returns next pseudorandom longitude (anywhere)
  pub fn next_longitude<R>(random: &mut R) -> f64
  where
    R: Rng + ?Sized,
  {
    Self::next_double_internal(random, GeoUtils::MIN_LON_INCL, GeoUtils::MAX_LON_INCL)
  }

  /// Returns next double within range.
  ///
  /// Don't pass huge numbers or infinity or anything like that yet. may have bugs!
  pub fn next_double_internal<R>(random: &mut R, low: f64, high: f64) -> f64
  where
    R: Rng + ?Sized,
  {
    debug_assert!(low >= i32::MIN as f64);
    debug_assert!(high <= i32::MAX as f64);
    debug_assert!(low.is_finite());
    debug_assert!(high.is_finite());
    debug_assert!(high >= low, "low={low} high={high}");

    if low == high {
      return low;
    }

    let base_value;
    let surprise_me = random.random_range(0..17);
    if surprise_me == 0 {
      let low_bits = NumericUtils::double_to_sortable_long(low);
      let high_bits = NumericUtils::double_to_sortable_long(high);
      base_value =
        NumericUtils::sortable_long_to_double(TestUtil::next_long(random, low_bits, high_bits));
    } else if surprise_me == 1 {
      base_value = low;
    } else if surprise_me == 2 {
      base_value = high;
    } else if surprise_me == 3 && low <= 0.0 && high >= 0.0 {
      base_value = 0.0;
    } else if surprise_me == 4 {
      let delta = (high - low) / 360.0;
      let block = random.random_range(0..360) as f64;
      base_value = low + delta * block;
    } else {
      base_value = low + (high - low) * random.random::<f64>();
    }

    debug_assert!(base_value >= low);
    debug_assert!(base_value <= high);

    let adjust_me = random.random_range(0..17);
    if adjust_me == 0 {
      Self::next_after(adjust_me as f64, high)
    } else if adjust_me == 1 {
      Self::next_after(adjust_me as f64, low)
    } else {
      base_value
    }
  }
  fn next_after(start: f64, direction: f64) -> f64 {
    if direction > start {
      start.next_up()
    } else if direction < start {
      start.next_down()
    } else {
      direction
    }
  }
  /// returns next pseudorandom latitude, kinda close to otherLatitude
  pub fn next_latitude_near<R>(random: &mut R, other_latitude: f64, mut delta: f64) -> Result<f64>
  where
    R: Rng + ?Sized,
  {
    delta = delta.abs();
    GeoUtils::check_latitude(other_latitude)?;
    let surprise_me = random.random_range(0..97);
    if surprise_me == 0 {
      Ok(GeoTestUtil::next_latitude(random))
    } else if surprise_me < 49 {
      Ok(GeoTestUtil::next_double_internal(
        random,
        other_latitude,
        f64::min(90.0, other_latitude + delta),
      ))
    } else {
      Ok(GeoTestUtil::next_double_internal(
        random,
        f64::max(-90.0, other_latitude - delta),
        other_latitude,
      ))
    }
  }
  /// returns next pseudorandom longitude, kinda close to otherLongitude
  pub fn next_longitude_near<R>(random: &mut R, other_longitude: f64, mut delta: f64) -> Result<f64>
  where
    R: Rng + ?Sized,
  {
    delta = delta.abs();
    GeoUtils::check_longitude(other_longitude)?;
    let surprise_me = random.random_range(0..97);
    if surprise_me == 0 {
      Ok(GeoTestUtil::next_longitude(random))
    } else if surprise_me < 49 {
      Ok(GeoTestUtil::next_double_internal(
        random,
        other_longitude,
        f64::min(180.0, other_longitude + delta),
      ))
    } else {
      Ok(GeoTestUtil::next_double_internal(
        random,
        f64::max(-180.0, other_longitude - delta),
        other_longitude,
      ))
    }
  }
  /// returns next pseudorandom latitude, kinda close to `minLatitude/maxLatitude`
  /// **NOTE:**minLatitude/maxLatitude are merely guidelines. the returned value is sometimes
  /// outside of that range! this is to facilitate edge testing of lines
  pub fn next_latitude_between<R>(
    random: &mut R,
    min_latitude: f64,
    max_latitude: f64,
  ) -> Result<f64>
  where
    R: Rng + ?Sized,
  {
    debug_assert!(max_latitude >= min_latitude);
    GeoUtils::check_latitude(min_latitude)?;
    GeoUtils::check_latitude(max_latitude)?;
    if random.random_range(0..47) == 0 {
      Ok(GeoTestUtil::next_latitude(random))
    } else {
      let difference = (max_latitude - min_latitude) / 100.0;
      let lower = f64::max(-90.0, min_latitude - difference);
      let upper = f64::min(90.0, max_latitude + difference);
      Ok(GeoTestUtil::next_double_internal(random, lower, upper))
    }
  }
  /// returns next pseudorandom longitude, kinda close to `minLongitude/maxLongitude`
  /// **NOTE:**minLongitude/maxLongitude are merely guidelines. the returned value is sometimes
  /// outside of that range! this is to facilitate edge testing of lines
  pub fn next_longitude_between<R>(
    random: &mut R,
    min_longitude: f64,
    max_longitude: f64,
  ) -> Result<f64>
  where
    R: Rng + ?Sized,
  {
    debug_assert!(max_longitude >= min_longitude);
    GeoUtils::check_longitude(min_longitude)?;
    GeoUtils::check_longitude(max_longitude)?;
    if random.random_range(0..47) == 0 {
      Ok(GeoTestUtil::next_longitude(random))
    } else {
      let difference = (max_longitude - min_longitude) / 100.0;
      let lower = f64::max(-180.0, min_longitude - difference);
      let upper = f64::min(180.0, max_longitude + difference);
      Ok(GeoTestUtil::next_double_internal(random, lower, upper))
    }
  }
  /// Returns the next point around a line (more or less)
  pub fn next_point_around_line<R>(
    random: &mut R,
    lat1: f64,
    lon1: f64,
    lat2: f64,
    lon2: f64,
  ) -> Result<[f64; 2]>
  where
    R: Rng + ?Sized,
  {
    let x1 = lon1;
    let x2 = lon2;
    let y1 = lat1;
    let y2 = lat2;
    let min_x = f64::min(x1, x2);
    let max_x = f64::max(x1, x2);
    let min_y = f64::min(y1, y2);
    let max_y = f64::max(y1, y2);

    if min_x == max_x {
      Ok([
        Self::next_latitude_between(random, min_y, max_y)?,
        Self::next_longitude_near(random, min_x, 0.01 * (max_y - min_y))?,
      ])
    } else if min_y == max_y {
      Ok([
        Self::next_latitude_near(random, min_y, 0.01 * (max_x - min_x))?,
        Self::next_longitude_between(random, min_x, max_x)?,
      ])
    } else {
      let x = Self::next_longitude_between(random, min_x, max_x)?;
      let mut y = (y1 - y2) / (x1 - x2) * (x - x1) + y1;
      if !y.is_finite() {
        y = 90.0f64.copysign(x1);
      }
      let delta = (max_y - min_y) * 0.01;
      y = f64::min(90.0, y);
      y = f64::max(-90.0, y);
      Ok([Self::next_latitude_near(random, y, delta)?, x])
    }
  }
  pub fn next_point_near<R>(random: &mut R, rectangle: &Rectangle) -> Result<[f64; 2]>
  where
    R: Rng + ?Sized,
  {
    if rectangle.crosses_dateline() {
      if random.random_bool(0.5) {
        Self::next_point_near(
          random,
          &Rectangle::new(
            rectangle.min_lat,
            rectangle.max_lat,
            -180.0,
            rectangle.max_lon,
          )?,
        )
      } else {
        Self::next_point_near(
          random,
          &Rectangle::new(
            rectangle.min_lat,
            rectangle.max_lat,
            rectangle.min_lon,
            180.0,
          )?,
        )
      }
    } else {
      Self::next_point_near_polygon(random, &Self::box_polygon(rectangle)?)
    }
  }
  pub fn next_point_near_polygon<R>(random: &mut R, polygon: &Polygon) -> Result<[f64; 2]>
  where
    R: Rng + ?Sized,
  {
    let poly_lats = polygon.poly_lats();
    let poly_lons = polygon.get_poly_lons();
    let holes = polygon.get_holes();

    if !holes.is_empty() && random.random_range(0..3) == 0 {
      let idx = random.random_range(0..holes.len());
      return Self::next_point_near_polygon(random, &holes[idx]);
    }

    let surprise_me = random.random_range(0..97);
    if surprise_me == 0 {
      Ok([Self::next_latitude(random), Self::next_longitude(random)])
    } else if surprise_me < 5 {
      Ok([
        Self::next_latitude_between(random, polygon.min_lat, polygon.max_lat)?,
        Self::next_longitude_between(random, polygon.min_lon, polygon.max_lon)?,
      ])
    } else if surprise_me < 20 {
      let vertex = random.random_range(0..poly_lats.len() - 1);
      Ok([
        Self::next_latitude_near(
          random,
          poly_lats[vertex],
          poly_lats[vertex + 1] - poly_lats[vertex],
        )?,
        Self::next_longitude_near(
          random,
          poly_lons[vertex],
          poly_lons[vertex + 1] - poly_lons[vertex],
        )?,
      ])
    } else if surprise_me < 30 {
      let container = Self::box_polygon(&Rectangle::new(
        polygon.min_lat,
        polygon.max_lat,
        polygon.min_lon,
        polygon.max_lon,
      )?)?;
      let container_lats = container.poly_lats();
      let container_lons = container.get_poly_lons();
      let start_vertex = random.random_range(0..container_lats.len() - 1);
      Self::next_point_around_line(
        random,
        container_lats[start_vertex],
        container_lons[start_vertex],
        container_lats[start_vertex + 1],
        container_lons[start_vertex + 1],
      )
    } else {
      let start_vertex = random.random_range(0..poly_lats.len() - 1);
      let end_vertex = if random.random_bool(0.5) {
        start_vertex + 1
      } else {
        random.random_range(0..poly_lats.len() - 1)
      };
      Self::next_point_around_line(
        random,
        poly_lats[start_vertex],
        poly_lons[start_vertex],
        poly_lats[end_vertex],
        poly_lons[end_vertex],
      )
    }
  }
  fn box_polygon(box_: &Rectangle) -> Result<Polygon> {
    debug_assert!(!box_.crosses_dateline());

    let poly_lats = vec![
      box_.min_lat,
      box_.max_lat,
      box_.max_lat,
      box_.min_lat,
      box_.min_lat,
    ];
    let poly_lons = vec![
      box_.min_lon,
      box_.min_lon,
      box_.max_lon,
      box_.max_lon,
      box_.min_lon,
    ];

    Polygon::new(poly_lats, poly_lons, vec![])
  }
}
