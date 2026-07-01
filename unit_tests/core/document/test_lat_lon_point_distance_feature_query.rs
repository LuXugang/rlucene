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
use crate::core::document::lat_lon_doc_values_field::LatLonDocValuesField;
use crate::core::document::lat_lon_point::LatLonPoint;
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::index::directory_reader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_reader::MultiReader;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::query::Query;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::SortField;
use crate::core::search::sort_field_enum::SortFieldEnum;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::util::SloppyMath;
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
use crate::test::support::core::search::check_hits::CheckHits;
use crate::test::support::core::search::query_utils::QueryUtils;
use crate::test::support::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_index_writer_config, new_log_merge_policy,
  new_searcher_with_reader, random,
};
use rand::RngExt;

#[allow(dead_code)] // for quick search
struct TestLatLonPointDistanceFeatureQuery;
#[test]
fn test_equals_and_hashcode() -> Result<()> {
  let q1 = LatLonPoint::new_distance_feature_query("foo", 3f32, 10.0, 10.0, 5.0)?;
  let q2 = LatLonPoint::new_distance_feature_query("foo", 3f32, 10.0, 10.0, 5.0)?;
  QueryUtils::check_equal::<Query>(&q1, &q2);

  let q3 = LatLonPoint::new_distance_feature_query("bar", 3f32, 10.0, 10.0, 5.0)?;
  QueryUtils::check_unequal::<Query>(&q1, &q3);

  let q4 = LatLonPoint::new_distance_feature_query("foo", 4f32, 10.0, 10.0, 5.0)?;
  QueryUtils::check_unequal::<Query>(&q1, &q4);

  let q5 = LatLonPoint::new_distance_feature_query("foo", 3f32, 9.0, 10.0, 5.0)?;
  QueryUtils::check_unequal::<Query>(&q1, &q5);

  let q6 = LatLonPoint::new_distance_feature_query("foo", 3f32, 10.0, 9.0, 5.0)?;
  QueryUtils::check_unequal::<Query>(&q1, &q6);

  let q7 = LatLonPoint::new_distance_feature_query("foo", 3f32, 10.0, 10.0, 6.0)?;
  QueryUtils::check_unequal::<Query>(&q1, &q7);

  Ok(())
}
#[test]
fn test_basics() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let mut doc = Document::new();
  let mut point = LatLonPoint::new("foo", 0.0, 0.0)?;
  doc.add(point.clone());
  let mut doc_value = LatLonDocValuesField::new("foo", 0.0, 0.0)?;
  doc.add(doc_value.clone());

  let pivot_distance = 5000f64;

  doc = Document::new();
  point.set_location_value(-7.0, -7.0)?;
  doc_value.set_location_value(-7.0, -7.0)?;
  doc.add(point.clone());
  doc.add(doc_value.clone());
  w.add_document(&mut random, doc)?;

  doc = Document::new();
  point.set_location_value(9.0, 9.0)?;
  doc_value.set_location_value(9.0, 9.0)?;
  doc.add(point.clone());
  doc.add(doc_value.clone());
  w.add_document(&mut random, doc.clone())?;

  doc = Document::new();
  point.set_location_value(8.0, 8.0)?;
  doc_value.set_location_value(8.0, 8.0)?;
  doc.add(point.clone());
  doc.add(doc_value.clone());
  w.add_document(&mut random, doc.clone())?;

  doc = Document::new();
  point.set_location_value(4.0, 4.0)?;
  doc_value.set_location_value(4.0, 4.0)?;
  doc.add(point.clone());
  doc.add(doc_value.clone());
  w.add_document(&mut random, doc.clone())?;

  doc = Document::new();
  point.set_location_value(-1.0, -1.0)?;
  doc_value.set_location_value(-1.0, -1.0)?;
  doc.add(point.clone());
  doc.add(doc_value.clone());
  w.add_document(&mut random, doc.clone())?;

  let reader = w.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let q = LatLonPoint::new_distance_feature_query("foo", 3f32, 10.0, 10.0, pivot_distance)?;
  let collector_manager = TopScoreDocCollectorManager::with_after(2, None, 1)?;
  let top_hits = searcher.search_with_collector_manager(q.clone(), &collector_manager)?;
  assert_eq!(2, top_hits.score_docs().len());

  let distance1 = SloppyMath::haversin_meters(
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(9.0)?),
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(9.0)?),
    10.0,
    10.0,
  );
  let distance2 = SloppyMath::haversin_meters(
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(8.0)?),
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(8.0)?),
    10.0,
    10.0,
  );

  CheckHits::check_equal(
    &q,
    &[
      ScoreDoc::new(
        1,
        3f32 * (pivot_distance / (pivot_distance + distance1)) as f32,
      ),
      ScoreDoc::new(
        2,
        3f32 * (pivot_distance / (pivot_distance + distance2)) as f32,
      ),
    ],
    top_hits.score_docs(),
  )?;

  let distance1 = SloppyMath::haversin_meters(
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(9.0)?),
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(9.0)?),
    9.0,
    9.0,
  );
  let distance2 = SloppyMath::haversin_meters(
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(8.0)?),
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(8.0)?),
    9.0,
    9.0,
  );

  let q = LatLonPoint::new_distance_feature_query("foo", 3f32, 9.0, 9.0, pivot_distance)?;
  let collector_manager = TopScoreDocCollectorManager::with_after(2, None, 1)?;
  let top_hits = searcher.search_with_collector_manager(q.clone(), &collector_manager)?;
  assert_eq!(2, top_hits.score_docs().len());
  CheckHits::check_explanations(&q, "", &searcher)?;

  CheckHits::check_equal(
    &q,
    &[
      ScoreDoc::new(
        1,
        3f32 * (pivot_distance / (pivot_distance + distance1)) as f32,
      ),
      ScoreDoc::new(
        2,
        3f32 * (pivot_distance / (pivot_distance + distance2)) as f32,
      ),
    ],
    top_hits.score_docs(),
  )?;

  w.close(&mut random)?;
  Ok(())
}
#[test]
fn test_crosses_date_line() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let mut point = LatLonPoint::new("foo", 0.0, 0.0)?;
  let mut doc_value = LatLonDocValuesField::new("foo", 0.0, 0.0)?;

  let pivot_distance = 5000f64;

  let mut doc = Document::new();
  point.set_location_value(0.0, -179.0)?;
  doc_value.set_location_value(0.0, -179.0)?;
  doc.add(point.clone());
  doc.add(doc_value.clone());
  w.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  point.set_location_value(0.0, 176.0)?;
  doc_value.set_location_value(0.0, 176.0)?;
  doc.add(point.clone());
  doc.add(doc_value.clone());
  w.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  point.set_location_value(0.0, -150.0)?;
  doc_value.set_location_value(0.0, -150.0)?;
  doc.add(point.clone());
  doc.add(doc_value.clone());
  w.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  point.set_location_value(0.0, -140.0)?;
  doc_value.set_location_value(0.0, -140.0)?;
  doc.add(point.clone());
  doc.add(doc_value.clone());
  w.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  point.set_location_value(0.0, 140.0)?;
  doc_value.set_location_value(1.0, 140.0)?;
  doc.add(point.clone());
  doc.add(doc_value.clone());
  w.add_document(&mut random, doc)?;

  let reader = w.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let q = LatLonPoint::new_distance_feature_query("foo", 3f32, 0.0, 179.0, pivot_distance)?;
  let collector_manager = TopScoreDocCollectorManager::with_after(2, None, 1)?;
  let top_hits = searcher.search_with_collector_manager(q.clone(), &collector_manager)?;
  assert_eq!(2, top_hits.score_docs().len());

  let distance1 = SloppyMath::haversin_meters(
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(0.0)?),
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(-179.0)?),
    0.0,
    179.0,
  );
  let distance2 = SloppyMath::haversin_meters(
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(0.0)?),
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(176.0)?),
    0.0,
    179.0,
  );

  CheckHits::check_equal(
    &q,
    &[
      ScoreDoc::new(
        0,
        3f32 * ((pivot_distance / (pivot_distance + distance1)) as f32),
      ),
      ScoreDoc::new(
        1,
        3f32 * ((pivot_distance / (pivot_distance + distance2)) as f32),
      ),
    ],
    top_hits.score_docs(),
  )?;

  w.close(&mut random)?;
  Ok(())
}
#[test]
fn test_missing_field() -> Result<()> {
  let reader = MultiReader::empty()?;
  let searcher = new_searcher_with_reader(reader)?;

  let q = LatLonPoint::new_distance_feature_query("foo", 3f32, 10.0, 10.0, 5000.0)?;
  let top_hits = searcher.search(q, 2)?;
  assert_eq!(0, top_hits.total_hits.value());

  Ok(())
}

#[test]
fn test_missing_value() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let mut point = LatLonPoint::new("foo", 0.0, 0.0)?;
  let mut doc_value = LatLonDocValuesField::new("foo", 0.0, 0.0)?;

  let mut doc = Document::new();
  point.set_location_value(3.0, 3.0)?;
  doc_value.set_location_value(3.0, 3.0)?;
  doc.add(point.clone());
  doc.add(doc_value.clone());
  w.add_document(&mut random, doc)?;

  w.add_document(&mut random, Document::new())?;

  let mut doc = Document::new();
  point.set_location_value(7.0, 7.0)?;
  doc_value.set_location_value(7.0, 7.0)?;
  doc.add(point.clone());
  doc.add(doc_value.clone());
  w.add_document(&mut random, doc)?;

  let reader = w.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let q = LatLonPoint::new_distance_feature_query("foo", 3f32, 10.0, 10.0, 5.0)?;
  let collector_manager = TopScoreDocCollectorManager::with_after(3, None, 1)?;
  let top_hits = searcher.search_with_collector_manager(q.clone(), &collector_manager)?;
  assert_eq!(2, top_hits.score_docs().len());

  let distance1 = SloppyMath::haversin_meters(
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(7.0)?),
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(7.0)?),
    10.0,
    10.0,
  );
  let distance2 = SloppyMath::haversin_meters(
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(3.0)?),
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(3.0)?),
    10.0,
    10.0,
  );

  CheckHits::check_equal(
    &q,
    &[
      ScoreDoc::new(2, 3f32 * ((5.0 / (5.0 + distance1)) as f32)),
      ScoreDoc::new(0, 3f32 * ((5.0 / (5.0 + distance2)) as f32)),
    ],
    top_hits.score_docs(),
  )?;

  CheckHits::check_explanations(&q, "", &searcher)?;

  w.close(&mut random)?;
  Ok(())
}

#[test]
fn test_multi_valued() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let mut doc = Document::new();
  for point in [[0.0, 0.0], [30.0, 30.0], [60.0, 60.0]] {
    doc.add(LatLonPoint::new("foo", point[0], point[1])?);
    doc.add(LatLonDocValuesField::new("foo", point[0], point[1])?);
  }
  w.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  for point in [[45.0, 0.0], [-45.0, 0.0], [-90.0, 0.0], [90.0, 0.0]] {
    doc.add(LatLonPoint::new("foo", point[0], point[1])?);
    doc.add(LatLonDocValuesField::new("foo", point[0], point[1])?);
  }
  w.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  for point in [[0.0, 90.0], [0.0, -90.0], [0.0, 180.0], [0.0, -180.0]] {
    doc.add(LatLonPoint::new("foo", point[0], point[1])?);
    doc.add(LatLonDocValuesField::new("foo", point[0], point[1])?);
  }
  w.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  {
    let point = [3.0, 2.0];
    doc.add(LatLonPoint::new("foo", point[0], point[1])?);
    doc.add(LatLonDocValuesField::new("foo", point[0], point[1])?);
  }
  w.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  for point in [[45.0, 45.0], [-45.0, -45.0]] {
    doc.add(LatLonPoint::new("foo", point[0], point[1])?);
    doc.add(LatLonDocValuesField::new("foo", point[0], point[1])?);
  }
  w.add_document(&mut random, doc)?;

  let reader = w.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut q = LatLonPoint::new_distance_feature_query("foo", 3f32, 0.0, 0.0, 200.0)?;
  let mut collector_manager = TopScoreDocCollectorManager::with_after(2, None, 1)?;
  let mut top_hits = searcher.search_with_collector_manager(q.clone(), &collector_manager)?;
  assert_eq!(2, top_hits.score_docs().len());

  let mut distance1 = SloppyMath::haversin_meters(
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(0.0)?),
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(0.0)?),
    0.0,
    0.0,
  );
  let mut distance2 = SloppyMath::haversin_meters(
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(3.0)?),
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(2.0)?),
    0.0,
    0.0,
  );

  CheckHits::check_equal(
    &q,
    &[
      ScoreDoc::new(0, 3f32 * ((200.0 / (200.0 + distance1)) as f32)),
      ScoreDoc::new(3, 3f32 * ((200.0 / (200.0 + distance2)) as f32)),
    ],
    top_hits.score_docs(),
  )?;

  q = LatLonPoint::new_distance_feature_query("foo", 3f32, -90.0, 0.0, 10000.0)?;
  collector_manager = TopScoreDocCollectorManager::with_after(2, None, 1)?;
  top_hits = searcher.search_with_collector_manager(q.clone(), &collector_manager)?;
  assert_eq!(2, top_hits.score_docs().len());
  CheckHits::check_explanations(&q, "", &searcher)?;

  distance1 = SloppyMath::haversin_meters(
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(-90.0)?),
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(0.0)?),
    -90.0,
    0.0,
  );
  distance2 = SloppyMath::haversin_meters(
    GeoEncodingUtils::decode_latitude(GeoEncodingUtils::encode_latitude(-45.0)?),
    GeoEncodingUtils::decode_longitude(GeoEncodingUtils::encode_longitude(-45.0)?),
    -90.0,
    0.0,
  );

  CheckHits::check_equal(
    &q,
    &[
      ScoreDoc::new(1, 3f32 * ((10000.0 / (10000.0 + distance1)) as f32)),
      ScoreDoc::new(4, 3f32 * ((10000.0 / (10000.0 + distance2)) as f32)),
    ],
    top_hits.score_docs(),
  )?;

  w.close(&mut random)?;
  Ok(())
}
#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  let mut point = LatLonPoint::new("foo", 0.0, 0.0)?;
  let mut doc_value = LatLonDocValuesField::new("foo", 0.0, 0.0)?;

  let num_docs = at_least(&mut random, 1000);
  for _ in 0..num_docs {
    let lat = random.random::<f64>() * 180.0 - 90.0;
    let lon = random.random::<f64>() * 360.0 - 180.0;

    let mut doc = Document::new();
    point.set_location_value(lat, lon)?;
    doc_value.set_location_value(lat, lon)?;
    doc.add(point.clone());
    doc.add(doc_value.clone());
    w.add_document(doc)?;
  }

  let reader = directory_reader::open_from_writer(&w)?;
  let searcher = new_searcher_with_reader(reader)?;

  let num_iters = at_least(&mut random, 3);
  for _ in 0..num_iters {
    let lat = random.random::<f64>() * 180.0 - 90.0;
    let lon = random.random::<f64>() * 360.0 - 180.0;
    let pivot_distance = random.random::<f64>()
      * random.random::<f64>()
      * std::f64::consts::PI
      * GeoUtils::EARTH_MEAN_RADIUS_METERS;
    let boost = (1 + random.random_range(0..10)) as f32 / 3f32;
    let q = LatLonPoint::new_distance_feature_query("foo", boost, lat, lon, pivot_distance)?;

    CheckHits::check_top_scores(&mut random, &q, &searcher)?;
  }

  w.close()?;
  Ok(())
}

#[test]
fn test_compare_sorting() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let num_docs = at_least(&mut random, 10000);
  for _ in 0..num_docs {
    let lat = random.random::<f64>() * 180.0 - 90.0;
    let lon = random.random::<f64>() * 360.0 - 180.0;

    let mut doc = Document::new();
    doc.add(LatLonPoint::new("foo", lat, lon)?);
    doc.add(LatLonDocValuesField::new("foo", lat, lon)?);
    w.add_document(&mut random, doc)?;
  }

  let reader = w.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let lat = random.random::<f64>() * 180.0 - 90.0;
  let lon = random.random::<f64>() * 360.0 - 180.0;
  let pivot_distance = random.random::<f64>()
    * random.random::<f64>()
    * GeoUtils::EARTH_MEAN_RADIUS_METERS
    * std::f64::consts::PI;
  let boost = (1 + random.random_range(0..10)) as f32 / 3.0;

  let query1 = LatLonPoint::new_distance_feature_query("foo", boost, lat, lon, pivot_distance)?;
  let sort_field: Vec<SortFieldEnum> = vec![
    SortField::get_field_score()?.into(),
    LatLonDocValuesField::new_distance_sort("foo", lat, lon)?.into(),
  ];
  let sort1 = Sort::with_fields(sort_field)?;

  let query2 = MatchAllDocsQuery::new();
  let sort_field2: Vec<SortFieldEnum> =
    vec![LatLonDocValuesField::new_distance_sort("foo", lat, lon)?.into()];
  let sort2 = Sort::with_fields(sort_field2)?;

  let top_docs1 = searcher.search_with_sort(query1, 10, sort1)?;
  let top_docs2 = searcher.search_with_sort(query2, 10, sort2)?;
  for i in 0..10 {
    assert_eq!(
      top_docs1.score_docs()[i].doc(),
      top_docs2.score_docs()[i].doc()
    );
  }

  w.close(&mut random)?;
  Ok(())
}
