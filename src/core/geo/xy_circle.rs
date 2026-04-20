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
use crate::core::geo::xy_geometry::XYGeometry;
use crate::core::util::error::lucene_error::Result;
pub struct XYCircle {
  pub x: f32,
  pub y: f32,
  pub radius: f32,
}
impl XYCircle {
  pub fn new(x: f32, y: f32, radius: f32) -> Result<Self> {
    Ok(Self { x, y, radius })
  }
  pub fn get_x(&self) -> f32 {
    self.x
  }
  pub fn get_y(&self) -> f32 {
    self.y
  }
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
