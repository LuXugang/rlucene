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
use crate::core::geo::geo_utils::{GeoUtils, WindingOrder};
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

/// Represents a closed polygon on the earth's surface. You can either construct the Polygon directly
/// yourself with `double[]` coordinates, or use [`Polygon::from_geo_json`] if you have a
/// polygon already encoded as a [GeoJSON](http://geojson.org/geojson-spec.html) string.
///
/// NOTES:
///
/// 1. Coordinates must be in clockwise order, except for holes. Holes must be in
///    counter-clockwise order.
/// 2. The polygon must be closed: the first and last coordinates need to have the same values.
/// 3. The polygon must not be self-crossing, otherwise may result in unexpected behavior.
/// 4. All latitude/longitude values must be in decimal degrees.
/// 5. Polygons cannot cross the 180th meridian. Instead, use two polygons: one on each side.
/// 6. For more advanced GeoSpatial indexing and query operations see the `spatial-extras`
///    module
#[derive(Clone, Debug)]
pub struct Polygon {
  poly_lats: Vec<f64>,
  poly_lons: Vec<f64>,
  holes: Vec<Polygon>,

  /// minimum latitude of this polygon's bounding box area
  pub min_lat: f64,

  /// maximum latitude of this polygon's bounding box area
  pub max_lat: f64,

  /// minimum longitude of this polygon's bounding box area
  pub min_lon: f64,

  /// maximum longitude of this polygon's bounding box area
  pub max_lon: f64,

  /// winding order of the vertices
  winding_order: WindingOrder,
}

impl Polygon {
  /// Creates a new Polygon from the supplied latitude/longitude array, and optionally any holes.
  pub fn new(poly_lats: Vec<f64>, poly_lons: Vec<f64>, holes: Vec<Polygon>) -> Result<Self> {
    if poly_lats.len() != poly_lons.len() {
      return Err(LuceneError::illegal_argument(
        "polyLats and polyLons must be equal length",
      ));
    }
    if poly_lats.len() < 4 {
      return Err(LuceneError::illegal_argument(
        "at least 4 polygon points required",
      ));
    }
    if poly_lats[0] != poly_lats[poly_lats.len() - 1] {
      return Err(LuceneError::illegal_argument(format!(
        "first and last points of the polygon must be the same (it must close itself): polyLats[0]={} polyLats[{}]={}",
        poly_lats[0],
        poly_lats.len() - 1,
        poly_lats[poly_lats.len() - 1]
      )));
    }
    if poly_lons[0] != poly_lons[poly_lons.len() - 1] {
      return Err(LuceneError::illegal_argument(format!(
        "first and last points of the polygon must be the same (it must close itself): polyLons[0]={} polyLons[{}]={}",
        poly_lons[0],
        poly_lons.len() - 1,
        poly_lons[poly_lons.len() - 1]
      )));
    }

    for i in 0..poly_lats.len() {
      GeoUtils::check_latitude(poly_lats[i])?;
      GeoUtils::check_longitude(poly_lons[i])?;
    }

    for inner in &holes {
      if !inner.holes.is_empty() {
        return Err(LuceneError::illegal_argument(
          "holes may not contain holes: polygons may not nest.",
        ));
      }
    }

    let mut min_lat = poly_lats[0];
    let mut max_lat = poly_lats[0];
    let mut min_lon = poly_lons[0];
    let mut max_lon = poly_lons[0];

    let mut winding_sum: f64 = 0f64;
    let num_pts = poly_lats.len() - 1;
    for i in 1..num_pts {
      let j = i - 1;
      min_lat = f64::min(poly_lats[i], min_lat);
      max_lat = f64::max(poly_lats[i], max_lat);
      min_lon = f64::min(poly_lons[i], min_lon);
      max_lon = f64::max(poly_lons[i], max_lon);
      winding_sum += (poly_lons[j] - poly_lons[num_pts]) * (poly_lats[i] - poly_lats[num_pts])
        - (poly_lats[j] - poly_lats[num_pts]) * (poly_lons[i] - poly_lons[num_pts]);
    }

    let winding_order = if winding_sum < 0.0 {
      WindingOrder::Ccw
    } else {
      WindingOrder::Cw
    };

    Ok(Self {
      poly_lats,
      poly_lons,
      holes,
      min_lat,
      max_lat,
      min_lon,
      max_lon,
      winding_order,
    })
  }
  /// returns the number of vertex points
  pub fn num_points(&self) -> usize {
    self.poly_lats.len()
  }

  /// Returns a copy of the internal latitude array
  pub fn poly_lats(&self) -> Vec<f64> {
    self.poly_lats.clone()
  }

  /// Returns latitude value at given index
  pub fn get_poly_lat(&self, vertex: usize) -> f64 {
    self.poly_lats[vertex]
  }

  /// Returns a copy of the internal longitude array
  pub fn get_poly_lons(&self) -> Vec<f64> {
    self.poly_lons.clone()
  }

  /// Returns longitude value at given index
  pub fn get_poly_lon(&self, vertex: usize) -> f64 {
    self.poly_lons[vertex]
  }

  /// Returns a copy of the internal holes array
  pub fn get_holes(&self) -> Vec<Polygon> {
    self.holes.clone()
  }

  fn get_hole(&self, i: usize) -> &Polygon {
    &self.holes[i]
  }
  /// Returns the winding order (CW, COLINEAR, CCW) for the polygon shell
  pub fn winding_order(&self) -> WindingOrder {
    self.winding_order
  }

  /// returns the number of holes for the polygon
  pub fn num_holes(&self) -> usize {
    self.holes.len()
  }

  pub fn vertices_to_geo_json(lats: &[f64], lons: &[f64]) -> String {
    let mut s = String::new();
    s.push('[');
    for i in 0..lats.len() {
      s.push_str(&format!("[{}, {}]", lons[i], lats[i]));
      if i != lats.len() - 1 {
        s.push_str(", ");
      }
    }
    s.push(']');
    s
  }
}
impl Display for Polygon {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "Polygon")?;
    for i in 0..self.poly_lats.len() {
      write!(f, "[{}, {}] ", self.poly_lats[i], self.poly_lons[i])?;
    }
    if !self.holes.is_empty() {
      write!(f, ", holes={:?}", self.holes)?;
    }
    Ok(())
  }
}
