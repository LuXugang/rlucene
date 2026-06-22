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
use crate::core::document::inet_address_point;
use crate::core::document::inet_address_range::InetAddressRange;
use crate::core::search::query::Query;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::search::base_range_field_query_test_case::{
  BaseRangeFieldQueryTestCase, Range, RangeBase,
};
use crate::test::core::util::lucene_test_case::random;
use rand::prelude::StdRng;
use rand::{Rng, RngExt};
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[allow(dead_code)] // for quick search
struct TestInetAddressRangeQueries;

const FIELD_NAME: &str = "ipRangeField";

impl TestInetAddressRangeQueries {
  fn next_inet_address<R>(&self, random: &mut R) -> IpAddr
  where
    R: Rng + ?Sized,
  {
    if random.random_bool(0.5) {
      let mut bytes = [0u8; 4];
      match random.random_range(0..5) {
        0 => {},
        1 => bytes.fill(0xff),
        2 => bytes.fill(42),
        _ => random.fill(&mut bytes),
      }
      IpAddr::V4(Ipv4Addr::from(bytes))
    } else {
      let mut bytes = [0u8; 16];
      match random.random_range(0..5) {
        0 => {},
        1 => bytes.fill(0xff),
        2 => bytes.fill(42),
        _ => random.fill(&mut bytes),
      }
      IpAddr::V6(Ipv6Addr::from(bytes))
    }
  }
}

impl BaseRangeFieldQueryTestCase for TestInetAddressRangeQueries {
  type Range = IpRange;
  type RangeField = InetAddressRange;

  fn new_range_field(&self, r: &Self::Range) -> Result<Self::RangeField> {
    InetAddressRange::new(FIELD_NAME, r.min_address, r.max_address)
  }

  fn new_intersects_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(InetAddressRange::new_intersects_query(FIELD_NAME, r.min_address, r.max_address)?.into())
  }

  fn new_contains_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(InetAddressRange::new_contains_query(FIELD_NAME, r.min_address, r.max_address)?.into())
  }

  fn new_within_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(InetAddressRange::new_within_query(FIELD_NAME, r.min_address, r.max_address)?.into())
  }

  fn new_crosses_query(&self, r: &Self::Range) -> Result<Query> {
    Ok(InetAddressRange::new_crosses_query(FIELD_NAME, r.min_address, r.max_address)?.into())
  }

  fn next_range<R>(&self, random: &mut R, _dimensions: usize) -> Result<Self::Range>
  where
    R: Rng + ?Sized,
  {
    let min = self.next_inet_address(random);
    let min_encoded = inet_address_point::encode_address(min);
    let max = self.next_inet_address(random);
    let max_encoded = inet_address_point::encode_address(max);
    if min_encoded[..] > max_encoded[..] {
      Ok(IpRange::new(max, min))
    } else {
      Ok(IpRange::new(min, max))
    }
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestInetAddressRangeQueries, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestInetAddressRangeQueries;
  f(&case, &mut random)
}

#[derive(Clone)]
pub(crate) struct IpRange {
  pub(crate) base: RangeBase,
  pub(crate) min_address: IpAddr,
  pub(crate) max_address: IpAddr,
  pub(crate) min: [u8; InetAddressRange::BYTES],
  pub(crate) max: [u8; InetAddressRange::BYTES],
}

impl IpRange {
  pub(crate) fn new(min_address: IpAddr, max_address: IpAddr) -> Self {
    Self {
      base: RangeBase::default(),
      min_address,
      max_address,
      min: inet_address_point::encode_address(min_address),
      max: inet_address_point::encode_address(max_address),
    }
  }
}

impl Range for IpRange {
  type Value = IpAddr;

  fn get_base(&self) -> &RangeBase {
    &self.base
  }

  fn get_base_mut(&mut self) -> &mut RangeBase {
    &mut self.base
  }

  fn num_dimensions(&self) -> usize {
    1
  }

  fn get_min(&self, _dim: usize) -> Self::Value {
    self.min_address
  }

  fn set_min(&mut self, _dim: usize, val: Self::Value) {
    let encoded = inet_address_point::encode_address(val);
    if self.min[..] < encoded[..] {
      self.max = encoded;
      self.max_address = val;
    } else {
      self.min = encoded;
      self.min_address = val;
    }
  }

  fn get_max(&self, _dim: usize) -> Self::Value {
    self.max_address
  }

  fn set_max(&mut self, _dim: usize, val: Self::Value) {
    let encoded = inet_address_point::encode_address(val);
    if self.max[..] > encoded[..] {
      self.min = encoded;
      self.min_address = val;
    } else {
      self.max = encoded;
      self.max_address = val;
    }
  }

  fn is_equal(&self, other: &Self) -> bool {
    self.min == other.min && self.max == other.max
  }

  fn is_disjoint(&self, other: &Self) -> bool {
    self.min[..] > other.max[..] || self.max[..] < other.min[..]
  }

  fn is_within(&self, other: &Self) -> bool {
    self.min[..] >= other.min[..] && self.max[..] <= other.max[..]
  }

  fn contains(&self, other: &Self) -> bool {
    self.min[..] <= other.min[..] && self.max[..] >= other.max[..]
  }
}

impl fmt::Display for IpRange {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Box({} TO {})", self.min_address, self.max_address)
  }
}

mod base_range_field_query_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::search::base_range_field_query_test_case::BaseRangeFieldQueryTestCase;
  use crate::test::core::search::test_inet_address_range_queries::run_case;

  #[test]
  fn test_random_tiny() -> Result<()> {
    run_case(|case, random| case.test_random_tiny(random))
  }

  #[test]
  fn test_multi_valued() -> Result<()> {
    run_case(|case, random| case.test_random_medium(random))
  }

  #[test]
  fn test_random_medium() -> Result<()> {
    run_case(|case, random| case.test_multi_valued(random))
  }

  #[cfg(feature = "nightly")]
  #[test]
  #[ignore = "nightly"]
  fn test_random_big() -> Result<()> {
    run_case(|case, random| case.test_random_big(random))
  }

  #[test]
  fn test_all_equal() -> Result<()> {
    run_case(|case, random| case.test_all_equal(random))
  }

  #[test]
  fn test_low_cardinality() -> Result<()> {
    run_case(|case, random| case.test_low_cardinality(random))
  }
}
