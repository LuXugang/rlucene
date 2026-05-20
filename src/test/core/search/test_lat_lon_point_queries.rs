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
use crate::core::document::document::Document;
use crate::core::document::lat_lon_point::LatLonPoint;
use crate::core::document::shape_field::QueryRelation;
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::geo::lat_lon_geometry::LatLonGeometryEnum;
use crate::core::geo::polygon::Polygon;
use crate::core::index::directory_reader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::search::query::Query;
use crate::core::util::bkd::bkd_config::BKDConfig;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::geo::base_geo_point_test_case::BaseGeoPointTestCase;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_directory_shared, new_index_writer_config, new_searcher_with_reader, random,
};
use rand::prelude::StdRng;

pub struct TestLatLonPointQueries;

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestLatLonPointQueries, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestLatLonPointQueries;
  f(&case, &mut random)
}

mod base_geo_point_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::geo::base_geo_point_test_case::BaseGeoPointTestCase;
  use crate::test::core::search::test_lat_lon_point_queries::run_case;

  #[test]
  fn test_index_extreme_values() -> Result<()> {
    run_case(|case, _random| case.test_index_extreme_values())
  }

  #[test]
  fn test_index_out_of_range_values() -> Result<()> {
    run_case(|case, _random| case.test_index_out_of_range_values())
  }

  #[test]
  fn test_index_nan_values() -> Result<()> {
    run_case(|case, _random| case.test_index_nan_values())
  }

  #[test]
  fn test_index_inf_values() -> Result<()> {
    run_case(|case, _random| case.test_index_inf_values())
  }

  #[test]
  fn test_box_basics() -> Result<()> {
    run_case(|case, random| case.test_box_basics(random))
  }

  #[test]
  fn test_box_null() -> Result<()> {
    run_case(|case, _random| case.test_box_null())
  }

  #[test]
  fn test_box_invalid_coordinates() -> Result<()> {
    run_case(|case, _random| case.test_box_invalid_coordinates())
  }

  #[test]
  fn test_distance_basics() -> Result<()> {
    run_case(|case, random| case.test_distance_basics(random))
  }

  #[test]
  fn test_distance_null() -> Result<()> {
    run_case(|case, _random| case.test_distance_null())
  }

  #[test]
  fn test_distance_illegal() -> Result<()> {
    run_case(|case, _random| case.test_distance_illegal())
  }

  #[test]
  fn test_distance_negative() -> Result<()> {
    run_case(|case, _random| case.test_distance_negative())
  }

  #[test]
  fn test_distance_nan() -> Result<()> {
    run_case(|case, _random| case.test_distance_nan())
  }

  #[test]
  fn test_distance_inf() -> Result<()> {
    run_case(|case, _random| case.test_distance_inf())
  }

  #[test]
  fn test_polygon_basics() -> Result<()> {
    run_case(|case, random| case.test_polygon_basics(random))
  }

  #[test]
  fn test_polygon_hole() -> Result<()> {
    run_case(|case, random| case.test_polygon_hole(random))
  }

  #[test]
  fn test_polygon_hole_excludes() -> Result<()> {
    run_case(|case, random| case.test_polygon_hole_excludes(random))
  }

  #[test]
  fn test_multi_polygon_basics() -> Result<()> {
    run_case(|case, random| case.test_multi_polygon_basics(random))
  }

  #[test]
  fn test_polygon_null_field() -> Result<()> {
    run_case(|case, _random| case.test_polygon_null_field())
  }

  #[test]
  fn test_same_point_many_times() -> Result<()> {
    run_case(|case, random| case.test_same_point_many_times(random))
  }

  #[test]
  fn test_low_cardinality() -> Result<()> {
    run_case(|case, random| case.test_low_cardinality(random))
  }

  #[test]
  fn test_all_lat_equal() -> Result<()> {
    run_case(|case, random| case.test_all_lat_equal(random))
  }

  #[test]
  fn test_all_lon_equal() -> Result<()> {
    run_case(|case, random| case.test_all_lon_equal(random))
  }

  #[test]
  fn test_multi_valued() -> Result<()> {
    run_case(|case, random| case.test_multi_valued(random))
  }

  #[test]
  fn test_rect_boundaries_are_inclusive() -> Result<()> {
    run_case(|case, random| case.test_rect_boundaries_are_inclusive(random))
  }

  #[test]
  fn test_random_distance() -> Result<()> {
    run_case(|case, random| case.test_random_distance(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_random_distance_huge() -> Result<()> {
    run_case(|case, random| case.test_random_distance_huge(random))
  }

  #[test]
  fn test_equals() -> Result<()> {
    run_case(|case, random| case.test_equals(random))
  }

  #[test]
  fn test_small_set_rect() -> Result<()> {
    run_case(|case, random| case.test_small_set_rect(random))
  }

  #[test]
  fn test_small_set_dateline() -> Result<()> {
    run_case(|case, random| case.test_small_set_dateline(random))
  }

  #[test]
  fn test_small_set_multi_valued() -> Result<()> {
    run_case(|case, random| case.test_small_set_multi_valued(random))
  }

  #[test]
  fn test_small_set_whole_map() -> Result<()> {
    run_case(|case, random| case.test_small_set_whole_map(random))
  }

  #[test]
  fn test_small_set_poly() -> Result<()> {
    run_case(|case, random| case.test_small_set_poly(random))
  }

  #[test]
  fn test_small_set_poly_whole_map() -> Result<()> {
    run_case(|case, random| case.test_small_set_poly_whole_map(random))
  }

  #[test]
  fn test_small_set_distance() -> Result<()> {
    run_case(|case, random| case.test_small_set_distance(random))
  }

  #[test]
  fn test_small_set_tiny_distance() -> Result<()> {
    run_case(|case, random| case.test_small_set_tiny_distance(random))
  }

  #[test]
  fn test_small_set_distance_not_empty() -> Result<()> {
    run_case(|case, random| case.test_small_set_distance_not_empty(random))
  }

  #[test]
  fn test_small_set_huge_distance() -> Result<()> {
    run_case(|case, random| case.test_small_set_huge_distance(random))
  }

  #[test]
  fn test_small_set_distance_dateline() -> Result<()> {
    run_case(|case, random| case.test_small_set_distance_dateline(random))
  }

  #[test]
  fn test_narrow_polygon_close_to_north_pole() -> Result<()> {
    run_case(|case, random| case.test_narrow_polygon_close_to_north_pole(random))
  }
}

impl BaseGeoPointTestCase for TestLatLonPointQueries {
  fn quantize_lat(&self, lat_raw: f64) -> f64 {
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(lat_raw).expect(""))
  }

  fn quantize_lon(&self, lon_raw: f64) -> f64 {
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(lon_raw).expect(""))
  }

  fn add_point_to_doc(&self, field: &str, doc: &mut Document, lat: f64, lon: f64) -> Result<()> {
    doc.add(LatLonPoint::new(field, lat, lon)?);
    Ok(())
  }

  fn new_rect_query(
    &self,
    field: &str,
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
  ) -> Result<Query> {
    LatLonPoint::new_box_query(field, min_lat, max_lat, min_lon, max_lon)
  }

  fn new_distance_query(
    &self,
    field: &str,
    center_lat: f64,
    center_lon: f64,
    radius_meters: f64,
  ) -> Result<Query> {
    Ok(LatLonPoint::new_distance_query(field, center_lat, center_lon, radius_meters)?.into())
  }

  fn new_polygon_query(&self, field: &str, polygons: Vec<Polygon>) -> Result<Query> {
    LatLonPoint::new_polygon_query(field, polygons)
  }

  fn new_geometry_query(&self, field: &str, geometries: Vec<LatLonGeometryEnum>) -> Result<Query> {
    LatLonPoint::new_geometry_query(field, QueryRelation::Intersects, geometries)
  }
}

#[test]
fn test_distance_query_with_inverted_intersection() -> Result<()> {
  let mut random = random();
  let case = TestLatLonPointQueries;
  let num_matching_docs = at_least(
    &mut random,
    (10 * BKDConfig::DEFAULT_MAX_POINTS_IN_LEAF_NODE) as i32,
  );

  let dir = new_directory_shared(&mut random)?;
  {
    let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
    for _ in 0..num_matching_docs as usize {
      let mut doc = Document::new();
      case.add_point_to_doc("field", &mut doc, 18.313694, -65.227444)?;
      w.add_document(doc)?;
    }

    for _ in 0..11 {
      let mut doc = Document::new();
      case.add_point_to_doc("field", &mut doc, 10.0, -65.227444)?;
      w.add_document(doc)?;
    }
    w.force_merge(1)?;
    w.close()?;
  }

  let r = directory_reader::open(dir)?;
  let searcher = new_searcher_with_reader(r)?;
  assert_eq!(
    num_matching_docs,
    searcher.count(case.new_distance_query("field", 18.0, -65.0, 50_000.0)?)?
  );
  Ok(())
}
