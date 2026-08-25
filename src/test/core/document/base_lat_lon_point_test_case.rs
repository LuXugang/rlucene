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
use crate::core::document::lat_lon_point_query::lat_lon_point_query;
use crate::core::document::shape_field::QueryRelation;
use crate::core::geo::circle::Circle;
use crate::core::geo::lat_lon_geometry::LatLonGeometryEnum;
use crate::core::geo::line::Line;
use crate::core::geo::point::Point;
use crate::core::geo::polygon::Polygon;
use crate::core::geo::rectangle::Rectangle;
use crate::core::index::index_reader::IndexReader;
use crate::core::search::query::Query;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::io_utils::IOUtils;
use crate::test::core::document::base_lat_lon_spatial_test_case::{
  BaseLatLonSpatialTestCase, BaseLatLonSpatialTestCaseDefaults, LatLonComponent2D, LatLonEncoder,
};
use crate::test::core::document::base_spatial_test_case::{
  BaseSpatialTestCase, FIELD_NAME, Validator,
};
use crate::test_framework::core::geo::geo_test_util::GeoTestUtil;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::query_utils::QueryUtils;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_searcher,
};
use rand::{Rng, RngExt};
use std::fmt::Debug;
use std::sync::Arc;

/// Base test case for testing geospatial indexing and search functionality for
/// [`LatLonPoint`](crate::core::document::lat_lon_point::LatLonPoint).
pub trait BaseLatLonPointTestCase: BaseLatLonSpatialTestCase {
  fn test_bounding_box_queries_equivalence<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    BaseLatLonPointTestCaseDefaults::test_bounding_box_queries_equivalence(self, random)
  }

  fn test_query_equals_and_hashcode<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    BaseLatLonPointTestCaseDefaults::test_query_equals_and_hashcode(self, random)
  }
}

pub struct BaseLatLonPointTestCaseDefaults;

impl BaseLatLonPointTestCaseDefaults {
  pub fn new_rect_query(
    field: &str,
    query_relation: QueryRelation,
    min_lon: f64,
    max_lon: f64,
    min_lat: f64,
    max_lat: f64,
  ) -> Result<Query> {
    LatLonPoint::new_geometry_query(
      field,
      query_relation,
      vec![Rectangle::new(min_lat, max_lat, min_lon, max_lon)?],
    )
  }

  pub fn new_line_query(
    field: &str,
    query_relation: QueryRelation,
    lines: Vec<Line>,
  ) -> Result<Query> {
    LatLonPoint::new_geometry_query(field, query_relation, lines)
  }

  pub fn new_polygon_query(
    field: &str,
    query_relation: QueryRelation,
    polygons: Vec<Polygon>,
  ) -> Result<Query> {
    LatLonPoint::new_geometry_query(field, query_relation, polygons)
  }

  pub fn new_distance_query(
    field: &str,
    query_relation: QueryRelation,
    circle: Circle,
  ) -> Result<Query> {
    LatLonPoint::new_geometry_query(field, query_relation, vec![circle])
  }

  pub fn new_points_query(
    field: &str,
    query_relation: QueryRelation,
    points: Vec<Point>,
  ) -> Result<Query> {
    LatLonPoint::new_geometry_query(field, query_relation, points)
  }

  pub fn test_bounding_box_queries_equivalence<T, R>(test_case: &T, random: &mut R) -> Result<()>
  where
    T: BaseLatLonPointTestCase + ?Sized,
    R: Rng + ?Sized,
  {
    let num_shapes = at_least(random, 20);

    let dir = new_directory_shared(random)?;
    let writer = RandomIndexWriter::new(random, dir.clone())?;

    for _ in 0..num_shapes {
      let shape = test_case.next_shape(random)?;
      test_case.index_random_shapes(random, &writer.w, &[Some(shape)])?;
    }
    if random.random_bool(0.5) {
      writer.force_merge(random, 1)?;
    }

    ///// Search //////
    let reader = Arc::new(writer.get_reader(random)?);
    writer.close(random)?;
    let searcher = new_searcher(random, reader.clone())?;

    let box_ = GeoTestUtil::next_box(random)?;

    let q1 = LatLonPoint::new_box_query(
      FIELD_NAME,
      box_.min_lat,
      box_.max_lat,
      box_.min_lon,
      box_.max_lon,
    )?;
    let q2: Query = lat_lon_point_query(
      FIELD_NAME.to_string(),
      QueryRelation::Intersects,
      vec![LatLonGeometryEnum::from(box_)],
    )?
    .into();
    assert_eq!(searcher.count(q1)?, searcher.count(q2)?);

    let close_result = reader.close();
    IOUtils::use_or_suppress_result(close_result, dir.close())
  }

  pub fn test_query_equals_and_hashcode<T, R>(test_case: &T, random: &mut R) -> Result<()>
  where
    T: BaseLatLonPointTestCase + ?Sized,
    R: Rng + ?Sized,
  {
    let polygon = GeoTestUtil::next_polygon(random)?;
    let query_relations = [QueryRelation::Intersects, QueryRelation::Disjoint];
    let query_relation = query_relations[random.random_range(0..query_relations.len())];
    let field_name = "foo";
    let q1 = BaseSpatialTestCase::new_polygon_query(
      test_case,
      field_name,
      query_relation,
      vec![polygon.clone()],
    )?;
    let q2 = BaseSpatialTestCase::new_polygon_query(
      test_case,
      field_name,
      query_relation,
      vec![polygon.clone()],
    )?;
    QueryUtils::check_equal(&q1, &q2);
    // Different field name.
    let q3 = BaseSpatialTestCase::new_polygon_query(
      test_case,
      "bar",
      query_relation,
      vec![polygon.clone()],
    )?;
    QueryUtils::check_unequal(&q1, &q3);
    // Different query relation.
    let new_query_relation = query_relations[random.random_range(0..query_relations.len())];
    let q4 = BaseSpatialTestCase::new_polygon_query(
      test_case,
      field_name,
      new_query_relation,
      vec![polygon.clone()],
    )?;
    if query_relation == new_query_relation {
      QueryUtils::check_equal(&q1, &q4);
    } else {
      QueryUtils::check_unequal(&q1, &q4);
    }
    // Different shape.
    let new_polygon = GeoTestUtil::next_polygon(random)?;
    let q5 = BaseSpatialTestCase::new_polygon_query(
      test_case,
      field_name,
      query_relation,
      vec![new_polygon.clone()],
    )?;
    if polygon == new_polygon {
      QueryUtils::check_equal(&q1, &q5);
    } else {
      QueryUtils::check_unequal(&q1, &q5);
    }
    Ok(())
  }
}

pub trait BaseLatLonPointTestCaseHook {
  type Shape: Clone + Debug;
  type Validator: Validator<Shape = Self::Shape, Encoder = LatLonEncoder>;

  #[allow(dead_code)]
  fn get_shape_type(&self) -> &'static str;

  fn next_shape<R>(&self, random: &mut R) -> Result<Self::Shape>
  where
    R: Rng + ?Sized;

  fn create_indexable_fields(&self, name: &str, shape: &Self::Shape) -> Result<Vec<Fields>>;

  fn get_validator(&self) -> Result<Self::Validator>;
}

pub struct BaseLatLonPointTestCaseOwner<H> {
  hook: H,
}

impl<H> BaseLatLonPointTestCaseOwner<H> {
  pub fn new(hook: H) -> Self {
    Self { hook }
  }
}

impl<H> BaseSpatialTestCase for BaseLatLonPointTestCaseOwner<H>
where
  H: BaseLatLonPointTestCaseHook,
{
  type Shape = H::Shape;
  type Line = Line;
  type Polygon = Polygon;
  type Rectangle = Rectangle;
  type Point = Point;
  type Circle = Circle;
  type Component2D = LatLonComponent2D;
  type Encoder = LatLonEncoder;
  type Validator = H::Validator;

  fn get_shape_type(&self) -> &'static str {
    self.hook.get_shape_type()
  }

  fn next_shape<R>(&self, random: &mut R) -> Result<Self::Shape>
  where
    R: Rng + ?Sized,
  {
    self.hook.next_shape(random)
  }

  fn get_encoder(&self) -> Self::Encoder {
    LatLonEncoder
  }

  fn create_indexable_fields(&self, field: &str, shape: &Self::Shape) -> Result<Vec<Fields>> {
    self.hook.create_indexable_fields(field, shape)
  }

  fn next_line<R>(&self, random: &mut R) -> Result<Self::Line>
  where
    R: Rng + ?Sized,
  {
    BaseLatLonSpatialTestCaseDefaults::next_line(random)
  }

  fn next_polygon<R>(&self, random: &mut R) -> Result<Self::Polygon>
  where
    R: Rng + ?Sized,
  {
    BaseLatLonSpatialTestCaseDefaults::next_polygon(random)
  }

  fn random_query_box<R>(&self, random: &mut R) -> Result<Self::Rectangle>
  where
    R: Rng + ?Sized,
  {
    BaseLatLonSpatialTestCaseDefaults::random_query_box(random)
  }

  fn next_points<R>(&self, random: &mut R) -> Result<Vec<Self::Point>>
  where
    R: Rng + ?Sized,
  {
    BaseLatLonSpatialTestCaseDefaults::next_points(random)
  }

  fn next_circle<R>(&self, random: &mut R) -> Result<Self::Circle>
  where
    R: Rng + ?Sized,
  {
    BaseLatLonSpatialTestCaseDefaults::next_circle(random)
  }

  fn rect_min_x(&self, rect: &Self::Rectangle) -> f64 {
    BaseLatLonSpatialTestCaseDefaults::rect_min_x(rect)
  }

  fn rect_max_x(&self, rect: &Self::Rectangle) -> f64 {
    BaseLatLonSpatialTestCaseDefaults::rect_max_x(rect)
  }

  fn rect_min_y(&self, rect: &Self::Rectangle) -> f64 {
    BaseLatLonSpatialTestCaseDefaults::rect_min_y(rect)
  }

  fn rect_max_y(&self, rect: &Self::Rectangle) -> f64 {
    BaseLatLonSpatialTestCaseDefaults::rect_max_y(rect)
  }

  fn rect_crosses_dateline(&self, rect: &Self::Rectangle) -> bool {
    BaseLatLonSpatialTestCaseDefaults::rect_crosses_dateline(rect)
  }

  fn new_rect_query(
    &self,
    field: &str,
    query_relation: QueryRelation,
    min_lon: f64,
    max_lon: f64,
    min_lat: f64,
    max_lat: f64,
  ) -> Result<Query> {
    BaseLatLonPointTestCaseDefaults::new_rect_query(
      field,
      query_relation,
      min_lon,
      max_lon,
      min_lat,
      max_lat,
    )
  }

  fn new_line_query(
    &self,
    field: &str,
    query_relation: QueryRelation,
    lines: Vec<Self::Line>,
  ) -> Result<Query> {
    BaseLatLonPointTestCaseDefaults::new_line_query(field, query_relation, lines)
  }

  fn new_polygon_query(
    &self,
    field: &str,
    query_relation: QueryRelation,
    polygons: Vec<Self::Polygon>,
  ) -> Result<Query> {
    BaseLatLonPointTestCaseDefaults::new_polygon_query(field, query_relation, polygons)
  }

  fn new_points_query(
    &self,
    field: &str,
    query_relation: QueryRelation,
    points: Vec<Self::Point>,
  ) -> Result<Query> {
    BaseLatLonPointTestCaseDefaults::new_points_query(field, query_relation, points)
  }

  fn new_distance_query(
    &self,
    field: &str,
    query_relation: QueryRelation,
    circle: Self::Circle,
  ) -> Result<Query> {
    BaseLatLonPointTestCaseDefaults::new_distance_query(field, query_relation, circle)
  }

  fn to_line_2d(&self, lines: Vec<Self::Line>) -> Result<Self::Component2D> {
    BaseLatLonSpatialTestCaseDefaults::to_line_2d(lines)
  }

  fn to_polygon_2d(&self, polygons: Vec<Self::Polygon>) -> Result<Self::Component2D> {
    BaseLatLonSpatialTestCaseDefaults::to_polygon_2d(polygons)
  }

  fn to_point_2d(&self, points: Vec<Self::Point>) -> Result<Self::Component2D> {
    BaseLatLonSpatialTestCaseDefaults::to_point_2d(points)
  }

  fn to_circle_2d(&self, circle: Self::Circle) -> Result<Self::Component2D> {
    BaseLatLonSpatialTestCaseDefaults::to_circle_2d(circle)
  }

  fn to_rectangle_2d(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
  ) -> Result<Self::Component2D> {
    BaseLatLonSpatialTestCaseDefaults::to_rectangle_2d(min_x, max_x, min_y, max_y)
  }

  fn get_validator(&self) -> Result<Self::Validator> {
    self.hook.get_validator()
  }
}

impl<H> BaseLatLonSpatialTestCase for BaseLatLonPointTestCaseOwner<H> where
  H: BaseLatLonPointTestCaseHook
{
}

impl<H> BaseLatLonPointTestCase for BaseLatLonPointTestCaseOwner<H> where
  H: BaseLatLonPointTestCaseHook
{
}
