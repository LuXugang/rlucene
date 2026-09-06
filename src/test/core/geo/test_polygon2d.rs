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
use crate::core::geo::component2d::Component2D;
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::geo::lat_lon_geometry;
use crate::core::geo::polygon::Polygon;
use crate::core::geo::polygon2d::create_from_polygon;
use crate::core::index::point_values::Relation;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::geo::geo_test_util::GeoTestUtil;
use crate::test_framework::core::util::lucene_test_case::{at_least, random};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::RngExt;

#[allow(dead_code)] // for quick search
struct TestPolygon2D;
#[test]
fn test_multi_polygon() -> Result<()> {
  let hole = Polygon::new(
    vec![-10.0, -10.0, 10.0, 10.0, -10.0],
    vec![-10.0, 10.0, 10.0, -10.0, -10.0],
    vec![],
  )?;
  let outer = Polygon::new(
    vec![-50.0, -50.0, 50.0, 50.0, -50.0],
    vec![-50.0, 50.0, 50.0, -50.0, -50.0],
    vec![hole],
  )?;
  let island = Polygon::new(
    vec![-5.0, -5.0, 5.0, 5.0, -5.0],
    vec![-5.0, 5.0, 5.0, -5.0, -5.0],
    vec![],
  )?;
  let polygon = lat_lon_geometry::create(&[outer, island])?;

  assert!(polygon.contains(-2.0, 2.0));
  assert!(!polygon.contains(-6.0, 6.0));
  assert!(polygon.contains(-25.0, 25.0));
  assert!(!polygon.contains(-51.0, 51.0));

  assert_eq!(
    Relation::CellInsideQuery,
    polygon.relate(-2.0, 2.0, -2.0, 2.0)?
  );
  assert_eq!(
    Relation::CellOutsideQuery,
    polygon.relate(6.0, 7.0, 6.0, 7.0)?
  );
  assert_eq!(
    Relation::CellInsideQuery,
    polygon.relate(24.0, 25.0, 24.0, 25.0)?
  );
  assert_eq!(
    Relation::CellOutsideQuery,
    polygon.relate(51.0, 52.0, 51.0, 52.0)?
  );
  assert_eq!(
    Relation::CellCrossesQuery,
    polygon.relate(-60.0, 60.0, -60.0, 60.0)?
  );
  assert_eq!(
    Relation::CellCrossesQuery,
    polygon.relate(49.0, 51.0, 49.0, 51.0)?
  );
  assert_eq!(
    Relation::CellCrossesQuery,
    polygon.relate(9.0, 11.0, 9.0, 11.0)?
  );
  assert_eq!(
    Relation::CellCrossesQuery,
    polygon.relate(5.0, 6.0, 5.0, 6.0)?
  );

  Ok(())
}
#[test]
fn test_pac_man() -> Result<()> {
  let px = vec![
    0.0, 10.0, 10.0, 0.0, -8.0, -10.0, -8.0, 0.0, 10.0, 10.0, 0.0,
  ];
  let py = vec![0.0, 5.0, 9.0, 10.0, 9.0, 0.0, -9.0, -10.0, -9.0, -5.0, 0.0];

  let x_min = 2.0;
  let x_max = 11.0;
  let y_min = -1.0;
  let y_max = 1.0;

  let polygon = create_from_polygon(&Polygon::new(py, px, vec![])?)?;
  assert_eq!(
    Relation::CellCrossesQuery,
    polygon.relate(y_min, y_max, x_min, x_max)?
  );
  Ok(())
}

#[test]
fn test_bounding_box() -> Result<()> {
  let mut random = random();
  for _ in 0..100 {
    let polygon = create_from_polygon(&GeoTestUtil::next_polygon(&mut random)?)?;

    for _ in 0..100 {
      let latitude = GeoTestUtil::next_latitude(&mut random);
      let longitude = GeoTestUtil::next_longitude(&mut random);
      if polygon.contains(longitude, latitude) {
        assert!(latitude >= polygon.get_min_y() && latitude <= polygon.get_max_y());
        assert!(longitude >= polygon.get_min_x() && longitude <= polygon.get_max_x());
      }
    }
  }
  Ok(())
}
#[test]
fn test_bounding_box_edge_cases() -> Result<()> {
  let mut random = random();
  for _ in 0..100 {
    let polygon = GeoTestUtil::next_polygon(&mut random)?;
    let impl_ = create_from_polygon(&polygon)?;

    for _ in 0..100 {
      let point = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
      let latitude = point[0];
      let longitude = point[1];
      if impl_.contains(longitude, latitude) {
        assert!(latitude >= polygon.min_lat && latitude <= polygon.max_lat);
        assert!(longitude >= polygon.min_lon && longitude <= polygon.max_lon);
      }
    }
  }
  Ok(())
}

#[test]
fn test_contains_random() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 50);
  for _ in 0..iters {
    let polygon = GeoTestUtil::next_polygon(&mut random)?;
    let impl_ = create_from_polygon(&polygon)?;

    for _ in 0..100 {
      let rectangle = GeoTestUtil::next_box_near(&mut random, &polygon)?;
      if impl_.relate(
        rectangle.min_lat,
        rectangle.max_lat,
        rectangle.min_lon,
        rectangle.max_lon,
      )? == Relation::CellInsideQuery
      {
        for _ in 0..500 {
          let point = GeoTestUtil::next_point_near(&mut random, &rectangle)?;
          let latitude = point[0];
          let longitude = point[1];
          if latitude >= rectangle.min_lat
            && latitude <= rectangle.max_lat
            && longitude >= rectangle.min_lon
            && longitude <= rectangle.max_lon
          {
            assert!(impl_.contains(latitude, longitude));
          }
        }
        for _ in 0..100 {
          let point = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
          let latitude = point[0];
          let longitude = point[1];
          if latitude >= rectangle.min_lat
            && latitude <= rectangle.max_lat
            && longitude >= rectangle.min_lon
            && longitude <= rectangle.max_lon
          {
            assert!(impl_.contains(latitude, longitude));
          }
        }
      }
    }
  }
  Ok(())
}
#[test]
fn test_contains_edge_cases() -> Result<()> {
  let mut random = random();
  for _ in 0..1000 {
    let polygon = GeoTestUtil::next_polygon(&mut random)?;
    let impl_ = create_from_polygon(&polygon)?;

    for _ in 0..10 {
      let rectangle = GeoTestUtil::next_box_near(&mut random, &polygon)?;
      if impl_.relate(
        rectangle.min_lon,
        rectangle.max_lon,
        rectangle.min_lat,
        rectangle.max_lat,
      )? == Relation::CellInsideQuery
      {
        for _ in 0..100 {
          let point = GeoTestUtil::next_point_near(&mut random, &rectangle)?;
          let latitude = point[0];
          let longitude = point[1];
          if latitude >= rectangle.min_lat
            && latitude <= rectangle.max_lat
            && longitude >= rectangle.min_lon
            && longitude <= rectangle.max_lon
          {
            assert!(impl_.contains(longitude, latitude));
          }
        }
        for _ in 0..20 {
          let point = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
          let latitude = point[0];
          let longitude = point[1];
          if latitude >= rectangle.min_lat
            && latitude <= rectangle.max_lat
            && longitude >= rectangle.min_lon
            && longitude <= rectangle.max_lon
          {
            assert!(impl_.contains(longitude, latitude));
          }
        }
      }
    }
  }
  Ok(())
}

#[test]
fn test_intersect_random() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 10);
  for _ in 0..iters {
    let polygon = GeoTestUtil::next_polygon(&mut random)?;
    let impl_ = create_from_polygon(&polygon)?;

    let inner_iters = at_least(&mut random, 10);
    for _ in 0..inner_iters {
      let rectangle = GeoTestUtil::next_box_near(&mut random, &polygon)?;
      if impl_.relate(
        rectangle.min_lon,
        rectangle.max_lon,
        rectangle.min_lat,
        rectangle.max_lat,
      )? == Relation::CellOutsideQuery
      {
        for _ in 0..1000 {
          let point = GeoTestUtil::next_point_near(&mut random, &rectangle)?;
          let latitude = point[0];
          let longitude = point[1];
          if latitude >= rectangle.min_lat
            && latitude <= rectangle.max_lat
            && longitude >= rectangle.min_lon
            && longitude <= rectangle.max_lon
          {
            assert!(!impl_.contains(longitude, latitude));
          }
        }
        for _ in 0..100 {
          let point = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
          let latitude = point[0];
          let longitude = point[1];
          if latitude >= rectangle.min_lat
            && latitude <= rectangle.max_lat
            && longitude >= rectangle.min_lon
            && longitude <= rectangle.max_lon
          {
            assert!(!impl_.contains(longitude, latitude));
          }
        }
      }
    }
  }
  Ok(())
}
#[test]
fn test_intersect_edge_cases() -> Result<()> {
  let mut random = random();
  for _ in 0..100 {
    let polygon = GeoTestUtil::next_polygon(&mut random)?;
    let impl_ = create_from_polygon(&polygon)?;

    for _ in 0..10 {
      let rectangle = GeoTestUtil::next_box_near(&mut random, &polygon)?;
      if impl_.relate(
        rectangle.min_lon,
        rectangle.max_lon,
        rectangle.min_lat,
        rectangle.max_lat,
      )? == Relation::CellOutsideQuery
      {
        for _ in 0..100 {
          let point = GeoTestUtil::next_point_near(&mut random, &rectangle)?;
          let latitude = point[0];
          let longitude = point[1];
          if latitude >= rectangle.min_lat
            && latitude <= rectangle.max_lat
            && longitude >= rectangle.min_lon
            && longitude <= rectangle.max_lon
          {
            assert!(!impl_.contains(longitude, latitude));
          }
        }
        for _ in 0..50 {
          let point = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
          let latitude = point[0];
          let longitude = point[1];
          if latitude >= rectangle.min_lat
            && latitude <= rectangle.max_lat
            && longitude >= rectangle.min_lon
            && longitude <= rectangle.max_lon
          {
            assert!(!impl_.contains(longitude, latitude));
          }
        }
      }
    }
  }
  Ok(())
}
#[test]
fn test_edge_insideness() -> Result<()> {
  let poly = create_from_polygon(&Polygon::new(
    vec![-2.0, -2.0, 2.0, 2.0, -2.0],
    vec![-2.0, 2.0, 2.0, -2.0, -2.0],
    vec![],
  )?)?;
  assert!(poly.contains(-2.0, -2.0));
  assert!(poly.contains(2.0, -2.0));
  assert!(poly.contains(-2.0, 2.0));
  assert!(poly.contains(2.0, 2.0));
  assert!(poly.contains(-1.0, -2.0));
  assert!(poly.contains(0.0, -2.0));
  assert!(poly.contains(1.0, -2.0));
  assert!(poly.contains(-1.0, 2.0));
  assert!(poly.contains(0.0, 2.0));
  assert!(poly.contains(1.0, 2.0));
  assert!(poly.contains(2.0, -1.0));
  assert!(poly.contains(2.0, 0.0));
  assert!(poly.contains(2.0, 1.0));
  assert!(poly.contains(-2.0, -1.0));
  assert!(poly.contains(-2.0, 0.0));
  assert!(poly.contains(-2.0, 1.0));
  Ok(())
}

#[test]
fn test_intersects_same_edge() -> Result<()> {
  let poly = create_from_polygon(&Polygon::new(
    vec![-2.0, -2.0, 2.0, 2.0, -2.0],
    vec![-2.0, 2.0, 2.0, -2.0, -2.0],
    vec![],
  )?)?;
  assert!(poly.contains_triangle_values(-1.0, -1.0, 1.0, 1.0, -1.0, -1.0));
  assert!(poly.contains_triangle_values(-2.0, -2.0, 2.0, 2.0, -2.0, -2.0));
  assert!(poly.intersects_triangle_values(-1.0, -1.0, 1.0, 1.0, -1.0, -1.0));
  assert!(poly.intersects_triangle_values(-2.0, -2.0, 2.0, 2.0, -2.0, -2.0));

  assert!(!poly.contains_triangle_values(-4.0, -4.0, 4.0, 4.0, -4.0, -4.0));
  assert!(!poly.contains_triangle_values(-2.0, -2.0, 4.0, 4.0, 4.0, 4.0));
  assert!(poly.intersects_triangle_values(-4.0, -4.0, 4.0, 4.0, -4.0, -4.0));
  assert!(poly.intersects_triangle_values(-2.0, -2.0, 4.0, 4.0, 4.0, 4.0));

  assert!(!poly.contains_triangle_values(-1.0, -1.0, 3.0, 3.0, 1.0, 1.0));
  assert!(!poly.contains_triangle_values(-2.0, -2.0, 3.0, 3.0, 2.0, 2.0));
  assert!(poly.intersects_triangle_values(-1.0, -1.0, 3.0, 3.0, 1.0, 1.0));
  assert!(poly.intersects_triangle_values(-2.0, -2.0, 3.0, 3.0, 2.0, 2.0));

  assert!(!poly.contains_triangle_values(-4.0, -4.0, 7.0, 7.0, 4.0, 4.0));
  assert!(!poly.contains_triangle_values(-2.0, -2.0, 7.0, 7.0, 4.0, 4.0));
  assert!(poly.intersects_triangle_values(-4.0, -4.0, 7.0, 7.0, 4.0, 4.0));
  assert!(poly.intersects_triangle_values(-2.0, -2.0, 7.0, 7.0, 4.0, 4.0));
  Ok(())
}

#[test]
fn test_contains_against_original() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 100);
  for _ in 0..iters {
    let mut polygon = GeoTestUtil::next_polygon(&mut random)?;
    while !polygon.get_holes().is_empty() {
      polygon = GeoTestUtil::next_polygon(&mut random)?;
    }
    let impl_ = create_from_polygon(&polygon)?;

    for _ in 0..1000 {
      let point = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
      let latitude = point[0];
      let longitude = point[1];
      let expected = GeoTestUtil::contains_slowly(&polygon, longitude, latitude);
      assert_eq!(expected, impl_.contains(latitude, longitude));
    }
  }
  Ok(())
}

#[test]
fn test_relate_triangle() -> Result<()> {
  let mut random = random();
  for _ in 0..100 {
    let polygon = GeoTestUtil::next_polygon(&mut random)?;
    let impl_ = create_from_polygon(&polygon)?;

    for _ in 0..100 {
      let a = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
      let b = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;
      let c = GeoTestUtil::next_point_near_polygon(&mut random, &polygon)?;

      if impl_.contains(a[1], a[0]) || impl_.contains(b[1], b[0]) || impl_.contains(c[1], c[0]) {
        assert!(impl_.intersects_triangle_values(a[1], a[0], b[1], b[0], c[1], c[0]));
      }
    }
  }
  Ok(())
}

#[test]
fn test_relate_triangle_contains_polygon() -> Result<()> {
  let polygon = Polygon::new(
    vec![0.0, 0.0, 1.0, 1.0, 0.0],
    vec![0.0, 1.0, 1.0, 0.0, 0.0],
    vec![],
  )?;
  let impl_ = create_from_polygon(&polygon)?;
  assert!(impl_.intersects_triangle_values(-10.0, -1.0, 2.0, -1.0, 10.0, 10.0));
  Ok(())
}

#[test]
fn test_relate_triangle_edge_cases() -> Result<()> {
  let mut random = random();
  for _ in 0..100 {
    let random_radius = random.random_range(1000..=100000);
    let num_vertices = random.random_range(100..=1000);
    let polygon =
      GeoTestUtil::create_regular_polygon(0.0, 0.0, random_radius as f64, num_vertices)?;
    let impl_ = create_from_polygon(&polygon)?;

    for j in 1..num_vertices {
      let a = [0.0, 0.0];
      let b = [polygon.get_poly_lat(j - 1), polygon.get_poly_lon(j - 1)];
      let c = if random.random_bool(0.5) {
        [polygon.get_poly_lat(j), polygon.get_poly_lon(j)]
      } else {
        [a[0], a[1]]
      };
      assert!(impl_.intersects_triangle_values(a[0], a[1], b[0], b[1], c[0], c[1]));
    }
  }
  Ok(())
}

#[test]
fn test_line_crossing_polygon_points() -> Result<()> {
  let p = Polygon::new(
    vec![0.0, -1.0, 0.0, 1.0, 0.0],
    vec![-1.0, 0.0, 1.0, 0.0, -1.0],
    vec![],
  )?;
  let polygon2d = create_from_polygon(&p)?;
  let intersects = polygon2d.intersects_triangle_values(
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(-1.5)?),
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(0.0)?),
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(1.5)?),
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(0.0)?),
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(-1.5)?),
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(0.0)?),
  );
  assert!(intersects);
  Ok(())
}

#[test]
fn test_random_line_crossing_polygon() -> Result<()> {
  let mut random = random();
  let p = GeoTestUtil::create_regular_polygon(
    0.0,
    0.0,
    1000_f64,
    TestUtil::next_usize(&mut random, 100, 10000),
  )?;
  let polygon2d = create_from_polygon(&p)?;
  for _ in 0..1000 {
    let longitude = GeoTestUtil::next_longitude(&mut random);
    let latitude = GeoTestUtil::next_latitude(&mut random);
    let intersects = polygon2d.intersects_triangle_values(
      GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(-longitude)?),
      GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(-latitude)?),
      GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(longitude)?),
      GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(latitude)?),
      GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(-longitude)?),
      GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(-latitude)?),
    );
    assert!(intersects);
  }
  Ok(())
}
