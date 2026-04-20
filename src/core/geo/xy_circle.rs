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
#[cfg(test)]
mod test_xy_circle {
  use super::*;
  use crate::test::core::geo::shape_test_util::ShapeTestUtil;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
  #[allow(dead_code)] // for quick search
  struct TestXYCircle;
  #[test]
  fn test_nan() {
    let err = XYCircle::new(f32::NAN, 45.23, 35.5);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(e.to_string().contains("invalid value NaN"));
    }

    let err = XYCircle::new(43.5, f32::NAN, 35.5);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(e.to_string().contains("invalid value NaN"));
    }
  }

  #[test]
  fn test_positive_inf() {
    let err = XYCircle::new(f32::INFINITY, 45.23, 35.5);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(e.to_string().contains("invalid value inf"));
    }

    let err = XYCircle::new(43.5, f32::INFINITY, 35.5);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(e.to_string().contains("invalid value inf"));
    }
  }

  #[test]
  fn test_negative_inf() {
    let err = XYCircle::new(f32::NEG_INFINITY, 45.23, 35.5);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(e.to_string().contains("invalid value -inf"));
    }

    let err = XYCircle::new(43.5, f32::NEG_INFINITY, 35.5);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(e.to_string().contains("invalid value -inf"));
    }
  }

  #[test]
  fn test_negative_radius() {
    let err = XYCircle::new(43.5, 45.23, -1000.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(
        e.to_string()
          .contains("radius must be bigger than 0, got -1000")
      );
    }
  }

  #[test]
  fn test_infinite_radius() {
    let err = XYCircle::new(43.5, 45.23, f32::INFINITY);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(
        e.to_string().contains("radius must be finite, got inf")
          || e
            .to_string()
            .contains("radius must be finite, got Infinity")
      );
    }
  }

  #[test]
  fn test_equals_and_hash_code() -> Result<()> {
    let mut random = random();
    let circle = ShapeTestUtil::next_circle(&mut random)?;
    let copy = XYCircle::new(circle.get_x(), circle.get_y(), circle.get_radius())?;
    assert_eq!(circle, copy);

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher1 = DefaultHasher::new();
    circle.hash(&mut hasher1);
    let hash1 = hasher1.finish();

    let mut hasher2 = DefaultHasher::new();
    copy.hash(&mut hasher2);
    let hash2 = hasher2.finish();

    assert_eq!(hash1, hash2);

    let other_circle = ShapeTestUtil::next_circle(&mut random)?;
    let mut hasher3 = DefaultHasher::new();
    other_circle.hash(&mut hasher3);
    let hash3 = hasher3.finish();

    if circle.get_x() != other_circle.get_x()
      || circle.get_y() != other_circle.get_y()
      || circle.get_radius() != other_circle.get_radius()
    {
      assert_ne!(circle, other_circle);
      assert_ne!(hash1, hash3);
    } else {
      assert_eq!(circle, other_circle);
      assert_eq!(hash1, hash3);
    }

    Ok(())
  }
}
