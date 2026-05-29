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
use crate::core::geo::rectangle2d::{Rectangle2D, create_from_xy_rectangle};
use crate::core::geo::xy_geometry::XYGeometry;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::fmt;
use std::fmt::{Display, Formatter};

/// Represents a x/y cartesian rectangle
#[derive(Debug, Clone)]
pub struct XYRectangle {
  /// minimum x value
  pub min_x: f32,

  /// maximum x value
  pub max_x: f32,

  /// minimum y value
  pub min_y: f32,

  /// maximum y value
  pub max_y: f32,
}

impl XYRectangle {
  /// Constructs a bounding box by first validating the provided x and y coordinates
  pub fn new(min_x: f32, max_x: f32, min_y: f32, max_y: f32) -> Result<Self> {
    if min_x > max_x {
      return Err(LuceneError::illegal_argument(format!(
        "minX must be lower than maxX, got {min_x} > {max_x}"
      )));
    }
    if min_y > max_y {
      return Err(LuceneError::illegal_argument(format!(
        "minY must be lower than maxY, got {min_y} > {max_y}"
      )));
    }

    Ok(Self {
      min_x: Self::check_val(min_x)?,
      max_x: Self::check_val(max_x)?,
      min_y: Self::check_val(min_y)?,
      max_y: Self::check_val(max_y)?,
    })
  }

  /// Compute Bounding Box for a circle in cartesian geometry
  pub fn from_point_distance(x: f32, y: f32, radius: f32) -> Result<Self> {
    Self::check_val(x)?;
    Self::check_val(y)?;

    if radius < 0.0 {
      return Err(LuceneError::illegal_argument(format!(
        "radius must be bigger than 0, got {radius}"
      )));
    }
    if !radius.is_finite() {
      return Err(LuceneError::illegal_argument(format!(
        "radius must be finite, got {radius}"
      )));
    }

    // LUCENE-9243: We round up the bounding box to avoid
    // numerical errors.
    let distance_box = radius.next_up();
    let min_x = (-f32::MAX).max(x - distance_box);
    let max_x = f32::MAX.min(x + distance_box);
    let min_y = (-f32::MAX).max(y - distance_box);
    let max_y = f32::MAX.min(y + distance_box);

    Self::new(min_x, max_x, min_y, max_y)
  }

  fn check_val(v: f32) -> Result<f32> {
    if !v.is_finite() {
      return Err(LuceneError::illegal_argument(format!(
        "invalid value {v}: must be finite"
      )));
    }
    Ok(v)
  }
}
impl Eq for XYRectangle {}
impl PartialEq for XYRectangle {
  fn eq(&self, other: &Self) -> bool {
    self.min_x.to_bits() == other.min_x.to_bits()
      && self.min_y.to_bits() == other.min_y.to_bits()
      && self.max_x.to_bits() == other.max_x.to_bits()
      && self.max_y.to_bits() == other.max_y.to_bits()
  }
}
impl std::hash::Hash for XYRectangle {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.min_x.to_bits().hash(state);
    self.min_y.to_bits().hash(state);
    self.max_x.to_bits().hash(state);
    self.max_y.to_bits().hash(state);
  }
}

impl Geometry for XYRectangle {
  type Component2D = Rectangle2D;

  fn to_component2d(&self) -> Result<Self::Component2D> {
    Ok(create_from_xy_rectangle(self))
  }
}

impl XYGeometry for XYRectangle {}
impl Display for XYRectangle {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "XYRectangle(x={} TO {} y={} TO {})",
      self.min_x, self.max_x, self.min_y, self.max_y
    )
  }
}
