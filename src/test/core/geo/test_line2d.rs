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
use crate::core::geo::component2d::{Component2D, WithinRelation};
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::geo::line::Line;
use crate::core::geo::line2d::create_from_line;
use crate::core::index::point_values::Relation;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::geo::geo_test_util::GeoTestUtil;
use crate::test_framework::core::util::lucene_test_case::random;
#[allow(dead_code)] // for quick search
struct TestLine2D;

#[test]
fn test_triangle_disjoint() -> Result<()> {
  let line = Line::new(vec![0.0, 1.0, 2.0, 3.0], vec![0.0, 0.0, 2.0, 2.0])?;
  let line2d = create_from_line(&line)?;
  let ax = GeoEncodingUtils::encode_longitude(4.0)? as f64;
  let ay = GeoEncodingUtils::encode_latitude(4.0)? as f64;
  let bx = GeoEncodingUtils::encode_longitude(5.0)? as f64;
  let by = GeoEncodingUtils::encode_latitude(5.0)? as f64;
  let cx = GeoEncodingUtils::encode_longitude(5.0)? as f64;
  let cy = GeoEncodingUtils::encode_latitude(4.0)? as f64;
  assert!(!line2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
  assert!(!line2d.intersects_line_values(ax, ay, bx, by));
  assert!(!line2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
  assert!(!line2d.contains_line_values(ax, ay, bx, by));
  assert_eq!(
    WithinRelation::Disjoint,
    line2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
  );
  Ok(())
}

#[test]
fn test_triangle_intersects() -> Result<()> {
  let line = Line::new(vec![0.5, 0.0, 1.0, 2.0, 3.0], vec![0.5, 0.0, 0.0, 2.0, 2.0])?;
  let line2d = create_from_line(&line)?;
  let ax = GeoEncodingUtils::encode_longitude(0.0)? as f64;
  let ay = GeoEncodingUtils::encode_latitude(0.0)? as f64;
  let bx = GeoEncodingUtils::encode_longitude(1.0)? as f64;
  let by = GeoEncodingUtils::encode_latitude(0.0)? as f64;
  let cx = GeoEncodingUtils::encode_longitude(0.0)? as f64;
  let cy = GeoEncodingUtils::encode_latitude(1.0)? as f64;
  assert!(line2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
  assert!(line2d.intersects_line_values(ax, ay, bx, by));
  assert!(!line2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
  assert!(!line2d.contains_line_values(ax, ay, bx, by));
  assert_eq!(
    WithinRelation::NotWithin,
    line2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
  );
  Ok(())
}

#[test]
fn test_triangle_contains() -> Result<()> {
  let line = Line::new(vec![0.5, 0.0, 1.0, 2.0, 3.0], vec![0.5, 0.0, 0.0, 2.0, 2.0])?;
  let line2d = create_from_line(&line)?;
  let ax = GeoEncodingUtils::encode_longitude(-10.0)? as f64;
  let ay = GeoEncodingUtils::encode_latitude(-10.0)? as f64;
  let bx = GeoEncodingUtils::encode_longitude(4.0)? as f64;
  let by = GeoEncodingUtils::encode_latitude(-10.0)? as f64;
  let cx = GeoEncodingUtils::encode_longitude(4.0)? as f64;
  let cy = GeoEncodingUtils::encode_latitude(30.0)? as f64;
  assert!(line2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
  assert!(!line2d.intersects_line_values(bx, by, cx, cy));
  assert!(!line2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
  assert!(!line2d.contains_line_values(bx, by, cx, cy));
  assert_eq!(
    WithinRelation::Candidate,
    line2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
  );
  Ok(())
}

#[test]
fn test_random_triangles() -> Result<()> {
  let mut rng = random();
  let line = GeoTestUtil::next_line(&mut rng)?;
  let line2d = create_from_line(&line)?;

  for _ in 0..100 {
    let ax = GeoTestUtil::next_longitude(&mut rng);
    let ay = GeoTestUtil::next_latitude(&mut rng);
    let bx = GeoTestUtil::next_longitude(&mut rng);
    let by = GeoTestUtil::next_latitude(&mut rng);
    let cx = GeoTestUtil::next_longitude(&mut rng);
    let cy = GeoTestUtil::next_latitude(&mut rng);

    let t_min_x = ax.min(bx).min(cx);
    let t_max_x = ax.max(bx).max(cx);
    let t_min_y = ay.min(by).min(cy);
    let t_max_y = ay.max(by).max(cy);

    let r = line2d.relate(t_min_x, t_max_x, t_min_y, t_max_y)?;
    if r == Relation::CellOutsideQuery {
      assert!(!line2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
      assert!(!line2d.intersects_line_values(ax, ay, bx, by));
      assert!(!line2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
      assert!(!line2d.contains_line_values(ax, ay, bx, by));
      assert_eq!(
        WithinRelation::Disjoint,
        line2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
      );
    } else if line2d.contains_triangle_values(ax, ay, bx, by, cx, cy) {
      assert_ne!(
        WithinRelation::Candidate,
        line2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
      );
    }
  }
  Ok(())
}
