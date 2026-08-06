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
use crate::core::document::fields::Fields;
use crate::core::document::lat_lon_doc_values_field::LatLonDocValuesField;
use crate::core::document::lat_lon_shape::LatLonShape;
use crate::core::document::shape_field::QueryRelation;
use crate::core::geo::component2d::{Component2D, WithinRelation};
use crate::core::geo::point::Point;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::document::base_lat_lon_doc_value_test_case::{
  BaseLatLonDocValueTestCaseHook, BaseLatLonDocValueTestCaseOwner,
};
use crate::test::core::document::base_lat_lon_spatial_test_case::LatLonEncoder;
use crate::test::core::document::base_spatial_test_case::{
  BaseSpatialTestCase, FIELD_NAME, Validator,
};
use crate::test_framework::core::geo::geo_test_util::GeoTestUtil;
use crate::test_framework::core::util::lucene_test_case::random;
use rand::Rng;

/// Random bounding box, line, and polygon query tests for random indexed arrays of latitude,
/// longitude points.
#[allow(dead_code)] // for quick search
struct TestLatLonDocValuesPointPointQueries;

impl BaseLatLonDocValueTestCaseHook for TestLatLonDocValuesPointPointQueries {
  type Shape = Point;
  type Validator = PointValidator;

  fn get_shape_type(&self) -> &'static str {
    "POINT"
  }

  fn next_shape<R>(&self, random: &mut R) -> Result<Self::Shape>
  where
    R: Rng + ?Sized,
  {
    GeoTestUtil::next_point(random)
  }

  fn create_indexable_fields(&self, _name: &str, point: &Point) -> Result<Vec<Fields>> {
    Ok(vec![
      LatLonDocValuesField::new(FIELD_NAME, point.get_lat(), point.get_lon())?.into(),
    ])
  }

  fn get_validator(&self) -> Result<Self::Validator> {
    Ok(PointValidator::new(LatLonEncoder))
  }
}

pub(crate) struct PointValidator {
  encoder: LatLonEncoder,
  query_relation: QueryRelation,
}

impl PointValidator {
  pub(crate) fn new(encoder: LatLonEncoder) -> Self {
    Self {
      encoder,
      query_relation: QueryRelation::Intersects,
    }
  }
}

impl Validator for PointValidator {
  type Shape = Point;
  type Encoder = LatLonEncoder;

  fn encoder(&self) -> &Self::Encoder {
    &self.encoder
  }

  fn query_relation(&self) -> QueryRelation {
    self.query_relation
  }

  fn set_relation(&mut self, relation: QueryRelation) {
    self.query_relation = relation;
  }

  fn test_component_query_with_shape(
    &self,
    query: &impl Component2D,
    point: &Point,
  ) -> Result<bool> {
    if self.query_relation == QueryRelation::Contains {
      return Ok(
        self.test_within_query(
          query,
          &LatLonShape::create_indexable_fields("dummy", point.get_lat(), point.get_lon())?,
        )? == WithinRelation::Candidate,
      );
    }
    self.test_component_query(
      query,
      &LatLonShape::create_indexable_fields("dummy", point.get_lat(), point.get_lon())?,
    )
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(
    &BaseLatLonDocValueTestCaseOwner<TestLatLonDocValuesPointPointQueries>,
    &mut rand::rngs::StdRng,
  ) -> Result<()>,
{
  let mut random = random();
  let case = BaseLatLonDocValueTestCaseOwner::new(TestLatLonDocValuesPointPointQueries);
  f(&case, &mut random)
}

mod lat_lon_doc_values_point_point_queries_tests {
  use super::*;

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_random_big() -> Result<()> {
    run_case(|case, random| case.do_test_random(random, 10_000))
  }
}

mod base_spatial_test_case_tests {
  use super::*;

  #[test]
  fn test_same_shape_many_times() -> Result<()> {
    run_case(|case, random| case.test_same_shape_many_times(random))
  }

  #[test]
  fn test_low_cardinality_shape_many_times() -> Result<()> {
    run_case(|case, random| case.test_low_cardinality_shape_many_times(random))
  }

  #[test]
  fn test_random_tiny() -> Result<()> {
    run_case(|case, random| case.test_random_tiny(random))
  }

  #[test]
  fn test_random_medium() -> Result<()> {
    run_case(|case, random| case.test_random_medium(random))
  }
}
