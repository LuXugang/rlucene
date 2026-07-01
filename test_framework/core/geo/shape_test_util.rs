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
use crate::core::geo::xy_circle::XYCircle;
use crate::core::geo::xy_encoding_utils::XYEncodingUtils;
use crate::core::geo::xy_line::XYLine;
use crate::core::geo::xy_point::XYPoint;
use crate::core::geo::xy_polygon::XYPolygon;
use crate::core::geo::xy_rectangle::XYRectangle;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::support::core::util::lucene_test_case::is_night_mode;
use crate::test::support::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
/// generates random cartesian geometry; heavy reuse of GeoTestUtil
pub struct ShapeTestUtil;
impl ShapeTestUtil {
  /// returns next pseudorandom polygon
  pub fn next_polygon<R>(random: &mut R) -> Result<XYPolygon>
  where
    R: Rng + ?Sized,
  {
    if random.random_bool(0.5) {
      return Self::surprise_me_polygon(random);
    } else if is_night_mode() && random.random_range(0..10) == 1 {
      loop {
        let gons = TestUtil::next_int(random, 4, 500) as usize;
        let radius = random.random::<f64>() * 0.5 * f32::MAX as f64 + 1.0;
        match Self::create_regular_polygon(
          Self::next_float(random) as f64,
          Self::next_float(random) as f64,
          radius,
          gons,
        ) {
          Ok(polygon) => return Ok(polygon),
          Err(LuceneError::IllegalArgument(_)) => {},
          Err(err) => return Err(err),
        }
      }
    }

    let box_ = Self::next_box(random)?;
    if random.random_bool(0.5) {
      Self::box_polygon(&box_)
    } else {
      Self::triangle_polygon(&box_)
    }
  }
  pub fn next_xy_point<R>(random: &mut R) -> Result<XYPoint>
  where
    R: Rng + ?Sized,
  {
    let x = Self::next_float(random);
    let y = Self::next_float(random);
    XYPoint::new(x, y)
  }

  pub fn next_line<R>(random: &mut R) -> Result<XYLine>
  where
    R: Rng + ?Sized,
  {
    let poly = Self::next_polygon(random)?;
    let mut x = vec![0f32; poly.num_points() - 1];
    let mut y = vec![0f32; x.len()];
    for i in 0..x.len() {
      x[i] = poly.get_poly_x_at(i);
      y[i] = poly.get_poly_y_at(i);
    }
    XYLine::new(x, y)
  }
  pub fn next_circle<R>(random: &mut R) -> Result<XYCircle>
  where
    R: Rng + ?Sized,
  {
    let x = Self::next_float(random);
    let y = Self::next_float(random);
    let mut radius = 0f32;
    while radius == 0f32 {
      radius = random.random::<f32>() * f32::MAX / 2.0;
    }
    debug_assert!(radius != 0f32);
    XYCircle::new(x, y, radius)
  }

  fn triangle_polygon(box_: &XYRectangle) -> Result<XYPolygon> {
    let poly_x = vec![box_.min_x, box_.max_x, box_.max_x, box_.min_x];
    let poly_y = vec![box_.min_y, box_.min_y, box_.max_y, box_.min_y];
    XYPolygon::new(poly_x, poly_y, vec![])
  }
  pub fn next_box<R>(random: &mut R) -> Result<XYRectangle>
  where
    R: Rng + ?Sized,
  {
    let mut x0 = Self::next_float(random);
    let mut x1 = Self::next_float(random);
    while x0 == x1 {
      x1 = Self::next_float(random);
    }

    let mut y0 = Self::next_float(random);
    let mut y1 = Self::next_float(random);
    while y0 == y1 {
      y1 = Self::next_float(random);
    }

    if x1 < x0 {
      std::mem::swap(&mut x0, &mut x1);
    }

    if y1 < y0 {
      std::mem::swap(&mut y0, &mut y1);
    }

    XYRectangle::new(x0, x1, y0, y1)
  }

  fn box_polygon(box_: &XYRectangle) -> Result<XYPolygon> {
    let poly_x = vec![box_.min_x, box_.max_x, box_.max_x, box_.min_x, box_.min_x];
    let poly_y = vec![box_.min_y, box_.min_y, box_.max_y, box_.max_y, box_.min_y];
    XYPolygon::new(poly_x, poly_y, vec![])
  }
  fn surprise_me_polygon<R>(random: &mut R) -> Result<XYPolygon>
  where
    R: Rng + ?Sized,
  {
    let center_x = Self::next_float(random);
    let center_y = Self::next_float(random);
    let radius = 0.1 + 20.0 * random.random::<f64>();
    let radius_delta = random.random::<f64>();

    let mut x_list = Vec::new();
    let mut y_list = Vec::new();
    let mut angle = 0.0f64;

    loop {
      angle += random.random::<f64>() * 40.0;
      if angle > 360.0 {
        break;
      }

      let mut len = radius * (1.0 - radius_delta + radius_delta * random.random::<f64>());
      let max_x = f32::min(
        ((f32::MAX as f64) - center_x as f64).abs() as f32,
        ((-f32::MAX as f64) - center_x as f64).abs() as f32,
      );
      let max_y = f32::min(
        ((f32::MAX as f64) - center_y as f64).abs() as f32,
        ((-f32::MAX as f64) - center_y as f64).abs() as f32,
      );

      len = f64::min(len, f64::min(max_x as f64, max_y as f64));

      let x = (center_x as f64 + len * angle.to_radians().cos()) as f32;
      let y = (center_y as f64 + len * angle.to_radians().sin()) as f32;

      x_list.push(x);
      y_list.push(y);
    }

    x_list.push(x_list[0]);
    y_list.push(y_list[0]);

    XYPolygon::new(x_list, y_list, vec![])
  }
  /// Makes an n-gon, centered at the provided x/y, and each vertex approximately
  /// distanceMeters away from the center.
  ///
  /// Do not invoke me across the dateline or a pole!!
  pub fn create_regular_polygon(
    center_x: f64,
    center_y: f64,
    mut radius: f64,
    gons: usize,
  ) -> Result<XYPolygon> {
    let max_x = f64::min(
      (f32::MAX as f64 - center_x).abs(),
      (-f32::MAX as f64 - center_x).abs(),
    );
    let max_y = f64::min(
      (f32::MAX as f64 - center_y).abs(),
      (-f32::MAX as f64 - center_y).abs(),
    );

    radius = f64::min(radius, f64::min(max_x, max_y));

    let mut y = vec![0f32; gons + 1];
    let mut x = vec![0f32; gons + 1];

    for i in 0..gons {
      let angle = 360.0 - i as f64 * (360.0 / gons as f64);
      let cos = angle.to_radians().cos();
      let sin = angle.to_radians().sin();
      y[i] = (center_y + sin * radius) as f32;
      x[i] = (center_x + cos * radius) as f32;
    }

    y[gons] = y[0];
    x[gons] = x[0];

    XYPolygon::new(x, y, vec![])
  }
  pub fn next_float<R>(random: &mut R) -> f32
  where
    R: Rng + ?Sized,
  {
    Self::random_float_between(random, -f32::MAX, f32::MAX)
  }

  pub fn random_float_between<R>(random: &mut R, min: f32, max: f32) -> f32
  where
    R: Rng + ?Sized,
  {
    assert!(!min.is_nan(), "min must not be NaN");
    assert!(!max.is_nan(), "max must not be NaN");
    assert!(min <= max, "min must be <= max");

    if min == max {
      return min;
    }

    match random.random_range(0..10) {
      0 => return min,
      1 => return max,
      2 if min <= 0.0 && 0.0 <= max => return 0.0,
      _ => {},
    }

    if min.is_infinite() || max.is_infinite() {
      loop {
        let v = Self::next_float(random);
        if v >= min && v <= max {
          return v;
        }
      }
    }

    let t = random.random::<f64>();
    let value = (min as f64) + ((max as f64) - (min as f64)) * t;
    let value = value.clamp(min as f64, max as f64) as f32;

    value.clamp(min, max)
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
  pub fn contains_slowly(polygon: &XYPolygon, x: f64, y: f64) -> bool {
    if !polygon.get_holes().is_empty() {
      panic!("this testing method does not support holes");
    }

    let poly_xs = XYEncodingUtils::float_array_to_double_array(polygon.get_poly_x());
    let poly_ys = XYEncodingUtils::float_array_to_double_array(polygon.get_poly_y());

    if x < polygon.min_x as f64
      || x > polygon.max_x as f64
      || y < polygon.min_y as f64
      || y > polygon.max_y as f64
    {
      return false;
    }

    let mut c = false;
    let nvert = poly_ys.len();
    let verty = &poly_ys;
    let vertx = &poly_xs;
    let testy = y;
    let testx = x;

    let mut i = 0usize;
    let mut j = 1usize;
    while j < nvert {
      if (testy == verty[j] && testy == verty[i])
        || ((testy <= verty[j] && testy >= verty[i]) != (testy >= verty[j] && testy <= verty[i]))
      {
        if (testx == vertx[j] && testx == vertx[i])
          || (((testx <= vertx[j] && testx >= vertx[i])
            != (testx >= vertx[j] && testx <= vertx[i]))
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
