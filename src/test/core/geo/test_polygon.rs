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
use crate::core::geo::polygon::{Polygon, from_geo_json};
use crate::core::util::error::lucene_error::{LuceneError, Result};
#[allow(dead_code)] // for quick search
struct TestPolygon;

#[test]
#[ignore = "Java-only: Rust coordinate vectors cannot be null"]
fn test_polygon_null_poly_lats() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: Rust coordinate vectors cannot be null"]
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
