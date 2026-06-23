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
// Migrated from src/core/util/math_util.rs

use crate::test::core::util::lucene_test_case::{at_least, random};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{FromPrimitive, ToPrimitive};
use rand::Rng;
use rand::RngExt;
use rand::prelude::IndexedRandom;

use crate::core::util::math_util::MathUtil;

#[allow(dead_code)] // for quick search
struct TestMathUtil;
/// List of prime numbers.
const PRIMES: [i64; 10] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29];

/// Generates a random `i64` value following the logic in the original Java
/// function.
fn random_long<R>(random: &mut R) -> i64
where
  R: Rng + ?Sized,
{
  if random.random_bool(0.5) {
    let mut l: i64 = 1;
    if random.random_bool(0.5) {
      l *= -1;
    }
    for &i in PRIMES.iter() {
      let m = random.random_range(0..3);
      for _ in 0..m {
        l = l.wrapping_mul(i);
      }
    }
    l
  } else if random.random_bool(0.5) {
    random.random::<i64>()
  } else {
    let values = [i64::MIN, i64::MAX, 0, -1, 1];
    *values.choose(random).unwrap()
  }
}
/// Slow version of GCD used for testing.
fn gcd(l1: i64, l2: i64) -> i64 {
  let big_l1 = BigInt::from_i64(l1).unwrap();
  let big_l2 = BigInt::from_i64(l2).unwrap();
  let gcd = big_l1.gcd(&big_l2);
  assert!(gcd.bits() <= 64);
  let two_64 = BigInt::from(1u128 << 64);
  let t = gcd.mod_floor(&two_64);
  let u = t.to_u64().unwrap();
  u as i64
}
#[test]
fn test_gcd() {
  let mut random = random();
  let iters = at_least(&mut random, 100); // Replace with an appropriate function

  for _ in 0..iters {
    let l1 = random_long(&mut random);
    let l2 = random_long(&mut random);
    let gcd_value = MathUtil::gcd(l1, l2);
    let actual_gcd = gcd(l1, l2);

    assert_eq!(
      actual_gcd, gcd_value,
      "Expected GCD({},{}) = {}",
      l1, l2, actual_gcd
    );

    if gcd_value != 0 {
      assert_eq!(
        l1,
        (l1 / gcd_value) * gcd_value,
        "l1 consistency check failed"
      );
      assert_eq!(
        l2,
        (l2 / gcd_value) * gcd_value,
        "l2 consistency check failed"
      );
    }
  }
}
#[test]
fn test_gcd2() {
  let a = 30;
  let b = 50;
  let c = 77;

  assert_eq!(0, MathUtil::gcd(0, 0));

  assert_eq!(b, MathUtil::gcd(0, b));
  assert_eq!(a, MathUtil::gcd(a, 0));

  assert_eq!(b, MathUtil::gcd(0, -b));
  assert_eq!(a, MathUtil::gcd(-a, 0));

  assert_eq!(10, MathUtil::gcd(a, b));
  assert_eq!(10, MathUtil::gcd(-a, b));
  assert_eq!(10, MathUtil::gcd(a, -b));
  assert_eq!(10, MathUtil::gcd(-a, -b));

  assert_eq!(1, MathUtil::gcd(a, c));
  assert_eq!(1, MathUtil::gcd(-a, c));
  assert_eq!(1, MathUtil::gcd(a, -c));
  assert_eq!(1, MathUtil::gcd(-a, -c));

  let lhs = 3i64.wrapping_mul(1i64 << 50);
  let rhs = 9i64.wrapping_mul(1i64 << 45);
  let expected = 3i64.wrapping_mul(1i64 << 45);
  assert_eq!(expected, MathUtil::gcd(lhs, rhs));

  let lhs = 1i64 << 45;
  let rhs = i64::MIN;
  assert_eq!(1i64 << 45, MathUtil::gcd(lhs, rhs));

  assert_eq!(i64::MAX, MathUtil::gcd(i64::MAX, 0));
  assert_eq!(i64::MAX, MathUtil::gcd(-i64::MAX, 0));

  assert_eq!(1, MathUtil::gcd(60247241209, 153092023));

  assert_eq!(i64::MIN, MathUtil::gcd(i64::MIN, 0));
  assert_eq!(i64::MIN, MathUtil::gcd(0, i64::MIN));
  assert_eq!(i64::MIN, MathUtil::gcd(i64::MIN, i64::MIN));
}
#[test]
fn test_acosh_method() {
  // acosh(NaN) == NaN
  assert!(MathUtil::acosh(f64::NAN).is_nan());
  // acosh(1) == +0
  assert_eq!(0, MathUtil::acosh(1.0).to_bits());
  // acosh(POSITIVE_INFINITY) == POSITIVE_INFINITY
  assert_eq!(
    f64::INFINITY.to_bits(),
    MathUtil::acosh(f64::INFINITY).to_bits()
  );
  // acosh(x) : x < 1 == NaN
  assert!(MathUtil::acosh(0.9).is_nan()); // x < 1
  assert!(MathUtil::acosh(0.0).is_nan()); // x == 0
  assert!(MathUtil::acosh(-0.0).is_nan()); // x == -0
  assert!(MathUtil::acosh(-0.9).is_nan()); // x < 0
  assert!(MathUtil::acosh(-1.0).is_nan()); // x == -1
  assert!(MathUtil::acosh(-10.0).is_nan()); // x < -1
  assert!(MathUtil::acosh(f64::NEG_INFINITY).is_nan()); // x == -Inf

  let epsilon = 0.000001;
  assert!((MathUtil::acosh(1.0) - 0.0).abs() < epsilon);
  assert!((MathUtil::acosh(2.5) - 1.5667992369724109).abs() < epsilon);
  assert!((MathUtil::acosh(1234567.89) - 14.719378760739708).abs() < epsilon);
}
#[test]
fn test_asinh_method() {
  // asinh(NaN) == NaN
  assert!(MathUtil::asinh(f64::NAN).is_nan());
  // asinh(+0) == +0
  assert_eq!(0, MathUtil::asinh(0.0).to_bits());
  // asinh(-0) == -0
  assert_eq!((-0.0f64).to_bits(), MathUtil::asinh(-0.0).to_bits());
  // asinh(POSITIVE_INFINITY) == POSITIVE_INFINITY
  assert_eq!(
    f64::INFINITY.to_bits(),
    MathUtil::asinh(f64::INFINITY).to_bits()
  );
  // asinh(NEGATIVE_INFINITY) == NEGATIVE_INFINITY
  assert_eq!(
    f64::NEG_INFINITY.to_bits(),
    MathUtil::asinh(f64::NEG_INFINITY).to_bits()
  );

  let epsilon = 0.000001;
  assert!((MathUtil::asinh(-1234567.89) - (-14.719378760740035)).abs() < epsilon);
  assert!((MathUtil::asinh(-2.5) - (-1.6472311463710958)).abs() < epsilon);
  assert!((MathUtil::asinh(-1.0) - (-0.8813735870195429)).abs() < epsilon);
  assert!((MathUtil::asinh(0.0) - 0.0).abs() < epsilon);
  assert!((MathUtil::asinh(1.0) - 0.8813735870195429).abs() < epsilon);
  assert!((MathUtil::asinh(2.5) - 1.6472311463710958).abs() < epsilon);
  assert!((MathUtil::asinh(1234567.89) - 14.719378760740035).abs() < epsilon);
}

#[test]
fn test_atanh_method() {
  // atanh(NaN) == NaN
  assert!(MathUtil::atanh(f64::NAN).is_nan());
  // atanh(+0) == +0
  assert_eq!(0, MathUtil::atanh(0.0).to_bits());
  // atanh(-0) == -0
  assert_eq!((-0.0f64).to_bits(), MathUtil::atanh(-0.0).to_bits());
  // atanh(1) == POSITIVE_INFINITY
  assert_eq!(f64::INFINITY.to_bits(), MathUtil::atanh(1.0).to_bits());
  // atanh(-1) == NEGATIVE_INFINITY
  assert_eq!(f64::NEG_INFINITY.to_bits(), MathUtil::atanh(-1.0).to_bits());
  // atanh(x) : Math.abs(x) > 1 == NaN
  assert!(MathUtil::atanh(1.1).is_nan()); // x > 1
  assert!(MathUtil::atanh(f64::INFINITY).is_nan()); // x == Inf
  assert!(MathUtil::atanh(-1.1).is_nan()); // x < -1
  assert!(MathUtil::atanh(f64::NEG_INFINITY).is_nan()); // x == -Inf

  let epsilon = 0.000001;
  assert_eq!(f64::NEG_INFINITY.to_bits(), MathUtil::atanh(-1.0).to_bits());
  assert!((MathUtil::atanh(-0.5) - (-0.5493061443340549)).abs() < epsilon);
  assert!((MathUtil::atanh(0.0) - 0.0).abs() < epsilon);
  assert!((MathUtil::atanh(0.5) - 0.5493061443340549).abs() < epsilon);
  assert_eq!(f64::INFINITY.to_bits(), MathUtil::atanh(1.0).to_bits());
}
