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
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::stored_field::StoredField;
use crate::core::document::xy_doc_values_field::XYDocValuesField;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::stored_fields::StoredFields;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::{MissingValueEnum, SortField, SortFieldType, SortFiledBase};
use crate::core::search::sort_field_enum::SortFieldEnum;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::geo::shape_test_util::ShapeTestUtil;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
#[cfg(feature = "nightly")]
use crate::test::core::util::lucene_test_case::is_night_mode;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, new_searcher_with_reader, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
#[allow(dead_code)] // for quick search
pub struct TestXYPointDistanceSort;
fn cartesian_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
  let diff_x = x1 - x2;
  let diff_y = y1 - y2;
  (diff_x * diff_x + diff_y * diff_y).sqrt()
}

/// Add three points and sort by distance
#[test]
fn test_distance_sort() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone())?;

  // add some docs
  let mut doc = Document::new();
  doc.add(XYDocValuesField::new(
    "location",
    40.759_01f32,
    -73.984_474f32,
  )?);
  iw.add_document(&mut random, doc)?;
  let d1 = cartesian_distance(
    40.759_01_f32 as f64,
    -73.984_474_f32 as f64,
    40.714_35_f32 as f64,
    -74.005_974_f32 as f64,
  );

  let mut doc = Document::new();
  doc.add(XYDocValuesField::new(
    "location",
    40.718266f32,
    -74.007_82f32,
  )?);
  iw.add_document(&mut random, doc)?;
  let d2 = cartesian_distance(
    40.718266f32 as f64,
    -74.007_82_f32 as f64,
    40.714_35_f32 as f64,
    -74.005_974_f32 as f64,
  );

  let mut doc = Document::new();
  doc.add(XYDocValuesField::new(
    "location",
    40.705_116f32,
    -74.008_83f32,
  )?);
  iw.add_document(&mut random, doc)?;
  let d3 = cartesian_distance(
    40.705_116_f32 as f64,
    -74.008_83_f32 as f64,
    40.714_35_f32 as f64,
    -74.005_974_f32 as f64,
  );

  let reader = iw.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;
  iw.close(&mut random)?;

  let sort = Sort::with_fields(vec![XYDocValuesField::new_distance_sort(
    "location",
    40.714_35,
    -74.005_974,
  )?])?;
  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 3, sort)?;

  let d = td.score_docs()[0].as_field().unwrap();
  assert_eq!(d2, *d.fields[0].as_f64().unwrap());

  let d = td.score_docs()[1].as_field().unwrap();
  assert_eq!(d3, *d.fields[0].as_f64().unwrap());

  let d = td.score_docs()[2].as_field().unwrap();
  assert_eq!(d1, *d.fields[0].as_f64().unwrap());

  Ok(())
}
/// Add two points (one doc missing) and sort by distance
#[test]
fn test_missing_last() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone())?;

  // missing
  let doc = Document::new();
  iw.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  doc.add(XYDocValuesField::new(
    "location",
    40.718266f32,
    -74.007_82f32,
  )?);
  iw.add_document(&mut random, doc)?;
  let d2 = cartesian_distance(
    40.718266f32 as f64,
    -74.007_82_f32 as f64,
    40.714_35_f32 as f64,
    -74.005_974_f32 as f64,
  );

  let mut doc = Document::new();
  doc.add(XYDocValuesField::new(
    "location",
    40.705_116f32,
    -74.008_83f32,
  )?);
  iw.add_document(&mut random, doc)?;
  let d3 = cartesian_distance(
    40.705_116_f32 as f64,
    -74.008_83_f32 as f64,
    40.714_35_f32 as f64,
    -74.005_974_f32 as f64,
  );

  let reader = iw.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;
  iw.close(&mut random)?;

  let sort = Sort::with_fields(vec![XYDocValuesField::new_distance_sort(
    "location",
    40.714_35,
    -74.005_974,
  )?])?;
  let td = searcher.search_with_sort(MatchAllDocsQuery::new(), 3, sort)?;

  let d = td.score_docs()[0].as_field().unwrap();
  assert_eq!(d2, *d.fields[0].as_f64().unwrap());

  let d = td.score_docs()[1].as_field().unwrap();
  assert_eq!(d3, *d.fields[0].as_f64().unwrap());

  let d = td.score_docs()[2].as_field().unwrap();
  assert_eq!(f64::INFINITY, *d.fields[0].as_f64().unwrap());

  Ok(())
}
/// Run a few iterations with just 10 docs, hopefully easy to debug
#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  for _ in 0..100 {
    do_random_test(&mut random, 10, 100)?;
  }
  Ok(())
}

/// Runs with thousands of docs
#[cfg(feature = "nightly")]
#[test]
#[ignore = "nightly"]
fn test_random_huge() -> Result<()> {
  let mut random = random();
  if !is_night_mode() {
    return Ok(());
  }

  for _ in 0..10 {
    do_random_test(&mut random, 2000, 100)?;
  }
  Ok(())
}

#[derive(Clone, Debug)]
struct ResultItem {
  id: i32,
  distance: f64,
}

impl PartialEq for ResultItem {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id && self.distance.to_bits() == other.distance.to_bits()
  }
}

impl Eq for ResultItem {}

impl PartialOrd for ResultItem {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for ResultItem {
  fn cmp(&self, other: &Self) -> Ordering {
    match self.distance.total_cmp(&other.distance) {
      Ordering::Equal => self.id.cmp(&other.id),
      cmp => cmp,
    }
  }
}

impl Hash for ResultItem {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.distance.to_bits().hash(state);
    self.id.hash(state);
  }
}

impl Display for ResultItem {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "Result [id={}, distance={}]", self.id, self.distance)
  }
}

fn do_random_test<R>(random: &mut R, num_docs: i32, num_queries: i32) -> Result<()>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let mut iwc = new_index_writer_config(random)?;
  // else seeds may not to reproduce:
  iwc.set_merge_scheduler(SerialMergeScheduler::new());
  let writer = RandomIndexWriter::with_config(random, dir.clone(), iwc);

  for i in 0..num_docs {
    let mut doc = Document::new();
    doc.add(StoredField::from_i32("id", i)?);
    doc.add(NumericDocValuesField::new("id", i as i64));

    if random.random_range(0..10) > 7 {
      let x = ShapeTestUtil::next_float(random);
      let y = ShapeTestUtil::next_float(random);

      doc.add(XYDocValuesField::new("field", x, y)?);
      doc.add(StoredField::from_f32("x", x)?);
      doc.add(StoredField::from_f32("y", y)?);
    } // otherwise "missing"

    writer.add_document(random, doc)?;
  }

  let reader = writer.get_reader(random)?;
  let max_doc = reader.max_doc()?;
  let mut stored_fields = reader.stored_fields()?;
  let searcher = new_searcher_with_reader(reader)?;

  for _ in 0..num_queries {
    let x = ShapeTestUtil::next_float(random);
    let y = ShapeTestUtil::next_float(random);
    let missing_value = f64::INFINITY;

    let mut expected = Vec::with_capacity(max_doc as usize);

    for doc in 0..max_doc {
      let target_doc = stored_fields.document(doc)?;
      let distance = match target_doc.get_field("x") {
        None => missing_value, // missing
        Some(_) => {
          let doc_x = target_doc
            .get_field("x")
            .unwrap()
            .numeric_value()?
            .unwrap()
            .to_f32()
            .unwrap() as f64;
          let doc_y = target_doc
            .get_field("y")
            .unwrap()
            .numeric_value()?
            .unwrap()
            .to_f32()
            .unwrap() as f64;
          cartesian_distance(x as f64, y as f64, doc_x, doc_y)
        },
      };

      let id = target_doc
        .get_field("id")
        .unwrap()
        .numeric_value()?
        .unwrap()
        .to_i32()
        .unwrap();

      expected.push(ResultItem { id, distance });
    }

    expected.sort();

    // randomize the topN a bit
    let top_n = TestUtil::next_int(random, 1, max_doc) as usize;
    // sort by distance, then ID
    let mut distance_sort = XYDocValuesField::new_distance_sort("field", x, y)?;
    distance_sort.set_missing_value(MissingValueEnum::Double(missing_value))?;
    let sort_field: Vec<SortFieldEnum> = vec![
      distance_sort.into(),
      SortField::new(Some("id"), SortFieldType::Int)?.into(),
    ];

    let sort = Sort::with_fields(sort_field)?;

    let top_docs = searcher.search_with_sort(MatchAllDocsQuery::new(), top_n, sort.clone())?;
    #[allow(clippy::needless_range_loop)]
    for result_number in 0..top_n {
      let field_doc = top_docs.score_docs()[result_number].as_field().unwrap();
      let actual = ResultItem {
        id: *field_doc.fields[1].as_i32().unwrap(),
        distance: *field_doc.fields[0].as_f64().unwrap(),
      };
      assert_eq!(expected[result_number], actual);
    }

    // get page2 with searchAfter()
    if top_n < max_doc as usize {
      let page2 = TestUtil::next_int(random, 1, max_doc - top_n as i32) as usize;
      let v = top_docs.score_docs()[top_n - 1].as_field().unwrap().clone();
      let top_docs2 = searcher.search_after(Some(v), MatchAllDocsQuery::new(), page2, sort)?;

      for result_number in 0..page2 {
        let field_doc = top_docs2.score_docs()[result_number].as_field().unwrap();
        let actual = ResultItem {
          id: *field_doc.fields[1].as_i32().unwrap(),
          distance: *field_doc.fields[0].as_f64().unwrap(),
        };
        assert_eq!(expected[top_n + result_number], actual);
      }
    }
  }

  writer.close(random)?;
  Ok(())
}
