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
use crate::core::document::lat_lon_point::LatLonPoint;
use crate::core::document::shape_field::QueryRelation;
use crate::core::geo::component2d::Component2D;
use crate::core::geo::point::Point;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::document::base_lat_lon_point_test_case::{
  BaseLatLonPointTestCase, BaseLatLonPointTestCaseHook, BaseLatLonPointTestCaseOwner,
};
use crate::test::core::document::base_lat_lon_spatial_test_case::LatLonEncoder;
use crate::test::core::document::base_spatial_test_case::{
  BaseSpatialTestCase, FIELD_NAME, Validator,
};
use crate::test::core::document::test_lat_lon_point_point_queries::PointValidator;
use crate::test_framework::core::geo::geo_test_util::GeoTestUtil;
use crate::test_framework::core::util::lucene_test_case::random;
use rand::{Rng, RngExt};

/// Random bounding box, line, and polygon query tests for random indexed arrays of latitude,
/// longitude points.
#[allow(dead_code)] // for quick search
struct TestLatLonMultiPointPointQueries;

impl BaseLatLonPointTestCaseHook for TestLatLonMultiPointPointQueries {
  type Shape = Vec<Point>;
  type Validator = MultiPointValidator;

  fn get_shape_type(&self) -> &'static str {
    "POINT"
  }

  fn next_shape<R>(&self, random: &mut R) -> Result<Self::Shape>
  where
    R: Rng + ?Sized,
  {
    let count = random.random_range(1..=4);
    let mut points = Vec::with_capacity(count);
    for _ in 0..count {
      points.push(GeoTestUtil::next_point(random)?);
    }
    Ok(points)
  }

  fn create_indexable_fields(&self, _name: &str, points: &Self::Shape) -> Result<Vec<Fields>> {
    points
      .iter()
      .map(|point| Ok(LatLonPoint::new(FIELD_NAME, point.get_lat(), point.get_lon())?.into()))
      .collect()
  }

  fn get_validator(&self) -> Result<Self::Validator> {
    Ok(MultiPointValidator::new(LatLonEncoder))
  }
}

struct MultiPointValidator {
  encoder: LatLonEncoder,
  query_relation: QueryRelation,
  point_validator: PointValidator,
}

impl MultiPointValidator {
  fn new(encoder: LatLonEncoder) -> Self {
    Self {
      encoder,
      query_relation: QueryRelation::Intersects,
      point_validator: PointValidator::new(encoder),
    }
  }
}

impl Validator for MultiPointValidator {
  type Shape = Vec<Point>;
  type Encoder = LatLonEncoder;

  fn encoder(&self) -> &Self::Encoder {
    &self.encoder
  }

  fn query_relation(&self) -> QueryRelation {
    self.query_relation
  }

  fn set_relation(&mut self, relation: QueryRelation) {
    self.query_relation = relation;
    self.point_validator.set_relation(relation);
  }

  #[allow(clippy::if_same_then_else)]
  fn test_component_query_with_shape(
    &self,
    query: &impl Component2D,
    points: &Self::Shape,
  ) -> Result<bool> {
    for point in points {
      let matches = self
        .point_validator
        .test_component_query_with_shape(query, point)?;
      if matches && self.query_relation == QueryRelation::Intersects {
        return Ok(true);
      } else if matches && self.query_relation == QueryRelation::Contains {
        return Ok(true);
      } else if !matches && self.query_relation == QueryRelation::Disjoint {
        return Ok(false);
      } else if !matches && self.query_relation == QueryRelation::Within {
        return Ok(false);
      }
    }
    Ok(!matches!(
      self.query_relation,
      QueryRelation::Intersects | QueryRelation::Contains
    ))
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(
    &BaseLatLonPointTestCaseOwner<TestLatLonMultiPointPointQueries>,
    &mut rand::rngs::StdRng,
  ) -> Result<()>,
{
  let mut random = random();
  let case = BaseLatLonPointTestCaseOwner::new(TestLatLonMultiPointPointQueries);
  f(&case, &mut random)
}

mod lat_lon_multi_point_point_queries_tests {
  use super::*;

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_random_big() -> Result<()> {
    run_case(|case, random| case.do_test_random(random, 10_000))
  }
}

mod base_lat_lon_point_test_case_tests {
  use super::*;

  #[test]
  fn test_bounding_box_queries_equivalence() -> Result<()> {
    run_case(|case, random| case.test_bounding_box_queries_equivalence(random))
  }

  #[test]
  fn test_query_equals_and_hashcode() -> Result<()> {
    run_case(|case, random| case.test_query_equals_and_hashcode(random))
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
