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
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::geo::lat_lon_geometry::{
  LatLonGeometryEnum, LatLonGeometryEnumComponent2D, LatLonGeometryType, create,
};
use crate::core::geo::line::Line;
use crate::core::geo::point::Point;
use crate::core::geo::polygon::Polygon;
use crate::core::geo::rectangle::Rectangle;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::document::base_spatial_test_case::{BaseSpatialTestCase, Encoder};
use crate::test_framework::core::geo::geo_test_util::GeoTestUtil;
use rand::{Rng, RngExt};

pub type LatLonComponent2D = LatLonGeometryType<LatLonGeometryEnumComponent2D>;

/// Base test case for testing geospatial indexing and search functionality.
pub trait BaseLatLonSpatialTestCase:
  BaseSpatialTestCase<
    Line = Line,
    Polygon = Polygon,
    Rectangle = Rectangle,
    Point = Point,
    Circle = Circle,
    Component2D = LatLonComponent2D,
    Encoder = LatLonEncoder,
  >
{
}

pub struct BaseLatLonSpatialTestCaseDefaults;

impl BaseLatLonSpatialTestCaseDefaults {
  pub fn to_line_2d(lines: Vec<Line>) -> Result<LatLonComponent2D> {
    let geometries = lines
      .into_iter()
      .map(LatLonGeometryEnum::from)
      .collect::<Vec<_>>();
    create(&geometries)
  }

  pub fn to_polygon_2d(polygons: Vec<Polygon>) -> Result<LatLonComponent2D> {
    let geometries = polygons
      .into_iter()
      .map(LatLonGeometryEnum::from)
      .collect::<Vec<_>>();
    create(&geometries)
  }

  pub fn to_rectangle_2d(
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
  ) -> Result<LatLonComponent2D> {
    let rectangle = Rectangle::new(min_y, max_y, min_x, max_x)?;
    create(&[LatLonGeometryEnum::from(rectangle)])
  }

  pub fn to_point_2d(points: Vec<Point>) -> Result<LatLonComponent2D> {
    let geometries = points
      .into_iter()
      .map(LatLonGeometryEnum::from)
      .collect::<Vec<_>>();
    create(&geometries)
  }

  pub fn to_circle_2d(circle: Circle) -> Result<LatLonComponent2D> {
    create(&[LatLonGeometryEnum::from(circle)])
  }

  pub fn next_circle<R>(random: &mut R) -> Result<Circle>
  where
    R: Rng + ?Sized,
  {
    let radius_meters =
      random.random::<f64>() * GeoUtils::EARTH_MEAN_RADIUS_METERS * std::f64::consts::PI / 2.0
        + 1.0;
    Circle::new(
      GeoTestUtil::next_latitude(random),
      GeoTestUtil::next_longitude(random),
      radius_meters,
    )
  }

  pub fn random_query_box<R>(random: &mut R) -> Result<Rectangle>
  where
    R: Rng + ?Sized,
  {
    GeoTestUtil::next_box(random)
  }

  pub fn next_points<R>(random: &mut R) -> Result<Vec<Point>>
  where
    R: Rng + ?Sized,
  {
    let num_points = random.random_range(1..=20);
    let mut points = Vec::with_capacity(num_points);
    for _ in 0..num_points {
      points.push(Point::new(
        GeoTestUtil::next_latitude(random),
        GeoTestUtil::next_longitude(random),
      )?);
    }
    Ok(points)
  }

  pub fn rect_min_x(rect: &Rectangle) -> f64 {
    rect.min_lon
  }

  pub fn rect_max_x(rect: &Rectangle) -> f64 {
    rect.max_lon
  }

  pub fn rect_min_y(rect: &Rectangle) -> f64 {
    rect.min_lat
  }

  pub fn rect_max_y(rect: &Rectangle) -> f64 {
    rect.max_lat
  }

  pub fn rect_crosses_dateline(rect: &Rectangle) -> bool {
    rect.crosses_dateline()
  }

  pub fn next_line<R>(random: &mut R) -> Result<Line>
  where
    R: Rng + ?Sized,
  {
    GeoTestUtil::next_line(random)
  }

  pub fn next_polygon<R>(random: &mut R) -> Result<Polygon>
  where
    R: Rng + ?Sized,
  {
    GeoTestUtil::next_polygon(random)
  }
}

#[derive(Clone, Copy)]
pub struct LatLonEncoder;

impl Encoder for LatLonEncoder {
  fn decode_x(&self, encoded: i32) -> f64 {
    GeoEncodingUtils::decode_longitude(encoded)
  }

  fn decode_y(&self, encoded: i32) -> f64 {
    GeoEncodingUtils::decode_latitude(encoded)
  }

  fn quantize_x(&self, raw: f64) -> f64 {
    GeoEncodingUtils::decode_longitude(
      GeoEncodingUtils::encode_longitude(raw).expect("longitude must be valid"),
    )
  }

  fn quantize_x_ceil(&self, raw: f64) -> f64 {
    GeoEncodingUtils::decode_longitude(
      GeoEncodingUtils::encode_longitude_ceil(raw).expect("longitude must be valid"),
    )
  }

  fn quantize_y(&self, raw: f64) -> f64 {
    GeoEncodingUtils::decode_latitude(
      GeoEncodingUtils::encode_latitude(raw).expect("latitude must be valid"),
    )
  }

  fn quantize_y_ceil(&self, raw: f64) -> f64 {
    GeoEncodingUtils::decode_latitude(
      GeoEncodingUtils::encode_latitude_ceil(raw).expect("latitude must be valid"),
    )
  }
}
