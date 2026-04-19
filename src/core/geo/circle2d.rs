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
use crate::core::geo::component2d::{Component2D, WithinRelation};
use crate::core::index::point_values::Relation;
/// 2D circle implementation containing spatial logic
pub struct Circle2D;
impl Component2D for Circle2D {
  fn get_min_x(&self) -> f64 {
    todo!()
  }

  fn get_max_x(&self) -> f64 {
    todo!()
  }

  fn get_min_y(&self) -> f64 {
    todo!()
  }

  fn get_max_y(&self) -> f64 {
    todo!()
  }

  fn contains(&self, _x: f64, _y: f64) -> bool {
    todo!()
  }

  fn relate(&self, _min_x: f64, _max_x: f64, _min_y: f64, _max_y: f64) -> Relation {
    todo!()
  }

  fn intersects_line(
    &self,
    _min_x: f64,
    _max_x: f64,
    _min_y: f64,
    _max_y: f64,
    _a_x: f64,
    _a_y: f64,
    _b_x: f64,
    _b_y: f64,
  ) -> bool {
    todo!()
  }

  fn intersects_triangle(
    &self,
    _min_x: f64,
    _max_x: f64,
    _min_y: f64,
    _max_y: f64,
    _a_x: f64,
    _a_y: f64,
    _b_x: f64,
    _b_y: f64,
    _c_x: f64,
    _c_y: f64,
  ) -> bool {
    todo!()
  }

  fn contains_line(
    &self,
    _min_x: f64,
    _max_x: f64,
    _min_y: f64,
    _max_y: f64,
    _a_x: f64,
    _a_y: f64,
    _b_x: f64,
    _b_y: f64,
  ) -> bool {
    todo!()
  }

  fn contains_triangle(
    &self,
    _min_x: f64,
    _max_x: f64,
    _min_y: f64,
    _max_y: f64,
    _a_x: f64,
    _a_y: f64,
    _b_x: f64,
    _b_y: f64,
    _c_x: f64,
    _c_y: f64,
  ) -> bool {
    todo!()
  }

  fn within_point(
    &self,
    _x: f64,
    _y: f64,
  ) -> crate::core::util::error::lucene_error::Result<WithinRelation> {
    todo!()
  }

  fn within_line(
    &self,
    _min_x: f64,
    _max_x: f64,
    _min_y: f64,
    _max_y: f64,
    _a_x: f64,
    _a_y: f64,
    _ab: bool,
    _b_x: f64,
    _b_y: f64,
  ) -> crate::core::util::error::lucene_error::Result<WithinRelation> {
    todo!()
  }

  fn within_triangle(
    &self,
    _min_x: f64,
    _max_x: f64,
    _min_y: f64,
    _max_y: f64,
    _a_x: f64,
    _a_y: f64,
    _ab: bool,
    _b_x: f64,
    _b_y: f64,
    _bc: bool,
    _c_x: f64,
    _c_y: f64,
    _ca: bool,
  ) -> crate::core::util::error::lucene_error::Result<WithinRelation> {
    todo!()
  }
}

trait DistanceCalculator {
  /// check if the point is within a distance
  fn contains(&self, x: f64, y: f64) -> bool;

  /// check if the line is within a distance
  fn intersects_line(&self, a_x: f64, a_y: f64, b_x: f64, b_y: f64) -> bool;

  /// Relates this calculator to the provided bounding box
  fn relate(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> Relation;

  /// check if the bounding box is disjoint with this calculator bounding box
  fn disjoint(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> bool;

  /// check if the bounding box is contains this calculator bounding box
  fn within(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> bool;

  /// get min X of this calculator
  fn get_min_x(&self) -> f64;

  /// get max X of this calculator
  fn get_max_x(&self) -> f64;

  /// get min Y of this calculator
  fn get_min_y(&self) -> f64;

  /// get max Y of this calculator
  fn get_max_y(&self) -> f64;

  /// get center X
  fn get_x(&self) -> f64;

  /// get center Y
  fn get_y(&self) -> f64;
}
