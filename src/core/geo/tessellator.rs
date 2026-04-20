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
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::geo::xy_encoding_utils::XYEncodingUtils;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::Result;
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
/// Circular Doubly-linked list used for polygon coordinates
#[derive(Clone)]
pub(crate) struct Node {
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

  /// simple deep copy constructor
  pub(crate) fn copy_from(other: &Node) -> Self {
    Self {
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
