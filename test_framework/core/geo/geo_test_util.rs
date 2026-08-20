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
use crate::core::geo::circle::Circle;
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::geo::line::Line;
use crate::core::geo::point::Point;
use crate::core::geo::polygon::Polygon;
use crate::core::geo::rectangle::Rectangle;
use crate::core::util::SloppyMath;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::numeric_utils::NumericUtils;
use crate::test_framework::core::util::test_util::TestUtil;
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
    assert!(low >= i32::MIN as f64);
    assert!(high <= i32::MAX as f64);
    assert!(low.is_finite());
    assert!(high.is_finite());
    assert!(high >= low, "low={low} high={high}");

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

    assert!(base_value >= low);
    assert!(base_value <= high);

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
    assert!(max_latitude >= min_latitude);
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
    assert!(max_longitude >= min_longitude);
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
    let poly_lats = polygon.get_poly_lats();
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
      let container_lats = container.get_poly_lats();
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
  /// Returns next box for testing near a Polygon
  pub fn next_box_near<R>(random: &mut R, polygon: &Polygon) -> Result<Rectangle>
  where
    R: Rng + ?Sized,
  {
    let point1: [f64; 2];
    let point2: [f64; 2];

    let holes = polygon.get_holes();
    if !holes.is_empty() && random.random_range(0..3) == 0 {
      let idx = random.random_range(0..holes.len());
      return Self::next_box_near(random, &holes[idx]);
    }

    let surprise_me = random.random_range(0..97);
    if surprise_me == 0 {
      point1 = Self::next_point_near_polygon(random, polygon)?;
      point2 = Self::next_point_near_polygon(random, polygon)?;
    } else {
      point1 = Self::next_point_near_polygon(random, polygon)?;
      let poly_lats = polygon.get_poly_lats();
      let poly_lons = polygon.get_poly_lons();
      let vertex = random.random_range(0..poly_lats.len() - 1);
      let delta_x = poly_lons[vertex + 1] - poly_lons[vertex];
      let delta_y = poly_lats[vertex + 1] - poly_lats[vertex];
      let edge_length = (delta_x * delta_x + delta_y * delta_y).sqrt();
      point2 = [
        Self::next_latitude_near(random, point1[0], edge_length)?,
        Self::next_longitude_near(random, point1[1], edge_length)?,
      ];
    }

    let min_lat = point1[0].min(point2[0]);
    let max_lat = point1[0].max(point2[0]);
    let min_lon = point1[1].min(point2[1]);
    let max_lon = point1[1].max(point2[1]);
    Rectangle::new(min_lat, max_lat, min_lon, max_lon)
  }
  /// returns next pseudorandom box: can cross the 180th meridian
  pub fn next_box<R>(random: &mut R) -> Result<Rectangle>
  where
    R: Rng + ?Sized,
  {
    Self::next_box_internal(random, true)
  }

  /// returns next pseudorandom box: does not cross the 180th meridian
  pub fn next_box_not_crossing_dateline<R>(random: &mut R) -> Result<Rectangle>
  where
    R: Rng + ?Sized,
  {
    Self::next_box_internal(random, false)
  }

  /// Makes an n-gon, centered at the provided lat/lon, and each vertex approximately
  /// distanceMeters away from the center.
  ///
  /// Do not invoke me across the dateline or a pole!!
  pub fn create_regular_polygon(
    center_lat: f64,
    center_lon: f64,
    radius_meters: f64,
    gons: usize,
  ) -> Result<Polygon> {
    let mut result = [vec![0f64; gons + 1], vec![0f64; gons + 1]];
    #[allow(clippy::needless_range_loop)]
    for i in 0..gons {
      let angle = 360.0 - i as f64 * (360.0 / gons as f64);
      let x = angle.to_radians().cos();
      let y = angle.to_radians().sin();
      let mut factor = 2.0f64;
      let mut step = 1.0f64;
      let mut last = 0i32;

      loop {
        let lat = center_lat + y * factor;
        GeoUtils::check_latitude(lat)?;
        let lon = center_lon + x * factor;
        GeoUtils::check_longitude(lon)?;
        let distance_meters = SloppyMath::haversin_meters(center_lat, center_lon, lat, lon);

        if (distance_meters - radius_meters).abs() < 0.1 {
          result[0][i] = lat;
          result[1][i] = lon;
          break;
        }

        if distance_meters > radius_meters {
          factor -= step;
          if last == 1 {
            step /= 2.0;
          }
          last = -1;
        } else if distance_meters < radius_meters {
          factor += step;
          if last == -1 {
            step /= 2.0;
          }
          last = 1;
        }
      }
    }

    result[0][gons] = result[0][0];
    result[1][gons] = result[1][0];

    Polygon::new(result[0].clone(), result[1].clone(), vec![])
  }

  pub fn next_point<R>(random: &mut R) -> Result<Point>
  where
    R: Rng + ?Sized,
  {
    let lat = Self::next_latitude(random);
    let lon = Self::next_longitude(random);
    Point::new(lat, lon)
  }

  pub fn next_line<R>(random: &mut R) -> Result<Line>
  where
    R: Rng + ?Sized,
  {
    let p = Self::next_polygon(random)?;
    let mut lats = vec![0f64; p.num_points() - 1];
    let mut lons = vec![0f64; lats.len()];
    for i in 0..lats.len() {
      lats[i] = p.get_poly_lat(i);
      lons[i] = p.get_poly_lon(i);
    }
    Line::new(lats, lons)
  }

  pub fn next_circle<R>(random: &mut R) -> Result<Circle>
  where
    R: Rng + ?Sized,
  {
    let lat = Self::next_latitude(random);
    let lon = Self::next_longitude(random);
    let radius_meters =
      random.random::<f64>() * GeoUtils::EARTH_MEAN_RADIUS_METERS * std::f64::consts::PI / 2.0
        + 1.0;
    Circle::new(lat, lon, radius_meters)
  }

  /// returns next pseudorandom polygon
  pub fn next_polygon<R>(random: &mut R) -> Result<Polygon>
  where
    R: Rng + ?Sized,
  {
    if random.random_bool(0.5) {
      return Self::surprise_me_polygon(random);
    } else if random.random_range(0..10) == 1 {
      loop {
        let gons = TestUtil::next_int(random, 4, 500) as usize;
        let radius_meters =
          random.random::<f64>() * GeoUtils::EARTH_MEAN_RADIUS_METERS * std::f64::consts::PI / 2.0
            + 1.0;
        match Self::create_regular_polygon(
          Self::next_latitude(random),
          Self::next_longitude(random),
          radius_meters,
          gons,
        ) {
          Ok(polygon) => return Ok(polygon),
          Err(LuceneError::IllegalArgument(_)) => {},
          Err(err) => return Err(err),
        }
      }
    }

    let box_ = Self::next_box_internal(random, false)?;
    if random.random_bool(0.5) {
      Self::box_polygon(&box_)
    } else {
      Self::triangle_polygon(&box_)
    }
  }

  fn next_box_internal<R>(random: &mut R, can_cross_date_line: bool) -> Result<Rectangle>
  where
    R: Rng + ?Sized,
  {
    let mut lat0 = Self::next_latitude(random);
    let mut lat1 = Self::next_latitude(random);
    while lat0 == lat1 {
      lat1 = Self::next_latitude(random);
    }

    let mut lon0 = Self::next_longitude(random);
    let mut lon1 = Self::next_longitude(random);
    while lon0 == lon1 {
      lon1 = Self::next_longitude(random);
    }

    if lat1 < lat0 {
      std::mem::swap(&mut lat0, &mut lat1);
    }

    if !can_cross_date_line && lon1 < lon0 {
      std::mem::swap(&mut lon0, &mut lon1);
    }

    Rectangle::new(lat0, lat1, lon0, lon1)
  }

  fn box_polygon(box_: &Rectangle) -> Result<Polygon> {
    assert!(!box_.crosses_dateline());
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

  fn triangle_polygon(box_: &Rectangle) -> Result<Polygon> {
    assert!(!box_.crosses_dateline());
    let poly_lats = vec![box_.min_lat, box_.max_lat, box_.max_lat, box_.min_lat];
    let poly_lons = vec![box_.min_lon, box_.min_lon, box_.max_lon, box_.min_lon];
    Polygon::new(poly_lats, poly_lons, vec![])
  }
  fn surprise_me_polygon<R>(random: &mut R) -> Result<Polygon>
  where
    R: Rng + ?Sized,
  {
    'new_poly: loop {
      let center_lat = Self::next_latitude(random);
      let center_lon = Self::next_longitude(random);
      let radius = 0.1 + 20.0 * random.random::<f64>();
      let radius_delta = random.random::<f64>();

      let mut lats = Vec::new();
      let mut lons = Vec::new();
      let mut angle = 0.0f64;
      loop {
        angle += random.random::<f64>() * 40.0;
        if angle > 360.0 {
          break;
        }
        let len = radius * (1.0 - radius_delta + radius_delta * random.random::<f64>());
        let lat = center_lat + len * angle.to_radians().cos();
        let lon = center_lon + len * angle.to_radians().sin();
        if lon <= GeoUtils::MIN_LON_INCL
          || lon >= GeoUtils::MAX_LON_INCL
          || !(-90.0..=90.0).contains(&lat)
        {
          continue 'new_poly;
        }
        lats.push(lat);
        lons.push(lon);
      }

      lats.push(lats[0]);
      lons.push(lons[0]);

      return Polygon::new(lats, lons, vec![]);
    }
  }
  /// Simple slow point in polygon check (for testing)
  // direct port of PNPOLY C code (https://www.ecse.rpi.edu/~wrf/Research/Short_Notes/pnpoly.html)
  // this allows us to improve the code yet still ensure we have its properties
  // it is under the BSD license
  // (https://www.ecse.rpi.edu/~wrf/Research/Short_Notes/pnpoly.html#License%20to%20Use)
  //
  // Copyright (c) 1970-2003, Wm. Randolph Franklin
  //
  // Permission is hereby granted, free of charge, to any person obtaining a copy of this software
  // and associated
  // documentation files (the "Software"), to deal in the Software without restriction, including
  // without limitation
  // the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
  // the Software, and
  // to permit persons to whom the Software is furnished to do so, subject to the following
  // conditions:
  //
  // 1. Redistributions of source code must retain the above copyright
  //    notice, this list of conditions and the following disclaimers.
  // 2. Redistributions in binary form must reproduce the above copyright
  //    notice in the documentation and/or other materials provided with
  //    the distribution.
  // 3. The name of W. Randolph Franklin may not be used to endorse or
  //    promote products derived from this Software without specific
  //    prior written permission.
  //
  // THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
  // BUT NOT LIMITED
  // TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
  // NO EVENT SHALL
  // THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
  // IN AN ACTION OF
  // CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE
  // OR OTHER DEALINGS
  // IN THE SOFTWARE.
  pub fn contains_slowly(polygon: &Polygon, latitude: f64, longitude: f64) -> bool {
    if !polygon.get_holes().is_empty() {
      panic!("this testing method does not support holes");
    }
    let poly_lats = polygon.get_poly_lats();
    let poly_lons = polygon.get_poly_lons();
    if latitude < polygon.min_lat
      || latitude > polygon.max_lat
      || longitude < polygon.min_lon
      || longitude > polygon.max_lon
    {
      return false;
    }

    let mut c = false;
    let nvert = poly_lats.len();
    let verty = &poly_lats;
    let vertx = &poly_lons;
    let testy = latitude;
    let testx = longitude;
    let mut i = 0usize;
    let mut j = 1usize;
    while j < nvert {
      if (testy == verty[j] && testy == verty[i])
        || ((testy <= verty[j] && testy >= verty[i]) != (testy >= verty[j] && testy <= verty[i]))
      {
        if (testx == vertx[j] && testx == vertx[i])
          || ((testx <= vertx[j] && testx >= vertx[i]) != (testx >= vertx[j] && testx <= vertx[i])
            && GeoUtils::orient(vertx[i], verty[i], vertx[j], verty[j], testx, testy) == 0)
        {
          return true;
        } else if ((verty[i] > testy) != (verty[j] > testy))
          && (testx < (vertx[j] - vertx[i]) * (testy - verty[i]) / (verty[j] - verty[i]) + vertx[i])
        {
          c = !c;
        }
      }
      i += 1;
      j += 1;
    }
    c
  }
}
