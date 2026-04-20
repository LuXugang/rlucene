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
#[cfg(test)]
mod tests {
  use crate::core::geo::component2d::Component2D;
  use crate::core::geo::geometry::Geometry;
  use crate::core::geo::xy_rectangle::XYRectangle;
  use crate::core::util::error::lucene_error::LuceneError;
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::geo::shape_test_util::ShapeTestUtil;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{at_least, random};
  use crate::test::core::util::test_util::TestUtil;
  use rand::RngExt;
  #[allow(dead_code)] // for quick search
  struct TestXYRectangle;

  #[test]
  fn test_invalid_min_max_x() {
    let err = XYRectangle::new(5.0, 4.0, 3.0, 4.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("5 > 4"));
  }

  #[test]
  fn test_invalid_min_max_y() {
    let err = XYRectangle::new(4.0, 5.0, 5.0, 4.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("5 > 4"));
  }

  #[test]
  fn test_nan() {
    let err = XYRectangle::new(f32::NAN, 4.0, 3.0, 4.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("invalid value NaN"));

    let err = XYRectangle::new(3.0, f32::NAN, 3.0, 4.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("invalid value NaN"));

    let err = XYRectangle::new(3.0, 4.0, f32::NAN, 4.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("invalid value NaN"));

    let err = XYRectangle::new(3.0, 4.0, 3.0, f32::NAN);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("invalid value NaN"));
  }

  #[test]
  fn test_positive_inf() {
    let err = XYRectangle::new(3.0, f32::INFINITY, 3.0, 4.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("invalid value inf"));

    let err = XYRectangle::new(3.0, 4.0, 3.0, f32::INFINITY);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("invalid value inf"));
  }

  #[test]
  fn test_negative_inf() {
    let err = XYRectangle::new(f32::NEG_INFINITY, 4.0, 3.0, 4.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("invalid value -inf"));

    let err = XYRectangle::new(3.0, 4.0, f32::NEG_INFINITY, 4.0);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert!(err.unwrap_err().to_string().contains("invalid value -inf"));
  }

  #[test]
  fn test_equals_and_hash_code() -> Result<()> {
    let mut random = random();
    let rectangle = ShapeTestUtil::next_box(&mut random)?;
    let copy = XYRectangle::new(
      rectangle.min_x,
      rectangle.max_x,
      rectangle.min_y,
      rectangle.max_y,
    )?;

    assert_eq!(rectangle, copy);
    use std::hash::{Hash, Hasher};
    let mut hasher1 = std::collections::hash_map::DefaultHasher::new();
    rectangle.hash(&mut hasher1);
    let mut hasher2 = std::collections::hash_map::DefaultHasher::new();
    copy.hash(&mut hasher2);
    assert_eq!(hasher1.finish(), hasher2.finish());

    let other_rectangle = ShapeTestUtil::next_box(&mut random)?;
    if rectangle.min_x.to_bits() != other_rectangle.min_x.to_bits()
      || rectangle.max_x.to_bits() != other_rectangle.max_x.to_bits()
      || rectangle.min_y.to_bits() != other_rectangle.min_y.to_bits()
      || rectangle.max_y.to_bits() != other_rectangle.max_y.to_bits()
    {
      assert_ne!(rectangle, other_rectangle);

      let mut hasher1 = std::collections::hash_map::DefaultHasher::new();
      rectangle.hash(&mut hasher1);
      let mut hasher2 = std::collections::hash_map::DefaultHasher::new();
      other_rectangle.hash(&mut hasher2);
      assert_ne!(hasher1.finish(), hasher2.finish());
    } else {
      assert_eq!(rectangle, other_rectangle);

      let mut hasher1 = std::collections::hash_map::DefaultHasher::new();
      rectangle.hash(&mut hasher1);
      let mut hasher2 = std::collections::hash_map::DefaultHasher::new();
      other_rectangle.hash(&mut hasher2);
      assert_eq!(hasher1.finish(), hasher2.finish());
    }

    Ok(())
  }

  #[test]
  fn test_random_circle_to_bbox() -> Result<()> {
    let mut random = random();
    let iters = at_least(&mut random, 100);

    for _iter in 0..iters {
      let center_x = ShapeTestUtil::next_float(&mut random);
      let center_y = ShapeTestUtil::next_float(&mut random);

      let radius = if random.random_bool(0.5) {
        random.random::<f32>() * TestUtil::next_int(&mut random, 1, 100000) as f32
      } else {
        ShapeTestUtil::next_float(&mut random).abs()
      };

      let bbox = XYRectangle::from_point_distance(center_x, center_y, radius)?;
      let component_2d = bbox.to_component2d()?;

      let num_points_to_try = 1000;
      for _ in 0..num_points_to_try {
        let x = if random.random_bool(0.5) {
          f32::MAX.min(center_x + radius + random.random::<f64>() as f32) as f64
        } else {
          (-f32::MAX).max(center_x + radius - random.random::<f64>() as f32) as f64
        };

        let y = if random.random_bool(0.5) {
          f32::MAX.min(center_y + radius + random.random::<f64>() as f32) as f64
        } else {
          (-f32::MAX).max(center_y + radius - random.random::<f64>() as f32) as f64
        };

        let cartesian_says = component_2d.contains(x, y);
        let bbox_says = x >= bbox.min_x as f64
          && x <= bbox.max_x as f64
          && y >= bbox.min_y as f64
          && y <= bbox.max_y as f64;

        if cartesian_says && !bbox_says {
          unreachable!(
            "point was within the distance according to cartesian distance, but the bbox doesn't contain it; centerX={} centerY={} radius={} bbox: x={} to {} y={} to {} point: x={} y={}",
            center_x, center_y, radius, bbox.min_x, bbox.max_x, bbox.min_y, bbox.max_y, x, y
          );
        }
      }
    }

    Ok(())
  }
}
