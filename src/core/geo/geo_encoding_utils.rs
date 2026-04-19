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
use crate::core::geo::component2d::Component2D;
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::index::point_values::Relation::{CellCrossesQuery, CellInsideQuery};
use crate::core::util::SloppyMath;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::numeric_utils::NumericUtils;
use std::sync::LazyLock;
pub static MIN_LON_ENCODED: LazyLock<i32> =
  LazyLock::new(|| encode_longitude(GeoUtils::MIN_LON_INCL).expect("MIN_LON_INCL must be valid"));

pub static MAX_LON_ENCODED: LazyLock<i32> =
  LazyLock::new(|| encode_longitude(GeoUtils::MAX_LON_INCL).expect("MAX_LON_INCL must be valid"));
/// Quantizes double (64 bit) longitude into 32 bits (rounding down: in the direction of -180)
///
/// # Arguments
///
/// * `longitude` - longitude value: must be within standard +/-180 coordinate bounds.
///
/// # Returns
///
/// encoded value as a 32-bit `i32`
///
/// # Errors
///
/// Returns an error if longitude is out of bounds.
pub fn encode_longitude(longitude: f64) -> Result<i32> {
  GeoUtils::check_longitude(longitude)?;
  let mut longitude = longitude;
  // the maximum possible value cannot be encoded without overflow
  if longitude == 180.0 {
    longitude = longitude.next_down();
  }
  Ok((longitude / GeoEncodingUtils::LON_DECODE).floor() as i32)
}
pub struct GeoEncodingUtils;
impl GeoEncodingUtils {
  /// number of bits used for quantizing latitude and longitude values
  pub const BITS: u32 = 32;

  const LAT_SCALE: f64 = (1u64 << Self::BITS) as f64 / 180.0;
  const LAT_DECODE: f64 = 1.0 / Self::LAT_SCALE;
  const LON_SCALE: f64 = (1u64 << Self::BITS) as f64 / 360.0;
  const LON_DECODE: f64 = 1.0 / Self::LON_SCALE;

  /// Quantizes double (64 bit) latitude into 32 bits (rounding down: in the direction of -90)
  ///
  /// # Arguments
  ///
  /// * `latitude` - latitude value: must be within standard +/-90 coordinate bounds.
  ///
  /// # Returns
  ///
  /// encoded value as a 32-bit `i32`
  ///
  /// # Errors
  ///
  /// Returns an error if latitude is out of bounds.
  pub fn encode_latitude(latitude: f64) -> Result<i32> {
    GeoUtils::check_latitude(latitude)?;
    let mut latitude = latitude;
    // the maximum possible value cannot be encoded without overflow
    if latitude == 90.0 {
      latitude = latitude.next_down();
    }
    Ok((latitude / Self::LAT_DECODE).floor() as i32)
  }

  /// Quantizes double (64 bit) latitude into 32 bits (rounding up: in the direction of +90)
  ///
  /// # Arguments
  ///
  /// * `latitude` - latitude value: must be within standard +/-90 coordinate bounds.
  ///
  /// # Returns
  ///
  /// encoded value as a 32-bit `i32`
  ///
  /// # Errors
  ///
  /// Returns an error if latitude is out of bounds.
  pub fn encode_latitude_ceil(latitude: f64) -> Result<i32> {
    GeoUtils::check_latitude(latitude)?;
    let mut latitude = latitude;
    // the maximum possible value cannot be encoded without overflow
    if latitude == 90.0 {
      latitude = latitude.next_down();
    }
    Ok((latitude / Self::LAT_DECODE).ceil() as i32)
  }

  /// Quantizes double (64 bit) longitude into 32 bits (rounding down: in the direction of -180)
  ///
  /// # Arguments
  ///
  /// * `longitude` - longitude value: must be within standard +/-180 coordinate bounds.
  ///
  /// # Returns
  ///
  /// encoded value as a 32-bit `i32`
  ///
  /// # Errors
  ///
  /// Returns an error if longitude is out of bounds.
  pub fn encode_longitude(longitude: f64) -> Result<i32> {
    encode_longitude(longitude)
  }

  /// Quantizes double (64 bit) longitude into 32 bits (rounding up: in the direction of +180)
  ///
  /// # Arguments
  ///
  /// * `longitude` - longitude value: must be within standard +/-180 coordinate bounds.
  ///
  /// # Returns
  ///
  /// encoded value as a 32-bit `i32`
  ///
  /// # Errors
  ///
  /// Returns an error if longitude is out of bounds.
  pub fn encode_longitude_ceil(longitude: f64) -> Result<i32> {
    GeoUtils::check_longitude(longitude)?;
    let mut longitude = longitude;
    // the maximum possible value cannot be encoded without overflow
    if longitude == 180.0 {
      longitude = longitude.next_down();
    }
    Ok((longitude / Self::LON_DECODE).ceil() as i32)
  }

  /// Turns quantized value from [`encode_latitude`] back into a double.
  ///
  /// # Arguments
  ///
  /// * `encoded` - encoded value: 32-bit quantized value.
  ///
  /// # Returns
  ///
  /// decoded latitude value.
  pub fn decode_latitude(encoded: i32) -> f64 {
    let result = encoded as f64 * Self::LAT_DECODE;
    debug_assert!((GeoUtils::MIN_LAT_INCL..GeoUtils::MAX_LAT_INCL).contains(&result));
    result
  }

  /// Turns quantized value from byte array back into a double.
  ///
  /// # Arguments
  ///
  /// * `src` - byte array containing 4 bytes to decode at `offset`
  /// * `offset` - offset into `src` to decode from.
  ///
  /// # Returns
  ///
  /// decoded latitude value.
  pub fn decode_latitude_from_bytes(src: &[u8], offset: usize) -> f64 {
    Self::decode_latitude(NumericUtils::sortable_bytes_to_int(src, offset))
  }

  /// Turns quantized value from [`encode_longitude`] back into a double.
  ///
  /// # Arguments
  ///
  /// * `encoded` - encoded value: 32-bit quantized value.
  ///
  /// # Returns
  ///
  /// decoded longitude value.
  pub fn decode_longitude(encoded: i32) -> f64 {
    let result = encoded as f64 * Self::LON_DECODE;
    debug_assert!((GeoUtils::MIN_LON_INCL..GeoUtils::MAX_LON_INCL).contains(&result));
    result
  }

  /// Turns quantized value from byte array back into a double.
  ///
  /// # Arguments
  ///
  /// * `src` - byte array containing 4 bytes to decode at `offset`
  /// * `offset` - offset into `src` to decode from.
  ///
  /// # Returns
  ///
  /// decoded longitude value.
  pub fn decode_longitude_from_bytes(src: &[u8], offset: usize) -> f64 {
    Self::decode_longitude(NumericUtils::sortable_bytes_to_int(src, offset))
  }
}
struct Grid {
  lat_shift: i32,
  lon_shift: i32,
  lat_base: i32,
  lon_base: i32,
  max_lat_delta: i32,
  max_lon_delta: i32,
  relations: Vec<u8>,
}

impl Grid {
  const ARITY: i32 = 64;

  fn new(
    lat_shift: i32,
    lon_shift: i32,
    lat_base: i32,
    lon_base: i32,
    max_lat_delta: i32,
    max_lon_delta: i32,
    relations: Vec<u8>,
  ) -> Result<Self> {
    if !(1..=31).contains(&lat_shift) {
      return Err(LuceneError::illegal_argument(
        "lat_shift must be between 1 and 31",
      ));
    }
    if !(1..=31).contains(&lon_shift) {
      return Err(LuceneError::illegal_argument(
        "lon_shift must be between 1 and 31",
      ));
    }

    Ok(Self {
      lat_shift,
      lon_shift,
      lat_base,
      lon_base,
      max_lat_delta,
      max_lon_delta,
      relations,
    })
  }
}
struct DistancePredicate {
  base: Grid,
  lat: f64,
  lon: f64,
  distance_key: f64,
}

impl DistancePredicate {
  #[allow(clippy::too_many_arguments)]
  fn new(
    lat_shift: i32,
    lon_shift: i32,
    lat_base: i32,
    lon_base: i32,
    max_lat_delta: i32,
    max_lon_delta: i32,
    relations: Vec<u8>,
    lat: f64,
    lon: f64,
    distance_key: f64,
  ) -> Result<Self> {
    Ok(Self {
      base: Grid::new(
        lat_shift,
        lon_shift,
        lat_base,
        lon_base,
        max_lat_delta,
        max_lon_delta,
        relations,
      )?,
      lat,
      lon,
      distance_key,
    })
  }
  /// Check whether the given point is within a distance of another point.
  /// NOTE: this operates directly on the encoded representation of points.
  fn test(&self, lat: i32, lon: i32) -> bool {
    let lat2 = ((lat.wrapping_sub(i32::MIN)) as u32 >> self.base.lat_shift) as i32;
    if lat2 < self.base.lat_base || lat2 - self.base.lat_base >= self.base.max_lat_delta {
      return false;
    }

    let mut lon2 = ((lon.wrapping_sub(i32::MIN)) as u32 >> self.base.lon_shift) as i32;
    if lon2 < self.base.lon_base {
      lon2 += 1i32 << (32 - self.base.lon_shift);
    }

    debug_assert!((lon2 as u32) >= (self.base.lon_base as u32));
    debug_assert!(lon2 - self.base.lon_base >= 0);

    if lon2 - self.base.lon_base >= self.base.max_lon_delta {
      return false;
    }

    let relation = self.base.relations[((lat2 - self.base.lat_base) * self.base.max_lon_delta
      + (lon2 - self.base.lon_base)) as usize];

    if relation == CellCrossesQuery.ordinal() as u8 {
      SloppyMath::haversin_sort_key(
        GeoEncodingUtils::decode_latitude(lat),
        GeoEncodingUtils::decode_longitude(lon),
        self.lat,
        self.lon,
      ) <= self.distance_key
    } else {
      relation == CellInsideQuery.ordinal() as u8
    }
  }
}
/// A predicate that checks whether a given point is within a component2D geometry.
pub struct Component2DPredicate<C>
where
  C: Component2D,
{
  base: Grid,
  tree: C,
}

impl<C> Component2DPredicate<C>
where
  C: Component2D,
{
  #[allow(clippy::too_many_arguments)]
  fn new(
    lat_shift: i32,
    lon_shift: i32,
    lat_base: i32,
    lon_base: i32,
    max_lat_delta: i32,
    max_lon_delta: i32,
    relations: Vec<u8>,
    tree: C,
  ) -> Result<Self> {
    Ok(Self {
      base: Grid::new(
        lat_shift,
        lon_shift,
        lat_base,
        lon_base,
        max_lat_delta,
        max_lon_delta,
        relations,
      )?,
      tree,
    })
  }

  /// Check whether the given point is within the considered polygon. NOTE: this operates directly
  /// on the encoded representation of points.
  pub fn test(&self, lat: i32, lon: i32) -> bool {
    let base = &self.base;

    let lat2 = ((lat.wrapping_sub(i32::MIN) as u32) >> base.lat_shift) as i32;
    if lat2 < base.lat_base || lat2 - base.lat_base >= base.max_lat_delta {
      return false;
    }

    let mut lon2 = ((lon.wrapping_sub(i32::MIN) as u32) >> base.lon_shift) as i32;
    if lon2 < base.lon_base {
      lon2 += 1i32 << (32 - base.lon_shift);
    }

    debug_assert!((lon2 as u32) >= (base.lon_base as u32));
    debug_assert!(lon2 - base.lon_base >= 0);

    if lon2 - base.lon_base >= base.max_lon_delta {
      return false;
    }

    let relation = base.relations
      [((lat2 - base.lat_base) * base.max_lon_delta + (lon2 - base.lon_base)) as usize];

    if relation == CellCrossesQuery.ordinal() as u8 {
      self.tree.contains(
        GeoEncodingUtils::decode_longitude(lon),
        GeoEncodingUtils::decode_latitude(lat),
      )
    } else {
      relation == CellInsideQuery.ordinal() as u8
    }
  }
}
