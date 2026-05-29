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
use crate::core::geo::circle2d::{CartesianDistance, Circle2D, create_from_xy_circle};
use crate::core::geo::geometry::Geometry;
use crate::core::geo::xy_encoding_utils::XYEncodingUtils;
use crate::core::geo::xy_geometry::XYGeometry;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// Represents a circle on the XY plane.
///
/// NOTES:
///
/// 1. X/Y precision is float.
/// 2. Radius precision is float.
#[derive(Clone, Copy, Debug)]
pub struct XYCircle {
  pub x: f32,
  pub y: f32,
  pub radius: f32,
}

impl XYCircle {
  /// Creates a new circle from the supplied x/y center and radius.
  pub fn new(x: f32, y: f32, radius: f32) -> Result<Self> {
    if radius <= 0.0 {
      return Err(LuceneError::illegal_argument(format!(
        "radius must be bigger than 0, got {radius}"
      )));
    }
    if !radius.is_finite() {
      return Err(LuceneError::illegal_argument(format!(
        "radius must be finite, got {radius}"
      )));
    }
    Ok(Self {
      x: XYEncodingUtils::check_val(x)?,
      y: XYEncodingUtils::check_val(y)?,
      radius,
    })
  }

  /// Returns the center's x
  pub fn get_x(&self) -> f32 {
    self.x
  }

  /// Returns the center's y
  pub fn get_y(&self) -> f32 {
    self.y
  }

  /// Returns the radius
  pub fn get_radius(&self) -> f32 {
    self.radius
  }
}

impl Geometry for XYCircle {
  type Component2D = Circle2D<CartesianDistance>;

  fn to_component2d(&self) -> Result<Self::Component2D> {
    create_from_xy_circle(self)
  }
}

impl XYGeometry for XYCircle {}

impl PartialEq for XYCircle {
  fn eq(&self, other: &Self) -> bool {
    self.x == other.x && self.y == other.y && self.radius == other.radius
  }
}

impl Eq for XYCircle {}

impl std::hash::Hash for XYCircle {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.x.to_bits().hash(state);
    self.y.to_bits().hash(state);
    self.radius.to_bits().hash(state);
  }
}

impl std::fmt::Display for XYCircle {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "XYCircle([{},{}] radius = {})",
      self.x, self.y, self.radius
    )
  }
}
