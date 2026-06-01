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
use crate::core::document::int_range::IntRange;
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
use crate::test::core::util::test_util::TestUtil;
use rand::prelude::StdRng;
use rand::{Rng, RngExt};
use std::fmt;

#[allow(dead_code)] // for quick search
struct TestIntRangeFieldQueries;

const FIELD_NAME: &str = "intRangeField";

impl TestIntRangeFieldQueries {
  fn next_int_internal<R>(&self, random: &mut R) -> i32
  where
    R: Rng + ?Sized,
  {
    match random.random_range(0..5) {
      0 => i32::MIN,
      1 => i32::MAX,
      _ => {
        let bpv = random.random_range(0..32);
        match bpv {
          32 => random.random(),
          _ => {
            let mut v = TestUtil::next_int(random, 0, ((1_i64 << bpv) - 1) as i32);
            if bpv > 0 {
              // negative values sometimes
              v -= 1 << (bpv - 1);
            }
            v
          },
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
    document.add(IntRange::new(FIELD_NAME, &[-10, -10], &[9, 10])?);
    writer.add_document(document)?;

    // intersects (crosses)
    let mut document = Document::new();
    document.add(IntRange::new(FIELD_NAME, &[10, -10], &[20, 10])?);
    writer.add_document(document)?;

    // intersects (contains / crosses)
    let mut document = Document::new();
    document.add(IntRange::new(FIELD_NAME, &[-20, -20], &[30, 30])?);
    writer.add_document(document)?;

    // intersects (within)
    let mut document = Document::new();
    document.add(IntRange::new(FIELD_NAME, &[-11, -11], &[1, 11])?);
    writer.add_document(document)?;

    // intersects (crosses)
    let mut document = Document::new();
    document.add(IntRange::new(FIELD_NAME, &[12, 1], &[15, 29])?);
    writer.add_document(document)?;

    // disjoint
    let mut document = Document::new();
    document.add(IntRange::new(FIELD_NAME, &[-122, 1], &[-115, 29])?);
    writer.add_document(document)?;

    // intersects (crosses)
    let mut document = Document::new();
    document.add(IntRange::new(FIELD_NAME, &[i32::MIN, 1], &[-11, 29])?);
    writer.add_document(document)?;

    // equal (within, contains, intersects)
    let mut document = Document::new();
    document.add(IntRange::new(FIELD_NAME, &[-11, -15], &[15, 20])?);
    writer.add_document(document)?;

    // search
    let reader = crate::core::index::directory_reader::open_from_writer(&writer)?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(
      7,
      searcher.count(IntRange::new_intersects_query(
        FIELD_NAME,
        &[-11, -15],
        &[15, 20]
      )?)?
    );
    assert_eq!(
      3,
      searcher.count(IntRange::new_within_query(
        FIELD_NAME,
        &[-11, -15],
        &[15, 20]
      )?)?
    );
    assert_eq!(
      2,
      searcher.count(IntRange::new_contains_query(
        FIELD_NAME,
        &[-11, -15],
        &[15, 20]
      )?)?
    );
    assert_eq!(
      4,
      searcher.count(IntRange::new_crosses_query(
        FIELD_NAME,
        &[-11, -15],
        &[15, 20]
      )?)?
    );

    searcher.get_index_reader().close()?;
    writer.close()?;
    Ok(())
  }
}

impl BaseRangeFieldQueryTestCase for TestIntRangeFieldQueries {
  type Range = IntTestRange;
  type RangeField = IntRange;

  fn new_range_field(&self, r: &Self::Range) -> Result<Self::RangeField> {
    IntRange::new(FIELD_NAME, &r.min, &r.max)
  }

  fn new_intersects_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(IntRange::new_intersects_query(FIELD_NAME, &r.min, &r.max)?.into())
  }

  fn new_contains_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(IntRange::new_contains_query(FIELD_NAME, &r.min, &r.max)?.into())
  }

  fn new_within_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(IntRange::new_within_query(FIELD_NAME, &r.min, &r.max)?.into())
  }

  fn new_crosses_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(IntRange::new_crosses_query(FIELD_NAME, &r.min, &r.max)?.into())
  }

  fn next_range<R>(&self, random: &mut R, dimensions: usize) -> Result<Self::Range>
  where
    R: Rng + ?Sized,
  {
    let mut min = vec![0; dimensions];
    let mut max = vec![0; dimensions];

    for d in 0..dimensions {
      let min_v = self.next_int_internal(random);
      let max_v = self.next_int_internal(random);
      min[d] = min_v.min(max_v);
      max[d] = min_v.max(max_v);
    }
    Ok(IntTestRange::new(min, max))
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestIntRangeFieldQueries, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestIntRangeFieldQueries;
  f(&case, &mut random)
}

#[derive(Clone)]
pub(crate) struct IntTestRange {
  pub(crate) base: RangeBase,
  pub(crate) min: Vec<i32>,
  pub(crate) max: Vec<i32>,
}

impl IntTestRange {
  pub(crate) fn new(min: Vec<i32>, max: Vec<i32>) -> Self {
    assert!(
      !min.is_empty() && !max.is_empty(),
      "test box: min/max cannot be null or empty"
    );
    assert_eq!(
      min.len(),
      max.len(),
      "test box: min/max length do not agree"
    );
    IntTestRange {
      base: RangeBase::default(),
      min,
      max,
    }
  }
}

impl Range for IntTestRange {
  type Value = i32;

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

impl fmt::Display for IntTestRange {
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
  use crate::test::core::search::test_int_range_field_queries::run_case;

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
