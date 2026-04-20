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
use crate::core::geo::component2d::{
  Component2D, WithinRelation, contains_point, disjoint, point_in_triangle, within,
};
use crate::core::geo::edge_tree::{EdgeTree, create_tree};
use crate::core::geo::line::Line;
use crate::core::geo::xy_encoding_utils::XYEncodingUtils;
use crate::core::geo::xy_line::XYLine;
use crate::core::index::point_values::Relation;
use crate::core::util::error::lucene_error::Result;

/// 2D geo line implementation represented as a balanced interval tree of edges.
///
/// `Line2D` construction takes `O(n log n)` time for sorting and tree construction.
/// [`Self::relate`] is `O(n)`, but for most practical lines is much faster than brute force.
pub struct Line2D {
  /// Minimum Y of this geometry's bounding box area.
  min_y: f64,

  /// Maximum Y of this geometry's bounding box area.
  max_y: f64,

  /// Minimum X of this geometry's bounding box area.
  min_x: f64,

  /// Maximum X of this geometry's bounding box area.
  max_x: f64,

  /// Lines represented as a 2-d interval tree.
  tree: EdgeTree,
}

impl Line2D {
  fn from_line(line: &Line) -> Result<Self> {
    Ok(Self {
      min_y: line.min_lat,
      max_y: line.max_lat,
      min_x: line.min_lon,
      max_x: line.max_lon,
      tree: create_tree(line.get_lons(), line.get_lats())?,
    })
  }

  fn from_xy_line(line: &XYLine) -> Result<Self> {
    Ok(Self {
      min_y: line.min_y as f64,
      max_y: line.max_y as f64,
      min_x: line.min_x as f64,
      max_x: line.max_x as f64,
      tree: create_tree(
        &XYEncodingUtils::float_array_to_double_array(line.get_xs()),
        &XYEncodingUtils::float_array_to_double_array(line.get_ys()),
      )?,
    })
  }
}
impl Component2D for Line2D {
  fn get_min_x(&self) -> f64 {
    self.min_x
  }

  fn get_max_x(&self) -> f64 {
    self.max_x
  }

  fn get_min_y(&self) -> f64 {
    self.min_y
  }

  fn get_max_y(&self) -> f64 {
    self.max_y
  }

  fn contains(&self, x: f64, y: f64) -> bool {
    if contains_point(x, y, self.min_x, self.max_x, self.min_y, self.max_y) {
      return self.tree.is_point_online(x, y);
    }
    false
  }

  fn relate(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> Result<Relation> {
    if disjoint(
      self.min_x, self.max_x, self.min_y, self.max_y, min_x, max_x, min_y, max_y,
    ) {
      return Ok(Relation::CellOutsideQuery);
    }
    if within(
      self.min_x, self.max_x, self.min_y, self.max_y, min_x, max_x, min_y, max_y,
    ) {
      return Ok(Relation::CellCrossesQuery);
    }
    if self.tree.crosses_box(min_x, max_x, min_y, max_y, true) {
      return Ok(Relation::CellCrossesQuery);
    }
    Ok(Relation::CellOutsideQuery)
  }

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
  ) -> bool {
    if disjoint(
      self.min_x, self.max_x, self.min_y, self.max_y, min_x, max_x, min_y, max_y,
    ) {
      return false;
    }
    self
      .tree
      .crosses_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, true)
  }

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
  ) -> bool {
    if disjoint(
      self.min_x, self.max_x, self.min_y, self.max_y, min_x, max_x, min_y, max_y,
    ) {
      return false;
    }
    point_in_triangle(
      min_x,
      max_x,
      min_y,
      max_y,
      self.tree.x1,
      self.tree.y1,
      a_x,
      a_y,
      b_x,
      b_y,
      c_x,
      c_y,
    ) || self.tree.crosses_triangle(
      min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, c_x, c_y, true,
    )
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
    false
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
    false
  }

  fn within_point(&self, x: f64, y: f64) -> Result<WithinRelation> {
    Ok(if self.contains(x, y) {
      WithinRelation::NotWithin
    } else {
      WithinRelation::Disjoint
    })
  }

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
  ) -> Result<WithinRelation> {
    if ab && self.intersects_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y) {
      return Ok(WithinRelation::NotWithin);
    }
    Ok(WithinRelation::Disjoint)
  }

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
  ) -> Result<WithinRelation> {
    if disjoint(
      self.min_x, self.max_x, self.min_y, self.max_y, min_x, max_x, min_y, max_y,
    ) {
      return Ok(WithinRelation::Disjoint);
    }

    let mut relation = WithinRelation::Disjoint;

    if self
      .tree
      .crosses_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, true)
    {
      if ab {
        return Ok(WithinRelation::NotWithin);
      } else {
        relation = WithinRelation::Candidate;
      }
    }

    if self
      .tree
      .crosses_line(min_x, max_x, min_y, max_y, b_x, b_y, c_x, c_y, true)
    {
      if bc {
        return Ok(WithinRelation::NotWithin);
      } else {
        relation = WithinRelation::Candidate;
      }
    }

    if self
      .tree
      .crosses_line(min_x, max_x, min_y, max_y, c_x, c_y, a_x, a_y, true)
    {
      if ca {
        return Ok(WithinRelation::NotWithin);
      } else {
        relation = WithinRelation::Candidate;
      }
    }

    if relation == WithinRelation::Candidate {
      return Ok(WithinRelation::Candidate);
    }

    if point_in_triangle(
      min_x,
      max_x,
      min_y,
      max_y,
      self.tree.x1,
      self.tree.y1,
      a_x,
      a_y,
      b_x,
      b_y,
      c_x,
      c_y,
    ) {
      return Ok(WithinRelation::Candidate);
    }

    Ok(relation)
  }
}

/// create a Line2D from the provided LatLon Linestring
pub(crate) fn create_from_line(line: &Line) -> Result<Line2D> {
  Line2D::from_line(line)
}
/// create a Line2D from the provided XY Linestring
pub(crate) fn create_from_xy_line(xy_line: &XYLine) -> Result<Line2D> {
  Line2D::from_xy_line(xy_line)
}
#[cfg(test)]
mod tests {
  use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;

  use super::*;
  use crate::test::core::geo::geo_test_util::GeoTestUtil;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
  #[allow(dead_code)] // for quick serach
  struct TestLine2D;

  #[test]
  fn test_triangle_disjoint() -> Result<()> {
    let line = Line::new(vec![0.0, 1.0, 2.0, 3.0], vec![0.0, 0.0, 2.0, 2.0])?;
    let line2d = create_from_line(&line)?;
    let ax = GeoEncodingUtils::encode_longitude(4.0)? as f64;
    let ay = GeoEncodingUtils::encode_latitude(4.0)? as f64;
    let bx = GeoEncodingUtils::encode_longitude(5.0)? as f64;
    let by = GeoEncodingUtils::encode_latitude(5.0)? as f64;
    let cx = GeoEncodingUtils::encode_longitude(5.0)? as f64;
    let cy = GeoEncodingUtils::encode_latitude(4.0)? as f64;
    assert!(!line2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(!line2d.intersects_line_values(ax, ay, bx, by));
    assert!(!line2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(!line2d.contains_line_values(ax, ay, bx, by));
    assert_eq!(
      WithinRelation::Disjoint,
      line2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
    );
    Ok(())
  }

  #[test]
  fn test_triangle_intersects() -> Result<()> {
    let line = Line::new(vec![0.5, 0.0, 1.0, 2.0, 3.0], vec![0.5, 0.0, 0.0, 2.0, 2.0])?;
    let line2d = create_from_line(&line)?;
    let ax = GeoEncodingUtils::encode_longitude(0.0)? as f64;
    let ay = GeoEncodingUtils::encode_latitude(0.0)? as f64;
    let bx = GeoEncodingUtils::encode_longitude(1.0)? as f64;
    let by = GeoEncodingUtils::encode_latitude(0.0)? as f64;
    let cx = GeoEncodingUtils::encode_longitude(0.0)? as f64;
    let cy = GeoEncodingUtils::encode_latitude(1.0)? as f64;
    assert!(line2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(line2d.intersects_line_values(ax, ay, bx, by));
    assert!(!line2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(!line2d.contains_line_values(ax, ay, bx, by));
    assert_eq!(
      WithinRelation::NotWithin,
      line2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
    );
    Ok(())
  }

  #[test]
  fn test_triangle_contains() -> Result<()> {
    let line = Line::new(vec![0.5, 0.0, 1.0, 2.0, 3.0], vec![0.5, 0.0, 0.0, 2.0, 2.0])?;
    let line2d = create_from_line(&line)?;
    let ax = GeoEncodingUtils::encode_longitude(-10.0)? as f64;
    let ay = GeoEncodingUtils::encode_latitude(-10.0)? as f64;
    let bx = GeoEncodingUtils::encode_longitude(4.0)? as f64;
    let by = GeoEncodingUtils::encode_latitude(-10.0)? as f64;
    let cx = GeoEncodingUtils::encode_longitude(4.0)? as f64;
    let cy = GeoEncodingUtils::encode_latitude(30.0)? as f64;
    assert!(line2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(!line2d.intersects_line_values(bx, by, cx, cy));
    assert!(!line2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
    assert!(!line2d.contains_line_values(bx, by, cx, cy));
    assert_eq!(
      WithinRelation::Candidate,
      line2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
    );
    Ok(())
  }

  #[test]
  fn test_random_triangles() -> Result<()> {
    let mut random = random();
    let line = GeoTestUtil::next_line(&mut random)?;
    let line2d = create_from_line(&line)?;

    for _ in 0..100 {
      let ax = GeoTestUtil::next_longitude(&mut random);
      let ay = GeoTestUtil::next_latitude(&mut random);
      let bx = GeoTestUtil::next_longitude(&mut random);
      let by = GeoTestUtil::next_latitude(&mut random);
      let cx = GeoTestUtil::next_longitude(&mut random);
      let cy = GeoTestUtil::next_latitude(&mut random);

      let t_min_x = ax.min(bx).min(cx);
      let t_max_x = ax.max(bx).max(cx);
      let t_min_y = ay.min(by).min(cy);
      let t_max_y = ay.max(by).max(cy);

      let r = line2d.relate(t_min_x, t_max_x, t_min_y, t_max_y)?;
      if r == Relation::CellOutsideQuery {
        assert!(!line2d.intersects_triangle_values(ax, ay, bx, by, cx, cy));
        assert!(!line2d.intersects_line_values(ax, ay, bx, by));
        assert!(!line2d.contains_triangle_values(ax, ay, bx, by, cx, cy));
        assert!(!line2d.contains_line_values(ax, ay, bx, by));
        assert_eq!(
          WithinRelation::Disjoint,
          line2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
        );
      } else if line2d.contains_triangle_values(ax, ay, bx, by, cx, cy) {
        assert_ne!(
          WithinRelation::Candidate,
          line2d.within_triangle_values(ax, ay, true, bx, by, true, cx, cy, true)?
        );
      }
    }
    Ok(())
  }
}
