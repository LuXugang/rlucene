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
use crate::core::geo::geometry::Geometry;
use crate::core::geo::lat_lon_geometry::LatLonGeometry;
use crate::core::geo::polygon2d;
use crate::core::geo::polygon2d::Polygon2D;
use crate::core::geo::simple_geo_json_polygon_parser::SimpleGeoJSONPolygonParser;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

/// Represents a closed polygon on the earth's surface. You can either construct the Polygon directly
/// yourself with `double[]` coordinates, or use `Polygon::from_geo_json` if you have a
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
  pub fn get_poly_lats(&self) -> &[f64] {
    self.poly_lats.as_slice()
  }

  /// Returns latitude value at given index
  pub fn get_poly_lat(&self, vertex: usize) -> f64 {
    self.poly_lats[vertex]
  }

  /// Returns a copy of the internal longitude array
  pub fn get_poly_lons(&self) -> &[f64] {
    self.poly_lons.as_slice()
  }

  /// Returns longitude value at given index
  pub fn get_poly_lon(&self, vertex: usize) -> f64 {
    self.poly_lons[vertex]
  }

  /// Returns a copy of the internal holes array
  pub fn get_holes(&self) -> &[Polygon] {
    self.holes.as_slice()
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

pub fn from_geo_json(geo_json: &str) -> Result<Vec<Polygon>> {
  SimpleGeoJSONPolygonParser::new(geo_json).parse()
}
impl Geometry for Polygon {
  type Component2D = Polygon2D;

  fn to_component2d(&self) -> Result<Self::Component2D> {
    polygon2d::create_from_polygon(self)
  }
}

impl LatLonGeometry for Polygon {}
impl PartialEq for Polygon {
  fn eq(&self, other: &Self) -> bool {
    self.holes == other.holes
      && self.poly_lats == other.poly_lats
      && self.poly_lons == other.poly_lons
  }
}

impl Eq for Polygon {}

impl std::hash::Hash for Polygon {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.holes.hash(state);
    for lat in &self.poly_lats {
      lat.to_bits().hash(state);
    }
    for lon in &self.poly_lons {
      lon.to_bits().hash(state);
    }
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
#[cfg(test)]
mod test_polygon {
  use super::*;
  #[allow(dead_code)] // for quick search
  struct TestPolygon;
  #[test]
  fn test_polygon_null_poly_lats() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  fn test_polygon_null_poly_lons() -> Result<()> {
    test_not_required_in_rust_lucene!();
  }

  #[test]
  fn test_polygon_line() {
    let err = Polygon::new(vec![18.0, 18.0, 18.0], vec![-66.0, -65.0, -66.0], vec![]);
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(e.to_string().contains("at least 4 polygon points required"));
    }
  }

  #[test]
  fn test_polygon_bogus() {
    let err = Polygon::new(
      vec![18.0, 18.0, 19.0, 19.0],
      vec![-66.0, -65.0, -65.0, -66.0, -66.0],
      vec![],
    );
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(e.to_string().contains("must be equal length"));
    }
  }

  #[test]
  fn test_polygon_not_closed() {
    let err = Polygon::new(
      vec![18.0, 18.0, 19.0, 19.0, 19.0],
      vec![-66.0, -65.0, -65.0, -66.0, -67.0],
      vec![],
    );
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    if let Err(e) = err {
      assert!(e.to_string().contains("it must close itself"));
    }
  }

  #[test]
  fn test_geojson_polygon() -> Result<()> {
    let mut b = String::new();
    b.push_str("{\n");
    b.push_str("  \"type\": \"Polygon\",\n");
    b.push_str("  \"coordinates\": [\n");
    b.push_str("    [ [100.0, 0.0], [101.0, 0.0], [101.0, 1.0],\n");
    b.push_str("      [100.0, 1.0], [100.0, 0.0] ]\n");
    b.push_str("  ]\n");
    b.push_str("}\n");

    let polygons = from_geo_json(&b)?;
    assert_eq!(1, polygons.len());
    assert_eq!(
      Polygon::new(
        vec![0.0, 0.0, 1.0, 1.0, 0.0],
        vec![100.0, 101.0, 101.0, 100.0, 100.0],
        vec![],
      )?,
      polygons[0]
    );
    Ok(())
  }

  #[test]
  fn test_geojson_polygon_with_hole() -> Result<()> {
    let mut b = String::new();
    b.push_str("{\n");
    b.push_str("  \"type\": \"Polygon\",\n");
    b.push_str("  \"coordinates\": [\n");
    b.push_str("    [ [100.0, 0.0], [101.0, 0.0], [101.0, 1.0],\n");
    b.push_str("      [100.0, 1.0], [100.0, 0.0] ],\n");
    b.push_str("    [ [100.5, 0.5], [100.5, 0.75], [100.75, 0.75], [100.75, 0.5], [100.5, 0.5]]\n");
    b.push_str("  ]\n");
    b.push_str("}\n");

    let hole = Polygon::new(
      vec![0.5, 0.75, 0.75, 0.5, 0.5],
      vec![100.5, 100.5, 100.75, 100.75, 100.5],
      vec![],
    )?;
    let expected = Polygon::new(
      vec![0.0, 0.0, 1.0, 1.0, 0.0],
      vec![100.0, 101.0, 101.0, 100.0, 100.0],
      vec![hole],
    )?;
    let polygons = from_geo_json(&b)?;

    assert_eq!(1, polygons.len());
    assert_eq!(expected, polygons[0]);
    Ok(())
  }

  #[test]
  fn test_geojson_multi_polygon() -> Result<()> {
    let mut b = String::new();
    b.push_str("{\n");
    b.push_str("  \"type\": \"MultiPolygon\",\n");
    b.push_str("  \"coordinates\": [\n");
    b.push_str("    [\n");
    b.push_str("      [ [100.0, 0.0], [101.0, 0.0], [101.0, 1.0],\n");
    b.push_str("        [100.0, 1.0], [100.0, 0.0] ]\n");
    b.push_str("    ],\n");
    b.push_str("    [\n");
    b.push_str("      [ [10.0, 2.0], [11.0, 2.0], [11.0, 3.0],\n");
    b.push_str("        [10.0, 3.0], [10.0, 2.0] ]\n");
    b.push_str("    ]\n");
    b.push_str("  ],\n");
    b.push_str("}\n");

    let polygons = from_geo_json(&b)?;
    assert_eq!(2, polygons.len());
    assert_eq!(
      Polygon::new(
        vec![0.0, 0.0, 1.0, 1.0, 0.0],
        vec![100.0, 101.0, 101.0, 100.0, 100.0],
        vec![],
      )?,
      polygons[0]
    );
    assert_eq!(
      Polygon::new(
        vec![2.0, 2.0, 3.0, 3.0, 2.0],
        vec![10.0, 11.0, 11.0, 10.0, 10.0],
        vec![],
      )?,
      polygons[1]
    );
    Ok(())
  }

  #[test]
  fn test_geojson_type_comes_last() -> Result<()> {
    let mut b = String::new();
    b.push_str("{\n");
    b.push_str("  \"coordinates\": [\n");
    b.push_str("    [ [100.0, 0.0], [101.0, 0.0], [101.0, 1.0],\n");
    b.push_str("      [100.0, 1.0], [100.0, 0.0] ]\n");
    b.push_str("  ],\n");
    b.push_str("  \"type\": \"Polygon\",\n");
    b.push_str("}\n");

    let polygons = from_geo_json(&b)?;
    assert_eq!(1, polygons.len());
    assert_eq!(
      Polygon::new(
        vec![0.0, 0.0, 1.0, 1.0, 0.0],
        vec![100.0, 101.0, 101.0, 100.0, 100.0],
        vec![],
      )?,
      polygons[0]
    );
    Ok(())
  }

  #[test]
  fn test_geojson_polygon_feature() -> Result<()> {
    let mut b = String::new();
    b.push_str("{ \"type\": \"Feature\",\n");
    b.push_str("  \"geometry\": {\n");
    b.push_str("    \"type\": \"Polygon\",\n");
    b.push_str("    \"coordinates\": [\n");
    b.push_str("      [ [100.0, 0.0], [101.0, 0.0], [101.0, 1.0],\n");
    b.push_str("        [100.0, 1.0], [100.0, 0.0] ]\n");
    b.push_str("      ]\n");
    b.push_str("  },\n");
    b.push_str("  \"properties\": {\n");
    b.push_str("    \"prop0\": \"value0\",\n");
    b.push_str("    \"prop1\": {\"this\": \"that\"}\n");
    b.push_str("  }\n");
    b.push_str("}\n");

    let polygons = from_geo_json(&b)?;
    assert_eq!(1, polygons.len());
    assert_eq!(
      Polygon::new(
        vec![0.0, 0.0, 1.0, 1.0, 0.0],
        vec![100.0, 101.0, 101.0, 100.0, 100.0],
        vec![],
      )?,
      polygons[0]
    );
    Ok(())
  }

  #[test]
  fn test_geojson_multi_polygon_feature() -> Result<()> {
    let mut b = String::new();
    b.push_str("{ \"type\": \"Feature\",\n");
    b.push_str("  \"geometry\": {\n");
    b.push_str("      \"type\": \"MultiPolygon\",\n");
    b.push_str("      \"coordinates\": [\n");
    b.push_str("        [\n");
    b.push_str("          [ [100.0, 0.0], [101.0, 0.0], [101.0, 1.0],\n");
    b.push_str("            [100.0, 1.0], [100.0, 0.0] ]\n");
    b.push_str("        ],\n");
    b.push_str("        [\n");
    b.push_str("          [ [10.0, 2.0], [11.0, 2.0], [11.0, 3.0],\n");
    b.push_str("            [10.0, 3.0], [10.0, 2.0] ]\n");
    b.push_str("        ]\n");
    b.push_str("      ]\n");
    b.push_str("  },\n");
    b.push_str("  \"properties\": {\n");
    b.push_str("    \"prop0\": \"value0\",\n");
    b.push_str("    \"prop1\": {\"this\": \"that\"}\n");
    b.push_str("  }\n");
    b.push_str("}\n");

    let polygons = from_geo_json(&b)?;
    assert_eq!(2, polygons.len());
    assert_eq!(
      Polygon::new(
        vec![0.0, 0.0, 1.0, 1.0, 0.0],
        vec![100.0, 101.0, 101.0, 100.0, 100.0],
        vec![],
      )?,
      polygons[0]
    );
    assert_eq!(
      Polygon::new(
        vec![2.0, 2.0, 3.0, 3.0, 2.0],
        vec![10.0, 11.0, 11.0, 10.0, 10.0],
        vec![],
      )?,
      polygons[1]
    );
    Ok(())
  }

  #[test]
  fn test_geojson_feature_collection_with_single_polygon() -> Result<()> {
    let mut b = String::new();
    b.push_str("{ \"type\": \"FeatureCollection\",\n");
    b.push_str("  \"features\": [\n");
    b.push_str("    { \"type\": \"Feature\",\n");
    b.push_str("      \"geometry\": {\n");
    b.push_str("        \"type\": \"Polygon\",\n");
    b.push_str("        \"coordinates\": [\n");
    b.push_str("          [ [100.0, 0.0], [101.0, 0.0], [101.0, 1.0],\n");
    b.push_str("            [100.0, 1.0], [100.0, 0.0] ]\n");
    b.push_str("          ]\n");
    b.push_str("      },\n");
    b.push_str("      \"properties\": {\n");
    b.push_str("        \"prop0\": \"value0\",\n");
    b.push_str("        \"prop1\": {\"this\": \"that\"}\n");
    b.push_str("      }\n");
    b.push_str("    }\n");
    b.push_str("  ]\n");
    b.push_str("}    \n");

    let expected = Polygon::new(
      vec![0.0, 0.0, 1.0, 1.0, 0.0],
      vec![100.0, 101.0, 101.0, 100.0, 100.0],
      vec![],
    )?;
    let actual = from_geo_json(&b)?;
    assert_eq!(1, actual.len());
    assert_eq!(expected, actual[0]);
    Ok(())
  }

  #[test]
  fn test_illegal_geojson_extra_crap_at_end() {
    let mut b = String::new();
    b.push_str("{\n");
    b.push_str("  \"type\": \"Polygon\",\n");
    b.push_str("  \"coordinates\": [\n");
    b.push_str("    [ [100.0, 0.0], [101.0, 0.0], [101.0, 1.0],\n");
    b.push_str("      [100.0, 1.0], [100.0, 0.0] ]\n");
    b.push_str("  ]\n");
    b.push_str("}\n");
    b.push_str("foo\n");

    let err = from_geo_json(&b);
    assert!(matches!(err, Err(LuceneError::IllegalState(_))));
    if let Err(e) = err {
      assert!(
        e.to_string()
          .contains("unexpected character 'f' after end of GeoJSON object")
      );
    }
  }

  #[test]
  fn test_illegal_geojson_linked_crs() {
    let mut b = String::new();
    b.push_str("{\n");
    b.push_str("  \"type\": \"Polygon\",\n");
    b.push_str("  \"coordinates\": [\n");
    b.push_str("    [ [100.0, 0.0], [101.0, 0.0], [101.0, 1.0],\n");
    b.push_str("      [100.0, 1.0], [100.0, 0.0] ]\n");
    b.push_str("  ],\n");
    b.push_str("  \"crs\": {\n");
    b.push_str("    \"type\": \"link\",\n");
    b.push_str("    \"properties\": {\n");
    b.push_str("      \"href\": \"http://example.com/crs/42\",\n");
    b.push_str("      \"type\": \"proj4\"\n");
    b.push_str("    }\n");
    b.push_str("  }    \n");
    b.push_str("}\n");

    let err = from_geo_json(&b);
    assert!(matches!(err, Err(LuceneError::IllegalState(_))));
    if let Err(e) = err {
      assert!(e.to_string().contains("cannot handle linked crs"));
    }
  }

  #[test]
  fn test_illegal_geojson_multiple_features() {
    let mut b = String::new();
    b.push_str("{ \"type\": \"FeatureCollection\",\n");
    b.push_str("  \"features\": [\n");
    b.push_str("    { \"type\": \"Feature\",\n");
    b.push_str("      \"geometry\": {\"type\": \"Point\", \"coordinates\": [102.0, 0.5]},\n");
    b.push_str("      \"properties\": {\"prop0\": \"value0\"}\n");
    b.push_str("    },\n");
    b.push_str("    { \"type\": \"Feature\",\n");
    b.push_str("      \"geometry\": {\n");
    b.push_str("      \"type\": \"LineString\",\n");
    b.push_str("      \"coordinates\": [\n");
    b.push_str("        [102.0, 0.0], [103.0, 1.0], [104.0, 0.0], [105.0, 1.0]\n");
    b.push_str("        ]\n");
    b.push_str("      },\n");
    b.push_str("      \"properties\": {\n");
    b.push_str("        \"prop0\": \"value0\",\n");
    b.push_str("        \"prop1\": 0.0\n");
    b.push_str("      }\n");
    b.push_str("    },\n");
    b.push_str("    { \"type\": \"Feature\",\n");
    b.push_str("      \"geometry\": {\n");
    b.push_str("        \"type\": \"Polygon\",\n");
    b.push_str("        \"coordinates\": [\n");
    b.push_str("          [ [100.0, 0.0], [101.0, 0.0], [101.0, 1.0],\n");
    b.push_str("            [100.0, 1.0], [100.0, 0.0] ]\n");
    b.push_str("          ]\n");
    b.push_str("      },\n");
    b.push_str("      \"properties\": {\n");
    b.push_str("        \"prop0\": \"value0\",\n");
    b.push_str("        \"prop1\": {\"this\": \"that\"}\n");
    b.push_str("      }\n");
    b.push_str("    }\n");
    b.push_str("  ]\n");
    b.push_str("}    \n");

    let err = from_geo_json(&b);
    assert!(matches!(err, Err(LuceneError::IllegalState(_))));
    if let Err(e) = err {
      assert!(e
          .to_string()
          .contains("can only handle type FeatureCollection (if it has a single polygon geometry), Feature, Polygon or MultiPolygon, but got Point"));
    }
  }

  #[test]
  fn test_polygon_properties_can_be_string_arrays() -> Result<()> {
    let mut b = String::new();
    b.push_str("{\n");
    b.push_str("  \"type\": \"Polygon\",\n");
    b.push_str("  \"coordinates\": [\n");
    b.push_str("    [ [100.0, 0.0], [101.0, 0.0], [101.0, 1.0],\n");
    b.push_str("      [100.0, 1.0], [100.0, 0.0] ]\n");
    b.push_str("  ],\n");
    b.push_str("  \"properties\": {\n");
    b.push_str("    \"array\": [ \"value\" ]\n");
    b.push_str("  }\n");
    b.push_str("}\n");

    let polygons = from_geo_json(&b)?;
    assert_eq!(1, polygons.len());
    Ok(())
  }
}
