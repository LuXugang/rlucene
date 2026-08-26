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
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::cmp::Ordering;

#[derive(Default)]
pub struct ComponentTree<T> {
  /// minimum Y of this geometry's bounding box area
  min_y: f64,

  /// maximum Y of this geometry's bounding box area
  max_y: f64,

  /// minimum X of this geometry's bounding box area
  min_x: f64,

  /// maximum X of this geometry's bounding box area
  max_x: f64,

  /// child components, or None. Note internal nodes might mot have
  /// a consistent bounding box. Internal nodes should not be accessed
  /// outside this type.
  left: Option<Box<ComponentTree<T>>>,
  right: Option<Box<ComponentTree<T>>>,

  /// which dimension was this node split on
  ///
  split_x: bool,

  /// root node of edge tree
  component: T,
}
impl<T> ComponentTree<T>
where
  T: Component2D,
{
  pub(crate) fn new(component: T, split_x: bool) -> Self {
    let min_y = component.get_min_y();
    let max_y = component.get_max_y();
    let min_x = component.get_min_x();
    let max_x = component.get_max_x();
    Self {
      min_y,
      max_y,
      min_x,
      max_x,
      left: None,
      right: None,
      split_x,
      component,
    }
  }
}
impl<T> Component2D for ComponentTree<T>
where
  T: Component2D,
{
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
    if y <= self.max_y && x <= self.max_x {
      if self.component.contains(x, y) {
        return true;
      }
      if let Some(left) = &self.left
        && left.contains(x, y)
      {
        return true;
      }
      if let Some(right) = &self.right
        && ((!self.split_x && y >= self.component.get_min_y())
          || (self.split_x && x >= self.component.get_min_x()))
        && right.contains(x, y)
      {
        return true;
      }
    }
    false
  }

  fn relate(&self, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> Result<Relation> {
    if min_y <= self.max_y && min_x <= self.max_x {
      let relation = self.component.relate(min_x, max_x, min_y, max_y)?;
      if relation != Relation::CellOutsideQuery {
        return Ok(relation);
      }
      if let Some(left) = &self.left {
        let relation = left.relate(min_x, max_x, min_y, max_y)?;
        if relation != Relation::CellOutsideQuery {
          return Ok(relation);
        }
      }
      if let Some(right) = &self.right
        && ((!self.split_x && max_y >= self.component.get_min_y())
          || (self.split_x && max_x >= self.component.get_min_x()))
      {
        let relation = right.relate(min_x, max_x, min_y, max_y)?;
        if relation != Relation::CellOutsideQuery {
          return Ok(relation);
        }
      }
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
    if min_y <= self.max_y && min_x <= self.max_x {
      if self
        .component
        .intersects_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y)
      {
        return true;
      }
      if let Some(left) = &self.left
        && left.intersects_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y)
      {
        return true;
      }
      if let Some(right) = &self.right
        && ((!self.split_x && max_y >= self.component.get_min_y())
          || (self.split_x && max_x >= self.component.get_min_x()))
        && right.intersects_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y)
      {
        return true;
      }
    }
    false
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
    if min_y <= self.max_y && min_x <= self.max_x {
      if self
        .component
        .intersects_triangle(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, c_x, c_y)
      {
        return true;
      }
      if let Some(left) = &self.left
        && left.intersects_triangle(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, c_x, c_y)
      {
        return true;
      }
      if let Some(right) = &self.right
        && ((!self.split_x && max_y >= self.component.get_min_y())
          || (self.split_x && max_x >= self.component.get_min_x()))
        && right.intersects_triangle(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, c_x, c_y)
      {
        return true;
      }
    }
    false
  }

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
  ) -> bool {
    if min_y <= self.max_y && min_x <= self.max_x {
      if self
        .component
        .contains_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y)
      {
        return true;
      }
      if let Some(left) = &self.left
        && left.contains_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y)
      {
        return true;
      }
      if let Some(right) = &self.right
        && ((!self.split_x && max_y >= self.component.get_min_y())
          || (self.split_x && max_x >= self.component.get_min_x()))
        && right.contains_line(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y)
      {
        return true;
      }
    }
    false
  }

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
  ) -> bool {
    if min_y <= self.max_y && min_x <= self.max_x {
      if self
        .component
        .contains_triangle(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, c_x, c_y)
      {
        return true;
      }
      if let Some(left) = &self.left
        && left.contains_triangle(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, c_x, c_y)
      {
        return true;
      }
      if let Some(right) = &self.right
        && ((!self.split_x && max_y >= self.component.get_min_y())
          || (self.split_x && max_x >= self.component.get_min_x()))
        && right.contains_triangle(min_x, max_x, min_y, max_y, a_x, a_y, b_x, b_y, c_x, c_y)
      {
        return true;
      }
    }
    false
  }

  fn within_point(&self, x: f64, y: f64) -> Result<WithinRelation> {
    if self.left.is_some() || self.right.is_some() {
      return Err(LuceneError::illegal_argument(
        "within_point is not supported for shapes with more than one component",
      ));
    }
    self.component.within_point(x, y)
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
    if self.left.is_some() || self.right.is_some() {
      return Err(LuceneError::illegal_argument(
        "within_line is not supported for shapes with more than one component",
      ));
    }
    self
      .component
      .within_line(min_x, max_x, min_y, max_y, a_x, a_y, ab, b_x, b_y)
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
    if self.left.is_some() || self.right.is_some() {
      return Err(LuceneError::illegal_argument(
        "within_triangle is not supported for shapes with more than one component",
      ));
    }
    self.component.within_triangle(
      min_x, max_x, min_y, max_y, a_x, a_y, ab, b_x, b_y, bc, c_x, c_y, ca,
    )
  }
}
pub(crate) fn create<T>(mut components: Vec<T>) -> Result<ComponentTree<T>>
where
  T: Component2D,
{
  if components.is_empty() {
    return Err(LuceneError::illegal_argument(
      "components must not be empty",
    ));
  }

  if components.len() == 1 {
    return Ok(ComponentTree::new(components.remove(0), false));
  }

  let mut min_y = f64::INFINITY;
  let mut min_x = f64::INFINITY;
  let mut component_slots: Vec<Option<T>> = components
    .into_iter()
    .map(|component| {
      min_y = min_y.min(component.get_min_y());
      min_x = min_x.min(component.get_min_x());
      Some(component)
    })
    .collect();

  let high = component_slots.len() - 1;
  let mut root = create_tree(&mut component_slots, 0, high, false)?
    .ok_or_else(|| LuceneError::illegal_argument("failed to build component tree"))?;
  root.min_y = min_y;
  root.min_x = min_x;
  Ok(root)
}

fn create_tree<T>(
  components: &mut [Option<T>],
  low: usize,
  high: usize,
  split_x: bool,
) -> Result<Option<ComponentTree<T>>>
where
  T: Component2D,
{
  if low > high {
    return Ok(None);
  }

  let mid = (low + high) >> 1;
  if low < high {
    let relative_mid = mid - low;
    components[low..=high].select_nth_unstable_by(relative_mid, |left, right| {
      compare_components(
        left
          .as_ref()
          .expect("component missing during tree construction"),
        right
          .as_ref()
          .expect("component missing during tree construction"),
        split_x,
      )
    });
  }

  let component = components[mid]
    .take()
    .ok_or_else(|| LuceneError::illegal_argument("component missing during tree construction"))?;
  let mut new_node = ComponentTree::new(component, split_x);

  new_node.left = if mid == 0 {
    None
  } else {
    create_tree(components, low, mid - 1, !split_x)?.map(Box::new)
  };
  new_node.right = create_tree(components, mid + 1, high, !split_x)?.map(Box::new);

  if let Some(left) = &new_node.left {
    new_node.max_x = new_node.max_x.max(left.get_max_x());
    new_node.max_y = new_node.max_y.max(left.get_max_y());
  }
  if let Some(right) = &new_node.right {
    new_node.max_x = new_node.max_x.max(right.get_max_x());
    new_node.max_y = new_node.max_y.max(right.get_max_y());
  }

  Ok(Some(new_node))
}

fn compare_components<T>(left: &T, right: &T, split_x: bool) -> Ordering
where
  T: Component2D,
{
  if split_x {
    left
      .get_min_x()
      .total_cmp(&right.get_min_x())
      .then_with(|| left.get_max_x().total_cmp(&right.get_max_x()))
  } else {
    left
      .get_min_y()
      .total_cmp(&right.get_min_y())
      .then_with(|| left.get_max_y().total_cmp(&right.get_max_y()))
  }
}
