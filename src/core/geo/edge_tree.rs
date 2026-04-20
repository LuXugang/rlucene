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
use crate::core::util::error::lucene_error::Result;
pub struct EdgeTree {
  pub(crate) y1: f64,
  pub(crate) y2: f64,
  pub(crate) x1: f64,
  pub(crate) x2: f64,

  /// Min Y of this edge.
  pub(crate) low: f64,

  /// Max Y of this edge or any children.
  pub(crate) max: f64,
}
impl EdgeTree {
  pub(crate) fn is_point_online(&self, _x: f64, _y: f64) -> bool {
    todo!()
  }
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn crosses_triangle(
    &self,
    _min_x: f64,
    _max_x: f64,
    _min_y: f64,
    _max_y: f64,
    _ax: f64,
    _ay: f64,
    _bx: f64,
    _by: f64,
    _cx: f64,
    _cy: f64,
    _include_boundary: bool,
  ) -> bool {
    todo!()
  }
  pub(crate) fn crosses_box(
    &self,
    _min_x: f64,
    _max_x: f64,
    _min_y: f64,
    _max_y: f64,
    _include_boundary: bool,
  ) -> bool {
    todo!()
  }
  /// Returns true if the line crosses any edge in this edge subtree
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn crosses_line(
    &self,
    _min_x: f64,
    _max_x: f64,
    _min_y: f64,
    _max_y: f64,
    _a2x: f64,
    _a2y: f64,
    _b2x: f64,
    _b2y: f64,
    _include_boundary: bool,
  ) -> bool {
    todo!()
  }
}
pub(crate) fn create_tree(_x: &[f64], _y: &[f64]) -> Result<EdgeTree> {
  todo!()
}
