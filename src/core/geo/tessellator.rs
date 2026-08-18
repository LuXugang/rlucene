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
#![allow(dead_code)] // Java Tessellator internals are retained while LatLonShape/XYShape tessellation entry points are not yet migrated.

use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::geo::point::Point;
use crate::core::geo::xy_encoding_utils::XYEncodingUtils;
use crate::core::index::index_reader::Identity;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt;
use std::sync::Arc;

pub struct Tessellator;
impl Tessellator {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
  Init,
  Cure,
  Split,
}

impl State {
  fn as_str(&self) -> &'static str {
    match self {
      State::Init => "INIT",
      State::Cure => "CURE",
      State::Split => "SPLIT",
    }
  }
}

pub const MONITOR_FAILED: &str = "FAILED";
pub const MONITOR_COMPLETED: &str = "COMPLETED";

/// Determines if two point vertices are equal.
fn is_vertex_equals(a: &Node, b: &Node) -> bool {
  is_vertex_equals_xy(a, b.get_x(), b.get_y())
}

/// Determines if two point vertices are equal.
fn is_vertex_equals_xy(a: &Node, x: f64, y: f64) -> bool {
  a.get_x() == x && a.get_y() == y
}

/// Compute signed area of triangle, negative means convex angle and positive
/// reflex angle.
fn area(a_x: f64, a_y: f64, b_x: f64, b_y: f64, c_x: f64, c_y: f64) -> f64 {
  (b_y - a_y) * (c_x - b_x) - (b_x - a_x) * (c_y - b_y)
}

/// Compute whether point is in a candidate ear.
#[allow(clippy::too_many_arguments)]
fn point_in_ear(x: f64, y: f64, ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> bool {
  (cx - x) * (ay - y) - (ax - x) * (cy - y) >= 0.0
    && (ax - x) * (by - y) - (bx - x) * (ay - y) >= 0.0
    && (bx - x) * (cy - y) - (cx - x) * (by - y) >= 0.0
}

/// Implementations of this trait will receive calls with internal data at each
/// step of the triangulation algorithm.
///
/// This is useful for debugging complex cases, as well as gaining insight into
/// the way the algorithm works. Data provided includes a status string
/// containing the current mode, list of points representing the current
/// linked-list of internal nodes used for triangulation, and a list of triangles
/// so far created by the algorithm.
pub trait Monitor {
  /// Each loop of the main earclip algorithm will call this with the current state.
  fn current_state(&mut self, status: &str, points: Option<&[Point]>, tessellation: &[Triangle]);

  /// When a new polygon split is entered for mode=SPLIT, this is called.
  fn start_split(&mut self, status: &str, left_polygon: &[Point], right_polygon: &[Point]);

  /// When a polygon split is completed, this is called.
  fn end_split(&mut self, status: &str);
}

fn get_points(start: &Node) -> Result<Vec<Point>> {
  let mut node = start;
  let mut points = Vec::new();

  loop {
    points.push(Point::new(node.get_y(), node.get_x())?);

    node = node
      .next
      .as_deref()
      .ok_or_else(|| LuceneError::illegal_state("Invalid polygon node list"))?;
    if node.id == start.id {
      return Ok(points);
    }
  }
}

fn notify_monitor_split<T>(
  depth: i32,
  monitor: Option<&mut T>,
  search_node: Option<&Node>,
  diagonal_node: Option<&Node>,
) -> Result<()>
where
  T: Monitor,
{
  if let Some(monitor) = monitor {
    let search_node =
      search_node.ok_or_else(|| LuceneError::illegal_state("Invalid split provided to monitor"))?;
    let diagonal_node = diagonal_node
      .ok_or_else(|| LuceneError::illegal_state("Invalid split provided to monitor"))?;
    monitor.start_split(
      &format!("SPLIT[{depth}]"),
      &get_points(search_node)?,
      &get_points(diagonal_node)?,
    );
  }
  Ok(())
}

fn notify_monitor_split_end<T>(depth: i32, monitor: Option<&mut T>)
where
  T: Monitor,
{
  if let Some(monitor) = monitor {
    monitor.end_split(&format!("SPLIT[{depth}]"));
  }
}

fn notify_monitor<T>(
  state: State,
  depth: i32,
  monitor: Option<&mut T>,
  start: Option<&Node>,
  tessellation: &[Triangle],
) -> Result<()>
where
  T: Monitor,
{
  if monitor.is_some() {
    let status = if depth == 0 {
      state.as_str().to_string()
    } else {
      format!("{}[{depth}]", state.as_str())
    };
    notify_monitor_status(&status, monitor, start, tessellation)?;
  }
  Ok(())
}

fn notify_monitor_status<T>(
  status: &str,
  monitor: Option<&mut T>,
  start: Option<&Node>,
  tessellation: &[Triangle],
) -> Result<()>
where
  T: Monitor,
{
  if let Some(monitor) = monitor {
    if let Some(start) = start {
      monitor.current_state(status, Some(&get_points(start)?), tessellation);
    } else {
      monitor.current_state(status, None, tessellation);
    }
  }
  Ok(())
}

/// Circular Doubly-linked list used for polygon coordinates
#[derive(Clone)]
pub(crate) struct Node {
  pub(crate) id: Identity,
  // node index in the linked list
  pub(crate) idx: i32,
  // vertex index in the polygon
  pub(crate) vrtx_idx: usize,
  // reference to the polygon for lat/lon values;
  pub(crate) poly_x: Arc<Vec<f64>>,
  pub(crate) poly_y: Arc<Vec<f64>>,
  // encoded x value
  pub(crate) x: i32,
  // encoded y value
  pub(crate) y: i32,
  // morton code for sorting
  pub(crate) morton: i64,

  // previous node
  pub(crate) previous: Option<Box<Node>>,
  // next node
  pub(crate) next: Option<Box<Node>>,
  // previous z node
  pub(crate) previous_z: Option<Box<Node>>,
  // next z node
  pub(crate) next_z: Option<Box<Node>>,
  // if the edge from this node to the next node is part of the polygon edges
  pub(crate) is_next_edge_from_polygon: bool,
}

impl Node {
  pub(crate) fn new(
    x: Arc<Vec<f64>>,
    y: Arc<Vec<f64>>,
    index: i32,
    vertex_index: usize,
    is_geo: bool,
  ) -> Result<Node> {
    let encoded_y = if is_geo {
      GeoEncodingUtils::encode_latitude(y[vertex_index])?
    } else {
      XYEncodingUtils::encode(y[vertex_index] as f32)?
    };

    let encoded_x = if is_geo {
      GeoEncodingUtils::encode_longitude(x[vertex_index])?
    } else {
      XYEncodingUtils::encode(x[vertex_index] as f32)?
    };

    let morton = BitUtil::interleave(
      encoded_x ^ 0x8000_0000u32 as i32,
      encoded_y ^ 0x8000_0000u32 as i32,
    );

    Ok(Self {
      id: Identity::new(),
      idx: index,
      vrtx_idx: vertex_index,
      poly_x: x,
      poly_y: y,
      x: encoded_x,
      y: encoded_y,
      morton,
      previous: None,
      next: None,
      previous_z: None,
      next_z: None,
      is_next_edge_from_polygon: true,
    })
  }

  /// Creates a deep copy.
  pub(crate) fn copy_from(other: &Node) -> Self {
    Self {
      id: Identity::new(),
      idx: other.idx,
      vrtx_idx: other.vrtx_idx,
      poly_x: other.poly_x.clone(),
      poly_y: other.poly_y.clone(),
      morton: other.morton,
      x: other.x,
      y: other.y,
      previous: other.previous.clone(),
      next: other.next.clone(),
      previous_z: other.previous_z.clone(),
      next_z: other.next_z.clone(),
      is_next_edge_from_polygon: other.is_next_edge_from_polygon,
    }
  }

  /// get the x value
  pub fn get_x(&self) -> f64 {
    self.poly_x[self.vrtx_idx]
  }

  /// get the y value
  pub fn get_y(&self) -> f64 {
    self.poly_y[self.vrtx_idx]
  }
}

impl fmt::Display for Node {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if let Some(previous) = &self.previous {
      write!(f, "{} <- ", previous.idx)?;
    } else {
      write!(f, "||-")?;
    }

    write!(f, "{}", self.idx)?;

    if let Some(next) = &self.next {
      write!(f, " -> {}", next.idx)
    } else {
      write!(f, " -||")
    }
  }
}

/// Triangle in the tessellated mesh
pub struct Triangle {
  vertex: [Node; 3],
  edge_from_polygon: [bool; 3],
}

impl Triangle {
  fn new(
    a: Node,
    is_ab_from_polygon: bool,
    b: Node,
    is_bc_from_polygon: bool,
    c: Node,
    is_ca_from_polygon: bool,
  ) -> Self {
    Self {
      vertex: [a, b, c],
      edge_from_polygon: [is_ab_from_polygon, is_bc_from_polygon, is_ca_from_polygon],
    }
  }

  /// get quantized x value for the given vertex
  pub fn get_encoded_x(&self, vertex: usize) -> i32 {
    self.vertex[vertex].x
  }

  /// get quantized y value for the given vertex
  pub fn get_encoded_y(&self, vertex: usize) -> i32 {
    self.vertex[vertex].y
  }

  /// get y value for the given vertex
  pub fn get_y(&self, vertex: usize) -> f64 {
    self.vertex[vertex].get_y()
  }

  /// get x value for the given vertex
  pub fn get_x(&self, vertex: usize) -> f64 {
    self.vertex[vertex].get_x()
  }

  /// get if edge is shared with the polygon for the given edge
  pub fn is_edge_from_polygon(&self, start_vertex: usize) -> bool {
    self.edge_from_polygon[start_vertex]
  }
}

impl std::fmt::Display for Triangle {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}, {} [{}] {}, {} [{}] {}, {} [{}]",
      self.vertex[0].x,
      self.vertex[0].y,
      self.edge_from_polygon[0],
      self.vertex[1].x,
      self.vertex[1].y,
      self.edge_from_polygon[1],
      self.vertex[2].x,
      self.vertex[2].y,
      self.edge_from_polygon[2]
    )
  }
}
