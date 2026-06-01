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
use crate::core::document::double_range_doc_values_field::DoubleRangeDocValuesField;
use crate::core::document::field::Store;
use crate::core::document::float_range_doc_values_field::FloatRangeDocValuesField;
use crate::core::document::int_range_doc_values_field::IntRangeDocValuesField;
use crate::core::document::long_range_doc_values_field::LongRangeDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::search::query::QueryBase;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_directory_shared, new_searcher_with_reader, random,
};

#[allow(dead_code)] // for quick search
struct TestRangeFieldsDocValuesQuery;

#[test]
fn test_double_range_doc_values_intersects_query() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone());
  let iters = at_least(&mut random, 10);
  let min = [112.7, 296.0, 512.4];
  let max = [119.3, 314.8, 524.3];
  for _ in 0..iters {
    let mut doc = Document::new();
    doc.add(DoubleRangeDocValuesField::new("dv", min, max)?);
    iw.add_document(doc)?;
  }
  iw.commit()?;

  let non_matching_min = [256.7, 296.0, 532.4];
  let non_matching_max = [259.3, 364.8, 534.3];

  let mut doc = Document::new();
  doc.add(DoubleRangeDocValuesField::new(
    "dv",
    non_matching_min,
    non_matching_max,
  )?);
  iw.add_document(doc)?;
  iw.commit()?;

  let reader = iw.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;
  iw.close()?;

  let low_range = [111.3, 294.4, 517.4];
  let high_range = [116.7, 319.4, 533.0];

  let query = DoubleRangeDocValuesField::new_slow_intersects_query("dv", low_range, high_range)?;
  assert_eq!(searcher.count(query)?, iters);

  let low_range2 = [116.3, 299.3, 517.0];
  let high_range2 = [121.0, 317.1, 531.2];

  let query = DoubleRangeDocValuesField::new_slow_intersects_query("dv", low_range2, high_range2)?;

  assert_eq!(searcher.count(query)?, iters);

  Ok(())
}

#[test]
fn test_int_range_doc_values_intersects_query() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone());
  let iters = at_least(&mut random, 10);
  let min = [3, 11, 17];
  let max = [27, 35, 49];
  for _ in 0..iters {
    let mut doc = Document::new();
    doc.add(IntRangeDocValuesField::new("dv", min, max)?);
    iw.add_document(doc)?;
  }

  let min2 = [11, 19, 27];
  let max2 = [29, 38, 56];

  let mut doc = Document::new();
  doc.add(IntRangeDocValuesField::new("dv", min2, max2)?);

  iw.commit()?;

  let reader = iw.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;
  iw.close()?;

  let low_range = [6, 16, 19];
  let high_range = [29, 41, 42];

  let query = IntRangeDocValuesField::new_slow_intersects_query("dv", low_range, high_range)?;

  assert_eq!(searcher.count(query)?, iters);

  let low_range2 = [2, 9, 18];
  let high_range2 = [25, 34, 41];

  let query = IntRangeDocValuesField::new_slow_intersects_query("dv", low_range2, high_range2)?;

  assert_eq!(searcher.count(query)?, iters);

  let low_range3 = [101, 121, 153];
  let high_range3 = [156, 127, 176];

  let query = IntRangeDocValuesField::new_slow_intersects_query("dv", low_range3, high_range3)?;

  assert_eq!(searcher.count(query)?, 0);

  Ok(())
}

#[test]
fn test_long_range_doc_values_intersect_query() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone());
  let iters = at_least(&mut random, 10);
  let min = [31i64, 15, 2];
  let max = [95i64, 27, 4];
  for _ in 0..iters {
    let mut doc = Document::new();
    doc.add(LongRangeDocValuesField::new("dv", min, max)?);
    iw.add_document(doc)?;
  }

  let min2 = [101i64, 124, 137];
  let max2 = [138i64, 145, 156];
  let mut doc = Document::new();
  doc.add(LongRangeDocValuesField::new("dv", min2, max2)?);

  iw.commit()?;

  let reader = iw.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;
  iw.close()?;

  let low_range = [6i64, 12, 1];
  let high_range = [34i64, 24, 3];

  let query = LongRangeDocValuesField::new_slow_intersects_query("dv", low_range, high_range)?;

  assert_eq!(searcher.count(query)?, iters);

  let low_range2 = [32i64, 18, 3];
  let high_range2 = [96i64, 29, 5];

  let query = LongRangeDocValuesField::new_slow_intersects_query("dv", low_range2, high_range2)?;

  assert_eq!(searcher.count(query)?, iters);

  Ok(())
}

#[test]
fn test_float_range_doc_values_intersect_query() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone());
  let iters = at_least(&mut random, 10);
  let min = [3.7f32, 11.0, 33.4];
  let max = [8.3f32, 21.6, 59.8];
  for _ in 0..iters {
    let mut doc = Document::new();
    doc.add(FloatRangeDocValuesField::new("dv", min, max)?);
    iw.add_document(doc)?;
  }

  let non_matching_min = [11.4f32, 29.7, 102.4];
  let non_matching_max = [17.6f32, 37.2, 160.2];
  let mut doc = Document::new();
  doc.add(FloatRangeDocValuesField::new(
    "dv",
    non_matching_min,
    non_matching_max,
  )?);
  iw.add_document(doc)?;

  iw.commit()?;

  let reader = iw.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;
  iw.close()?;

  let low_range = [1.2f32, 8.3, 21.4];
  let high_range = [6.0f32, 17.6, 47.1];

  let query = FloatRangeDocValuesField::new_slow_intersects_query("dv", low_range, high_range)?;

  assert_eq!(searcher.count(query)?, iters);

  let low_range2 = [6.1f32, 17.0, 31.3];
  let high_range2 = [14.2f32, 23.4, 61.1];

  let query = FloatRangeDocValuesField::new_slow_intersects_query("dv", low_range2, high_range2)?;

  assert_eq!(searcher.count(query)?, iters);

  Ok(())
}

#[test]
fn test_to_string() -> Result<()> {
  let double_min = [112.7, 296.0, 512.4f32 as f64];
  let double_max = [119.3, 314.8, 524.3f32 as f64];
  let q1 = DoubleRangeDocValuesField::new_slow_intersects_query("foo", double_min, double_max)?;
  assert_eq!(
    "foo:[[112.7, 296.0, 512.4000244140625] TO [119.3, 314.8, 524.2999877929688]]",
    q1.as_string("")?
  );

  let int_min = [3, 11, 17];
  let int_max = [27, 35, 49];
  let q2 = IntRangeDocValuesField::new_slow_intersects_query("foo", int_min, int_max)?;
  assert_eq!("foo:[[3, 11, 17] TO [27, 35, 49]]", q2.as_string("")?);

  let float_min = [3.7f32, 11.0, 33.4];
  let float_max = [8.3f32, 21.6, 59.8];
  let q3 = FloatRangeDocValuesField::new_slow_intersects_query("foo", float_min, float_max)?;
  assert_eq!(
    "foo:[[3.7, 11.0, 33.4] TO [8.3, 21.6, 59.8]]",
    q3.as_string("")?
  );

  let long_min = [101i64, 124, 137];
  let long_max = [138i64, 145, 156];
  let q4 = LongRangeDocValuesField::new_slow_intersects_query("foo", long_min, long_max)?;
  assert_eq!(
    "foo:[[101, 124, 137] TO [138, 145, 156]]",
    q4.as_string("")?
  );

  Ok(())
}

#[test]
fn test_no_data() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone());
  let mut doc = Document::new();
  doc.add(StringField::from_string("foo", "abc", Store::No)?);
  iw.add_document(doc)?;

  let reader = iw.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;
  iw.close()?;

  // test on field that doesn't exist
  let q1 = LongRangeDocValuesField::new_slow_intersects_query("bar", [20i64], [27i64])?;
  let r = searcher.search(q1, 10)?;
  assert_eq!(0, r.total_hits.value());

  // test on field of wrong type
  let q2 = LongRangeDocValuesField::new_slow_intersects_query("foo", [20i64], [27i64])?;
  let err = match searcher.search(q2, 10) {
    Ok(_) => unreachable!("search should fail"),
    Err(err) => err,
  };
  assert!(matches!(err, LuceneError::IllegalState(_)));

  Ok(())
}
