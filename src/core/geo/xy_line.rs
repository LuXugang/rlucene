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
use crate::core::geo::line2d::{Line2D, create_from_xy_line};
use crate::core::geo::xy_encoding_utils::XYEncodingUtils;
use crate::core::geo::xy_geometry::XYGeometry;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
/// Represents a line in cartesian space. You can construct the line directly with `float[]`,
/// `float[]` x, y arrays coordinates.
#[derive(Debug, Clone)]
pub struct XYLine {
  /// Array of x coordinates.
  x: Vec<f32>,

  /// Array of y coordinates.
  y: Vec<f32>,

  /// Minimum x of this line's bounding box.
  pub min_x: f32,

  /// Maximum x of this line's bounding box.
  pub max_x: f32,

  /// Minimum y of this line's bounding box.
  pub min_y: f32,

  /// Maximum y of this line's bounding box.
  pub max_y: f32,
}

impl XYLine {
  /// Creates a new `XYLine` from the supplied X/Y array.
  pub fn new(x: Vec<f32>, y: Vec<f32>) -> Result<Self> {
    if x.len() != y.len() {
      return Err(LuceneError::illegal_argument(
        "x and y must be equal length",
      ));
    }
    if x.len() < 2 {
      return Err(LuceneError::illegal_argument(
        "at least 2 line points required",
      ));
    }

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = -f32::MAX;
    let mut max_y = -f32::MAX;
    for i in 0..x.len() {
      min_x = XYEncodingUtils::check_val(x[i])?.min(min_x);
      min_y = XYEncodingUtils::check_val(y[i])?.min(min_y);
      max_x = x[i].max(max_x);
      max_y = y[i].max(max_y);
    }

    Ok(Self {
      x,
      y,
      min_x,
      max_x,
      min_y,
      max_y,
    })
  }

  /// Returns the number of vertex points.
  pub fn num_points(&self) -> usize {
    self.x.len()
  }

  /// Returns x value at given index.
  pub fn get_x(&self, vertex: usize) -> f32 {
    self.x[vertex]
  }

  /// Returns y value at given index.
  pub fn get_y(&self, vertex: usize) -> f32 {
    self.y[vertex]
  }

  /// Returns a copy of the internal x array.
  pub fn get_xs(&self) -> &[f32] {
    self.x.as_slice()
  }

  /// Returns a copy of the internal y array.
  pub fn get_ys(&self) -> &[f32] {
    self.y.as_slice()
  }
}

impl Geometry for XYLine {
  type Component2D = Line2D;

  fn to_component2d(&self) -> Result<Self::Component2D> {
    create_from_xy_line(self)
  }
}

impl XYGeometry for XYLine {}

impl PartialEq for XYLine {
  fn eq(&self, other: &Self) -> bool {
    self.x == other.x && self.y == other.y
  }
}

impl Eq for XYLine {}

impl Hash for XYLine {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for x in &self.x {
      x.to_bits().hash(state);
    }
    for y in &self.y {
      y.to_bits().hash(state);
    }
  }
}

impl Display for XYLine {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "XYLine(")?;
    for i in 0..self.x.len() {
      write!(f, "[{}, {}]", self.x[i], self.y[i])?;
    }
    write!(f, ")")
  }
}
#[cfg(test)]
mod test_xy_line {
  use super::*;
  use crate::test::core::geo::shape_test_util::ShapeTestUtil;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
  #[allow(dead_code)] // for quick search
  struct TestXYLine;
  #[test]
  fn test_line_null_xs() {
    // this test is not required in Rust Lucene
  }

  #[test]
  fn test_polygon_null_ys() {
    // this test is not required in Rust Lucene
  }

  #[test]
  fn test_line_enough_points() {
    let err = XYLine::new(vec![18.0], vec![-66.0]);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(e.to_string().contains("at least 2 line points required"));
    }
  }

  #[test]
  fn test_lines_bogus() {
    let err = XYLine::new(
      vec![18.0, 18.0, 19.0, 19.0],
      vec![-66.0, -65.0, -65.0, -66.0, -66.0],
    );
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(e.to_string().contains("must be equal length"));
    }
  }

  #[test]
  fn test_line_nan() {
    let err = XYLine::new(
      vec![18.0, 18.0, 19.0, f32::NAN, 18.0],
      vec![-66.0, -65.0, -65.0, -66.0, -66.0],
    );
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(e.to_string().contains("invalid value NaN"));
    }
  }

  #[test]
  fn test_line_positive_infinite() {
    let err = XYLine::new(
      vec![18.0, 18.0, 19.0, 19.0, 18.0],
      vec![-66.0, f32::INFINITY, -65.0, -66.0, -66.0],
    );
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(
        e.to_string().contains("invalid value inf") || e.to_string().contains("invalid value Inf")
      );
    }
  }

  #[test]
  fn test_line_negative_infinite() {
    let err = XYLine::new(
      vec![18.0, 18.0, 19.0, 19.0, 18.0],
      vec![-66.0, -65.0, -65.0, f32::NEG_INFINITY, -66.0],
    );
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(
        e.to_string().contains("invalid value -inf")
          || e.to_string().contains("invalid value -Inf")
      );
    }
  }

  #[test]
  fn test_equals_and_hash_code() -> Result<()> {
    let mut random = random();
    let line = ShapeTestUtil::next_line(&mut random)?;
    let copy = XYLine::new(line.get_xs().to_vec(), line.get_ys().to_vec())?;
    assert_eq!(line, copy);

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher1 = DefaultHasher::new();
    line.hash(&mut hasher1);
    let hash1 = hasher1.finish();

    let mut hasher2 = DefaultHasher::new();
    copy.hash(&mut hasher2);
    let hash2 = hasher2.finish();

    assert_eq!(hash1, hash2);

    let other_line = ShapeTestUtil::next_line(&mut random)?;
    if line.get_xs() != other_line.get_xs() || line.get_ys() != other_line.get_ys() {
      assert_ne!(line, other_line);

      let mut hasher3 = DefaultHasher::new();
      other_line.hash(&mut hasher3);
      let hash3 = hasher3.finish();

      assert_ne!(hash1, hash3);
    } else {
      assert_eq!(line, other_line);

      let mut hasher3 = DefaultHasher::new();
      other_line.hash(&mut hasher3);
      let hash3 = hasher3.finish();

      assert_eq!(hash1, hash3);
    }

    Ok(())
  }
}
