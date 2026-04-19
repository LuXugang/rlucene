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
use crate::core::util::error::lucene_error::Result;

pub struct Rectangle2D {}
impl Component2D for Rectangle2D {
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

  fn within_point(&self, _x: f64, _y: f64) -> Result<WithinRelation> {
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
  ) -> Result<WithinRelation> {
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
  ) -> Result<WithinRelation> {
    todo!()
  }
}
