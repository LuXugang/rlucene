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
use crate::core::document::shape_field::{
  DecodedTriangle, DecodedTriangleType, ShapeField, decode_triangle, encode_triangle,
};
use crate::core::geo::component2d::Component2D;
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::util::error::lucene_error::Result;
use rand::Rng;
use std::slice;
/// Base test support for encoding tessellated `XYShape` and `LatLonShape` values.
pub trait BaseShapeEncodingTestCase {
  fn encode_x(&self, x: f64) -> Result<i32>;

  fn decode_x(&self, x: i32) -> f64;

  fn encode_y(&self, y: f64) -> Result<i32>;

  fn decode_y(&self, y: i32) -> f64;

  fn next_x<R>(&mut self, random: &mut R) -> Result<f64>
  where
    R: Rng + ?Sized;

  fn next_y<R>(&mut self, random: &mut R) -> Result<f64>
  where
    R: Rng + ?Sized;

  type T;

  fn next_polygon<R>(&mut self, random: &mut R) -> Result<Self::T>
  where
    R: Rng + ?Sized;

  type Component2D: Component2D;
  fn create_polygon_2d(&self, polygon: &[Self::T]) -> Result<Self::Component2D>;

  fn test_polygon_encoding_min_lat_min_lon(&self) -> Result<()> {
    let ay = 0.0;
    let ax = 0.0;
    let by = 1.0;
    let blon = 2.0;
    let cy = 2.0;
    let cx = 1.0;

    let ay_enc = self.encode_y(ay)?;
    let ax_enc = self.encode_x(ax)?;
    let by_enc = self.encode_y(by)?;
    let bx_enc = self.encode_x(blon)?;
    let cy_enc = self.encode_y(cy)?;
    let cx_enc = self.encode_x(cx)?;

    self.verify_encoding_permutations(ay_enc, ax_enc, by_enc, bx_enc, cy_enc, cx_enc)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    encode_triangle(
      &mut b, ay_enc, ax_enc, true, by_enc, bx_enc, true, cy_enc, cx_enc, true,
    )?;
    let mut encoded = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded)?;

    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, cy_enc);
    assert_eq!(encoded.c_x, cx_enc);

    Ok(())
  }

  fn test_polygon_encoding_min_lat_max_lon(&self) -> Result<()> {
    let ay = 1.0;
    let ax = 0.0;
    let by = 0.0;
    let blon = 2.0;
    let cy = 2.0;
    let cx = 1.0;

    let ay_enc = self.encode_y(ay)?;
    let ax_enc = self.encode_x(ax)?;
    let by_enc = self.encode_y(by)?;
    let bx_enc = self.encode_x(blon)?;
    let cy_enc = self.encode_y(cy)?;
    let cx_enc = self.encode_x(cx)?;

    self.verify_encoding_permutations(ay_enc, ax_enc, by_enc, bx_enc, cy_enc, cx_enc)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    encode_triangle(
      &mut b, ay_enc, ax_enc, true, by_enc, bx_enc, true, cy_enc, cx_enc, true,
    )?;
    let mut encoded = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded)?;

    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, cy_enc);
    assert_eq!(encoded.c_x, cx_enc);

    Ok(())
  }

  fn test_polygon_encoding_max_lat_max_lon(&self) -> Result<()> {
    let ay = 1.0;
    let ax = 0.0;
    let by = 2.0;
    let blon = 2.0;
    let cy = 0.0;
    let cx = 1.0;

    let ay_enc = self.encode_y(ay)?;
    let ax_enc = self.encode_x(ax)?;
    let by_enc = self.encode_y(cy)?;
    let bx_enc = self.encode_x(cx)?;
    let cy_enc = self.encode_y(by)?;
    let cx_enc = self.encode_x(blon)?;

    self.verify_encoding_permutations(ay_enc, ax_enc, by_enc, bx_enc, cy_enc, cx_enc)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    encode_triangle(
      &mut b, ay_enc, ax_enc, true, by_enc, bx_enc, true, cy_enc, cx_enc, true,
    )?;
    let mut encoded = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded)?;

    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, cy_enc);
    assert_eq!(encoded.c_x, cx_enc);

    Ok(())
  }

  fn test_polygon_encoding_max_lat_min_lon(&self) -> Result<()> {
    let ay = 2.0;
    let ax = 0.0;
    let by = 1.0;
    let blon = 2.0;
    let cy = 0.0;
    let cx = 1.0;

    let ay_enc = self.encode_y(ay)?;
    let ax_enc = self.encode_x(ax)?;
    let by_enc = self.encode_y(cy)?;
    let bx_enc = self.encode_x(cx)?;
    let cy_enc = self.encode_y(by)?;
    let cx_enc = self.encode_x(blon)?;

    self.verify_encoding_permutations(ay_enc, ax_enc, by_enc, bx_enc, cy_enc, cx_enc)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    encode_triangle(
      &mut b, ay_enc, ax_enc, true, by_enc, bx_enc, true, cy_enc, cx_enc, true,
    )?;
    let mut encoded = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded)?;

    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, cy_enc);
    assert_eq!(encoded.c_x, cx_enc);

    Ok(())
  }
  fn test_polygon_encoding_min_lat_min_lon_max_lat_max_lon_below(&self) -> Result<()> {
    let ay = 0.0;
    let ax = 0.0;
    let by = 0.25;
    let blon = 0.75;
    let cy = 2.0;
    let cx = 2.0;

    let ay_enc = self.encode_y(ay)?;
    let ax_enc = self.encode_x(ax)?;
    let by_enc = self.encode_y(by)?;
    let bx_enc = self.encode_x(blon)?;
    let cy_enc = self.encode_y(cy)?;
    let cx_enc = self.encode_x(cx)?;

    self.verify_encoding_permutations(ay_enc, ax_enc, by_enc, bx_enc, cy_enc, cx_enc)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    encode_triangle(
      &mut b, ay_enc, ax_enc, true, by_enc, bx_enc, true, cy_enc, cx_enc, true,
    )?;
    let mut encoded = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded)?;

    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, cy_enc);
    assert_eq!(encoded.c_x, cx_enc);

    Ok(())
  }

  fn test_polygon_encoding_min_lat_min_lon_max_lat_max_lon_above(&self) -> Result<()> {
    let ay = 0.0;
    let ax = 0.0;
    let by = 2.0;
    let bx = 2.0;
    let cy = 1.75;
    let cx = 1.25;

    let ay_enc = self.encode_y(ay)?;
    let ax_enc = self.encode_x(ax)?;
    let by_enc = self.encode_y(by)?;
    let bx_enc = self.encode_x(bx)?;
    let cy_enc = self.encode_y(cy)?;
    let cx_enc = self.encode_x(cx)?;

    self.verify_encoding_permutations(ay_enc, ax_enc, by_enc, bx_enc, cy_enc, cx_enc)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    encode_triangle(
      &mut b, ay_enc, ax_enc, true, by_enc, bx_enc, true, cy_enc, cx_enc, true,
    )?;
    let mut encoded = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded)?;

    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, cy_enc);
    assert_eq!(encoded.c_x, cx_enc);

    Ok(())
  }

  fn test_polygon_encoding_min_lat_max_lon_max_lat_min_lon_below(&self) -> Result<()> {
    let ay = 8.0;
    let ax = 6.0;
    let by = 6.25;
    let bx = 6.75;
    let cy = 6.0;
    let cx = 8.0;

    let ay_enc = self.encode_y(ay)?;
    let ax_enc = self.encode_x(ax)?;
    let by_enc = self.encode_y(by)?;
    let bx_enc = self.encode_x(bx)?;
    let cy_enc = self.encode_y(cy)?;
    let cx_enc = self.encode_x(cx)?;

    self.verify_encoding_permutations(ay_enc, ax_enc, by_enc, bx_enc, cy_enc, cx_enc)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    encode_triangle(
      &mut b, ay_enc, ax_enc, true, by_enc, bx_enc, true, cy_enc, cx_enc, true,
    )?;
    let mut encoded = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded)?;

    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, cy_enc);
    assert_eq!(encoded.c_x, cx_enc);

    Ok(())
  }
  fn test_polygon_encoding_min_lat_max_lon_max_lat_min_lon_above(&self) -> Result<()> {
    let ay = 2.0;
    let ax = 0.0;
    let by = 0.0;
    let bx = 2.0;
    let cy = 1.75;
    let cx = 1.25;

    let ay_enc = self.encode_y(ay)?;
    let ax_enc = self.encode_x(ax)?;
    let by_enc = self.encode_y(by)?;
    let bx_enc = self.encode_x(bx)?;
    let cy_enc = self.encode_y(cy)?;
    let cx_enc = self.encode_x(cx)?;

    self.verify_encoding_permutations(ay_enc, ax_enc, by_enc, bx_enc, cy_enc, cx_enc)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    encode_triangle(
      &mut b, ay_enc, ax_enc, true, by_enc, bx_enc, true, cy_enc, cx_enc, true,
    )?;
    let mut encoded = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded)?;

    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, cy_enc);
    assert_eq!(encoded.c_x, cx_enc);

    Ok(())
  }

  fn test_polygon_encoding_all_shared_above(&self) -> Result<()> {
    let ay = 0.0;
    let ax = 0.0;
    let by = 0.0;
    let bx = 2.0;
    let cy = 2.0;
    let cx = 2.0;

    let ay_enc = self.encode_y(ay)?;
    let ax_enc = self.encode_x(ax)?;
    let by_enc = self.encode_y(by)?;
    let bx_enc = self.encode_x(bx)?;
    let cy_enc = self.encode_y(cy)?;
    let cx_enc = self.encode_x(cx)?;

    self.verify_encoding_permutations(ay_enc, ax_enc, by_enc, bx_enc, cy_enc, cx_enc)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    encode_triangle(
      &mut b, ay_enc, ax_enc, true, by_enc, bx_enc, true, cy_enc, cx_enc, true,
    )?;
    let mut encoded = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded)?;

    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, cy_enc);
    assert_eq!(encoded.c_x, cx_enc);

    Ok(())
  }

  fn test_polygon_encoding_all_shared_below(&self) -> Result<()> {
    let ay = 2.0;
    let ax = 0.0;
    let by = 0.0;
    let bx = 0.0;
    let cy = 2.0;
    let cx = 2.0;

    let ay_enc = self.encode_y(ay)?;
    let ax_enc = self.encode_x(ax)?;
    let by_enc = self.encode_y(by)?;
    let bx_enc = self.encode_x(bx)?;
    let cy_enc = self.encode_y(cy)?;
    let cx_enc = self.encode_x(cx)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    encode_triangle(
      &mut b, ay_enc, ax_enc, true, by_enc, bx_enc, true, cy_enc, cx_enc, true,
    )?;
    let mut encoded = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded)?;

    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, cy_enc);
    assert_eq!(encoded.c_x, cx_enc);

    Ok(())
  }
  fn verify_encoding_permutations(
    &self,
    ay_enc: i32,
    ax_enc: i32,
    by_enc: i32,
    bx_enc: i32,
    cy_enc: i32,
    cx_enc: i32,
  ) -> Result<()> {
    assert_ne!(
      GeoUtils::orient(
        ay_enc as f64,
        ax_enc as f64,
        by_enc as f64,
        bx_enc as f64,
        cy_enc as f64,
        cx_enc as f64
      ),
      0
    );

    let mut b = vec![0u8; 7 * ShapeField::BYTES];

    encode_triangle(
      &mut b, ay_enc, ax_enc, true, by_enc, bx_enc, true, cy_enc, cx_enc, false,
    )?;
    let mut encoded_abc = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded_abc)?;

    encode_triangle(
      &mut b, cy_enc, cx_enc, false, ay_enc, ax_enc, true, by_enc, bx_enc, true,
    )?;
    let mut encoded_cab = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded_cab)?;
    assert_eq!(encoded_abc, encoded_cab);

    encode_triangle(
      &mut b, by_enc, bx_enc, true, cy_enc, cx_enc, false, ay_enc, ax_enc, true,
    )?;
    let mut encoded_bca = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded_bca)?;
    assert_eq!(encoded_abc, encoded_bca);

    encode_triangle(
      &mut b, cy_enc, cx_enc, true, by_enc, bx_enc, true, ay_enc, ax_enc, false,
    )?;
    let mut encoded_cba = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded_cba)?;
    assert_eq!(encoded_abc, encoded_cba);

    encode_triangle(
      &mut b, by_enc, bx_enc, true, ay_enc, ax_enc, false, cy_enc, cx_enc, true,
    )?;
    let mut encoded_bac = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded_bac)?;
    assert_eq!(encoded_abc, encoded_bac);

    encode_triangle(
      &mut b, ay_enc, ax_enc, false, cy_enc, cx_enc, true, by_enc, bx_enc, true,
    )?;
    let mut encoded_acb = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded_acb)?;
    assert_eq!(encoded_abc, encoded_acb);

    Ok(())
  }

  fn test_point_encoding(&self) -> Result<()> {
    let lat = 45.0;
    let lon = 45.0;
    let lat_enc = self.encode_y(lat)?;
    let lon_enc = self.encode_x(lon)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    encode_triangle(
      &mut b, lat_enc, lon_enc, true, lat_enc, lon_enc, true, lat_enc, lon_enc, true,
    )?;
    let mut encoded = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded)?;

    assert_eq!(encoded.a_y, lat_enc);
    assert_eq!(encoded.a_x, lon_enc);
    assert_eq!(encoded.b_y, lat_enc);
    assert_eq!(encoded.b_x, lon_enc);
    assert_eq!(encoded.c_y, lat_enc);
    assert_eq!(encoded.c_x, lon_enc);

    Ok(())
  }

  fn test_line_encoding_same_lat(&self) -> Result<()> {
    let lat = 2.0;
    let ax = 0.0;
    let bx = 2.0;
    let lat_enc = self.encode_y(lat)?;
    let ax_enc = self.encode_x(ax)?;
    let bx_enc = self.encode_x(bx)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    let mut encoded = DecodedTriangle::default();

    encode_triangle(
      &mut b, lat_enc, ax_enc, true, lat_enc, bx_enc, true, lat_enc, ax_enc, true,
    )?;
    decode_triangle(&b, &mut encoded)?;
    assert_eq!(encoded.a_y, lat_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, lat_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, lat_enc);
    assert_eq!(encoded.c_x, ax_enc);

    encode_triangle(
      &mut b, lat_enc, ax_enc, true, lat_enc, ax_enc, true, lat_enc, bx_enc, true,
    )?;
    decode_triangle(&b, &mut encoded)?;
    assert_eq!(encoded.a_y, lat_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, lat_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, lat_enc);
    assert_eq!(encoded.c_x, ax_enc);

    encode_triangle(
      &mut b, lat_enc, bx_enc, true, lat_enc, ax_enc, true, lat_enc, ax_enc, true,
    )?;
    decode_triangle(&b, &mut encoded)?;
    assert_eq!(encoded.a_y, lat_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, lat_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, lat_enc);
    assert_eq!(encoded.c_x, ax_enc);

    Ok(())
  }

  fn test_line_encoding_same_lon(&self) -> Result<()> {
    let ay = 0.0;
    let by = 2.0;
    let lon = 2.0;
    let ay_enc = self.encode_y(ay)?;
    let by_enc = self.encode_y(by)?;
    let lon_enc = self.encode_x(lon)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    let mut encoded = DecodedTriangle::default();

    encode_triangle(
      &mut b, ay_enc, lon_enc, true, by_enc, lon_enc, true, ay_enc, lon_enc, true,
    )?;
    decode_triangle(&b, &mut encoded)?;
    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, lon_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, lon_enc);
    assert_eq!(encoded.c_y, ay_enc);
    assert_eq!(encoded.c_x, lon_enc);

    encode_triangle(
      &mut b, ay_enc, lon_enc, true, ay_enc, lon_enc, true, by_enc, lon_enc, true,
    )?;
    decode_triangle(&b, &mut encoded)?;
    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, lon_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, lon_enc);
    assert_eq!(encoded.c_y, ay_enc);
    assert_eq!(encoded.c_x, lon_enc);

    encode_triangle(
      &mut b, by_enc, lon_enc, true, ay_enc, lon_enc, true, ay_enc, lon_enc, true,
    )?;
    decode_triangle(&b, &mut encoded)?;
    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, lon_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, lon_enc);
    assert_eq!(encoded.c_y, ay_enc);
    assert_eq!(encoded.c_x, lon_enc);

    Ok(())
  }

  fn test_line_encoding(&self) -> Result<()> {
    let ay = 0.0;
    let by = 2.0;
    let ax = 0.0;
    let bx = 2.0;

    let ay_enc = self.encode_y(ay)?;
    let by_enc = self.encode_y(by)?;
    let ax_enc = self.encode_x(ax)?;
    let bx_enc = self.encode_x(bx)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    let mut encoded = DecodedTriangle::default();

    encode_triangle(
      &mut b, ay_enc, ax_enc, true, by_enc, bx_enc, true, ay_enc, ax_enc, true,
    )?;
    decode_triangle(&b, &mut encoded)?;
    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, ay_enc);
    assert_eq!(encoded.c_x, ax_enc);

    encode_triangle(
      &mut b, ay_enc, ax_enc, true, ay_enc, ax_enc, true, by_enc, bx_enc, true,
    )?;
    decode_triangle(&b, &mut encoded)?;
    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, ay_enc);
    assert_eq!(encoded.c_x, ax_enc);

    encode_triangle(
      &mut b, by_enc, bx_enc, true, ay_enc, ax_enc, true, ay_enc, ax_enc, true,
    )?;
    decode_triangle(&b, &mut encoded)?;
    assert_eq!(encoded.a_y, ay_enc);
    assert_eq!(encoded.a_x, ax_enc);
    assert_eq!(encoded.b_y, by_enc);
    assert_eq!(encoded.b_x, bx_enc);
    assert_eq!(encoded.c_y, ay_enc);
    assert_eq!(encoded.c_x, ax_enc);

    Ok(())
  }
  fn test_random_point_encoding<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let ay = self.next_y(random)?;
    let ax = self.next_x(random)?;
    self.verify_encoding(ay, ax, ay, ax, ay, ax, random)
  }
  fn test_random_line_encoding<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let ay = self.next_y(random)?;
    let ax = self.next_x(random)?;
    let by = self.next_y(random)?;
    let bx = self.next_x(random)?;
    self.verify_encoding(ay, ax, by, bx, ay, ax, random)
  }

  fn test_random_polygon_encoding<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let ay = self.next_y(random)?;
    let ax = self.next_x(random)?;
    let by = self.next_y(random)?;
    let bx = self.next_x(random)?;
    let cy = self.next_y(random)?;
    let cx = self.next_x(random)?;
    self.verify_encoding(ay, ax, by, bx, cy, cx, random)
  }
  #[allow(clippy::too_many_arguments)]
  fn verify_encoding<R>(
    &mut self,
    ay: f64,
    ax: f64,
    by: f64,
    bx: f64,
    cy: f64,
    cx: f64,
    random: &mut R,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let original = [
      self.encode_x(ax)?,
      self.encode_y(ay)?,
      self.encode_x(bx)?,
      self.encode_y(by)?,
      self.encode_x(cx)?,
      self.encode_y(cy)?,
    ];

    let mut bytes = vec![0u8; 7 * ShapeField::BYTES];
    encode_triangle(
      &mut bytes,
      original[1],
      original[0],
      true,
      original[3],
      original[2],
      true,
      original[5],
      original[4],
      true,
    )?;

    let mut encoded = DecodedTriangle::new();
    decode_triangle(&bytes, &mut encoded)?;

    let encoded_quantized = [
      self.decode_x(encoded.a_x),
      self.decode_y(encoded.a_y),
      self.decode_x(encoded.b_x),
      self.decode_y(encoded.b_y),
      self.decode_x(encoded.c_x),
      self.decode_y(encoded.c_y),
    ];
    let original_quantized = self.order_triangle(
      original[0],
      original[1],
      original[2],
      original[3],
      original[4],
      original[5],
    );

    for _ in 0..100 {
      let polygon = self.next_polygon(random)?;
      let polygon_2d = self.create_polygon_2d(slice::from_ref(&polygon))?;

      let (original_intersects, encoded_intersects, original_contains, encoded_contains) =
        match encoded.type_ {
          DecodedTriangleType::Point => {
            let oi = polygon_2d.contains(original_quantized[0], original_quantized[1]);
            let ei = polygon_2d.contains(encoded_quantized[0], encoded_quantized[1]);
            (oi, ei, oi, ei)
          },
          DecodedTriangleType::Line => (
            polygon_2d.intersects_line_values(
              original_quantized[0],
              original_quantized[1],
              original_quantized[2],
              original_quantized[3],
            ),
            polygon_2d.intersects_line_values(
              encoded_quantized[0],
              encoded_quantized[1],
              encoded_quantized[2],
              encoded_quantized[3],
            ),
            polygon_2d.contains_line_values(
              original_quantized[0],
              original_quantized[1],
              original_quantized[2],
              original_quantized[3],
            ),
            polygon_2d.contains_line_values(
              encoded_quantized[0],
              encoded_quantized[1],
              encoded_quantized[2],
              encoded_quantized[3],
            ),
          ),
          DecodedTriangleType::Triangle => (
            polygon_2d.intersects_triangle_values(
              original_quantized[0],
              original_quantized[1],
              original_quantized[2],
              original_quantized[3],
              original_quantized[4],
              original_quantized[5],
            ),
            polygon_2d.intersects_triangle_values(
              encoded_quantized[0],
              encoded_quantized[1],
              encoded_quantized[2],
              encoded_quantized[3],
              encoded_quantized[4],
              encoded_quantized[5],
            ),
            polygon_2d.contains_triangle_values(
              original_quantized[0],
              original_quantized[1],
              original_quantized[2],
              original_quantized[3],
              original_quantized[4],
              original_quantized[5],
            ),
            polygon_2d.contains_triangle_values(
              encoded_quantized[0],
              encoded_quantized[1],
              encoded_quantized[2],
              encoded_quantized[3],
              encoded_quantized[4],
              encoded_quantized[5],
            ),
          ),
        };

      assert_eq!(original_intersects, encoded_intersects);
      assert_eq!(original_contains, encoded_contains);
    }

    Ok(())
  }
  fn order_triangle(&self, ax: i32, ay: i32, bx: i32, by: i32, cx: i32, cy: i32) -> [f64; 6] {
    let orientation = GeoUtils::orient(
      ax as f64, ay as f64, bx as f64, by as f64, cx as f64, cy as f64,
    );

    if orientation == -1 {
      [
        self.decode_x(cx),
        self.decode_y(cy),
        self.decode_x(bx),
        self.decode_y(by),
        self.decode_x(ax),
        self.decode_y(ay),
      ]
    } else if ax == bx && ay == by {
      if ax != cx || ay != cy {
        if ax < cx {
          [
            self.decode_x(ax),
            self.decode_y(ay),
            self.decode_x(cx),
            self.decode_y(cy),
            self.decode_x(ax),
            self.decode_y(ay),
          ]
        } else {
          [
            self.decode_x(cx),
            self.decode_y(cy),
            self.decode_x(ax),
            self.decode_y(ay),
            self.decode_x(cx),
            self.decode_y(cy),
          ]
        }
      } else {
        [
          self.decode_x(ax),
          self.decode_y(ay),
          self.decode_x(bx),
          self.decode_y(by),
          self.decode_x(cx),
          self.decode_y(cy),
        ]
      }
    } else if (ax == cx && ay == cy) || (bx == cx && by == cy) {
      if ax < bx {
        [
          self.decode_x(ax),
          self.decode_y(ay),
          self.decode_x(bx),
          self.decode_y(by),
          self.decode_x(ax),
          self.decode_y(ay),
        ]
      } else {
        [
          self.decode_x(bx),
          self.decode_y(by),
          self.decode_x(ax),
          self.decode_y(ay),
          self.decode_x(bx),
          self.decode_y(by),
        ]
      }
    } else {
      [
        self.decode_x(ax),
        self.decode_y(ay),
        self.decode_x(bx),
        self.decode_y(by),
        self.decode_x(cx),
        self.decode_y(cy),
      ]
    }
  }

  fn test_degenerated_triangle(&self) -> Result<()> {
    let ay = 1e-26_f64;
    let ax = 0.0_f64;
    let by = -1.0_f64;
    let bx = 0.0_f64;
    let cy = 1.0_f64;
    let cx = 0.0_f64;

    let ay_enc = self.encode_y(ay)?;
    let ax_enc = self.encode_x(ax)?;
    let by_enc = self.encode_y(by)?;
    let bx_enc = self.encode_x(bx)?;
    let cy_enc = self.encode_y(cy)?;
    let cx_enc = self.encode_x(cx)?;

    let mut b = vec![0u8; 7 * ShapeField::BYTES];
    encode_triangle(
      &mut b, ay_enc, ax_enc, true, by_enc, bx_enc, true, cy_enc, cx_enc, true,
    )?;
    let mut encoded = DecodedTriangle::default();
    decode_triangle(&b, &mut encoded)?;

    assert!(encoded.a_y == by_enc);
    assert!(encoded.a_x == bx_enc);
    assert!(encoded.b_y == cy_enc);
    assert!(encoded.b_x == cx_enc);
    assert!(encoded.c_y == ay_enc);
    assert!(encoded.c_x == ax_enc);

    Ok(())
  }
}
