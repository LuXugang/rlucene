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
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::stored_field::StoredField;
use crate::core::document::string_field::StringField;
use crate::core::geo::circle::Circle;
use crate::core::geo::component2d::Component2D;
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::geo::lat_lon_geometry;
use crate::core::geo::lat_lon_geometry::LatLonGeometryEnum;
use crate::core::geo::polygon::Polygon;
use crate::core::geo::rectangle::Rectangle;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::{DefaultIndexWriterType, IndexWriter};
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_bits::get_live_docs;
use crate::core::index::multi_doc_values::MultiDocValues;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::Query;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::top_docs::TopDocs;
use crate::core::store::directory::Directory;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::sloppy_math::SloppyMath;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::geo::geo_test_util::GeoTestUtil;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::fixed_bit_set_collector::FixedBitSetCollector;
use crate::test::core::util::lucene_test_case::{
  at_least, create_temp_dir_with_prefix, new_directory_shared, new_fs_directory,
  new_index_writer_config, new_index_writer_config_with_analyzer, new_log_merge_policy,
  new_searcher_with_reader,
};
use rand::{Rng, RngExt};
use std::collections::HashSet;
const FIELD_NAME: &str = "point";
/// Base test support for geospatial implementations (high-level fields and queries). NOTE: This
/// test focuses on geospatial (distance queries, polygon queries, etc) indexing and search, not any
/// underlying storage format or encoding: it merely supplies two hooks for the encoding so that
/// tests can be exact. The [stretch] goal is for this test to be so thorough in testing a new geo
/// impl that if this test passes, then all Lucene tests should also pass. Ie, if there is some bug
/// in a given geo impl that this test fails to catch then this test needs to be improved!
pub trait BaseGeoPointTestCase {
  fn next_longitude<R>(&self, random: &mut R) -> f64
  where
    R: Rng + ?Sized,
  {
    GeoTestUtil::next_longitude(random)
  }

  fn next_latitude<R>(&self, random: &mut R) -> f64
  where
    R: Rng + ?Sized,
  {
    GeoTestUtil::next_latitude(random)
  }

  fn next_box<R>(&self, random: &mut R) -> Result<Rectangle>
  where
    R: Rng + ?Sized,
  {
    GeoTestUtil::next_box(random)
  }

  fn next_circle<R>(&self, random: &mut R) -> Result<Circle>
  where
    R: Rng + ?Sized,
  {
    GeoTestUtil::next_circle(random)
  }

  fn next_polygon<R>(&self, random: &mut R) -> Result<Polygon>
  where
    R: Rng + ?Sized,
  {
    GeoTestUtil::next_polygon(random)
  }

  fn next_geometry<R>(&self, random: &mut R) -> Result<Vec<LatLonGeometryEnum>>
  where
    R: Rng + ?Sized,
  {
    let length = random.random_range(1..=4);
    let mut geometries = Vec::with_capacity(length);
    for _ in 0..length {
      let geometry = match random.random_range(0..3) {
        0 => self.next_box(random)?.into(),
        1 => self.next_circle(random)?.into(),
        _ => self.next_polygon(random)?.into(),
      };
      geometries.push(geometry);
    }
    Ok(geometries)
  }

  /// Valid values that should not cause error.
  fn test_index_extreme_values(&self) -> Result<()> {
    let mut document = Document::new();
    self.add_point_to_doc("foo", &mut document, 90.0, 180.0)?;
    self.add_point_to_doc("foo", &mut document, 90.0, -180.0)?;
    self.add_point_to_doc("foo", &mut document, -90.0, 180.0)?;
    self.add_point_to_doc("foo", &mut document, -90.0, -180.0)?;
    Ok(())
  }

  /// Invalid values.
  fn test_index_out_of_range_values(&self) -> Result<()> {
    let mut document = Document::new();

    self.assert_add_point_error_contains(
      "foo",
      &mut document,
      90.0f64.next_up(),
      50.0,
      "invalid latitude",
    );
    self.assert_add_point_error_contains(
      "foo",
      &mut document,
      (-90.0f64).next_down(),
      50.0,
      "invalid latitude",
    );
    self.assert_add_point_error_contains(
      "foo",
      &mut document,
      90.0,
      180.0f64.next_up(),
      "invalid longitude",
    );
    self.assert_add_point_error_contains(
      "foo",
      &mut document,
      90.0,
      (-180.0f64).next_down(),
      "invalid longitude",
    );
    Ok(())
  }

  /// NaN: illegal.
  fn test_index_nan_values(&self) -> Result<()> {
    let mut document = Document::new();

    self.assert_add_point_error_contains("foo", &mut document, f64::NAN, 50.0, "invalid latitude");
    self.assert_add_point_error_contains("foo", &mut document, 50.0, f64::NAN, "invalid longitude");
    Ok(())
  }

  /// Inf: illegal.
  fn test_index_inf_values(&self) -> Result<()> {
    let mut document = Document::new();

    self.assert_add_point_error_contains(
      "foo",
      &mut document,
      f64::INFINITY,
      50.0,
      "invalid latitude",
    );
    self.assert_add_point_error_contains(
      "foo",
      &mut document,
      f64::NEG_INFINITY,
      50.0,
      "invalid latitude",
    );
    self.assert_add_point_error_contains(
      "foo",
      &mut document,
      50.0,
      f64::INFINITY,
      "invalid longitude",
    );
    self.assert_add_point_error_contains(
      "foo",
      &mut document,
      50.0,
      f64::NEG_INFINITY,
      "invalid longitude",
    );
    Ok(())
  }

  /// Add a single point and search for it in a box.
  fn test_box_basics<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let writer = RandomIndexWriter::new(random, dir);

    let mut document = Document::new();
    self.add_point_to_doc("field", &mut document, 18.313694, -65.227444)?;
    writer.add_document(random, document)?;

    let reader = writer.get_reader(random)?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(
      1,
      searcher.count(self.new_rect_query("field", 18.0, 19.0, -66.0, -65.0)?)?
    );

    writer.close(random)?;
    Ok(())
  }

  /// Null field name not allowed.
  fn test_box_null(&self) -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  /// Box should not accept invalid lat/lon.
  fn test_box_invalid_coordinates(&self) -> Result<()> {
    assert!(
      self
        .new_rect_query("field", -92.0, -91.0, 179.0, 181.0)
        .is_err()
    );
    Ok(())
  }

  fn assert_add_point_error_contains(
    &self,
    field: &str,
    doc: &mut Document,
    lat: f64,
    lon: f64,
    expected_message: &str,
  ) {
    let expected = self.add_point_to_doc(field, doc, lat, lon);
    assert!(expected.is_err());
    assert!(expected.unwrap_err().to_string().contains(expected_message));
  }

  fn test_distance_basics<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let writer = RandomIndexWriter::new(random, dir);

    let mut document = Document::new();
    self.add_point_to_doc("field", &mut document, 18.313694, -65.227444)?;
    writer.add_document(random, document)?;

    let reader = writer.get_reader(random)?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(
      1,
      searcher.count(self.new_distance_query("field", 18.0, -65.0, 50_000.0)?)?
    );

    writer.close(random)?;
    Ok(())
  }

  /// Null field name not allowed.
  fn test_distance_null(&self) -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  /// Distance query should not accept invalid lat/lon as origin.
  fn test_distance_illegal(&self) -> Result<()> {
    assert!(
      self
        .new_distance_query("field", 92.0, 181.0, 120_000.0)
        .is_err()
    );
    Ok(())
  }

  /// Negative distance queries are not allowed.
  fn test_distance_negative(&self) -> Result<()> {
    self.assert_distance_error_contains("field", 18.0, 19.0, -1.0, &["radiusMeters", "invalid"]);
    Ok(())
  }

  /// NaN distance queries are not allowed.
  fn test_distance_nan(&self) -> Result<()> {
    self.assert_distance_error_contains(
      "field",
      18.0,
      19.0,
      f64::NAN,
      &["radiusMeters", "invalid"],
    );
    Ok(())
  }

  /// Inf distance queries are not allowed.
  fn test_distance_inf(&self) -> Result<()> {
    self.assert_distance_error_contains(
      "field",
      18.0,
      19.0,
      f64::INFINITY,
      &["radiusMeters", "invalid"],
    );
    self.assert_distance_error_contains(
      "field",
      18.0,
      19.0,
      f64::NEG_INFINITY,
      &["radiusMeters", "invalid"],
    );
    Ok(())
  }

  fn assert_distance_error_contains(
    &self,
    field: &str,
    center_lat: f64,
    center_lon: f64,
    radius_meters: f64,
    expected_messages: &[&str],
  ) {
    let expected = self.new_distance_query(field, center_lat, center_lon, radius_meters);
    assert!(expected.is_err());
    let message = expected.unwrap_err().to_string();
    for expected_message in expected_messages {
      assert!(message.contains(expected_message));
    }
  }

  /// Test we can search for a polygon.
  fn test_polygon_basics<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let writer = RandomIndexWriter::new(random, dir);

    let mut document = Document::new();
    self.add_point_to_doc("field", &mut document, 18.313694, -65.227444)?;
    writer.add_document(random, document)?;

    let reader = writer.get_reader(random)?;
    let searcher = new_searcher_with_reader(reader)?;
    let polygon = Polygon::new(
      vec![18.0, 18.0, 19.0, 19.0, 18.0],
      vec![-66.0, -65.0, -65.0, -66.0, -66.0],
      vec![],
    )?;
    assert_eq!(
      1,
      searcher.count(self.new_polygon_query("field", vec![polygon])?)?
    );

    writer.close(random)?;
    Ok(())
  }

  /// Test we can search for a polygon with a hole (but still includes the doc).
  fn test_polygon_hole<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let writer = RandomIndexWriter::new(random, dir);

    let mut document = Document::new();
    self.add_point_to_doc("field", &mut document, 18.313694, -65.227444)?;
    writer.add_document(random, document)?;

    let reader = writer.get_reader(random)?;
    let searcher = new_searcher_with_reader(reader)?;
    let inner = Polygon::new(
      vec![18.5, 18.5, 18.7, 18.7, 18.5],
      vec![-65.7, -65.4, -65.4, -65.7, -65.7],
      vec![],
    )?;
    let outer = Polygon::new(
      vec![18.0, 18.0, 19.0, 19.0, 18.0],
      vec![-66.0, -65.0, -65.0, -66.0, -66.0],
      vec![inner],
    )?;
    assert_eq!(
      1,
      searcher.count(self.new_polygon_query("field", vec![outer])?)?
    );

    writer.close(random)?;
    Ok(())
  }

  /// Test we can search for a polygon with a hole (that excludes the doc).
  fn test_polygon_hole_excludes<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let writer = RandomIndexWriter::new(random, dir);

    let mut document = Document::new();
    self.add_point_to_doc("field", &mut document, 18.313694, -65.227444)?;
    writer.add_document(random, document)?;

    let reader = writer.get_reader(random)?;
    let searcher = new_searcher_with_reader(reader)?;
    let inner = Polygon::new(
      vec![18.2, 18.2, 18.4, 18.4, 18.2],
      vec![-65.3, -65.2, -65.2, -65.3, -65.3],
      vec![],
    )?;
    let outer = Polygon::new(
      vec![18.0, 18.0, 19.0, 19.0, 18.0],
      vec![-66.0, -65.0, -65.0, -66.0, -66.0],
      vec![inner],
    )?;
    assert_eq!(
      0,
      searcher.count(self.new_polygon_query("field", vec![outer])?)?
    );

    writer.close(random)?;
    Ok(())
  }

  /// Test we can search for a multi-polygon.
  fn test_multi_polygon_basics<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let writer = RandomIndexWriter::new(random, dir);

    let mut document = Document::new();
    self.add_point_to_doc("field", &mut document, 18.313694, -65.227444)?;
    writer.add_document(random, document)?;

    let reader = writer.get_reader(random)?;
    let searcher = new_searcher_with_reader(reader)?;
    let a = Polygon::new(
      vec![28.0, 28.0, 29.0, 29.0, 28.0],
      vec![-56.0, -55.0, -55.0, -56.0, -56.0],
      vec![],
    )?;
    let b = Polygon::new(
      vec![18.0, 18.0, 19.0, 19.0, 18.0],
      vec![-66.0, -65.0, -65.0, -66.0, -66.0],
      vec![],
    )?;
    assert_eq!(
      1,
      searcher.count(self.new_polygon_query("field", vec![a, b])?)?
    );

    writer.close(random)?;
    Ok(())
  }

  /// Null field name not allowed.
  fn test_polygon_null_field(&self) -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  fn test_same_point_many_times<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_points = at_least(random, 1000) as usize;

    let the_lat = self.next_latitude(random);
    let the_lon = self.next_longitude(random);

    let mut lats = vec![the_lat; num_points];
    let mut lons = vec![the_lon; num_points];

    self.verify(random, &mut lats, &mut lons)
  }

  fn test_low_cardinality<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_points = at_least(random, 1000) as usize;
    let cardinality = random.random_range(2..=20);

    let mut diff_lons = vec![0.0; cardinality];
    let mut diff_lats = vec![0.0; cardinality];
    for i in 0..cardinality {
      diff_lats[i] = self.next_latitude(random);
      diff_lons[i] = self.next_longitude(random);
    }

    let mut lats = vec![0.0; num_points];
    let mut lons = vec![0.0; num_points];
    for i in 0..num_points {
      let index = random.random_range(0..cardinality);
      lats[i] = diff_lats[index];
      lons[i] = diff_lons[index];
    }

    self.verify(random, &mut lats, &mut lons)
  }

  fn test_all_lat_equal<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_points = at_least(random, 1000) as usize;
    let lat = self.next_latitude(random);
    let mut lats = vec![0.0; num_points];
    let mut lons = vec![0.0; num_points];

    let mut have_real_doc = false;

    for doc_id in 0..num_points {
      let x = random.random_range(0..20);
      if x == 17 {
        lats[doc_id] = f64::NAN;
        continue;
      }

      if doc_id > 0 && x == 14 && have_real_doc {
        let old_doc_id = loop {
          let old_doc_id = random.random_range(0..doc_id);
          if !lats[old_doc_id].is_nan() {
            break old_doc_id;
          }
        };
        lons[doc_id] = lons[old_doc_id];
      } else {
        lons[doc_id] = self.next_longitude(random);
        have_real_doc = true;
      }
      lats[doc_id] = lat;
    }

    self.verify(random, &mut lats, &mut lons)
  }

  fn test_all_lon_equal<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_points = at_least(random, 1000) as usize;
    let the_lon = self.next_longitude(random);
    let mut lats = vec![0.0; num_points];
    let mut lons = vec![0.0; num_points];

    let mut have_real_doc = false;

    for doc_id in 0..num_points {
      let x = random.random_range(0..20);
      if x == 17 {
        lats[doc_id] = f64::NAN;
        continue;
      }

      if doc_id > 0 && x == 14 && have_real_doc {
        let old_doc_id = loop {
          let old_doc_id = random.random_range(0..doc_id);
          if !lats[old_doc_id].is_nan() {
            break old_doc_id;
          }
        };
        lats[doc_id] = lats[old_doc_id];
      } else {
        lats[doc_id] = self.next_latitude(random);
        have_real_doc = true;
      }
      lons[doc_id] = the_lon;
    }

    self.verify(random, &mut lats, &mut lons)
  }

  fn test_multi_valued<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_points = at_least(random, 1000) as usize;
    let mut lats = vec![0.0; 2 * num_points];
    let mut lons = vec![0.0; 2 * num_points];
    let dir = new_directory_shared(random)?;
    let mut iwc = new_index_writer_config(random);

    iwc.set_merge_policy(new_log_merge_policy(random)?);
    iwc.set_merge_scheduler(SerialMergeScheduler::new());
    let w = RandomIndexWriter::with_config(random, dir, iwc);

    for id in 0..num_points {
      let mut doc = Document::new();
      lats[2 * id] = self.quantize_lat(self.next_latitude(random));
      lons[2 * id] = self.quantize_lon(self.next_longitude(random));
      doc.add(StringField::from_string("id", id.to_string(), Store::Yes)?);
      self.add_point_to_doc(FIELD_NAME, &mut doc, lats[2 * id], lons[2 * id])?;
      lats[2 * id + 1] = self.quantize_lat(self.next_latitude(random));
      lons[2 * id + 1] = self.quantize_lon(self.next_longitude(random));
      self.add_point_to_doc(FIELD_NAME, &mut doc, lats[2 * id + 1], lons[2 * id + 1])?;

      w.add_document(random, doc)?;
    }

    if random.random_bool(0.5) {
      w.force_merge(random, 1)?;
    }
    let r = w.get_reader(random)?;
    w.close(random)?;

    let s = new_searcher_with_reader(r)?;

    let iters = at_least(random, 25);
    for _iter in 0..iters {
      let rect = self.next_box(random)?;
      let query = self.new_rect_query(
        FIELD_NAME,
        rect.min_lat,
        rect.max_lat,
        rect.min_lon,
        rect.max_lon,
      )?;

      let hits = self.search_index(&s, query, s.get_index_reader().max_doc()?)?;

      let mut fail = false;

      let mut stored_fields = s.stored_fields()?;
      for doc_id in 0..(lats.len() / 2) {
        let lat_doc1 = lats[2 * doc_id];
        let lon_doc1 = lons[2 * doc_id];
        let lat_doc2 = lats[2 * doc_id + 1];
        let lon_doc2 = lons[2 * doc_id + 1];

        let result1 = self.rect_contains_point(&rect, lat_doc1, lon_doc1);
        let result2 = self.rect_contains_point(&rect, lat_doc2, lon_doc2);

        let expected = result1 || result2;

        if hits.get(doc_id)? != expected {
          let id = stored_fields
            .document(doc_id as i32)?
            .get("id")?
            .map(|id| id.into_owned())
            .unwrap_or_default();
          if expected {
            println!("TEST: id={id} docID={doc_id} should match but did not");
          } else {
            println!("TEST: id={id} docID={doc_id} should not match but did");
          }
          println!("  rect={rect}");
          println!("  lat={lat_doc1} lon={lon_doc1}\n  lat={lat_doc2} lon={lon_doc2}");
          println!("  result1={result1} result2={result2}");
          fail = true;
        }
      }

      if fail {
        unreachable!("some hits were wrong");
      }
    }
    Ok(())
  }

  fn rect_contains_point(&self, rect: &Rectangle, lat: f64, lon: f64) -> bool {
    debug_assert!(!lat.is_nan());

    if lat < rect.min_lat || lat > rect.max_lat {
      return false;
    }

    if rect.min_lon <= rect.max_lon {
      lon >= rect.min_lon && lon <= rect.max_lon
    } else {
      lon <= rect.max_lon || lon >= rect.min_lon
    }
  }

  fn verify<R>(&self, random: &mut R, lats: &mut [f64], lons: &mut [f64]) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    for lat in lats.iter_mut() {
      if !lat.is_nan() {
        *lat = self.quantize_lat(*lat);
      }
    }
    for lon in lons.iter_mut() {
      if !lon.is_nan() {
        *lon = self.quantize_lon(*lon);
      }
    }
    self.verify_random_rectangles(random, lats, lons)?;
    self.verify_random_distances(random, lats, lons)?;
    self.verify_random_polygons(random, lats, lons)?;
    self.verify_random_geometries(random, lats, lons)?;
    Ok(())
  }

  fn verify_random_rectangles<R>(&self, random: &mut R, lats: &[f64], lons: &[f64]) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut iwc = new_index_writer_config(random);
    iwc.set_merge_scheduler(SerialMergeScheduler::new());
    let mbd = iwc.get_max_buffered_docs();
    if mbd != -1 && mbd < (lats.len() / 100) as i32 {
      iwc.set_max_buffered_docs((lats.len() / 100) as i32);
    }
    let dir = if lats.len() > 100_000 {
      // TODO IMPORTANT setCodec未实现
      new_fs_directory(
        random,
        create_temp_dir_with_prefix(std::any::type_name::<Self>())?,
      )?
    } else {
      new_directory_shared(random)?
    };

    let mut deleted = HashSet::new();
    let w = IndexWriter::new(dir.clone(), iwc)?;
    self.index_points(random, lats, lons, &mut deleted, &w)?;

    let r = directory_reader::open_from_writer(&w)?;
    w.close()?;

    let s = new_searcher_with_reader(r)?;
    let iters = at_least(random, 25);

    let live_docs = get_live_docs(s.get_index_reader())?;
    let max_doc = s.get_index_reader().max_doc()?;

    for _iter in 0..iters {
      let rect = self.next_box(random)?;
      let query = self.new_rect_query(
        FIELD_NAME,
        rect.min_lat,
        rect.max_lat,
        rect.min_lon,
        rect.max_lon,
      )?;

      let hits = self.search_index(&s, query.clone(), max_doc)?;

      let mut fail = false;
      let mut doc_id_to_id = MultiDocValues::get_numeric_values(s.get_index_reader(), "id")?
        .expect("id doc values should exist");
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, doc_id_to_id.next_doc()?);
        let id = doc_id_to_id.long_value()? as usize;
        let is_live = live_docs
          .as_ref()
          .is_none_or(|live_docs| live_docs.get(doc_id as usize).expect(""));

        let expected = if !is_live || lats[id].is_nan() {
          false
        } else {
          self.rect_contains_point(&rect, lats[id], lons[id])
        };

        if hits.get(doc_id as usize)? != expected {
          self.build_error(
            doc_id,
            expected,
            id,
            lats,
            lons,
            &query,
            live_docs.as_ref().map(|live_docs| live_docs as &dyn Bits),
            |b| b.push_str(&format!("  rect={rect}")),
          );
          fail = true;
        }
      }
      if fail {
        unreachable!("some hits were wrong");
      }
    }

    Ok(())
  }

  fn verify_random_distances<R>(&self, random: &mut R, lats: &[f64], lons: &[f64]) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut iwc = new_index_writer_config(random);
    iwc.set_merge_scheduler(SerialMergeScheduler::new());
    let mbd = iwc.get_max_buffered_docs();
    if mbd != -1 && mbd < (lats.len() / 100) as i32 {
      iwc.set_max_buffered_docs((lats.len() / 100) as i32);
    }
    let dir = if lats.len() > 100_000 {
      // TODO IMPORTANT setCodec未实现
      new_fs_directory(
        random,
        create_temp_dir_with_prefix(std::any::type_name::<Self>())?,
      )?
    } else {
      new_directory_shared(random)?
    };

    let mut deleted = HashSet::new();
    let w = IndexWriter::new(dir.clone(), iwc)?;
    self.index_points(random, lats, lons, &mut deleted, &w)?;

    let r = directory_reader::open_from_writer(&w)?;
    w.close()?;

    let s = new_searcher_with_reader(r)?;
    let iters = at_least(random, 25);

    let live_docs = get_live_docs(s.get_index_reader())?;
    let max_doc = s.get_index_reader().max_doc()?;

    for _iter in 0..iters {
      let center_lat = self.next_latitude(random);
      let center_lon = self.next_longitude(random);
      let radius_meters =
        random.random::<f64>() * GeoUtils::EARTH_MEAN_RADIUS_METERS * std::f64::consts::PI / 2.0
          + 1.0;

      let query = self.new_distance_query(FIELD_NAME, center_lat, center_lon, radius_meters)?;

      let hits = self.search_index(&s, query.clone(), max_doc)?;

      let mut fail = false;
      let mut doc_id_to_id = MultiDocValues::get_numeric_values(s.get_index_reader(), "id")?
        .expect("id doc values should exist");
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, doc_id_to_id.next_doc()?);
        let id = doc_id_to_id.long_value()? as usize;
        let is_live = live_docs
          .as_ref()
          .is_none_or(|live_docs| live_docs.get(doc_id as usize).expect(""));

        let expected = if !is_live || lats[id].is_nan() {
          false
        } else {
          SloppyMath::haversin_meters(center_lat, center_lon, lats[id], lons[id]) <= radius_meters
        };

        if hits.get(doc_id as usize)? != expected {
          self.build_error(
            doc_id,
            expected,
            id,
            lats,
            lons,
            &query,
            live_docs.as_ref().map(|live_docs| live_docs as &dyn Bits),
            |b| {
              if !lats[id].is_nan() {
                let distance_meters =
                  SloppyMath::haversin_meters(center_lat, center_lon, lats[id], lons[id]);
                b.push_str(&format!(
                  "  centerLat={center_lat} centerLon={center_lon} distanceMeters={distance_meters} vs radiusMeters={radius_meters}"
                ));
              }
            },
          );
          fail = true;
        }
      }
      if fail {
        unreachable!("some hits were wrong");
      }
    }

    Ok(())
  }

  fn verify_random_polygons<R>(&self, random: &mut R, lats: &[f64], lons: &[f64]) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut iwc = new_index_writer_config(random);
    iwc.set_merge_scheduler(SerialMergeScheduler::new());
    let mbd = iwc.get_max_buffered_docs();
    if mbd != -1 && mbd < (lats.len() / 100) as i32 {
      iwc.set_max_buffered_docs((lats.len() / 100) as i32);
    }
    let dir = if lats.len() > 100_000 {
      // TODO IMPORTANT setCodec未实现
      new_fs_directory(
        random,
        create_temp_dir_with_prefix(std::any::type_name::<Self>())?,
      )?
    } else {
      new_directory_shared(random)?
    };

    let mut deleted = HashSet::new();
    let w = IndexWriter::new(dir.clone(), iwc)?;
    self.index_points(random, lats, lons, &mut deleted, &w)?;

    let r = directory_reader::open_from_writer(&w)?;
    w.close()?;

    let s = new_searcher_with_reader(r)?;
    let iters = at_least(random, 75);

    let live_docs = get_live_docs(s.get_index_reader())?;
    let max_doc = s.get_index_reader().max_doc()?;

    for _iter in 0..iters {
      let polygon = self.next_polygon(random)?;
      let query = self.new_polygon_query(FIELD_NAME, vec![polygon.clone()])?;

      let hits = self.search_index(&s, query.clone(), max_doc)?;

      let mut fail = false;
      let mut doc_id_to_id = MultiDocValues::get_numeric_values(s.get_index_reader(), "id")?
        .expect("id doc values should exist");
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, doc_id_to_id.next_doc()?);
        let id = doc_id_to_id.long_value()? as usize;
        let is_live = live_docs
          .as_ref()
          .is_none_or(|live_docs| live_docs.get(doc_id as usize).expect(""));

        let expected = if !is_live || lats[id].is_nan() {
          false
        } else {
          GeoTestUtil::contains_slowly(&polygon, lats[id], lons[id])
        };

        if hits.get(doc_id as usize)? != expected {
          self.build_error(
            doc_id,
            expected,
            id,
            lats,
            lons,
            &query,
            live_docs.as_ref().map(|live_docs| live_docs as &dyn Bits),
            |b| b.push_str(&format!("  polygon={polygon}")),
          );
          fail = true;
        }
      }
      if fail {
        unreachable!("some hits were wrong");
      }
    }

    Ok(())
  }

  fn verify_random_geometries<R>(&self, random: &mut R, lats: &[f64], lons: &[f64]) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut iwc = new_index_writer_config(random);
    iwc.set_merge_scheduler(SerialMergeScheduler::new());
    let mbd = iwc.get_max_buffered_docs();
    if mbd != -1 && mbd < (lats.len() / 100) as i32 {
      iwc.set_max_buffered_docs((lats.len() / 100) as i32);
    }
    let dir = if lats.len() > 100_000 {
      // TODO IMPORTANT setCodec未实现
      new_fs_directory(
        random,
        create_temp_dir_with_prefix(std::any::type_name::<Self>())?,
      )?
    } else {
      new_directory_shared(random)?
    };

    let mut deleted = HashSet::new();

    let w = IndexWriter::new(dir.clone(), iwc)?;
    self.index_points(random, lats, lons, &mut deleted, &w)?;

    let r = directory_reader::open_from_writer(&w)?;
    w.close()?;

    let s = new_searcher_with_reader(r)?;
    let iters = at_least(random, 75);

    let live_docs = get_live_docs(s.get_index_reader())?;
    let max_doc = s.get_index_reader().max_doc()?;

    for _iter in 0..iters {
      let geometries = self.next_geometry(random)?;
      let query = self.new_geometry_query(FIELD_NAME, geometries.clone())?;

      let hits = self.search_index(&s, query.clone(), max_doc)?;

      let component2d = lat_lon_geometry::create(&geometries)?;

      let mut fail = false;
      let mut doc_id_to_id = MultiDocValues::get_numeric_values(s.get_index_reader(), "id")?
        .expect("id doc values should exist");
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, doc_id_to_id.next_doc()?);
        let id = doc_id_to_id.long_value()? as usize;
        let is_live = live_docs
          .as_ref()
          .is_none_or(|live_docs| live_docs.get(doc_id as usize).expect(""));

        let expected = if !is_live || lats[id].is_nan() {
          false
        } else {
          component2d.contains(self.quantize_lon(lons[id]), self.quantize_lat(lats[id]))
        };

        if hits.get(doc_id as usize)? != expected {
          self.build_error(
            doc_id,
            expected,
            id,
            lats,
            lons,
            &query,
            live_docs.as_ref().map(|live_docs| live_docs as &dyn Bits),
            |b| {
              let geometries = geometries
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
              b.push_str(&format!("  geometry={geometries:?}"));
            },
          );
          fail = true;
        }
      }
      if fail {
        unreachable!("some hits were wrong");
      }
    }

    Ok(())
  }

  fn index_points<R, D>(
    &self,
    random: &mut R,
    lats: &[f64],
    lons: &[f64],
    deleted: &mut HashSet<i32>,
    w: &DefaultIndexWriterType<D>,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    D: Directory,
  {
    for id in 0..lats.len() {
      let mut doc = Document::new();
      doc.add(StringField::from_string("id", id.to_string(), Store::No)?);
      doc.add(NumericDocValuesField::new("id", id as i64));
      if !lats[id].is_nan() {
        self.add_point_to_doc(FIELD_NAME, &mut doc, lats[id], lons[id])?;
      }
      w.add_document(doc)?;
      if id > 0 && random.random_range(0..100) == 42 {
        let id_to_delete = random.random_range(0..id) as i32;
        w.delete_documents_with_terms(vec![Term::from_text("id", id_to_delete.to_string())])?;
        deleted.insert(id_to_delete);
      }
    }

    if random.random_bool(0.5) {
      w.force_merge(1)?;
    }
    Ok(())
  }

  fn search_index<IRC>(
    &self,
    s: &IndexSearcher<IRC>,
    query: Query,
    max_doc: i32,
  ) -> Result<FixedBitSet>
  where
    IRC: IndexReaderContext + 'static + Sync,
  {
    s.search_with_collector_manager(query, &FixedBitSetCollector::create_manager(max_doc))
  }
  #[allow(clippy::too_many_arguments)]
  fn build_error<E>(
    &self,
    doc_id: i32,
    expected: bool,
    id: usize,
    lats: &[f64],
    lons: &[f64],
    query: &Query,
    live_docs: Option<&dyn Bits>,
    explain: E,
  ) where
    E: FnOnce(&mut String),
  {
    let mut b = String::new();
    if expected {
      b.push_str(&format!("FAIL: id={id} should match but did not\n"));
    } else {
      b.push_str(&format!("FAIL: id={id} should not match but did\n"));
    }
    b.push_str(&format!("  query={query:?} docID={doc_id}\n"));
    b.push_str(&format!("  lat={} lon={}\n", lats[id], lons[id]));
    let deleted = live_docs
      .map(|live_docs| !live_docs.get(doc_id as usize).unwrap_or(false))
      .unwrap_or(false);
    b.push_str(&format!("  deleted?={deleted}"));
    explain(&mut b);
    panic!("wrong hit (first of possibly more):\n\n{b}");
  }

  fn test_rect_boundaries_are_inclusive<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let rect = loop {
      let rect = self.next_box(random)?;
      if !rect.crosses_dateline() {
        break rect;
      }
    };

    let rect = Rectangle::new(
      self.quantize_lat(rect.min_lat),
      self.quantize_lat(rect.max_lat),
      self.quantize_lon(rect.min_lon),
      self.quantize_lon(rect.max_lon),
    )?;
    let dir = new_directory_shared(random)?;
    let mut iwc = new_index_writer_config(random);
    iwc.set_merge_scheduler(SerialMergeScheduler::new());
    let w = RandomIndexWriter::with_config(random, dir, iwc);
    for x in 0..3 {
      let lat = if x == 0 {
        rect.min_lat
      } else if x == 1 {
        self.quantize_lat((rect.min_lat + rect.max_lat) / 2.0)
      } else {
        rect.max_lat
      };
      for y in 0..3 {
        let lon = if y == 0 {
          rect.min_lon
        } else if y == 1 {
          if x == 1 {
            continue;
          }
          self.quantize_lon((rect.min_lon + rect.max_lon) / 2.0)
        } else {
          rect.max_lon
        };

        let mut doc = Document::new();
        self.add_point_to_doc(FIELD_NAME, &mut doc, lat, lon)?;
        w.add_document(random, doc)?;
      }
    }
    let r = w.get_reader(random)?;
    let s = new_searcher_with_reader(r)?;
    assert_eq!(
      8,
      s.count(self.new_rect_query(
        FIELD_NAME,
        rect.min_lat,
        rect.max_lat,
        rect.min_lon,
        rect.max_lon
      )?)?
    );

    if rect.min_lat != -90.0 {
      assert_eq!(
        8,
        s.count(self.new_rect_query(
          FIELD_NAME,
          rect.min_lat.next_down(),
          rect.max_lat,
          rect.min_lon,
          rect.max_lon
        )?)?
      );
    }
    if rect.max_lat != 90.0 {
      assert_eq!(
        8,
        s.count(self.new_rect_query(
          FIELD_NAME,
          rect.min_lat,
          rect.max_lat.next_up(),
          rect.min_lon,
          rect.max_lon
        )?)?
      );
    }
    if rect.min_lon != -180.0 {
      assert_eq!(
        8,
        s.count(self.new_rect_query(
          FIELD_NAME,
          rect.min_lat,
          rect.max_lat,
          rect.min_lon.next_down(),
          rect.max_lon
        )?)?
      );
    }
    if rect.max_lon != 180.0 {
      assert_eq!(
        8,
        s.count(self.new_rect_query(
          FIELD_NAME,
          rect.min_lat,
          rect.max_lat,
          rect.min_lon,
          rect.max_lon.next_up()
        )?)?
      );
    }

    if rect.min_lat != 90.0
      && rect.max_lat != -90.0
      && rect.min_lon != 80.0
      && rect.max_lon != -180.0
      && rect.min_lon != rect.max_lon
    {
      assert_eq!(
        0,
        s.count(self.new_rect_query(
          FIELD_NAME,
          rect.min_lat.next_up(),
          rect.max_lat.next_down(),
          rect.min_lon.next_up(),
          rect.max_lon.next_down()
        )?)?
      );
    }

    w.close(random)?;
    Ok(())
  }

  fn test_random_distance<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_iters = at_least(random, 1);
    for _iters in 0..num_iters {
      self.do_random_distance_test(random, 10, 100)?;
    }
    Ok(())
  }

  fn test_random_distance_huge<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    for _iters in 0..10 {
      self.do_random_distance_test(random, 2000, 100)?;
    }
    Ok(())
  }

  fn do_random_distance_test<R>(
    &self,
    random: &mut R,
    num_docs: usize,
    num_queries: usize,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let mut iwc = new_index_writer_config(random);
    iwc.set_merge_scheduler(SerialMergeScheduler::new());
    let points_in_leaf = 2 + random.random_range(0..4);
    let _ = points_in_leaf;
    // TODO: setCodec未实现
    let writer = RandomIndexWriter::with_config(random, dir, iwc);

    for _ in 0..num_docs {
      let lat_raw = self.next_latitude(random);
      let lon_raw = self.next_longitude(random);
      let lat = self.quantize_lat(lat_raw);
      let lon = self.quantize_lon(lon_raw);
      let mut doc = Document::new();
      self.add_point_to_doc("field", &mut doc, lat, lon)?;
      doc.add(StoredField::from_f64("lat", lat)?);
      doc.add(StoredField::from_f64("lon", lon)?);
      writer.add_document(random, doc)?;
    }
    let reader = writer.get_reader(random)?;
    let searcher = new_searcher_with_reader(reader)?;

    let mut stored_fields = searcher.stored_fields()?;
    for _ in 0..num_queries {
      let lat = self.next_latitude(random);
      let lon = self.next_longitude(random);
      let radius = 50_000_000.0 * random.random::<f64>();

      let max_doc = searcher.get_index_reader().max_doc()? as usize;
      let mut expected = FixedBitSet::new(max_doc);
      for doc in 0..max_doc {
        let stored_doc = stored_fields.document(doc as i32)?;
        let doc_latitude = stored_doc
          .get_field("lat")
          .expect("lat field should exist")
          .numeric_value()?
          .expect("lat field should be numeric")
          .to_f64()
          .expect("lat field should be f64");
        let doc_longitude = stored_doc
          .get_field("lon")
          .expect("lon field should exist")
          .numeric_value()?
          .expect("lon field should be numeric")
          .to_f64()
          .expect("lon field should be f64");
        let distance = SloppyMath::haversin_meters(lat, lon, doc_latitude, doc_longitude);
        if distance <= radius {
          expected.set(doc);
        }
      }

      let top_docs =
        searcher.search(self.new_distance_query("field", lat, lon, radius)?, max_doc)?;
      let mut actual = FixedBitSet::new(max_doc);
      for doc in top_docs.score_docs {
        actual.set(doc.doc as usize);
      }

      if expected != actual {
        println!("center: ({lat},{lon}), radius={radius}");
        for doc in 0..max_doc {
          let stored_doc = stored_fields.document(doc as i32)?;
          let doc_latitude = stored_doc
            .get_field("lat")
            .expect("lat field should exist")
            .numeric_value()?
            .expect("lat field should be numeric")
            .to_f64()
            .expect("lat field should be f64");
          let doc_longitude = stored_doc
            .get_field("lon")
            .expect("lon field should exist")
            .numeric_value()?
            .expect("lon field should be numeric")
            .to_f64()
            .expect("lon field should be f64");
          let distance = SloppyMath::haversin_meters(lat, lon, doc_latitude, doc_longitude);
          println!("{doc}: ({doc_latitude},{doc_longitude}), distance={distance}");
        }
        assert_eq!(expected, actual);
      }
    }
    writer.close(random)?;
    Ok(())
  }

  fn test_equals<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let rect = self.next_box(random)?;

    let q1 = self.new_rect_query(
      "field",
      rect.min_lat,
      rect.max_lat,
      rect.min_lon,
      rect.max_lon,
    )?;
    let q2 = self.new_rect_query(
      "field",
      rect.min_lat,
      rect.max_lat,
      rect.min_lon,
      rect.max_lon,
    )?;
    assert_eq!(q1, q2);
    if !matches!(q1, Query::MatchNoDocs(_)) {
      assert_ne!(
        q1,
        self.new_rect_query(
          "field2",
          rect.min_lat,
          rect.max_lat,
          rect.min_lon,
          rect.max_lon
        )?
      );
    }

    let lat = self.next_latitude(random);
    let lon = self.next_longitude(random);
    let q1 = self.new_distance_query("field", lat, lon, 10000.0)?;
    let q2 = self.new_distance_query("field", lat, lon, 10000.0)?;
    assert_eq!(q1, q2);
    assert_ne!(q1, self.new_distance_query("field2", lat, lon, 10000.0)?);

    let mut lats = vec![0.0; 5];
    let mut lons = vec![0.0; 5];
    lats[0] = rect.min_lat;
    lons[0] = rect.min_lon;
    lats[1] = rect.max_lat;
    lons[1] = rect.min_lon;
    lats[2] = rect.max_lat;
    lons[2] = rect.max_lon;
    lats[3] = rect.min_lat;
    lons[3] = rect.max_lon;
    lats[4] = rect.min_lat;
    lons[4] = rect.min_lon;
    let q1 = self.new_polygon_query(
      "field",
      vec![Polygon::new(lats.clone(), lons.clone(), vec![])?],
    )?;
    let q2 = self.new_polygon_query(
      "field",
      vec![Polygon::new(lats.clone(), lons.clone(), vec![])?],
    )?;
    assert_eq!(q1, q2);
    assert_ne!(
      q1,
      self.new_polygon_query("field2", vec![Polygon::new(lats, lons, vec![])?])?
    );
    Ok(())
  }

  fn search_small_set<R>(
    &self,
    random: &mut R,
    query: Query,
    size: usize,
  ) -> Result<TopDocs<ScoreDoc>>
  where
    R: Rng + ?Sized,
  {
    let pts = [
      [32.763420, -96.774],
      [32.7559529921407, -96.7759895324707],
      [32.77866942010977, -96.77701950073242],
      [32.7756745755423, -96.7706036567688],
      [27.703618681345585, -139.73458170890808],
      [32.94823588839368, -96.4538113027811],
      [33.06047141970814, -96.65084838867188],
      [32.778650, -96.7772],
      [-88.56029371730983, -177.23537676036358],
      [33.541429799076354, -26.779373834241003],
      [26.774024500421728, -77.35379276106497],
      [-90.0, -14.796283808944777],
      [32.94823588839368, -178.8538113027811],
      [32.94823588839368, 178.8538113027811],
      [40.720611, -73.998776],
      [-44.5, -179.5],
    ];

    let directory = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let mut iwc = new_index_writer_config_with_analyzer(random, analyzer);
    iwc.set_max_buffered_docs(random.random_range(100..=1000));
    iwc.set_merge_policy(new_log_merge_policy(random)?);
    iwc.set_merge_scheduler(SerialMergeScheduler::new());
    let writer = RandomIndexWriter::with_config(random, directory, iwc);

    for p in pts {
      let mut doc = Document::new();
      self.add_point_to_doc("point", &mut doc, p[0], p[1])?;
      writer.add_document(random, doc)?;
    }

    for i in (0..pts.len()).step_by(2) {
      let mut doc = Document::new();
      self.add_point_to_doc("point", &mut doc, pts[i][0], pts[i][1])?;
      self.add_point_to_doc("point", &mut doc, pts[i + 1][0], pts[i + 1][1])?;
      writer.add_document(random, doc)?;
    }

    for i in 0..random.random_range(0..10) {
      let mut doc = Document::new();
      doc.add(StringField::from_string(
        "string",
        i.to_string(),
        Store::No,
      )?);
      writer.add_document(random, doc)?;
    }

    let reader = writer.get_reader(random)?;
    writer.close(random)?;

    let searcher = new_searcher_with_reader(reader)?;
    searcher.search(query, size)
  }

  fn test_small_set_rect<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_rect_query("point", 32.778, 32.779, -96.778, -96.777)?,
      5,
    )?;
    assert_eq!(4, td.total_hits.value());
    Ok(())
  }

  fn test_small_set_dateline<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_rect_query("point", -45.0, -44.0, 179.0, -179.0)?,
      20,
    )?;
    assert_eq!(2, td.total_hits.value());
    Ok(())
  }

  fn test_small_set_multi_valued<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_rect_query("point", 32.755, 32.776, -96.454, -96.770)?,
      20,
    )?;
    assert_eq!(5, td.total_hits.value());
    Ok(())
  }

  fn test_small_set_whole_map<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_rect_query(
        "point",
        GeoUtils::MIN_LAT_INCL,
        GeoUtils::MAX_LAT_INCL,
        GeoUtils::MIN_LON_INCL,
        GeoUtils::MAX_LON_INCL,
      )?,
      20,
    )?;
    assert_eq!(24, td.total_hits.value());
    Ok(())
  }

  fn test_small_set_poly<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let polygon = Polygon::new(
      vec![
        33.073130, 32.9942669, 32.938386, 33.0374494, 33.1369762, 33.1162747, 33.073130, 33.073130,
      ],
      vec![
        -96.7682647,
        -96.8280029,
        -96.6288757,
        -96.4929199,
        -96.6041564,
        -96.7449188,
        -96.76826477,
        -96.7682647,
      ],
      vec![],
    )?;
    let td = self.search_small_set(random, self.new_polygon_query("point", vec![polygon])?, 5)?;
    assert_eq!(2, td.total_hits.value());
    Ok(())
  }

  fn test_small_set_poly_whole_map<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let polygon = Polygon::new(
      vec![
        GeoUtils::MIN_LAT_INCL,
        GeoUtils::MAX_LAT_INCL,
        GeoUtils::MAX_LAT_INCL,
        GeoUtils::MIN_LAT_INCL,
        GeoUtils::MIN_LAT_INCL,
      ],
      vec![
        GeoUtils::MIN_LON_INCL,
        GeoUtils::MIN_LON_INCL,
        GeoUtils::MAX_LON_INCL,
        GeoUtils::MAX_LON_INCL,
        GeoUtils::MIN_LON_INCL,
      ],
      vec![],
    )?;
    let td = self.search_small_set(random, self.new_polygon_query("point", vec![polygon])?, 20)?;
    assert_eq!(24, td.total_hits.value(), "testWholeMap failed");
    Ok(())
  }

  fn test_small_set_distance<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_distance_query("point", 32.94823588839368, -96.4538113027811, 6000.0)?,
      20,
    )?;
    assert_eq!(2, td.total_hits.value());
    Ok(())
  }

  fn test_small_set_tiny_distance<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_distance_query("point", 40.720611, -73.998776, 1.0)?,
      20,
    )?;
    assert_eq!(2, td.total_hits.value());
    Ok(())
  }

  fn test_small_set_distance_not_empty<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_distance_query(
        "point",
        -88.56029371730983,
        -177.23537676036358,
        7757.999232959935,
      )?,
      20,
    )?;
    assert_eq!(2, td.total_hits.value());
    Ok(())
  }

  fn test_small_set_huge_distance<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_distance_query("point", 32.94823588839368, -96.4538113027811, 6000000.0)?,
      20,
    )?;
    assert_eq!(16, td.total_hits.value());
    Ok(())
  }

  fn test_small_set_distance_dateline<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_distance_query("point", 32.94823588839368, -179.9538113027811, 120000.0)?,
      20,
    )?;
    assert_eq!(3, td.total_hits.value());
    Ok(())
  }

  fn test_narrow_polygon_close_to_north_pole<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut iwc = new_index_writer_config(random);
    iwc.set_merge_scheduler(SerialMergeScheduler::new());
    let dir = new_directory_shared(random)?;
    let w = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    let base = i32::MAX;
    self.add_point_to_doc(
      FIELD_NAME,
      &mut doc,
      GeoEncodingUtils::decode_latitude(base - 2),
      GeoEncodingUtils::decode_longitude(base - 2),
    )?;
    w.add_document(doc)?;
    w.flush()?;

    let reader = directory_reader::open_from_writer(&w)?;
    let s = new_searcher_with_reader(reader)?;

    let min_lat = GeoEncodingUtils::decode_latitude(base - 3);
    let max_lat = GeoEncodingUtils::decode_latitude(base);
    let min_lon = GeoEncodingUtils::decode_longitude(base - 3);
    let max_lon = GeoEncodingUtils::decode_longitude(base);

    let query = self.new_polygon_query(
      FIELD_NAME,
      vec![Polygon::new(
        vec![min_lat, min_lat, max_lat, max_lat, min_lat],
        vec![min_lon, max_lon, max_lon, min_lon, min_lon],
        vec![],
      )?],
    )?;

    assert_eq!(1, s.count(query)?);
    w.close()?;
    Ok(())
  }

  /// Implement this to quantize randomly generated latitudes so tests do not fail due to quantization.
  /// errors.
  fn quantize_lat(&self, lat: f64) -> f64 {
    lat
  }

  /// Implement this to quantize randomly generated longitudes so tests do not fail due to quantization.
  /// errors.
  fn quantize_lon(&self, lon: f64) -> f64 {
    lon
  }

  fn add_point_to_doc(&self, field: &str, doc: &mut Document, lat: f64, lon: f64) -> Result<()>;

  fn new_rect_query(
    &self,
    field: &str,
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
  ) -> Result<Query>;

  fn new_distance_query(
    &self,
    field: &str,
    center_lat: f64,
    center_lon: f64,
    radius_meters: f64,
  ) -> Result<Query>;

  fn new_polygon_query(&self, field: &str, polygons: Vec<Polygon>) -> Result<Query>;

  fn new_geometry_query(&self, field: &str, geometries: Vec<LatLonGeometryEnum>) -> Result<Query>;
}
