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
use crate::core::document::fields::Fields;
use crate::core::document::shape_field::Triangle;
use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
use crate::core::geo::line::Line;
use crate::core::util::error::lucene_error::Result;

/// A geo shape utility type for indexing and searching GIS geometries whose vertices are
/// latitude, longitude values (in decimal degrees).
///
/// **WARNING**: Like [`LatLonPoint`](crate::core::document::lat_lon_point::LatLonPoint), vertex values are indexed with some loss of precision from
/// the original `f64` values (4.190951585769653E-8 for the latitude component and
/// 8.381903171539307E-8 for longitude).
pub struct LatLonShape {
  _private: (),
}

impl LatLonShape {
  /// Creates indexable fields for line geometry.
  pub fn create_indexable_fields_from_line(field_name: &str, line: &Line) -> Result<Vec<Fields>> {
    let num_points = line.num_points();
    let mut fields = Vec::with_capacity(num_points - 1);
    // create "flat" triangles
    let mut i = 0;
    let mut j = 1;
    while j < num_points {
      fields.push(
        Triangle::new(
          field_name,
          GeoEncodingUtils::encode_longitude(line.get_lon(i))?,
          GeoEncodingUtils::encode_latitude(line.get_lat(i))?,
          GeoEncodingUtils::encode_longitude(line.get_lon(j))?,
          GeoEncodingUtils::encode_latitude(line.get_lat(j))?,
          GeoEncodingUtils::encode_longitude(line.get_lon(i))?,
          GeoEncodingUtils::encode_latitude(line.get_lat(i))?,
        )?
        .into(),
      );
      i += 1;
      j += 1;
    }
    Ok(fields)
  }

  /// Creates indexable fields for point geometry.
  pub fn create_indexable_fields(field_name: &str, lat: f64, lon: f64) -> Result<Vec<Fields>> {
    Ok(vec![
      Triangle::new(
        field_name,
        GeoEncodingUtils::encode_longitude(lon)?,
        GeoEncodingUtils::encode_latitude(lat)?,
        GeoEncodingUtils::encode_longitude(lon)?,
        GeoEncodingUtils::encode_latitude(lat)?,
        GeoEncodingUtils::encode_longitude(lon)?,
        GeoEncodingUtils::encode_latitude(lat)?,
      )?
      .into(),
    ])
  }
}
