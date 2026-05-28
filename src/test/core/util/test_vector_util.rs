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
// Migrated from src/core/util/vector_util.rs

use crate::core::index::BytesRef;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::vector_util::{VECTOR_UTIL, VectorUtil};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use crate::test::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};

const DELTA: f32 = 1e-4;

#[allow(dead_code)]
struct TestVectorUtil;

#[test]
fn test_basic_dot_product() {
  assert_eq!(
    5.0,
    VECTOR_UTIL
      .dot_product_f32(&[1.0, 2.0, 3.0], &[-10.0, 0.0, 5.0])
      .unwrap()
  );
}

#[test]
fn test_self_dot_product() {
  let mut random = random();
  let v = random_vector(&mut random);
  assert_approx_eq(
    l2_f32(&v),
    VECTOR_UTIL.dot_product_f32(&v, &v).unwrap(),
    DELTA,
  );
}

#[test]
fn test_orthogonal_dot_product() {
  let mut rng = random();
  let v = [
    rng.random_range(0..100) as f32,
    rng.random_range(0..100) as f32,
  ];
  let u = [v[1], -v[0]];
  assert_approx_eq(0.0, VECTOR_UTIL.dot_product_f32(&u, &v).unwrap(), DELTA);
}

#[test]
fn test_dot_product_throws_for_dimension_mismatch() {
  let v = [1.0, 0.0, 0.0];
  let u = [0.0, 1.0];
  assert!(matches!(
    VECTOR_UTIL.dot_product_f32(&u, &v),
    Err(LuceneError::IllegalArgument(_))
  ));
}

#[test]
fn test_self_square_distance() {
  let mut random = random();
  let v = random_vector(&mut random);
  assert_approx_eq(0.0, VECTOR_UTIL.square_distance_f32(&v, &v).unwrap(), DELTA);
}

#[test]
fn test_basic_square_distance() {
  assert_eq!(
    12.0,
    VECTOR_UTIL
      .square_distance_f32(&[1.0, 2.0, 3.0], &[-1.0, 0.0, 5.0])
      .unwrap()
  );
}

#[test]
fn test_square_distance_throws_for_dimension_mismatch() {
  let v = [1.0, 0.0, 0.0];
  let u = [0.0, 1.0];
  assert!(matches!(
    VECTOR_UTIL.square_distance_f32(&u, &v),
    Err(LuceneError::IllegalArgument(_))
  ));
}

#[test]
fn test_random_square_distance() {
  let mut random = random();
  let v = random_vector(&mut random);
  let u = negative_f32(&v);
  assert_approx_eq(
    4.0 * l2_f32(&v),
    VECTOR_UTIL.square_distance_f32(&u, &v).unwrap(),
    DELTA,
  );
}

#[test]
fn test_basic_cosine() {
  assert_approx_eq(
    0.11952,
    VECTOR_UTIL
      .cosine_f32(&[1.0, 2.0, 3.0], &[-10.0, 0.0, 5.0])
      .unwrap(),
    DELTA,
  );
}

#[test]
fn test_self_cosine() {
  let mut random = random();
  let mut v = random_vector(&mut random);
  v[0] = random.random_range(0.01..1.01);
  assert_approx_eq(1.0, VECTOR_UTIL.cosine_f32(&v, &v).unwrap(), DELTA);
}

#[test]
fn test_orthogonal_cosine() {
  let mut rng = random();
  let v = [
    rng.random_range(0..100) as f32,
    rng.random_range(1..100) as f32,
  ];
  let u = [v[1], -v[0]];
  assert_approx_eq(0.0, VECTOR_UTIL.cosine_f32(&u, &v).unwrap(), DELTA);
}

#[test]
fn test_cosine_throws_for_dimension_mismatch() {
  let v = [1.0, 0.0, 0.0];
  let u = [0.0, 1.0];
  assert!(matches!(
    VECTOR_UTIL.cosine_f32(&u, &v),
    Err(LuceneError::IllegalArgument(_))
  ));
}

#[test]
fn test_normalize() {
  let mut random = random();
  let mut v = random_vector(&mut random);
  let idx = random.random_range(0..v.len());
  v[idx] = 1.0;
  VectorUtil::l2normalize(&mut v).unwrap();
  assert_approx_eq(1.0, l2_f32(&v), DELTA);
}

#[test]
fn test_normalize_zero_throws() {
  let mut v = [0.0, 0.0, 0.0];
  assert!(matches!(
    VectorUtil::l2normalize(&mut v),
    Err(LuceneError::IllegalArgument(_))
  ));
}

#[test]
fn test_extreme_numerics() -> Result<()> {
  let v1 = vec![0.888888f32; 1536];
  let v2 = vec![-0.777777f32; 1536];
  for similarity in [
    VectorSimilarityFunction::Euclidean,
    VectorSimilarityFunction::DotProduct,
    VectorSimilarityFunction::Cosine,
    VectorSimilarityFunction::MaximumInnerProduct,
  ] {
    let value = similarity.compare_f32(&v1, &v2)?;
    assert!(value >= 0.0, "{similarity} expected >=0 got:{value}");
  }
  Ok(())
}

#[test]
fn test_basic_dot_product_bytes() {
  let a = bytes(&[1, 2, 3]);
  let b = bytes(&[-10, 0, 5]);
  assert_eq!(5, VECTOR_UTIL.dot_product_u8(&a, &b).unwrap());
  let denom = (a.len() * (1 << 15)) as f32;
  assert_approx_eq(
    0.5 + 5.0 / denom,
    VECTOR_UTIL.dot_product_score(&a, &b).unwrap(),
    DELTA,
  );

  let zero = bytes(&[0, 0, 0]);
  assert_approx_eq(
    0.5,
    VECTOR_UTIL.dot_product_score(&a, &zero).unwrap(),
    DELTA,
  );

  let min = bytes(&[-128, -128]);
  let max = bytes(&[127, 127]);
  assert_approx_eq(
    0.0039,
    VECTOR_UTIL.dot_product_score(&min, &max).unwrap(),
    DELTA,
  );
  assert_approx_eq(
    1.0,
    VECTOR_UTIL.dot_product_score(&min, &min).unwrap(),
    DELTA,
  );
}

#[test]
fn test_self_dot_product_bytes() {
  let mut random = random();
  let v = random_vector_bytes(&mut random);
  assert_approx_eq(
    l2_u8(&v),
    VECTOR_UTIL.dot_product_u8(&v, &v).unwrap() as f32,
    DELTA,
  );
}

#[test]
fn test_orthogonal_dot_product_bytes() {
  let mut rng = random();
  let a0 = rng.random_range(0..100) as i8;
  let a1 = rng.random_range(0..100) as i8;
  let a = [a0 as u8, a1 as u8];
  let b = [a1 as u8, (-a0) as u8];
  assert_eq!(0, VECTOR_UTIL.dot_product_u8(&a, &b).unwrap());
}

#[test]
fn test_self_square_distance_bytes() {
  let mut random = random();
  let v = random_vector_bytes(&mut random);
  assert_eq!(0, VECTOR_UTIL.square_distance_u8(&v, &v).unwrap());
}

#[test]
fn test_basic_square_distance_bytes() {
  assert_eq!(
    12,
    VECTOR_UTIL
      .square_distance_u8(&bytes(&[1, 2, 3]), &bytes(&[-1, 0, 5]))
      .unwrap()
  );
}

#[test]
fn test_random_square_distance_bytes() {
  let mut random = random();
  let v = random_vector_bytes(&mut random);
  let u = negative_u8(&v);
  assert_approx_eq(
    4.0 * l2_u8(&v),
    VECTOR_UTIL.square_distance_u8(&u, &v).unwrap() as f32,
    DELTA,
  );
}

#[test]
fn test_basic_cosine_bytes() {
  assert_approx_eq(
    0.11952,
    VECTOR_UTIL
      .cosine_u8(&bytes(&[1, 2, 3]), &bytes(&[-10, 0, 5]))
      .unwrap(),
    DELTA,
  );
}

#[test]
fn test_self_cosine_bytes() {
  let mut random = random();
  let mut v = random_vector_bytes(&mut random);
  v[0] = (random.random_range(1..127) as i8) as u8;
  assert_approx_eq(1.0, VECTOR_UTIL.cosine_u8(&v, &v).unwrap(), DELTA);
}

#[test]
fn test_orthogonal_cosine_bytes() {
  let mut rng = random();
  let v0 = rng.random_range(0..100) as i8;
  let v1 = rng.random_range(1..100) as i8;
  let v = [v0 as u8, v1 as u8];
  let u = [v1 as u8, (-v0) as u8];
  assert_approx_eq(0.0, VECTOR_UTIL.cosine_u8(&u, &v).unwrap(), DELTA);
}

#[test]
fn test_basic_xor_bit_count() {
  test_basic_xor_bit_count_impl(|a, b| VECTOR_UTIL.xor_bit_count(a, b).unwrap());
  test_basic_xor_bit_count_impl(|a, b| VECTOR_UTIL.xor_bit_count_int(a, b));
  test_basic_xor_bit_count_impl(|a, b| VECTOR_UTIL.xor_bit_count_long(a, b));
  test_basic_xor_bit_count_impl(slow_xor_bit_count);
}

#[test]
fn test_xor_bit_count() {
  let mut rng = random();
  for _ in 0..100 {
    let size = rng.random_range(0..1024);
    let mut a = vec![0u8; size];
    let mut b = vec![0u8; size];
    rng.fill(&mut a[..]);
    rng.fill(&mut b[..]);

    let expected = slow_xor_bit_count(&a, &b);
    assert_eq!(expected, VECTOR_UTIL.xor_bit_count(&a, &b).unwrap());
    assert_eq!(expected, VECTOR_UTIL.xor_bit_count_int(&a, &b));
    assert_eq!(expected, VECTOR_UTIL.xor_bit_count_long(&a, &b));
  }
}

#[test]
fn test_find_next_geq() {
  let mut rng = random();
  let padding = rng.random_range(0..=5);
  let mut values = vec![0i32; 128 + padding];
  let mut v = 0i32;
  for value in values.iter_mut().take(128) {
    v += rng.random_range(1..=1000);
    *value = v;
  }

  for _ in 0..1000 {
    let from = rng.random_range(0..128);
    let target = rng.random_range(values[from]..=values[127]) + rng.random_range(-5..=4);
    assert_eq!(
      slow_find_next_geq(&values, 128, target, from),
      VECTOR_UTIL.find_next_geq(&values, target, from, 128),
    );
  }
}

fn test_basic_xor_bit_count_impl<F>(xor_bit_count: F)
where
  F: Fn(&[u8], &[u8]) -> i32,
{
  assert_eq!(0, xor_bit_count(&[1], &[1]));
  assert_eq!(0, xor_bit_count(&[1, 2, 3], &[1, 2, 3]));
  assert_eq!(1, xor_bit_count(&[1, 2, 3], &[0, 2, 3]));
  assert_eq!(2, xor_bit_count(&[1, 2, 3], &[0, 6, 3]));
  assert_eq!(3, xor_bit_count(&[1, 2, 3], &[0, 6, 7]));
  assert_eq!(4, xor_bit_count(&[1, 2, 3], &[2, 6, 7]));

  assert_eq!(0, xor_bit_count(&[1, 2, 3, 4], &[1, 2, 3, 4]));
  assert_eq!(1, xor_bit_count(&[1, 2, 3, 4], &[0, 2, 3, 4]));
  assert_eq!(0, xor_bit_count(&[1, 2, 3, 4, 5], &[1, 2, 3, 4, 5]));
  assert_eq!(1, xor_bit_count(&[1, 2, 3, 4, 5], &[0, 2, 3, 4, 5]));

  assert_eq!(
    0,
    xor_bit_count(&[1, 2, 3, 4, 5, 6, 7, 8], &[1, 2, 3, 4, 5, 6, 7, 8])
  );
  assert_eq!(
    1,
    xor_bit_count(&[1, 2, 3, 4, 5, 6, 7, 8], &[0, 2, 3, 4, 5, 6, 7, 8])
  );

  assert_eq!(
    0,
    xor_bit_count(&[1, 2, 3, 4, 5, 6, 7, 8, 9], &[1, 2, 3, 4, 5, 6, 7, 8, 9])
  );
  assert_eq!(
    1,
    xor_bit_count(&[1, 2, 3, 4, 5, 6, 7, 8, 9], &[0, 2, 3, 4, 5, 6, 7, 8, 9])
  );
}

fn l2_f32(v: &[f32]) -> f32 {
  v.iter().map(|x| x * x).sum()
}

fn negative_f32(v: &[f32]) -> Vec<f32> {
  v.iter().map(|x| -*x).collect()
}

fn negative_u8(v: &[u8]) -> Vec<u8> {
  v.iter().map(|&x| (-(x as i8)) as u8).collect()
}

fn l2_u8(v: &[u8]) -> f32 {
  v.iter()
    .map(|&x| {
      let x = x as i8 as f32;
      x * x
    })
    .sum()
}

fn random_vector<R>(random: &mut R) -> Vec<f32>
where
  R: Rng + ?Sized,
{
  let dim = random.random_range(1..=100);
  random_vector_dim(random, dim)
}

pub fn random_vector_dim<R>(random: &mut R, dim: usize) -> Vec<f32>
where
  R: Rng + ?Sized,
{
  (0..dim).map(|_| random.random::<f32>()).collect()
}

fn random_vector_bytes<R>(random: &mut R) -> Vec<u8>
where
  R: Rng + ?Sized,
{
  let len = TestUtil::next_usize(random, 1, 100);
  let mut v: BytesRef<Vec<u8>> = TestUtil::random_binary_term_with_len(random, len);
  for i in v.offset..(v.offset + v.length) {
    if v.bytes[i] == i8::MIN as u8 {
      v.bytes[i] = (-127i8) as u8;
    }
  }
  assert_eq!(v.offset, 0);
  v.bytes
}

pub fn random_vector_bytes_dim<R>(random: &mut R, dim: usize) -> Vec<u8>
where
  R: Rng + ?Sized,
{
  let mut v: BytesRef<Vec<u8>> = TestUtil::random_binary_term_with_len(random, dim);
  for i in v.offset..(v.offset + v.length) {
    if v.bytes[i] == i8::MIN as u8 {
      v.bytes[i] = (-127i8) as u8;
    }
  }
  v.bytes
}

fn slow_xor_bit_count(a: &[u8], b: &[u8]) -> i32 {
  let mut res = 0;
  for i in 0..a.len() {
    let mut x = a[i];
    let mut y = b[i];
    for _ in 0..u8::BITS {
      if x == y {
        break;
      }
      if (x & 0x01) != (y & 0x01) {
        res += 1;
      }
      x >>= 1;
      y >>= 1;
    }
  }
  res
}

fn slow_find_next_geq(buffer: &[i32], length: usize, target: i32, from: usize) -> usize {
  for (i, value) in buffer.iter().enumerate().take(length).skip(from) {
    if *value >= target {
      return i;
    }
  }
  length
}

fn bytes(values: &[i8]) -> Vec<u8> {
  values.iter().map(|&v| v as u8).collect()
}

fn assert_approx_eq(expected: f32, actual: f32, delta: f32) {
  assert!(
    (expected - actual).abs() <= delta,
    "expected {expected}, got {actual}, delta {delta}"
  );
}
