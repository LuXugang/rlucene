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
use crate::core::geo::rectangle::Rectangle;
use crate::core::util::SloppyMath;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::geo::earth_debugger::EarthDebugger;
use crate::test::core::geo::geo_test_util::GeoTestUtil;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{at_least, random};
use rand::{Rng, RngExt};

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
