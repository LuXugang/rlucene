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
use crate::core::document::field::Store;
use crate::core::document::lat_lon_doc_values_field::LatLonDocValuesField;
use crate::core::document::lat_lon_point::LatLonPoint;
use crate::core::document::stored_field::StoredField;
use crate::core::document::string_field::StringField;
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::field_doc::FieldDoc;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::score_doc::ScoreDocLike;
use crate::core::search::sort::Sort;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::SloppyMath;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::geo::geo_test_util::GeoTestUtil;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, at_least_usize, new_directory_shared, new_index_writer_config, new_log_merge_policy,
  new_searcher_with_reader, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::cmp::Ordering;
use std::sync::Arc;
#[allow(dead_code)] // for quick search
struct TestNearest;
#[test]
fn test_nearest_neighbor_with_deleted_docs() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iwc = new_index_writer_config(&mut random);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let mut doc = Document::new();
  doc.add(LatLonPoint::new("point", 40.0, 50.0)?);
  doc.add(StringField::from_string("id", "0", Store::Yes)?);
  w.add_document(doc)?;

  let mut doc = Document::new();
  doc.add(LatLonPoint::new("point", 45.0, 55.0)?);
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  w.add_document(doc)?;

  let r = Arc::new(w.get_reader()?);
  // can't wrap because we require Lucene60PointsFormat directly but e.g. ParallelReader wraps
  // with its own points impl:
  let mut s = new_searcher_with_reader(r.clone())?;
  let top_field_docs = LatLonPoint::nearest(&s, "point", 40.0, 50.0, 1)?;
  let hit = top_field_docs.score_docs()[0].as_field().unwrap();
  assert_eq!(
    "0",
    r.stored_fields()?
      .document(hit.doc())?
      .get_field("id")
      .unwrap()
      .string_value()?
      .unwrap()
      .as_ref()
  );
  r.close()?;

  w.delete_documents_with_terms(vec![Term::from_text("id", "0")])?;
  let r = Arc::new(w.get_reader()?);
  // can't wrap because we require Lucene60PointsFormat directly but e.g. ParallelReader wraps
  // with its own points impl:
  s = new_searcher_with_reader(r.clone())?;
  let top_field_docs = LatLonPoint::nearest(&s, "point", 40.0, 50.0, 1)?;
  let hit_ref = top_field_docs.score_docs()[0].as_field();
  let hit = hit_ref.as_ref().unwrap();
  assert_eq!(
    "1",
    r.stored_fields()?
      .document(hit.doc())?
      .get_field("id")
      .unwrap()
      .string_value()?
      .unwrap()
      .as_ref()
  );
  r.close()?;
  w.close()?;
  Ok(())
}
#[test]
fn test_nearest_neighbor_with_all_deleted_docs() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iwc = new_index_writer_config(&mut random);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let mut doc = Document::new();
  doc.add(LatLonPoint::new("point", 40.0, 50.0)?);
  doc.add(StringField::from_string("id", "0", Store::Yes)?);
  w.add_document(doc)?;

  let mut doc = Document::new();
  doc.add(LatLonPoint::new("point", 45.0, 55.0)?);
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  w.add_document(doc)?;

  let r = Arc::new(w.get_reader()?);
  // can't wrap because we require Lucene60PointsFormat directly but e.g. ParallelReader wraps
  // with its own points impl:
  let mut s = new_searcher_with_reader(r.clone())?;
  let top_field_docs = LatLonPoint::nearest(&s, "point", 40.0, 50.0, 1)?;
  let hit = top_field_docs.score_docs()[0].as_field().unwrap();
  assert_eq!(
    "0",
    r.stored_fields()?
      .document(hit.doc())?
      .get_field("id")
      .unwrap()
      .string_value()?
      .unwrap()
      .as_ref()
  );
  r.close()?;

  w.delete_documents_with_terms(vec![Term::from_text("id", "0")])?;
  w.delete_documents_with_terms(vec![Term::from_text("id", "1")])?;

  let r = Arc::new(w.get_reader()?);
  // can't wrap because we require Lucene60PointsFormat directly but e.g. ParallelReader wraps
  // with its own points impl:
  s = new_searcher_with_reader(r.clone())?;
  assert_eq!(
    0,
    LatLonPoint::nearest(&s, "point", 40.0, 50.0, 1)?
      .score_docs()
      .len()
  );
  r.close()?;
  w.close()?;
  Ok(())
}

#[test]
fn test_tie_break_by_doc_id() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iwc = new_index_writer_config(&mut random);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  let mut doc = Document::new();
  doc.add(LatLonPoint::new("point", 40.0, 50.0)?);
  doc.add(StringField::from_string("id", "0", Store::Yes)?);
  w.add_document(doc)?;

  let mut doc = Document::new();
  doc.add(LatLonPoint::new("point", 40.0, 50.0)?);
  doc.add(StringField::from_string("id", "1", Store::Yes)?);
  w.add_document(doc)?;

  let r = Arc::new(directory_reader::open_from_writer(&w)?);
  // can't wrap because we require Lucene60PointsFormat directly but e.g. ParallelReader wraps
  // with its own points impl:
  let searcher = new_searcher_with_reader(r.clone())?;
  let top_field_docs = LatLonPoint::nearest(&searcher, "point", 45.0, 50.0, 2)?;

  let hit = top_field_docs.score_docs()[0].as_field().unwrap();
  assert_eq!(
    "0",
    r.stored_fields()?
      .document(hit.doc())?
      .get_field("id")
      .unwrap()
      .string_value()?
      .unwrap()
      .as_ref()
  );

  let hit = top_field_docs.score_docs()[1].as_field().unwrap();
  assert_eq!(
    "1",
    r.stored_fields()?
      .document(hit.doc())?
      .get_field("id")
      .unwrap()
      .string_value()?
      .unwrap()
      .as_ref()
  );

  r.close()?;
  w.close()?;
  Ok(())
}

#[test]
fn test_nearest_neighbor_with_no_docs() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iwc = new_index_writer_config(&mut random);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let r = Arc::new(w.get_reader()?);
  // can't wrap because we require Lucene60PointsFormat directly but e.g. ParallelReader wraps
  // with its own points impl:
  let searcher = new_searcher_with_reader(r.clone())?;
  assert_eq!(
    0,
    LatLonPoint::nearest(&searcher, "point", 40.0, 50.0, 1)?
      .score_docs()
      .len()
  );
  r.close()?;
  w.close()?;
  Ok(())
}

fn quantize_lat(lat_raw: f64) -> Result<f64> {
  Ok(GeoEncodingUtils::decode_latitude(
    GeoEncodingUtils::encode_latitude(lat_raw)?,
  ))
}

fn quantize_lon(lon_raw: f64) -> Result<f64> {
  Ok(GeoEncodingUtils::decode_longitude(
    GeoEncodingUtils::encode_longitude(lon_raw)?,
  ))
}
// TODO IMPORTANT 测试未通过：15830251830580146327/1930376928975005192
fn test_nearest_neighbor_random() -> Result<()> {
  let mut random = random();

  let num_points = at_least_usize(&mut random, 1000);
  let dir = new_directory_shared(&mut random)?;

  let mut lats = vec![0.0; num_points];
  let mut lons = vec![0.0; num_points];

  let mut iwc = new_index_writer_config(&mut random);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  iwc.set_merge_scheduler(SerialMergeScheduler::new());
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  for id in 0..num_points {
    lats[id] = quantize_lat(GeoTestUtil::next_latitude(&mut random))?;
    lons[id] = quantize_lon(GeoTestUtil::next_longitude(&mut random))?;

    let mut doc = Document::new();
    doc.add(LatLonPoint::new("point", lats[id], lons[id])?);
    doc.add(LatLonDocValuesField::new("point", lats[id], lons[id])?);
    doc.add(StoredField::from_i32("id", id as i32)?);
    w.add_document(doc)?;
  }

  if random.random_bool(0.5) {
    w.force_merge(1)?;
  }

  let r = Arc::new(w.get_reader()?);

  // can't wrap because we require Lucene60PointsFormat directly but e.g. ParallelReader wraps
  // with its own points impl:
  let s = new_searcher_with_reader(r.clone())?;
  let iters = at_least(&mut random, 100);

  for iter in 0..iters {
    if cfg!(feature = "test_log_verbose") {
      println!("\nTEST: iter={}", iter);
    }

    let point_lat = GeoTestUtil::next_latitude(&mut random);
    let point_lon = GeoTestUtil::next_longitude(&mut random);

    // dumb brute force search to get the expected result:
    let mut expected_hits = Vec::with_capacity(lats.len());
    for id in 0..lats.len() {
      let distance = SloppyMath::haversin_meters(point_lat, point_lon, lats[id], lons[id]);
      expected_hits.push(FieldDoc::with_fields(id as i32, 0.0, vec![distance.into()]));
    }

    expected_hits.sort_by(|a, b| {
      let cmp = a.fields[0]
        .as_f64()
        .unwrap()
        .total_cmp(b.fields[0].as_f64().unwrap());
      if cmp != Ordering::Equal {
        return cmp;
      }

      // tie break by smaller docID:
      a.doc().cmp(&b.doc())
    });

    let top_n = TestUtil::next_int(&mut random, 1, lats.len() as i32) as usize;
    // Also test with MatchAllDocsQuery, sorting by distance:
    let field_docs = s.search_with_sort(
      MatchAllDocsQuery::new(),
      top_n,
      Sort::with_fields(vec![LatLonDocValuesField::new_distance_sort(
        "point", point_lat, point_lon,
      )?])?,
    )?;

    let hits = LatLonPoint::nearest(&s, "point", point_lat, point_lon, top_n as i32)?;
    let mut stored_fields = r.stored_fields()?;

    #[allow(clippy::needless_range_loop)]
    for i in 0..top_n {
      let expected = &expected_hits[i];
      let expected2 = field_docs.score_docs()[i].as_field().unwrap();
      let actual = hits.score_docs()[i].as_field().unwrap();
      let _actual_doc = stored_fields.document(actual.doc())?;

      assert_eq!(expected.doc(), actual.doc());
      assert_eq!(
        *expected.fields[0].as_f64().unwrap(),
        *actual.fields[0].as_f64().unwrap()
      );

      assert_eq!(expected2.doc(), actual.doc());
      assert_eq!(
        *expected2.fields[0].as_f64().unwrap(),
        *actual.fields[0].as_f64().unwrap()
      );
    }
  }

  r.close()?;
  w.close()?;
  Ok(())
}
fn get_index_writer_config<R>(random: &mut R) -> IndexWriterConfig
where
  R: Rng + ?Sized,
{
  let iwc = new_index_writer_config(random);
  // TODO IMPORTANT setCodec未实现
  iwc
}
