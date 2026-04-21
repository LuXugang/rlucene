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
use crate::core::geo::rectangle::Rectangle;
use crate::core::index::point_values::Relation;
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
  /// Create a predicate that checks whether points are within a distance of a given point. It works
  /// by computing the bounding box around the circle that is defined by the given points/distance
  /// and splitting it into between 1024 and 4096 smaller boxes (4096*0.75^2=2304 on average). Then
  /// for each sub box, it computes the relation between this box and the distance query. Finally at
  /// search time, it first computes the sub box that the point belongs to, most of the time, no
  /// distance computation will need to be performed since all points from the sub box will either be
  /// in or out of the circle.
  pub fn create_distance_predicate(
    lat: f64,
    lon: f64,
    radius_meters: f64,
  ) -> Result<DistancePredicate> {
    let bounding_box = Rectangle::from_point_distance(lat, lon, radius_meters)?;
    let axis_lat = Rectangle::axis_lat(lat, radius_meters);
    let distance_sort_key = GeoUtils::distance_query_sort_key(radius_meters);
    let sub_boxes = create_sub_boxes(
      bounding_box.min_lat,
      bounding_box.max_lat,
      bounding_box.min_lon,
      bounding_box.max_lon,
      |box_| {
        GeoUtils::relate(
          box_.min_lat,
          box_.max_lat,
          box_.min_lon,
          box_.max_lon,
          lat,
          lon,
          distance_sort_key,
          axis_lat,
        )
      },
    )?;

    DistancePredicate::new(
      sub_boxes.lat_shift,
      sub_boxes.lon_shift,
      sub_boxes.lat_base,
      sub_boxes.lon_base,
      sub_boxes.max_lat_delta,
      sub_boxes.max_lon_delta,
      sub_boxes.relations,
      lat,
      lon,
      distance_sort_key,
    )
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
pub struct DistancePredicate {
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
  pub fn test(&self, lat: i32, lon: i32) -> bool {
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

fn create_sub_boxes<F>(
  shape_min_lat: f64,
  shape_max_lat: f64,
  shape_min_lon: f64,
  shape_max_lon: f64,
  box_to_relation: F,
) -> Result<Grid>
where
  F: Fn(Rectangle) -> Result<Relation>,
{
  let min_lat = GeoEncodingUtils::encode_latitude_ceil(shape_min_lat)?;
  let max_lat = GeoEncodingUtils::encode_latitude(shape_max_lat)?;
  let min_lon = GeoEncodingUtils::encode_longitude_ceil(shape_min_lon)?;
  let max_lon = GeoEncodingUtils::encode_longitude(shape_max_lon)?;

  if max_lat < min_lat || (shape_max_lon >= shape_min_lon && max_lon < min_lon) {
    return Grid::new(1, 1, 0, 0, 0, 0, vec![]);
  }

  let min_lat2 = min_lat as i64 - i32::MIN as i64;
  let max_lat2 = max_lat as i64 - i32::MIN as i64;
  let lat_shift = compute_shift(min_lat2, max_lat2);
  let lat_base = (min_lat2 as u64 >> lat_shift) as i32;
  let max_lat_delta = (max_lat2 as u64 >> lat_shift) as i32 - lat_base + 1;
  debug_assert!(max_lat_delta > 0);

  let min_lon2 = min_lon as i64 - i32::MIN as i64;
  let mut max_lon2 = max_lon as i64 - i32::MIN as i64;
  if shape_max_lon < shape_min_lon {
    max_lon2 += 1i64 << 32;
  }
  let lon_shift = compute_shift(min_lon2, max_lon2);
  let lon_base = (min_lon2 as u64 >> lon_shift) as i32;
  let max_lon_delta = (max_lon2 as u64 >> lon_shift) as i32 - lon_base + 1;

  let mut relations = vec![0u8; (max_lat_delta * max_lon_delta) as usize];
  for i in 0..max_lat_delta {
    for j in 0..max_lon_delta {
      let box_min_lat = ((lat_base + i) << lat_shift).wrapping_add(i32::MIN);
      let box_min_lon = ((lon_base + j) << lon_shift).wrapping_add(i32::MIN);
      let box_max_lat = box_min_lat.wrapping_add((1 << lat_shift) - 1);
      let box_max_lon = box_min_lon.wrapping_add((1 << lon_shift) - 1);
      let rect = Rectangle::new(
        GeoEncodingUtils::decode_latitude(box_min_lat),
        GeoEncodingUtils::decode_latitude(box_max_lat),
        GeoEncodingUtils::decode_longitude(box_min_lon),
        GeoEncodingUtils::decode_longitude(box_max_lon),
      )?;
      relations[(i * max_lon_delta + j) as usize] = box_to_relation(rect)?.ordinal() as u8;
    }
  }

  Grid::new(
    lat_shift,
    lon_shift,
    lat_base,
    lon_base,
    max_lat_delta,
    max_lon_delta,
    relations,
  )
}

fn compute_shift(a: i64, b: i64) -> i32 {
  debug_assert!(a <= b);
  for shift in 1.. {
    let delta = (b as u64 >> shift) as i64 - (a as u64 >> shift) as i64;
    if (0..Grid::ARITY as i64).contains(&delta) {
      return shift;
    }
  }
  unreachable!()
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
#[cfg(test)]
mod tests {
  use crate::core::geo::geo_encoding_utils::GeoEncodingUtils;
  use crate::core::geo::geo_utils::GeoUtils;
  use crate::core::util::error::lucene_error::Result;
  use crate::core::util::numeric_utils::NumericUtils;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
  use crate::test::core::util::test_util::TestUtil;
  use rand::RngExt;

  #[allow(dead_code)] // for quick search
  struct TestGeoEncodingUtils;
  #[test]
  fn test_latitude_quantization() -> Result<()> {
    let latitude_decode = 180.0f64 / ((1u64 << 32) as f64);
    let mut random = random();

    for _ in 0..10000 {
      let encoded: i32 = random.random();
      let min =
        GeoUtils::MIN_LAT_INCL + ((encoded as i64 - i32::MIN as i64) as f64) * latitude_decode;

      let decoded = GeoEncodingUtils::decode_latitude(encoded);
      assert_eq!(min, decoded);

      assert_eq!(encoded, GeoEncodingUtils::encode_latitude(decoded)?);
      assert_eq!(encoded, GeoEncodingUtils::encode_latitude_ceil(decoded)?);

      if encoded != i32::MAX {
        let max = min + latitude_decode;
        assert_eq!(max, GeoEncodingUtils::decode_latitude(encoded + 1));
        assert_eq!(encoded + 1, GeoEncodingUtils::encode_latitude(max)?);
        assert_eq!(encoded + 1, GeoEncodingUtils::encode_latitude_ceil(max)?);

        let min_edge = min.next_up();
        let max_edge = max.next_down();

        assert_eq!(encoded, GeoEncodingUtils::encode_latitude(min_edge)?);
        assert_eq!(
          encoded + 1,
          GeoEncodingUtils::encode_latitude_ceil(min_edge)?
        );
        assert_eq!(encoded, GeoEncodingUtils::encode_latitude(max_edge)?);
        assert_eq!(
          encoded + 1,
          GeoEncodingUtils::encode_latitude_ceil(max_edge)?
        );

        let min_bits = NumericUtils::double_to_sortable_long(min_edge);
        let max_bits = NumericUtils::double_to_sortable_long(max_edge);

        for _ in 0..100 {
          let value = NumericUtils::sortable_long_to_double(TestUtil::next_long(
            &mut random,
            min_bits,
            max_bits,
          ));

          assert_eq!(encoded, GeoEncodingUtils::encode_latitude(value)?);
          assert_eq!(encoded + 1, GeoEncodingUtils::encode_latitude_ceil(value)?);
        }
      }
    }

    Ok(())
  }
  /// step through some integers, ensuring they decode to their expected double values. double values
  /// start at -180 and increase by LONGITUDE_DECODE for each integer. check edge cases within the
  /// double range and a random doubles within the range too.
  #[test]
  fn test_longitude_quantization() -> Result<()> {
    let longitude_decode = 360.0f64 / ((1u64 << 32) as f64);
    let mut random = random();

    for _ in 0..10000 {
      let encoded: i32 = random.random();
      let min =
        GeoUtils::MIN_LON_INCL + ((encoded as i64 - i32::MIN as i64) as f64) * longitude_decode;

      let decoded = GeoEncodingUtils::decode_longitude(encoded);
      assert_eq!(min, decoded);

      assert_eq!(encoded, GeoEncodingUtils::encode_longitude(decoded)?);
      assert_eq!(encoded, GeoEncodingUtils::encode_longitude_ceil(decoded)?);

      if encoded != i32::MAX {
        let max = min + longitude_decode;
        assert_eq!(max, GeoEncodingUtils::decode_longitude(encoded + 1));
        assert_eq!(encoded + 1, GeoEncodingUtils::encode_longitude(max)?);
        assert_eq!(encoded + 1, GeoEncodingUtils::encode_longitude_ceil(max)?);

        let min_edge = min.next_up();
        let max_edge = max.next_down();

        assert_eq!(encoded, GeoEncodingUtils::encode_longitude(min_edge)?);
        assert_eq!(
          encoded + 1,
          GeoEncodingUtils::encode_longitude_ceil(min_edge)?
        );
        assert_eq!(encoded, GeoEncodingUtils::encode_longitude(max_edge)?);
        assert_eq!(
          encoded + 1,
          GeoEncodingUtils::encode_longitude_ceil(max_edge)?
        );

        let min_bits = NumericUtils::double_to_sortable_long(min_edge);
        let max_bits = NumericUtils::double_to_sortable_long(max_edge);

        for _ in 0..100 {
          let value = NumericUtils::sortable_long_to_double(TestUtil::next_long(
            &mut random,
            min_bits,
            max_bits,
          ));

          assert_eq!(encoded, GeoEncodingUtils::encode_longitude(value)?);
          assert_eq!(encoded + 1, GeoEncodingUtils::encode_longitude_ceil(value)?);
        }
      }
    }

    Ok(())
  }

  #[test]
  fn test_encode_edge_cases() -> Result<()> {
    assert_eq!(
      i32::MIN,
      GeoEncodingUtils::encode_latitude(GeoUtils::MIN_LAT_INCL)?
    );
    assert_eq!(
      i32::MIN,
      GeoEncodingUtils::encode_latitude_ceil(GeoUtils::MIN_LAT_INCL)?
    );
    assert_eq!(
      i32::MAX,
      GeoEncodingUtils::encode_latitude(GeoUtils::MAX_LAT_INCL)?
    );
    assert_eq!(
      i32::MAX,
      GeoEncodingUtils::encode_latitude_ceil(GeoUtils::MAX_LAT_INCL)?
    );

    assert_eq!(
      i32::MIN,
      GeoEncodingUtils::encode_longitude(GeoUtils::MIN_LON_INCL)?
    );
    assert_eq!(
      i32::MIN,
      GeoEncodingUtils::encode_longitude_ceil(GeoUtils::MIN_LON_INCL)?
    );
    assert_eq!(
      i32::MAX,
      GeoEncodingUtils::encode_longitude(GeoUtils::MAX_LON_INCL)?
    );
    assert_eq!(
      i32::MAX,
      GeoEncodingUtils::encode_longitude_ceil(GeoUtils::MAX_LON_INCL)?
    );

    Ok(())
  }
}
