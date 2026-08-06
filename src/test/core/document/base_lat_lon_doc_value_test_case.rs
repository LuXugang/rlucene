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
use crate::core::document::shape_field::QueryRelation;
use crate::core::geo::circle::Circle;
use crate::core::geo::line::Line;
use crate::core::geo::point::Point;
use crate::core::geo::polygon::Polygon;
use crate::core::geo::rectangle::Rectangle;
use crate::core::search::query::Query;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::document::base_lat_lon_spatial_test_case::{
  BaseLatLonSpatialTestCase, BaseLatLonSpatialTestCaseDefaults, LatLonComponent2D, LatLonEncoder,
};
use crate::test::core::document::base_spatial_test_case::{BaseSpatialTestCase, Validator};
use rand::Rng;
use std::fmt::Debug;

/// Base test case for testing geospatial indexing and search functionality for
/// `LatLonDocValuesField`.
pub trait BaseLatLonDocValueTestCase: BaseLatLonSpatialTestCase {}

pub struct BaseLatLonDocValueTestCaseDefaults;

impl BaseLatLonDocValueTestCaseDefaults {
  pub fn new_rect_query(
    field: &str,
    query_relation: QueryRelation,
    min_lon: f64,
    max_lon: f64,
    min_lat: f64,
    max_lat: f64,
  ) -> Result<Query> {
    LatLonDocValuesField::new_slow_geometry_query(
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
    LatLonDocValuesField::new_slow_geometry_query(field, query_relation, lines)
  }

  pub fn new_polygon_query(
    field: &str,
    query_relation: QueryRelation,
    polygons: Vec<Polygon>,
  ) -> Result<Query> {
    LatLonDocValuesField::new_slow_geometry_query(field, query_relation, polygons)
  }

  pub fn new_distance_query(
    field: &str,
    query_relation: QueryRelation,
    circle: Circle,
  ) -> Result<Query> {
    LatLonDocValuesField::new_slow_geometry_query(field, query_relation, vec![circle])
  }

  pub fn new_points_query(
    field: &str,
    query_relation: QueryRelation,
    points: Vec<Point>,
  ) -> Result<Query> {
    LatLonDocValuesField::new_slow_geometry_query(field, query_relation, points)
  }
}

pub trait BaseLatLonDocValueTestCaseHook {
  type Shape: Clone + Debug;
  type Validator: Validator<Shape = Self::Shape, Encoder = LatLonEncoder>;

  fn get_shape_type(&self) -> &'static str;

  fn next_shape<R>(&self, random: &mut R) -> Result<Self::Shape>
  where
    R: Rng + ?Sized;

  fn create_indexable_fields(&self, name: &str, shape: &Self::Shape) -> Result<Vec<Fields>>;

  fn get_validator(&self) -> Result<Self::Validator>;
}

pub struct BaseLatLonDocValueTestCaseOwner<H> {
  hook: H,
}

impl<H> BaseLatLonDocValueTestCaseOwner<H> {
  pub fn new(hook: H) -> Self {
    Self { hook }
  }
}

impl<H> BaseSpatialTestCase for BaseLatLonDocValueTestCaseOwner<H>
where
  H: BaseLatLonDocValueTestCaseHook,
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
    BaseLatLonSpatialTestCaseDefaults::get_encoder()
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
    BaseLatLonDocValueTestCaseDefaults::new_rect_query(
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
    BaseLatLonDocValueTestCaseDefaults::new_line_query(field, query_relation, lines)
  }

  fn new_polygon_query(
    &self,
    field: &str,
    query_relation: QueryRelation,
    polygons: Vec<Self::Polygon>,
  ) -> Result<Query> {
    BaseLatLonDocValueTestCaseDefaults::new_polygon_query(field, query_relation, polygons)
  }

  fn new_points_query(
    &self,
    field: &str,
    query_relation: QueryRelation,
    points: Vec<Self::Point>,
  ) -> Result<Query> {
    BaseLatLonDocValueTestCaseDefaults::new_points_query(field, query_relation, points)
  }

  fn new_distance_query(
    &self,
    field: &str,
    query_relation: QueryRelation,
    circle: Self::Circle,
  ) -> Result<Query> {
    BaseLatLonDocValueTestCaseDefaults::new_distance_query(field, query_relation, circle)
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

impl<H> BaseLatLonSpatialTestCase for BaseLatLonDocValueTestCaseOwner<H> where
  H: BaseLatLonDocValueTestCaseHook
{
}

impl<H> BaseLatLonDocValueTestCase for BaseLatLonDocValueTestCaseOwner<H> where
  H: BaseLatLonDocValueTestCaseHook
{
}
