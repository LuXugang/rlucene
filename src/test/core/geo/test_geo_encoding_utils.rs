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
use crate::core::geo::geo_utils::GeoUtils;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::numeric_utils::NumericUtils;
use crate::test_framework::core::util::lucene_test_case::random;
use crate::test_framework::core::util::test_util::TestUtil;
use rand::RngExt;
#[allow(dead_code)] // for quick search
struct TestGeoEncodingUtils;

#[test]
fn test_latitude_quantization() -> Result<()> {
  let latitude_decode = 180.0f64 / ((1u64 << 32) as f64);
  let mut rng = random();

  for _ in 0..10000 {
    let encoded: i32 = rng.random();
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
        let value =
          NumericUtils::sortable_long_to_double(TestUtil::next_long(&mut rng, min_bits, max_bits));

        assert_eq!(encoded, GeoEncodingUtils::encode_latitude(value)?);
        assert_eq!(encoded + 1, GeoEncodingUtils::encode_latitude_ceil(value)?);
      }
    }
  }

  Ok(())
}

#[test]
fn test_longitude_quantization() -> Result<()> {
  let longitude_decode = 360.0f64 / ((1u64 << 32) as f64);
  let mut rng = random();

  for _ in 0..10000 {
    let encoded: i32 = rng.random();
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
        let value =
          NumericUtils::sortable_long_to_double(TestUtil::next_long(&mut rng, min_bits, max_bits));

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
