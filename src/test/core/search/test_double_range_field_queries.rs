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
use crate::core::document::double_range::DoubleRange;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::search::query::Query;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::search::base_range_field_query_test_case::{
  BaseRangeFieldQueryTestCase, Range, RangeBase,
};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config, new_searcher_with_reader, random,
};
use rand::prelude::StdRng;
use rand::{Rng, RngExt};
use std::fmt;

#[allow(dead_code)] // for quick search
struct TestDoubleRangeFieldQueries;

const FIELD_NAME: &str = "doubleRangeField";

impl TestDoubleRangeFieldQueries {
  fn next_double_internal<R>(&self, random: &mut R) -> f64
  where
    R: Rng + ?Sized,
  {
    match random.random_range(0..5) {
      0 => f64::NEG_INFINITY,
      1 => f64::INFINITY,
      _ => {
        if random.random_bool(0.5) {
          random.random()
        } else {
          (random.random_range(0..15) - 7) as f64 / 3.0
        }
      },
    }
  }

  fn test_basics<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let writer = IndexWriter::new(dir, new_index_writer_config(random))?;

    // intersects (within)
    let mut document = Document::new();
    document.add(DoubleRange::new(FIELD_NAME, &[-10.0, -10.0], &[9.1, 10.1])?);
    writer.add_document(document)?;

    // intersects (crosses)
    let mut document = Document::new();
    document.add(DoubleRange::new(FIELD_NAME, &[10.0, -10.0], &[20.0, 10.0])?);
    writer.add_document(document)?;

    // intersects (contains, crosses)
    let mut document = Document::new();
    document.add(DoubleRange::new(
      FIELD_NAME,
      &[-20.0, -20.0],
      &[30.0, 30.1],
    )?);
    writer.add_document(document)?;

    // intersects (crosses)
    let mut document = Document::new();
    document.add(DoubleRange::new(
      FIELD_NAME,
      &[-11.1, -11.2],
      &[1.23, 11.5],
    )?);
    writer.add_document(document)?;

    // intersects (crosses)
    let mut document = Document::new();
    document.add(DoubleRange::new(FIELD_NAME, &[12.33, 1.2], &[15.1, 29.9])?);
    writer.add_document(document)?;

    // disjoint
    let mut document = Document::new();
    document.add(DoubleRange::new(
      FIELD_NAME,
      &[-122.33, 1.2],
      &[-115.1, 29.9],
    )?);
    writer.add_document(document)?;

    // intersects (crosses)
    let mut document = Document::new();
    document.add(DoubleRange::new(
      FIELD_NAME,
      &[f64::NEG_INFINITY, 1.2],
      &[-11.0, 29.9],
    )?);
    writer.add_document(document)?;

    // equal (within, contains, intersects)
    let mut document = Document::new();
    document.add(DoubleRange::new(
      FIELD_NAME,
      &[-11.0, -15.0],
      &[15.0, 20.0],
    )?);
    writer.add_document(document)?;

    // search
    let reader = crate::core::index::directory_reader::open_from_writer(&writer)?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(
      7,
      searcher.count(DoubleRange::new_intersects_query(
        FIELD_NAME,
        &[-11.0, -15.0],
        &[15.0, 20.0]
      )?)?
    );
    assert_eq!(
      2,
      searcher.count(DoubleRange::new_within_query(
        FIELD_NAME,
        &[-11.0, -15.0],
        &[15.0, 20.0]
      )?)?
    );
    assert_eq!(
      2,
      searcher.count(DoubleRange::new_contains_query(
        FIELD_NAME,
        &[-11.0, -15.0],
        &[15.0, 20.0]
      )?)?
    );
    assert_eq!(
      5,
      searcher.count(DoubleRange::new_crosses_query(
        FIELD_NAME,
        &[-11.0, -15.0],
        &[15.0, 20.0]
      )?)?
    );

    searcher.get_index_reader().close()?;
    writer.close()?;
    Ok(())
  }
}

impl BaseRangeFieldQueryTestCase for TestDoubleRangeFieldQueries {
  type Range = DoubleTestRange;
  type RangeField = DoubleRange;

  fn new_range_field(&self, r: &Self::Range) -> Result<Self::RangeField> {
    DoubleRange::new(FIELD_NAME, &r.min, &r.max)
  }

  fn new_intersects_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(DoubleRange::new_intersects_query(FIELD_NAME, &r.min, &r.max)?.into())
  }

  fn new_contains_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(DoubleRange::new_contains_query(FIELD_NAME, &r.min, &r.max)?.into())
  }

  fn new_within_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(DoubleRange::new_within_query(FIELD_NAME, &r.min, &r.max)?.into())
  }

  fn new_crosses_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(DoubleRange::new_crosses_query(FIELD_NAME, &r.min, &r.max)?.into())
  }

  fn next_range<R>(&self, random: &mut R, dimensions: usize) -> Result<Self::Range>
  where
    R: Rng + ?Sized,
  {
    let mut min = vec![0.0; dimensions];
    let mut max = vec![0.0; dimensions];

    for d in 0..dimensions {
      let min_v = self.next_double_internal(random);
      let max_v = self.next_double_internal(random);
      min[d] = min_v.min(max_v);
      max[d] = min_v.max(max_v);
    }
    Ok(DoubleTestRange::new(min, max))
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestDoubleRangeFieldQueries, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestDoubleRangeFieldQueries;
  f(&case, &mut random)
}

#[derive(Clone)]
pub(crate) struct DoubleTestRange {
  pub(crate) base: RangeBase,
  pub(crate) min: Vec<f64>,
  pub(crate) max: Vec<f64>,
}

impl DoubleTestRange {
  pub(crate) fn new(min: Vec<f64>, max: Vec<f64>) -> Self {
    assert!(
      !min.is_empty() && !max.is_empty(),
      "test box: min/max cannot be null or empty"
    );
    assert_eq!(
      min.len(),
      max.len(),
      "test box: min/max length do not agree"
    );
    DoubleTestRange {
      base: RangeBase::default(),
      min,
      max,
    }
  }
}

impl Range for DoubleTestRange {
  type Value = f64;

  fn get_base(&self) -> &RangeBase {
    &self.base
  }

  fn get_base_mut(&mut self) -> &mut RangeBase {
    &mut self.base
  }

  fn num_dimensions(&self) -> usize {
    self.min.len()
  }

  fn get_min(&self, dim: usize) -> Self::Value {
    self.min[dim]
  }

  fn set_min(&mut self, dim: usize, val: Self::Value) {
    if self.min[dim] < val {
      self.max[dim] = val;
    } else {
      self.min[dim] = val;
    }
  }

  fn get_max(&self, dim: usize) -> Self::Value {
    self.max[dim]
  }

  fn set_max(&mut self, dim: usize, val: Self::Value) {
    if self.max[dim] > val {
      self.min[dim] = val;
    } else {
      self.max[dim] = val;
    }
  }

  fn is_equal(&self, other: &Self) -> bool {
    self.min == other.min && self.max == other.max
  }

  fn is_disjoint(&self, other: &Self) -> bool {
    for d in 0..self.min.len() {
      if self.min[d] > other.max[d] || self.max[d] < other.min[d] {
        return true;
      }
    }
    false
  }

  fn is_within(&self, other: &Self) -> bool {
    for d in 0..self.min.len() {
      if !(self.min[d] >= other.min[d] && self.max[d] <= other.max[d]) {
        return false;
      }
    }
    true
  }

  fn contains(&self, other: &Self) -> bool {
    for d in 0..self.min.len() {
      if !(self.min[d] <= other.min[d] && self.max[d] >= other.max[d]) {
        return false;
      }
    }
    true
  }
}

impl fmt::Display for DoubleTestRange {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Box({} TO {}", self.min[0], self.max[0])?;
    for d in 1..self.min.len() {
      write!(f, ", {} TO {}", self.min[d], self.max[d])?;
    }
    write!(f, ")")
  }
}

mod base_range_field_query_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::search::base_range_field_query_test_case::BaseRangeFieldQueryTestCase;
  use crate::test::core::search::test_double_range_field_queries::run_case;

  #[test]
  fn test_basics() -> Result<()> {
    run_case(|case, random| case.test_basics(random))
  }

  #[test]
  fn test_random_tiny() -> Result<()> {
    run_case(|case, random| case.test_random_tiny(random))
  }

  #[test]
  fn test_random_medium() -> Result<()> {
    run_case(|case, random| case.test_random_medium(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_random_big() -> Result<()> {
    run_case(|case, random| case.test_random_big(random))
  }

  #[test]
  fn test_multi_valued() -> Result<()> {
    run_case(|case, random| case.test_multi_valued(random))
  }

  #[test]
  fn test_all_equal() -> Result<()> {
    run_case(|case, random| case.test_all_equal(random))
  }

  #[test]
  fn test_low_cardinality() -> Result<()> {
    run_case(|case, random| case.test_low_cardinality(random))
  }
}
