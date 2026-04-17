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
use crate::core::index::point_values::Relation;
/// 2D Geometry object that supports spatial relationships with bounding boxes, triangles and points
pub trait Component2D {
  /// min X value for the component
  fn get_min_x(&self) -> f64;

  /// max X value for the component
  fn get_max_x(&self) -> f64;

  /// min Y value for the component
  fn get_min_y(&self) -> f64;

  /// max Y value for the component
  fn get_max_y(&self) -> f64;

  /// relates this component2D with a point
  fn contains(&self, x: f64, y: f64) -> bool;

  /// relates this component2D with a bounding box
  fn relate(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> Relation;

  /// return true if this component2D intersects the provided line
  #[allow(clippy::too_many_arguments)]
  fn intersects_line(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    a_x: f64,
    a_y: f64,
    b_x: f64,
    b_y: f64,
  ) -> bool;

  /// return true if this component2D intersects the provided triangle
  #[allow(clippy::too_many_arguments)]
  fn intersects_triangle(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    a_x: f64,
    a_y: f64,
    b_x: f64,
    b_y: f64,
    c_x: f64,
    c_y: f64,
  ) -> bool;

  /// return true if this component2D contains the provided line
  #[allow(clippy::too_many_arguments)]
  fn contains_line(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    a_x: f64,
    a_y: f64,
    b_x: f64,
    b_y: f64,
  ) -> bool;

  /// return true if this component2D contains the provided triangle
  #[allow(clippy::too_many_arguments)]
  fn contains_triangle(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    a_x: f64,
    a_y: f64,
    b_x: f64,
    b_y: f64,
    c_x: f64,
    c_y: f64,
  ) -> bool;

  /// Compute the within relation of this component2D with a point
  fn within_point(&self, x: f64, y: f64) -> WithinRelation;

  /// Compute the within relation of this component2D with a line
  #[allow(clippy::too_many_arguments)]
  fn within_line(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    a_x: f64,
    a_y: f64,
    ab: bool,
    b_x: f64,
    b_y: f64,
  ) -> WithinRelation;

  /// Compute the within relation of this component2D with a triangle
  #[allow(clippy::too_many_arguments)]
  fn within_triangle(
    &self,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    a_x: f64,
    a_y: f64,
    ab: bool,
    b_x: f64,
    b_y: f64,
    bc: bool,
    c_x: f64,
    c_y: f64,
    ca: bool,
  ) -> WithinRelation;

  /// return true if this component2D intersects the provided line
  fn intersects_line_values(&self, a_x: f64, a_y: f64, b_x: f64, b_y: f64) -> bool {
    let min_y = a_y.min(b_y);
    let min_x = a_x.min(b_x);
    let max_y = a_y.max(b_y);
    let max_x = a_x.max(b_x);
    self.intersects_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y)
  }

  /// return true if this component2D intersects the provided triangle
  fn intersects_triangle_values(
    &self,
    a_x: f64,
    a_y: f64,
    b_x: f64,
    b_y: f64,
    c_x: f64,
    c_y: f64,
  ) -> bool {
    let min_y = a_y.min(b_y).min(c_y);
    let min_x = a_x.min(b_x).min(c_x);
    let max_y = a_y.max(b_y).max(c_y);
    let max_x = a_x.max(b_x).max(c_x);
    self.intersects_triangle(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, c_x, c_y)
  }

  /// return true if this component2D contains the provided line
  fn contains_line_values(&self, a_x: f64, a_y: f64, b_x: f64, b_y: f64) -> bool {
    let min_y = a_y.min(b_y);
    let min_x = a_x.min(b_x);
    let max_y = a_y.max(b_y);
    let max_x = a_x.max(b_x);
    self.contains_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y)
  }

  /// return true if this component2D contains the provided triangle
  fn contains_triangle_values(
    &self,
    a_x: f64,
    a_y: f64,
    b_x: f64,
    b_y: f64,
    c_x: f64,
    c_y: f64,
  ) -> bool {
    let min_y = a_y.min(b_y).min(c_y);
    let min_x = a_x.min(b_x).min(c_x);
    let max_y = a_y.max(b_y).max(c_y);
    let max_x = a_x.max(b_x).max(c_x);
    self.contains_triangle(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, c_x, c_y)
  }

  /// Compute the within relation of this component2D with a triangle
  fn within_line_values(&self, a_x: f64, a_y: f64, ab: bool, b_x: f64, b_y: f64) -> WithinRelation {
    let min_y = a_y.min(b_y);
    let min_x = a_x.min(b_x);
    let max_y = a_y.max(b_y);
    let max_x = a_x.max(b_x);
    self.within_line(min_x, max_x, min_y, max_y, a_x, a_y, ab, b_x, b_y)
  }

  /// Compute the within relation of this component2D with a triangle
  #[allow(clippy::too_many_arguments)]
  fn within_triangle_values(
    &self,
    a_x: f64,
    a_y: f64,
    ab: bool,
    b_x: f64,
    b_y: f64,
    bc: bool,
    c_x: f64,
    c_y: f64,
    ca: bool,
  ) -> WithinRelation {
    let min_y = a_y.min(b_y).min(c_y);
    let min_x = a_x.min(b_x).min(c_x);
    let max_y = a_y.max(b_y).max(c_y);
    let max_x = a_x.max(b_x).max(c_x);
    self.within_triangle(
      min_x, max_x, min_y, max_y, a_x, a_y, ab, b_x, b_y, bc, c_x, c_y, ca,
    )
  }

  /// Compute whether the bounding boxes are disjoint
  #[allow(clippy::too_many_arguments)]
  fn disjoint(
    min_x1: f64,
    max_x1: f64,
    min_y1: f64,
    max_y1: f64,
    min_x2: f64,
    max_x2: f64,
    min_y2: f64,
    max_y2: f64,
  ) -> bool {
    max_y1 < min_y2 || min_y1 > max_y2 || max_x1 < min_x2 || min_x1 > max_x2
  }

  /// Compute whether the first bounding box 1 is within the second bounding box
  #[allow(clippy::too_many_arguments)]
  fn within(
    min_x1: f64,
    max_x1: f64,
    min_y1: f64,
    max_y1: f64,
    min_x2: f64,
    max_x2: f64,
    min_y2: f64,
    max_y2: f64,
  ) -> bool {
    min_y2 <= min_y1 && max_y2 >= max_y1 && min_x2 <= min_x1 && max_x2 >= max_x1
  }

  /// returns true if rectangle (defined by minX, maxX, minY, maxY) contains the X Y point
  fn contains_point(x: f64, y: f64, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> bool {
    x >= min_x && x <= max_x && y >= min_y && y <= max_y
  }

  // /// Compute whether the given x, y point is in a triangle; uses the winding order method
  // fn point_in_triangle(
  //     min_x: f64,
  //     max_x: f64,
  //     min_y: f64,
  //     max_y: f64,
  //     x: f64,
  //     y: f64,
  //     a_x: f64,
  //     a_y: f64,
  //     b_x: f64,
  //     b_y: f64,
  //     c_x: f64,
  //     c_y: f64,
  // ) -> bool {
  //     if x >= min_x && x <= max_x && y >= min_y && y <= max_y {
  //         let a = orient(x, y, a_x, a_y, b_x, b_y);
  //         let b = orient(x, y, b_x, b_y, c_x, c_y);
  //         if a == 0 || b == 0 || (a < 0) == (b < 0) {
  //             let c = orient(x, y, c_x, c_y, a_x, a_y);
  //             c == 0 || (c < 0) == ((b < 0) || (a < 0))
  //         } else {
  //             false
  //         }
  //     } else {
  //         false
  //     }
  // }
}

/**
 * Used by withinTriangle to check the within relationship between a triangle and the query shape
 * (e.g. if the query shape is within the triangle).
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WithinRelation {
  /**
   * If the shape is a candidate for within. Typically this is return if the query shape is fully
   * inside the triangle or if the query shape intersects only edges that do not belong to the
   * original shape.
   */
  Candidate,
  /**
   * The query shape intersects an edge that does belong to the original shape or any point of the
   * triangle is inside the shape.
   */
  NotWithin,
  /// The query shape is disjoint with the triangle.
  Disjoint,
}
