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
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::search::query::Query;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::sandbox::document::lat_lon_bounding_box::LatLonBoundingBox;
use crate::test_framework::core::geo::geo_test_util::GeoTestUtil;
use crate::test_framework::core::search::base_range_field_query_test_case::{
  BaseRangeFieldQueryTestCase, Range, RangeBase,
};
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, new_searcher_with_reader, random,
};
use rand::Rng;
use rand::prelude::StdRng;
use std::fmt;

#[allow(dead_code)] // for quick search
struct TestLatLonBoundingBoxQueries;

const FIELD_NAME: &str = "geoBoundingBoxField";

impl BaseRangeFieldQueryTestCase for TestLatLonBoundingBoxQueries {
  type Range = GeoBBox;
  type RangeField = LatLonBoundingBox;

  fn new_range_field(&self, _r: &Self::Range) -> Result<Self::RangeField> {
    Err(LuceneError::unsupported_operation(
      "this method should never be called",
    ))
  }

  fn add_range(&self, doc: &mut Document, r: &Self::Range) -> Result<()> {
    doc.add(LatLonBoundingBox::new(
      FIELD_NAME, r.min_lat, r.min_lon, r.max_lat, r.max_lon,
    )?);
    Ok(())
  }

  fn new_intersects_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(
      LatLonBoundingBox::new_intersects_query(
        FIELD_NAME, r.min_lat, r.min_lon, r.max_lat, r.max_lon,
      )?
      .into(),
    )
  }

  fn new_contains_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(
      LatLonBoundingBox::new_contains_query(
        FIELD_NAME, r.min_lat, r.min_lon, r.max_lat, r.max_lon,
      )?
      .into(),
    )
  }

  fn new_within_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(
      LatLonBoundingBox::new_within_query(FIELD_NAME, r.min_lat, r.min_lon, r.max_lat, r.max_lon)?
        .into(),
    )
  }

  fn new_crosses_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(
      LatLonBoundingBox::new_crosses_query(FIELD_NAME, r.min_lat, r.min_lon, r.max_lat, r.max_lon)?
        .into(),
    )
  }

  fn next_range<R>(&self, random: &mut R, dimensions: usize) -> Result<Self::Range>
  where
    R: Rng + ?Sized,
  {
    GeoBBox::new(random, dimensions)
  }

  fn dimension<R>(&self, _random: &mut R) -> usize
  where
    R: Rng + ?Sized,
  {
    2
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestLatLonBoundingBoxQueries, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestLatLonBoundingBoxQueries;
  f(&case, &mut random)
}

#[derive(Clone)]
pub(crate) struct GeoBBox {
  pub(crate) base: RangeBase,
  pub(crate) min_lat: f64,
  pub(crate) min_lon: f64,
  pub(crate) max_lat: f64,
  pub(crate) max_lon: f64,
  pub(crate) dimension: usize,
}

impl GeoBBox {
  pub(crate) fn new<R>(random: &mut R, dimension: usize) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    let box_ = GeoTestUtil::next_box_not_crossing_dateline(random)?;
    Ok(Self {
      base: RangeBase::default(),
      min_lat: Self::quantize_lat(box_.min_lat)?,
      min_lon: Self::quantize_lon(box_.min_lon)?,
      max_lat: Self::quantize_lat(box_.max_lat)?,
      max_lon: Self::quantize_lon(box_.max_lon)?,
      dimension,
    })
  }

  fn quantize_lat(lat: f64) -> Result<f64> {
    Ok(GeoEncodingUtils::decode_latitude(
      GeoEncodingUtils::encode_latitude(lat)?,
    ))
  }

  fn quantize_lon(lon: f64) -> Result<f64> {
    Ok(GeoEncodingUtils::decode_longitude(
      GeoEncodingUtils::encode_longitude(lon)?,
    ))
  }

  fn set_min_lat(&mut self, d: f64) {
    if d > self.max_lat {
      self.min_lat = self.max_lat;
      self.max_lat = d;
    } else {
      self.min_lat = d;
    }
  }

  fn set_min_lon(&mut self, d: f64) {
    if d > self.max_lon {
      self.min_lon = self.max_lon;
      self.max_lon = d;
    } else {
      self.min_lon = d;
    }
  }

  fn set_max_lat(&mut self, d: f64) {
    if d < self.min_lat {
      self.max_lat = self.min_lat;
      self.min_lat = d;
    } else {
      self.max_lat = d;
    }
  }

  fn set_max_lon(&mut self, d: f64) {
    if d < self.min_lon {
      self.max_lon = self.min_lon;
      self.min_lon = d;
    } else {
      self.max_lon = d;
    }
  }
}

impl Range for GeoBBox {
  type Value = f64;

  fn get_base(&self) -> &RangeBase {
    &self.base
  }

  fn get_base_mut(&mut self) -> &mut RangeBase {
    &mut self.base
  }

  fn num_dimensions(&self) -> usize {
    self.dimension
  }

  fn get_min(&self, dim: usize) -> Self::Value {
    match dim {
      0 => self.min_lat,
      1 => self.min_lon,
      _ => panic!("dimension {} is greater than {}", dim, self.dimension),
    }
  }

  fn set_min(&mut self, dim: usize, val: Self::Value) {
    match dim {
      0 => self.set_min_lat(val),
      1 => self.set_min_lon(val),
      _ => panic!("dimension {} is greater than {}", dim, self.dimension),
    }
  }

  fn get_max(&self, dim: usize) -> Self::Value {
    match dim {
      0 => self.max_lat,
      1 => self.max_lon,
      _ => panic!("dimension {} is greater than {}", dim, self.dimension),
    }
  }

  fn set_max(&mut self, dim: usize, val: Self::Value) {
    match dim {
      0 => self.set_max_lat(val),
      1 => self.set_max_lon(val),
      _ => panic!("dimension {} is greater than {}", dim, self.dimension),
    }
  }

  fn is_equal(&self, other: &Self) -> bool {
    self.dimension == other.dimension
      && self.min_lat == other.min_lat
      && self.min_lon == other.min_lon
      && self.max_lat == other.max_lat
      && self.max_lon == other.max_lon
  }

  fn is_disjoint(&self, other: &Self) -> bool {
    if self.min_lat > other.max_lat || self.max_lat < other.min_lat {
      return true;
    }
    if self.min_lon > other.max_lon || self.max_lon < other.min_lon {
      return true;
    }
    false
  }

  fn is_within(&self, other: &Self) -> bool {
    other.contains(self)
  }

  fn contains(&self, other: &Self) -> bool {
    if self.min_lat > other.min_lat || self.max_lat < other.max_lat {
      return false;
    }
    if self.min_lon > other.min_lon || self.max_lon < other.max_lon {
      return false;
    }
    true
  }
}

impl fmt::Display for GeoBBox {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "GeoBoundingBox(lat: {} TO {}, lon: {} TO {})",
      self.min_lat, self.max_lat, self.min_lon, self.max_lon
    )
  }
}
#[test]
fn test_basics() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(dir, new_index_writer_config(&mut random)?)?;

  // Shared meridian test (disjoint)
  let mut document = Document::new();
  document.add(LatLonBoundingBox::new(
    FIELD_NAME, -20.0, -180.0, 20.0, -100.0,
  )?);
  writer.add_document(document)?;

  // intersects (crosses)
  let mut document = Document::new();
  document.add(LatLonBoundingBox::new(
    FIELD_NAME,
    0.0,
    14.096488952636719,
    10.0,
    20.0,
  )?);
  writer.add_document(document)?;

  // intersects (contains, crosses)
  let mut document = Document::new();
  document.add(LatLonBoundingBox::new(
    FIELD_NAME,
    -10.282592503353953,
    -1.0,
    1.0,
    14.096488952636719,
  )?);
  writer.add_document(document)?;

  // intersects (crosses)
  let mut document = Document::new();
  document.add(LatLonBoundingBox::new(FIELD_NAME, -1.0, -11.0, 1.0, 1.0)?);
  writer.add_document(document)?;

  // intersects (crosses)
  let mut document = Document::new();
  document.add(LatLonBoundingBox::new(
    FIELD_NAME,
    -1.0,
    14.096488952636719,
    5.0,
    30.0,
  )?);
  writer.add_document(document)?;

  // intersects (within)
  let mut document = Document::new();
  document.add(LatLonBoundingBox::new(
    FIELD_NAME,
    -5.0,
    0.0,
    -1.0,
    14.096488952636719,
  )?);
  writer.add_document(document)?;

  // search
  let reader = crate::core::index::directory_reader::open_from_writer(&writer)?;
  let searcher = new_searcher_with_reader(reader)?;
  assert_eq!(
    5,
    searcher.count(LatLonBoundingBox::new_intersects_query(
      FIELD_NAME,
      -10.282592503353953,
      0.0,
      0.0,
      14.096488952636719,
    )?)?
  );
  assert_eq!(
    1,
    searcher.count(LatLonBoundingBox::new_within_query(
      FIELD_NAME,
      -10.282592503353953,
      0.0,
      0.0,
      14.096488952636719,
    )?)?
  );
  assert_eq!(
    1,
    searcher.count(LatLonBoundingBox::new_contains_query(
      FIELD_NAME,
      -10.282592503353953,
      0.0,
      0.0,
      14.096488952636719,
    )?)?
  );
  assert_eq!(
    4,
    searcher.count(LatLonBoundingBox::new_crosses_query(
      FIELD_NAME,
      -10.282592503353953,
      0.0,
      0.0,
      14.096488952636719,
    )?)?
  );

  searcher.get_index_reader().close()?;
  writer.close()?;
  Ok(())
}
#[test]
fn test_to_string() -> Result<()> {
  let field = LatLonBoundingBox::new(FIELD_NAME, -20.0, -180.0, 20.0, -100.0)?;
  let expected = "LatLonBoundingBox <geoBoundingBoxField:[-20.000000023283064,-180.0,19.99999998137355,-100.0000000745058]>";
  assert_eq!(expected, field.to_string());
  Ok(())
}

mod base_range_field_query_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::sandbox_search_tests::test_lat_lon_bounding_box_queries::run_case;
  use crate::test_framework::core::search::base_range_field_query_test_case::BaseRangeFieldQueryTestCase;

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
