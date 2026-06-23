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
use crate::core::geo::component2d::Component2D;
use crate::core::geo::xy_geometry;
use crate::core::geo::xy_geometry::XYGeometryEnum;
use crate::core::geo::xy_point::XYPoint;
use crate::core::geo::xy_polygon::XYPolygon;
use crate::core::geo::xy_rectangle::XYRectangle;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
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
use crate::core::search::sort::Sort;
use crate::core::search::top_docs::{TopDocs, TopDocsLike};
use crate::core::store::directory::Directory;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::geo::shape_test_util::ShapeTestUtil;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::fixed_bit_set_collector::FixedBitSetCollector;
use crate::test::core::util::lucene_test_case::{
  at_least, at_least_usize, create_temp_dir, is_night_mode, new_directory_shared, new_fs_directory,
  new_index_writer_config, new_index_writer_config_with_analyzer, new_log_merge_policy,
  new_searcher_with_reader,
};
use crate::test::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::collections::HashSet;
use std::sync::Arc;

const FIELD_NAME: &str = "point";
/// Base test support for XY spatial implementations (high-level fields and queries).
pub trait BaseXYPointTestCase {
  fn next_x<R>(&self, random: &mut R) -> f32
  where
    R: Rng + ?Sized,
  {
    ShapeTestUtil::next_float(random)
  }

  fn next_y<R>(&self, random: &mut R) -> f32
  where
    R: Rng + ?Sized,
  {
    ShapeTestUtil::next_float(random)
  }

  fn next_box<R>(&self, random: &mut R) -> Result<XYRectangle>
  where
    R: Rng + ?Sized,
  {
    ShapeTestUtil::next_box(random)
  }

  fn next_polygon<R>(&self, random: &mut R) -> Result<XYPolygon>
  where
    R: Rng + ?Sized,
  {
    ShapeTestUtil::next_polygon(random)
  }

  fn next_geometry<R>(&self, random: &mut R) -> Result<Vec<XYGeometryEnum>>
  where
    R: Rng + ?Sized,
  {
    let len = random.random_range(1..=4);
    let mut geometries = Vec::with_capacity(len);
    for _ in 0..len {
      let geometry = match random.random_range(0..3) {
        0 => XYPoint::new(self.next_x(random), self.next_y(random))?.into(),
        1 => self.next_box(random)?.into(),
        _ => self.next_polygon(random)?.into(),
      };
      geometries.push(geometry);
    }
    Ok(geometries)
  }

  /// Valid values that should not cause error.
  fn test_index_extreme_values(&self) -> Result<()> {
    let mut document = Document::new();
    self.add_point_to_doc("foo", &mut document, f32::MAX, f32::MAX)?;
    self.add_point_to_doc("foo", &mut document, f32::MAX, -f32::MAX)?;
    self.add_point_to_doc("foo", &mut document, -f32::MAX, f32::MAX)?;
    self.add_point_to_doc("foo", &mut document, -f32::MAX, -f32::MAX)?;
    Ok(())
  }

  /// NaN: illegal.
  fn test_index_nan_values(&self) -> Result<()> {
    let mut document = Document::new();

    let expected = self.add_point_to_doc("foo", &mut document, f32::NAN, 50.0);
    assert!(matches!(expected, Err(err) if err.to_string().contains("invalid value")));

    let expected = self.add_point_to_doc("foo", &mut document, 50.0, f32::NAN);
    assert!(matches!(expected, Err(err) if err.to_string().contains("invalid value")));

    Ok(())
  }

  /// Inf: illegal.
  /// Inf: illegal.
  fn test_index_inf_values(&self) -> Result<()> {
    let mut document = Document::new();

    let expected = self.add_point_to_doc("foo", &mut document, f32::INFINITY, 0.0);
    assert!(matches!(expected, Err(err) if err.to_string().contains("invalid value")));

    let expected = self.add_point_to_doc("foo", &mut document, f32::NEG_INFINITY, 0.0);
    assert!(matches!(expected, Err(err) if err.to_string().contains("invalid value")));

    let expected = self.add_point_to_doc("foo", &mut document, 0.0, f32::INFINITY);
    assert!(matches!(expected, Err(err) if err.to_string().contains("invalid value")));

    let expected = self.add_point_to_doc("foo", &mut document, 0.0, f32::NEG_INFINITY);
    assert!(matches!(expected, Err(err) if err.to_string().contains("invalid value")));

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
    self.add_point_to_doc("field", &mut document, 18.313694, -65.227_45)?;
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

  /// Box should not accept invalid x/y.
  fn test_box_invalid_coordinates(&self) -> Result<()> {
    let expected = self.new_rect_query("field", f32::NAN, f32::NAN, f32::NAN, f32::NAN);
    assert!(expected.is_err());
    Ok(())
  }
  /// Test we can search for a point.
  fn test_distance_basics<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let writer = RandomIndexWriter::new(random, dir);

    let mut document = Document::new();
    self.add_point_to_doc("field", &mut document, 18.313694, -65.227_45)?;
    writer.add_document(random, document)?;

    let reader = writer.get_reader(random)?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(
      1,
      searcher.count(self.new_distance_query("field", 18.0, -65.0, 20.0)?)?
    );

    writer.close(random)?;
    Ok(())
  }

  /// Null field name not allowed.
  fn test_distance_null(&self) -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  /// Distance query should not accept invalid x/y as origin.
  fn test_distance_illegal(&self) -> Result<()> {
    let expected = self.new_distance_query("field", f32::NAN, f32::NAN, 120_000.0);
    assert!(expected.is_err());
    Ok(())
  }

  /// Negative distance queries are not allowed.
  fn test_distance_negative(&self) -> Result<()> {
    let expected = self.new_distance_query("field", 18.0, 19.0, -1.0);
    assert!(matches!(expected, Err(err) if err.to_string().contains("radius")));
    Ok(())
  }

  /// NaN distance queries are not allowed.
  fn test_distance_nan(&self) -> Result<()> {
    let expected = self.new_distance_query("field", 18.0, 19.0, f32::NAN);
    assert!(
      matches!(expected, Err(err) if err.to_string().contains("radius") && err.to_string().contains("NaN"))
    );
    Ok(())
  }

  /// Inf distance queries are not allowed.
  fn test_distance_inf(&self) -> Result<()> {
    let expected = self.new_distance_query("field", 18.0, 19.0, f32::INFINITY);
    assert!(
      matches!(expected, Err(err) if err.to_string().contains("radius") && err.to_string().contains("finite"))
    );

    let expected = self.new_distance_query("field", 18.0, 19.0, f32::NEG_INFINITY);
    assert!(
      matches!(expected, Err(err) if err.to_string().contains("radius") && err.to_string().contains("bigger than 0"))
    );

    Ok(())
  }
  /// Test we can search for a polygon.
  fn test_polygon_basics<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let writer = RandomIndexWriter::new(random, dir);

    let mut document = Document::new();
    self.add_point_to_doc("field", &mut document, 18.313694, -65.227_45)?;
    writer.add_document(random, document)?;

    let reader = writer.get_reader(random)?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(
      1,
      searcher.count(self.new_polygon_query(
        "field",
        vec![XYPolygon::new(
          vec![18.0, 18.0, 19.0, 19.0, 18.0],
          vec![-66.0, -65.0, -65.0, -66.0, -66.0],
          vec![],
        )?],
      )?)?
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
    self.add_point_to_doc("field", &mut document, 18.313694, -65.227_45)?;
    writer.add_document(random, document)?;

    let reader = writer.get_reader(random)?;
    let searcher = new_searcher_with_reader(reader)?;
    let inner = XYPolygon::new(
      vec![18.5, 18.5, 18.7, 18.7, 18.5],
      vec![-65.7, -65.4, -65.4, -65.7, -65.7],
      vec![],
    )?;
    let outer = XYPolygon::new(
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
    self.add_point_to_doc("field", &mut document, 18.313694, -65.227_45)?;
    writer.add_document(random, document)?;

    let reader = writer.get_reader(random)?;
    let searcher = new_searcher_with_reader(reader)?;
    let inner = XYPolygon::new(
      vec![18.2, 18.2, 18.4, 18.4, 18.2],
      vec![-65.3, -65.2, -65.2, -65.3, -65.3],
      vec![],
    )?;
    let outer = XYPolygon::new(
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
    self.add_point_to_doc("field", &mut document, 18.313694, -65.227_45)?;
    writer.add_document(random, document)?;

    let reader = writer.get_reader(random)?;
    let searcher = new_searcher_with_reader(reader)?;
    let a = XYPolygon::new(
      vec![28.0, 28.0, 29.0, 29.0, 28.0],
      vec![-56.0, -55.0, -55.0, -56.0, -56.0],
      vec![],
    )?;
    let b = XYPolygon::new(
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
  // A particularly tricky adversary for BKD tree.
  fn test_same_point_many_times<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_points = at_least_usize(random, 1000);

    // Every doc has 2 points:
    let the_x = self.next_x(random);
    let the_y = self.next_y(random);

    let xs = vec![the_x; num_points];
    let ys = vec![the_y; num_points];

    self.verify(random, &xs, &ys)
  }

  // A particularly tricky adversary for BKD tree.
  fn test_low_cardinality<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_points = at_least_usize(random, 1000);
    let cardinality = TestUtil::next_int(random, 2, 20) as usize;

    let mut diff_xs = vec![0.0; cardinality];
    let mut diff_ys = vec![0.0; cardinality];
    for i in 0..cardinality {
      diff_xs[i] = self.next_x(random);
      diff_ys[i] = self.next_y(random);
    }

    let mut xs = vec![0.0; num_points];
    let mut ys = vec![0.0; num_points];
    for i in 0..num_points {
      let index = random.random_range(0..cardinality);
      xs[i] = diff_xs[index];
      ys[i] = diff_ys[index];
    }

    self.verify(random, &xs, &ys)
  }

  fn test_all_y_equal<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_points = at_least_usize(random, 1000);
    let y = self.next_y(random);
    let mut xs = vec![0.0; num_points];
    let mut ys = vec![0.0; num_points];

    let mut have_real_doc = false;

    for doc_id in 0..num_points {
      let x = random.random_range(0..20);
      if x == 17 {
        // Some docs don't have a point:
        ys[doc_id] = f32::NAN;
        continue;
      }

      if doc_id > 0 && x == 14 && have_real_doc {
        let old_doc_id = loop {
          let old_doc_id = random.random_range(0..doc_id);
          if !ys[old_doc_id].is_nan() {
            break old_doc_id;
          }
        };

        // Fully identical point:
        xs[doc_id] = xs[old_doc_id];
      } else {
        xs[doc_id] = self.next_x(random);
        have_real_doc = true;
      }
      ys[doc_id] = y;
    }

    self.verify(random, &xs, &ys)
  }
  fn test_all_x_equal<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_points = at_least_usize(random, 1000);
    let the_x = self.next_x(random);
    let mut xs = vec![0.0; num_points];
    let mut ys = vec![0.0; num_points];

    let mut have_real_doc = false;

    for doc_id in 0..num_points {
      let x = random.random_range(0..20);
      if x == 17 {
        // Some docs don't have a point:
        ys[doc_id] = f32::NAN;

        continue;
      }

      if doc_id > 0 && x == 14 && have_real_doc {
        let old_doc_id = loop {
          let old_doc_id = random.random_range(0..doc_id);
          if !ys[old_doc_id].is_nan() {
            break old_doc_id;
          }
        };

        // Fully identical point:
        ys[doc_id] = ys[old_doc_id];
      } else {
        ys[doc_id] = self.next_y(random);
        have_real_doc = true;
      }
      xs[doc_id] = the_x;
    }

    self.verify(random, &xs, &ys)
  }
  fn test_multi_valued<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_points = at_least_usize(random, 1000);

    // Every doc has 2 points:
    let mut xs = vec![0.0; 2 * num_points];
    let mut ys = vec![0.0; 2 * num_points];

    let dir = new_directory_shared(random)?;
    let mut iwc = new_index_writer_config(random);

    // We rely on docID order:
    iwc.set_merge_policy(new_log_merge_policy(random)?);
    // and on seeds being able to reproduce:
    iwc.set_merge_scheduler(SerialMergeScheduler::new());

    let w = RandomIndexWriter::with_config(random, dir.clone(), iwc);

    for id in 0..num_points {
      let mut doc = Document::new();

      xs[2 * id] = self.next_x(random);
      ys[2 * id] = self.next_y(random);
      doc.add(StringField::from_string("id", id.to_string(), Store::Yes)?);
      self.add_point_to_doc(FIELD_NAME, &mut doc, xs[2 * id], ys[2 * id])?;

      xs[2 * id + 1] = self.next_x(random);
      ys[2 * id + 1] = self.next_y(random);
      self.add_point_to_doc(FIELD_NAME, &mut doc, xs[2 * id + 1], ys[2 * id + 1])?;
      w.add_document(random, doc)?;
    }

    // TODO: share w/ verify; just need parallel array of the expected ids
    if random.random_bool(0.5) {
      w.force_merge(random, 1)?;
    }

    let r = w.get_reader(random)?;
    w.close(random)?;
    let max_doc = r.max_doc()?;
    let s = new_searcher_with_reader(r)?;

    let iters = at_least(random, 25);
    for _ in 0..iters {
      let rect = self.next_box(random)?;

      if cfg!(feature = "test_log_verbose") {
        println!("\nTEST: rect={rect}");
      }

      let query =
        self.new_rect_query(FIELD_NAME, rect.min_x, rect.max_x, rect.min_y, rect.max_y)?;

      let hits = self.search_index(&s, query, max_doc)?;

      let fail = false;

      let _stored_fields = s.stored_fields()?;
      for doc_id in 0..ys.len() / 2 {
        let y_doc1 = ys[2 * doc_id];
        let x_doc1 = xs[2 * doc_id];
        let y_doc2 = ys[2 * doc_id + 1];
        let x_doc2 = xs[2 * doc_id + 1];

        let result1 = Self::rect_contains_point(&rect, x_doc1 as f64, y_doc1 as f64);
        let result2 = Self::rect_contains_point(&rect, x_doc2 as f64, y_doc2 as f64);

        let expected = result1 || result2;

        if hits.get(doc_id)? != expected {
          unreachable!("")
        }
      }

      if fail {
        panic!("some hits were wrong");
      }
    }

    Ok(())
  }
  fn test_random_tiny<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // Make sure single-leaf-node case is OK:
    self.do_test_random(random, 10)
  }

  fn test_random_medium<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.do_test_random(random, 1000)
  }

  fn test_random_big<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    if !is_night_mode() {
      return Ok(());
    }
    self.do_test_random(random, 200_000)
  }

  fn do_test_random<R>(&self, random: &mut R, count: i32) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_points = at_least_usize(random, count as usize);

    let mut xs = vec![0.0; num_points];
    let mut ys = vec![0.0; num_points];

    let mut have_real_doc = false;

    for id in 0..num_points {
      let x = random.random_range(0..20);
      if x == 17 {
        // Some docs don't have a point:
        ys[id] = f32::NAN;

        continue;
      }

      if id > 0 && x < 3 && have_real_doc {
        let old_id = loop {
          let old_id = random.random_range(0..id);
          if !ys[old_id].is_nan() {
            break old_id;
          }
        };

        if x == 0 {
          // Identical x to old point
          ys[id] = ys[old_id];
          xs[id] = self.next_x(random);
        } else if x == 1 {
          // Identical y to old point
          ys[id] = self.next_y(random);
          xs[id] = xs[old_id];
        } else {
          assert_eq!(2, x);
          // Fully identical point:
          xs[id] = xs[old_id];
          ys[id] = ys[old_id];
        }
      } else {
        xs[id] = self.next_x(random);
        ys[id] = self.next_y(random);
        have_real_doc = true;
      }
    }

    self.verify(random, &xs, &ys)
  }

  fn add_point_to_doc(&self, field: &str, doc: &mut Document, x: f32, y: f32) -> Result<()>;

  fn new_rect_query(
    &self,
    field: &str,
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
  ) -> Result<Query>;

  fn new_distance_query(
    &self,
    field: &str,
    center_x: f32,
    center_y: f32,
    radius: f32,
  ) -> Result<Query>;

  fn new_polygon_query(&self, field: &str, polygon: Vec<XYPolygon>) -> Result<Query>;

  fn new_geometry_query(&self, field: &str, geometries: Vec<XYGeometryEnum>) -> Result<Query>;
  fn rect_contains_point(rect: &XYRectangle, x: f64, y: f64) -> bool {
    if y < rect.min_y as f64 || y > rect.max_y as f64 {
      return false;
    }
    x >= rect.min_x as f64 && x <= rect.max_x as f64
  }
  fn cartesian_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let diff_x = x1 - x2;
    let diff_y = y1 - y2;
    (diff_x * diff_x + diff_y * diff_y).sqrt()
  }
  fn verify<R>(&self, random: &mut R, xs: &[f32], ys: &[f32]) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // NaN means missing for the doc!!!!!
    self.verify_random_rectangles(random, xs, ys)?;
    self.verify_random_distances(random, xs, ys)?;
    self.verify_random_polygons(random, xs, ys)?;
    self.verify_random_geometries(random, xs, ys)?;
    Ok(())
  }
  fn verify_random_rectangles<R>(&self, random: &mut R, xs: &[f32], ys: &[f32]) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut iwc = new_index_writer_config(random);
    iwc.set_merge_scheduler(SerialMergeScheduler::new());

    let mbd = iwc.get_max_buffered_docs();
    if mbd != -1 && mbd < (xs.len() / 100) as i32 {
      iwc.set_max_buffered_docs((xs.len() / 100) as i32);
    }

    let dir = if xs.len() > 100_000 {
      // TODO IMPORTANT: set default codec once set_codec is supported
      let _dir_name = std::any::type_name::<Self>();
      new_fs_directory(random, create_temp_dir()?)?
    } else {
      new_directory_shared(random)?
    };

    let mut deleted = HashSet::new();
    let w = IndexWriter::new(dir.clone(), iwc)?;
    self.index_points(random, xs, ys, &mut deleted, &w)?;
    let r = Arc::new(directory_reader::open_from_writer(&w)?);
    w.close()?;

    let s = new_searcher_with_reader(r.clone())?;

    let iters = at_least(random, 25);

    let live_docs = get_live_docs(s.get_index_reader())?;
    let max_doc = s.get_index_reader().max_doc()?;

    for _ in 0..iters {
      let rect = self.next_box(random)?;

      let query =
        self.new_rect_query(FIELD_NAME, rect.min_x, rect.max_x, rect.min_y, rect.max_y)?;

      let hits = self.search_index(&s, query.clone(), max_doc)?;

      let mut doc_id_to_id = MultiDocValues::get_numeric_values(&r, "id")?.unwrap();
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, doc_id_to_id.next_doc()?);
        let id = doc_id_to_id.long_value()? as usize;
        #[allow(clippy::if_same_then_else)]
        let expected = if live_docs
          .as_ref()
          .is_some_and(|live_docs| !live_docs.get(doc_id as usize).expect(""))
        {
          false
        } else if xs[id].is_nan() || ys[id].is_nan() {
          false
        } else {
          Self::rect_contains_point(&rect, xs[id] as f64, ys[id] as f64)
        };

        if hits.get(doc_id as usize)? != expected {
          unreachable!("")
        }
      }
    }

    Ok(())
  }
  fn verify_random_distances<R>(&self, random: &mut R, xs: &[f32], ys: &[f32]) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut iwc = new_index_writer_config(random);
    iwc.set_merge_scheduler(SerialMergeScheduler::new());

    let mbd = iwc.get_max_buffered_docs();
    if mbd != -1 && mbd < (xs.len() / 100) as i32 {
      iwc.set_max_buffered_docs((xs.len() / 100) as i32);
    }

    let dir = if xs.len() > 100_000 {
      // TODO IMPORTANT: set default codec once set_codec is supported
      new_fs_directory(random, create_temp_dir()?)?
    } else {
      new_directory_shared(random)?
    };

    let mut deleted = HashSet::new();
    let w = IndexWriter::new(dir.clone(), iwc)?;
    self.index_points(random, xs, ys, &mut deleted, &w)?;
    let r = Arc::new(directory_reader::open_from_writer(&w)?);
    w.close()?;

    let s = new_searcher_with_reader(r.clone())?;

    let iters = at_least(random, 25);

    let live_docs = get_live_docs(s.get_index_reader())?;
    let max_doc = s.get_index_reader().max_doc()?;

    for _ in 0..iters {
      let center_x = self.next_x(random);
      let center_y = self.next_y(random);

      let mut radius = random.random::<f32>() * f32::MAX / 2.0;
      if radius == 0.0 {
        // no meaning value, prevents 0.0:
        radius = 8.955_251E37_f32;
      }

      let query = self.new_distance_query(FIELD_NAME, center_x, center_y, radius)?;

      let hits = self.search_index(&s, query.clone(), max_doc)?;

      let mut doc_id_to_id = MultiDocValues::get_numeric_values(&r, "id")?.unwrap();
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, doc_id_to_id.next_doc()?);
        let id = doc_id_to_id.long_value()? as usize;
        #[allow(clippy::if_same_then_else)]
        let expected = if live_docs
          .as_ref()
          .is_some_and(|live_docs| !live_docs.get(doc_id as usize).expect(""))
        {
          false
        } else if xs[id].is_nan() || ys[id].is_nan() {
          false
        } else {
          Self::cartesian_distance(
            center_x as f64,
            center_y as f64,
            xs[id] as f64,
            ys[id] as f64,
          ) <= radius as f64
        };

        if hits.get(doc_id as usize)? != expected {
          unreachable!();
        }
      }
    }

    Ok(())
  }

  fn verify_random_polygons<R>(&self, random: &mut R, xs: &[f32], ys: &[f32]) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut iwc = new_index_writer_config(random);
    iwc.set_merge_scheduler(SerialMergeScheduler::new());

    let mbd = iwc.get_max_buffered_docs();
    if mbd != -1 && mbd < (xs.len() / 100) as i32 {
      iwc.set_max_buffered_docs((xs.len() / 100) as i32);
    }

    let dir = if xs.len() > 100_000 {
      // TODO IMPORTANT: set default codec once set_codec is supported
      new_fs_directory(random, create_temp_dir()?)?
    } else {
      new_directory_shared(random)?
    };

    let mut deleted = HashSet::new();
    let w = IndexWriter::new(dir.clone(), iwc)?;
    self.index_points(random, xs, ys, &mut deleted, &w)?;
    let r = Arc::new(directory_reader::open_from_writer(&w)?);
    w.close()?;

    let s = new_searcher_with_reader(r.clone())?;

    let iters = at_least(random, 75);

    let live_docs = get_live_docs(s.get_index_reader())?;
    let max_doc = s.get_index_reader().max_doc()?;

    for _ in 0..iters {
      let polygon = self.next_polygon(random)?;
      let query = self.new_polygon_query(FIELD_NAME, vec![polygon.clone()])?;

      let hits = self.search_index(&s, query.clone(), max_doc)?;

      let mut doc_id_to_id = MultiDocValues::get_numeric_values(&r, "id")?.unwrap();
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, doc_id_to_id.next_doc()?);
        let id = doc_id_to_id.long_value()? as usize;
        #[allow(clippy::if_same_then_else)]
        let expected = if live_docs
          .as_ref()
          .is_some_and(|live_docs| !live_docs.get(doc_id as usize).expect(""))
        {
          false
        } else if xs[id].is_nan() || ys[id].is_nan() {
          false
        } else {
          ShapeTestUtil::contains_slowly(&polygon, xs[id] as f64, ys[id] as f64)
        };

        if hits.get(doc_id as usize)? != expected {
          unreachable!();
        }
      }
    }

    Ok(())
  }

  fn verify_random_geometries<R>(&self, random: &mut R, xs: &[f32], ys: &[f32]) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut iwc = new_index_writer_config(random);
    iwc.set_merge_scheduler(SerialMergeScheduler::new());

    let mbd = iwc.get_max_buffered_docs();
    if mbd != -1 && mbd < (xs.len() / 100) as i32 {
      iwc.set_max_buffered_docs((xs.len() / 100) as i32);
    }

    let dir = if xs.len() > 100_000 {
      // TODO IMPORTANT: set default codec once set_codec is supported
      new_fs_directory(random, create_temp_dir()?)?
    } else {
      new_directory_shared(random)?
    };

    let mut deleted = HashSet::new();
    let w = IndexWriter::new(dir.clone(), iwc)?;
    self.index_points(random, xs, ys, &mut deleted, &w)?;
    let r = Arc::new(directory_reader::open_from_writer(&w)?);
    w.close()?;

    let s = new_searcher_with_reader(r.clone())?;

    let iters = at_least(random, 75);

    let live_docs = get_live_docs(s.get_index_reader())?;
    let max_doc = s.get_index_reader().max_doc()?;

    for _ in 0..iters {
      let geometries = self.next_geometry(random)?;
      let query = self.new_geometry_query(FIELD_NAME, geometries.clone())?;
      let component_2d = xy_geometry::create(&geometries)?;

      let hits = self.search_index(&s, query.clone(), max_doc)?;

      let mut doc_id_to_id = MultiDocValues::get_numeric_values(&r, "id")?.unwrap();
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, doc_id_to_id.next_doc()?);
        let id = doc_id_to_id.long_value()? as usize;
        #[allow(clippy::if_same_then_else)]
        let expected = if live_docs
          .as_ref()
          .is_some_and(|live_docs| !live_docs.get(doc_id as usize).expect(""))
        {
          false
        } else if xs[id].is_nan() || ys[id].is_nan() {
          false
        } else {
          component_2d.contains(xs[id] as f64, ys[id] as f64)
        };

        if hits.get(doc_id as usize)? != expected {
          unreachable!();
        }
      }
    }

    Ok(())
  }
  fn index_points<R, D>(
    &self,
    random: &mut R,
    xs: &[f32],
    ys: &[f32],
    deleted: &mut HashSet<i32>,
    w: &IndexWriter<D>,
  ) -> Result<()>
  where
    D: Directory + 'static,
    R: Rng + ?Sized,
  {
    for id in 0..xs.len() {
      let mut doc = Document::new();
      doc.add(StringField::from_string("id", id.to_string(), Store::No)?);
      doc.add(NumericDocValuesField::new("id", id as i64));

      if !xs[id].is_nan() && !ys[id].is_nan() {
        self.add_point_to_doc(FIELD_NAME, &mut doc, xs[id], ys[id])?;
      }

      w.add_document(doc)?;

      if id > 0 && random.random_range(0..100) == 42 {
        let id_to_delete = random.random_range(0..id);
        w.delete_documents_with_terms(vec![Term::from_text("id", id_to_delete.to_string())])?;
        deleted.insert(id_to_delete as i32);
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
    IRC: IndexReaderContext + Sync,
  {
    s.search_with_collector_manager(query, &FixedBitSetCollector::create_manager(max_doc))
  }
  fn test_rect_boundaries_are_inclusive<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let rect = ShapeTestUtil::next_box(random)?;
    let dir = new_directory_shared(random)?;
    let mut iwc = new_index_writer_config(random);
    // Else seeds may not reproduce:
    iwc.set_merge_scheduler(SerialMergeScheduler::new());

    let w = RandomIndexWriter::with_config(random, dir, iwc);

    for i in 0..3 {
      let y = if i == 0 {
        rect.min_y
      } else if i == 1 {
        ((rect.min_y as f64 + rect.max_y as f64) / 2.0) as f32
      } else {
        rect.max_y
      };

      for j in 0..3 {
        let x = if j == 0 {
          rect.min_x
        } else if j == 1 {
          if i == 1 {
            continue;
          }
          ((rect.min_x as f64 + rect.max_x as f64) / 2.0) as f32
        } else {
          rect.max_x
        };

        let mut doc = Document::new();
        self.add_point_to_doc(FIELD_NAME, &mut doc, x, y)?;
        w.add_document(random, doc)?;
      }
    }

    let r = w.get_reader(random)?;
    let s = new_searcher_with_reader(r)?;

    // Exact edge cases
    assert_eq!(
      8,
      s.count(self.new_rect_query(FIELD_NAME, rect.min_x, rect.max_x, rect.min_y, rect.max_y,)?)?
    );

    // Expand 1 ulp in each direction if possible and test a slightly larger box!
    if rect.min_x != -f32::MAX {
      assert_eq!(
        8,
        s.count(self.new_rect_query(
          FIELD_NAME,
          rect.min_x.next_down(),
          rect.max_x,
          rect.min_y,
          rect.max_y,
        )?)?
      );
    }

    if rect.max_x != f32::MAX {
      assert_eq!(
        8,
        s.count(self.new_rect_query(
          FIELD_NAME,
          rect.min_x,
          rect.max_x.next_up(),
          rect.min_y,
          rect.max_y,
        )?)?
      );
    }

    if rect.min_y != -f32::MAX {
      assert_eq!(
        8,
        s.count(self.new_rect_query(
          FIELD_NAME,
          rect.min_x,
          rect.max_x,
          rect.min_y.next_down(),
          rect.max_y,
        )?)?
      );
    }

    if rect.max_y != f32::MAX {
      assert_eq!(
        8,
        s.count(self.new_rect_query(
          FIELD_NAME,
          rect.min_x,
          rect.max_x,
          rect.min_y,
          rect.max_y.next_up(),
        )?)?
      );
    }

    w.close(random)?;
    Ok(())
  }
  /// Run a few iterations with just 10 docs, hopefully easy to debug.
  fn test_random_distance<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_iters = at_least(random, 1);
    for _ in 0..num_iters {
      self.do_random_distance_test(random, 10, 100)?;
    }
    Ok(())
  }

  /// Runs with thousands of docs.
  fn test_random_distance_huge<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    if !is_night_mode() {
      return Ok(());
    }

    for _ in 0..10 {
      self.do_random_distance_test(random, 2000, 100)?;
    }
    Ok(())
  }

  fn do_random_distance_test<R>(
    &self,
    random: &mut R,
    num_docs: i32,
    num_queries: i32,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let mut iwc = new_index_writer_config(random);
    // Else seeds may not reproduce:
    iwc.set_merge_scheduler(SerialMergeScheduler::new());

    let writer = RandomIndexWriter::with_config(random, dir, iwc);

    for _ in 0..num_docs {
      let x = self.next_x(random);
      let y = self.next_y(random);
      // Pre-normalize up front, so we can just use quantized value for testing and do simple exact
      // comparisons.

      let mut doc = Document::new();
      self.add_point_to_doc("field", &mut doc, x, y)?;
      doc.add(StoredField::from_f32("x", x)?);
      doc.add(StoredField::from_f32("y", y)?);
      writer.add_document(random, doc)?;
    }
    let reader = writer.get_reader(random)?;
    let max_doc = reader.max_doc()?;

    let mut stored_fields = reader.stored_fields()?;
    let searcher = new_searcher_with_reader(reader)?;
    for _ in 0..num_queries {
      let circle = ShapeTestUtil::next_circle(random)?;
      let x = circle.get_x();
      let y = circle.get_y();
      let radius = circle.get_radius();

      let mut expected = FixedBitSet::new(max_doc as usize);
      for doc in 0..max_doc {
        let document = stored_fields.document(doc)?;
        let doc_x = document
          .get_field("x")
          .unwrap()
          .numeric_value()?
          .unwrap()
          .to_f32()
          .unwrap();
        let doc_y = document
          .get_field("y")
          .unwrap()
          .numeric_value()?
          .unwrap()
          .to_f32()
          .unwrap();
        let distance = Self::cartesian_distance(x as f64, y as f64, doc_x as f64, doc_y as f64);
        if distance <= radius as f64 {
          expected.set(doc as usize);
        }
      }

      let top_docs = searcher.search_after_field_with_score(
        None,
        self.new_distance_query("field", x, y, radius)?,
        max_doc as usize,
        Sort::get_index_order()?,
        false,
      )?;

      let mut actual = FixedBitSet::new(max_doc as usize);
      for score_doc in top_docs.score_docs() {
        actual.set(score_doc.doc() as usize);
      }

      if expected != actual {
        unreachable!("")
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

    let q1 = self.new_rect_query(FIELD_NAME, rect.min_x, rect.max_x, rect.min_y, rect.max_y)?;
    let q2 = self.new_rect_query(FIELD_NAME, rect.min_x, rect.max_x, rect.min_y, rect.max_y)?;
    assert_eq!(q1, q2);

    let x = self.next_x(random);
    let y = self.next_y(random);
    let q1 = self.new_distance_query(FIELD_NAME, x, y, 10000.0)?;
    let q2 = self.new_distance_query(FIELD_NAME, x, y, 10000.0)?;
    assert_eq!(q1, q2);
    assert_ne!(q1, self.new_distance_query("field2", x, y, 10000.0)?);

    let xs = vec![rect.min_x, rect.max_x, rect.max_x, rect.min_x, rect.min_x];
    let ys = vec![rect.min_y, rect.min_y, rect.max_y, rect.max_y, rect.min_y];

    let q1 = self.new_polygon_query(
      FIELD_NAME,
      vec![XYPolygon::new(xs.clone(), ys.clone(), vec![])?],
    )?;
    let q2 = self.new_polygon_query(
      FIELD_NAME,
      vec![XYPolygon::new(xs.clone(), ys.clone(), vec![])?],
    )?;
    assert_eq!(q1, q2);
    assert_ne!(
      q1,
      self.new_polygon_query(
        "field2",
        vec![XYPolygon::new(xs.clone(), ys.clone(), vec![])?]
      )?
    );
    Ok(())
  }

  /// Return topdocs over a small set of points in field "point".
  fn search_small_set<R>(
    &self,
    random: &mut R,
    query: Query,
    size: usize,
  ) -> Result<TopDocs<ScoreDoc>>
  where
    R: Rng + ?Sized,
  {
    // This is a simple systematic test, indexing these points.
    let pts = vec![
      vec![32.763420, -96.774],
      vec![32.7559529921407, -96.7759895324707],
      vec![32.77866942010977, -96.77701950073242],
      vec![32.7756745755423, -96.7706036567688],
      vec![27.703618681345585, -139.73458170890808],
      vec![32.94823588839368, -96.4538113027811],
      vec![33.06047141970814, -96.65084838867188],
      vec![32.778650, -96.7772],
      vec![-88.56029371730983, -177.23537676036358],
      vec![33.541429799076354, -26.779373834241003],
      vec![26.774024500421728, -77.35379276106497],
      vec![-90.0, -14.796283808944777],
      vec![32.94823588839368, -178.8538113027811],
      vec![32.94823588839368, 178.8538113027811],
      vec![40.720611, -73.998776],
      vec![-44.5, -179.5],
    ];

    let directory = new_directory_shared(random)?;

    // TODO: must these simple tests really rely on docid order?
    let mock = MockAnalyzer::new(random);
    let mut iwc = new_index_writer_config_with_analyzer(random, mock);
    iwc.set_max_buffered_docs(TestUtil::next_int(random, 100, 1000));
    iwc.set_merge_policy(new_log_merge_policy(random)?);
    // Else seeds may not reproduce:
    iwc.set_merge_scheduler(SerialMergeScheduler::new());

    let writer = RandomIndexWriter::with_config(random, directory, iwc);

    for p in &pts {
      let mut doc = Document::new();
      self.add_point_to_doc("point", &mut doc, p[0] as f32, p[1] as f32)?;
      writer.add_document(random, doc)?;
    }

    // Add explicit multi-valued docs.
    for i in (0..pts.len()).step_by(2) {
      let mut doc = Document::new();
      self.add_point_to_doc("point", &mut doc, pts[i][0] as f32, pts[i][1] as f32)?;
      self.add_point_to_doc(
        "point",
        &mut doc,
        pts[i + 1][0] as f32,
        pts[i + 1][1] as f32,
      )?;
      writer.add_document(random, doc)?;
    }

    // Index random string documents.
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

  fn test_small_set_rect2<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_rect_query("point", -45.0, -44.0, -180.0, 180.0)?,
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
      self.new_rect_query("point", 32.755, 32.776, -180.0, 180.770)?,
      20,
    )?;
    // 3 single valued docs + 2 multi-valued docs
    assert_eq!(5, td.total_hits.value());
    Ok(())
  }

  fn test_small_set_whole_space<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_rect_query("point", -f32::MAX, f32::MAX, -f32::MAX, f32::MAX)?,
      20,
    )?;
    assert_eq!(24, td.total_hits.value());
    Ok(())
  }

  fn test_small_set_poly<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_polygon_query(
        "point",
        vec![XYPolygon::new(
          vec![
            33.073_13, 32.994_267, 32.938386, 33.037_45, 33.136_974, 33.116_276, 33.073_13,
            33.073_13,
          ],
          vec![
            -96.768_265,
            -96.828,
            -96.628_876,
            -96.492_92,
            -96.604_16,
            -96.744_92,
            -96.768_265,
            -96.768_265,
          ],
          vec![],
        )?],
      )?,
      5,
    )?;
    assert_eq!(2, td.total_hits.value());
    Ok(())
  }

  fn test_small_set_poly_whole_space<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_polygon_query(
        "point",
        vec![XYPolygon::new(
          vec![-f32::MAX, f32::MAX, f32::MAX, -f32::MAX, -f32::MAX],
          vec![-f32::MAX, -f32::MAX, f32::MAX, f32::MAX, -f32::MAX],
          vec![],
        )?],
      )?,
      20,
    )?;
    assert_eq!(24, td.total_hits.value(), "testWholeMap failed");
    Ok(())
  }

  fn test_small_set_distance<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_distance_query("point", 32.948_235, -96.453_81, 6.0)?,
      20,
    )?;
    assert_eq!(11, td.total_hits.value());
    Ok(())
  }

  fn test_small_set_tiny_distance<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_distance_query("point", 40.720_61, -73.998_78, 0.1)?,
      20,
    )?;
    assert_eq!(2, td.total_hits.value());
    Ok(())
  }

  /// Explicitly large.
  fn test_small_set_huge_distance<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let td = self.search_small_set(
      random,
      self.new_distance_query("point", 32.948_235, -96.453_81, f32::MAX)?,
      20,
    )?;
    assert_eq!(24, td.total_hits.value());
    Ok(())
  }
}
