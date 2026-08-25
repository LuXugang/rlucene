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
use crate::core::geo::geometry::Geometry;
use crate::core::geo::point2d::{Point2D, create_from_xy_point};
use crate::core::geo::xy_encoding_utils::XYEncodingUtils;
use crate::core::geo::xy_geometry::XYGeometry;
use crate::core::util::error::lucene_error::Result;
/// Represents a point on the x/y plane. You can construct the point directly
/// with `f32` coordinates.
///
/// NOTES:
///
/// 1. x/y values must be finite.
/// 2. For more advanced spatial indexing and query operations see the
///    `spatial-extras` module.
#[derive(Clone, Copy, Debug)]
pub struct XYPoint {
  /// x coordinate
  x: f32,

  /// y coordinate
  y: f32,
}

impl XYPoint {
  /// Creates a new Point from the supplied x/y.
  pub fn new(x: f32, y: f32) -> Result<Self> {
    Ok(Self {
      x: XYEncodingUtils::check_val(x)?,
      y: XYEncodingUtils::check_val(y)?,
    })
  }

  /// Returns x value at given index
  pub fn get_x(&self) -> f32 {
    self.x
  }

  /// Returns y value at given index
  pub fn get_y(&self) -> f32 {
    self.y
  }
}

impl Geometry for XYPoint {
  type Component2D = Point2D;

  fn to_component2d(&self) -> Result<Self::Component2D> {
    Ok(create_from_xy_point(self))
  }
}

impl XYGeometry for XYPoint {}

impl PartialEq for XYPoint {
  fn eq(&self, other: &Self) -> bool {
    self.x == other.x && self.y == other.y
  }
}

impl Eq for XYPoint {}

impl std::hash::Hash for XYPoint {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.x.to_bits().hash(state);
    self.y.to_bits().hash(state);
  }
}

impl std::fmt::Display for XYPoint {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "XYPoint({},{})", self.x, self.y)
  }
}
