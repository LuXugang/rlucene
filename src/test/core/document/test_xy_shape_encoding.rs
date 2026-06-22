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
use crate::core::geo::polygon2d::Polygon2D;
use crate::core::geo::xy_encoding_utils::XYEncodingUtils;
use crate::core::geo::xy_geometry;
use crate::core::geo::xy_geometry::XYGeometryType;
use crate::core::geo::xy_polygon::XYPolygon;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::document::base_shape_encoding_test_case::BaseShapeEncodingTestCase;
use crate::test::core::geo::shape_test_util::ShapeTestUtil;
use crate::test::core::util::lucene_test_case::random;
use rand::Rng;
use rand::prelude::StdRng;

/// Test case for LatLonShape encoding
pub struct TestXYShapeEncoding;
impl BaseShapeEncodingTestCase for TestXYShapeEncoding {
  fn encode_x(&self, x: f64) -> Result<i32> {
    XYEncodingUtils::encode(x as f32)
  }

  fn decode_x(&self, x: i32) -> f64 {
    XYEncodingUtils::decode(x) as f64
  }

  fn encode_y(&self, y: f64) -> Result<i32> {
    XYEncodingUtils::encode(y as f32)
  }

  fn decode_y(&self, y: i32) -> f64 {
    XYEncodingUtils::decode(y) as f64
  }

  fn next_x<R>(&mut self, random: &mut R) -> Result<f64>
  where
    R: Rng + ?Sized,
  {
    Ok(ShapeTestUtil::next_float(random) as f64)
  }

  fn next_y<R>(&mut self, random: &mut R) -> Result<f64>
  where
    R: Rng + ?Sized,
  {
    Ok(ShapeTestUtil::next_float(random) as f64)
  }

  type T = XYPolygon;

  fn next_polygon<R>(
    &mut self,
    random: &mut R,
  ) -> crate::core::util::error::lucene_error::Result<Self::T>
  where
    R: Rng + ?Sized,
  {
    ShapeTestUtil::next_polygon(random)
  }

  type Component2D = XYGeometryType<Polygon2D>;

  fn create_polygon_2d(
    &self,
    polygon: &[Self::T],
  ) -> crate::core::util::error::lucene_error::Result<Self::Component2D> {
    xy_geometry::create(polygon)
  }
}
fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&mut TestXYShapeEncoding, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let mut case = TestXYShapeEncoding;
  f(&mut case, &mut random)
}

mod base_shape_encoding_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::document::base_shape_encoding_test_case::BaseShapeEncodingTestCase;
  use crate::test::core::document::test_xy_shape_encoding::run_case;

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
#[test]
fn test_rotation_changes_orientation() -> Result<()> {
  let mut random = random();
  let mut case = TestXYShapeEncoding;
  let ay = -3.4028218437925203E38;
  let ax = 3.4028220466166163E38;
  let by = 3.4028218437925203E38;
  let bx = -3.4028218437925203E38;
  let cy = 3.4028230607370965E38;
  let cx = -3.4028230607370965E38;

  case.verify_encoding(ay, ax, by, bx, cy, cx, &mut random)
}
