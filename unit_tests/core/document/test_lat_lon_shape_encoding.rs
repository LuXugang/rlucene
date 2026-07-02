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
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::geo::lat_lon_geometry;
use crate::core::geo::lat_lon_geometry::LatLonGeometryType;
use crate::core::geo::polygon::Polygon;
use crate::core::geo::polygon2d::Polygon2D;
use crate::core::util::error::lucene_error::Result;
use crate::document_tests::base_shape_encoding_test_case::BaseShapeEncodingTestCase;
use crate::test_framework::core::geo::geo_test_util::GeoTestUtil;
use crate::test_framework::core::util::lucene_test_case::random;
use rand::Rng;
use rand::prelude::StdRng;
/// Test case for LatLonShape encoding
pub struct TestLatLonShapeEncoding;

impl BaseShapeEncodingTestCase for TestLatLonShapeEncoding {
  fn encode_x(&self, x: f64) -> Result<i32> {
    GeoEncodingUtils::encode_longitude(x)
  }

  fn decode_x(&self, x: i32) -> f64 {
    GeoEncodingUtils::decode_longitude(x)
  }

  fn encode_y(&self, y: f64) -> Result<i32> {
    GeoEncodingUtils::encode_latitude(y)
  }

  fn decode_y(&self, y: i32) -> f64 {
    GeoEncodingUtils::decode_latitude(y)
  }

  fn next_x<R>(&mut self, random: &mut R) -> Result<f64>
  where
    R: Rng + ?Sized,
  {
    Ok(GeoTestUtil::next_longitude(random))
  }

  fn next_y<R>(&mut self, random: &mut R) -> Result<f64>
  where
    R: Rng + ?Sized,
  {
    Ok(GeoTestUtil::next_latitude(random))
  }

  type T = Polygon;

  fn next_polygon<R>(&mut self, random: &mut R) -> Result<Self::T>
  where
    R: Rng + ?Sized,
  {
    GeoTestUtil::next_polygon(random)
  }

  type Component2D = LatLonGeometryType<Polygon2D>;

  fn create_polygon_2d(&self, polygon: &[Self::T]) -> Result<Self::Component2D> {
    lat_lon_geometry::create(polygon)
  }
}
fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&mut TestLatLonShapeEncoding, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let mut case = TestLatLonShapeEncoding;
  f(&mut case, &mut random)
}

mod base_shape_encoding_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::document_tests::base_shape_encoding_test_case::BaseShapeEncodingTestCase;
  use crate::document_tests::test_lat_lon_shape_encoding::run_case;

  #[test]
  fn test_polygon_encoding_min_lat_min_lon() -> Result<()> {
    run_case(|case, _| case.test_polygon_encoding_min_lat_min_lon())
  }

  #[test]
  fn test_polygon_encoding_min_lat_max_lon() -> Result<()> {
    run_case(|case, _| case.test_polygon_encoding_min_lat_max_lon())
  }

  #[test]
  fn test_polygon_encoding_max_lat_max_lon() -> Result<()> {
    run_case(|case, _| case.test_polygon_encoding_max_lat_max_lon())
  }

  #[test]
  fn test_polygon_encoding_max_lat_min_lon() -> Result<()> {
    run_case(|case, _| case.test_polygon_encoding_max_lat_min_lon())
  }

  #[test]
  fn test_polygon_encoding_min_lat_min_lon_max_lat_max_lon_below() -> Result<()> {
    run_case(|case, _| case.test_polygon_encoding_min_lat_min_lon_max_lat_max_lon_below())
  }

  #[test]
  fn test_polygon_encoding_min_lat_min_lon_max_lat_max_lon_above() -> Result<()> {
    run_case(|case, _| case.test_polygon_encoding_min_lat_min_lon_max_lat_max_lon_above())
  }

  #[test]
  fn test_polygon_encoding_min_lat_max_lon_max_lat_min_lon_below() -> Result<()> {
    run_case(|case, _| case.test_polygon_encoding_min_lat_max_lon_max_lat_min_lon_below())
  }

  #[test]
  fn test_polygon_encoding_min_lat_max_lon_max_lat_min_lon_above() -> Result<()> {
    run_case(|case, _| case.test_polygon_encoding_min_lat_max_lon_max_lat_min_lon_above())
  }

  #[test]
  fn test_polygon_encoding_all_shared_above() -> Result<()> {
    run_case(|case, _| case.test_polygon_encoding_all_shared_above())
  }

  #[test]
  fn test_polygon_encoding_all_shared_below() -> Result<()> {
    run_case(|case, _| case.test_polygon_encoding_all_shared_below())
  }

  #[test]
  fn test_point_encoding() -> Result<()> {
    run_case(|case, _| case.test_point_encoding())
  }

  #[test]
  fn test_line_encoding_same_lat() -> Result<()> {
    run_case(|case, _| case.test_line_encoding_same_lat())
  }

  #[test]
  fn test_line_encoding_same_lon() -> Result<()> {
    run_case(|case, _| case.test_line_encoding_same_lon())
  }

  #[test]
  fn test_line_encoding() -> Result<()> {
    run_case(|case, _| case.test_line_encoding())
  }

  #[test]
  fn test_random_point_encoding() -> Result<()> {
    run_case(|case, random| case.test_random_point_encoding(random))
  }

  #[test]
  fn test_random_line_encoding() -> Result<()> {
    run_case(|case, random| case.test_random_line_encoding(random))
  }

  #[test]
  fn test_random_polygon_encoding() -> Result<()> {
    run_case(|case, random| case.test_random_polygon_encoding(random))
  }

  #[test]
  fn test_degenerated_triangle() -> Result<()> {
    run_case(|case, _| case.test_degenerated_triangle())
  }
}
