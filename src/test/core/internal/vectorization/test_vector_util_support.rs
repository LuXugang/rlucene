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
use crate::core::internal::vectorization::vector_util_support::VectorUtilSupport;
use crate::core::internal::vectorization::vectorization_provider::VectorizationProvider;
use crate::test_framework::core::internal::vectorization::base_vectorization_test_case::BaseVectorizationTestCase;
use crate::test_framework::core::util::lucene_test_case::random;
use crate::test_framework::f64_equals;
use rand::rngs::StdRng;
use rand::{Rng, RngExt};

const DELTA: f64 = 1e-3;
const VECTOR_SIZES: [usize; 22] = [
  1, 4, 6, 8, 13, 16, 25, 32, 64, 100, 128, 207, 256, 300, 512, 702, 1024, 1536, 2046, 2048, 4096,
  4098,
];

struct TestVectorUtilSupport {
  size: usize,
}

impl BaseVectorizationTestCase for TestVectorUtilSupport {}

impl TestVectorUtilSupport {
  fn parameters_factory() -> impl Iterator<Item = Self> {
    VECTOR_SIZES.into_iter().map(|size| Self { size })
  }

  fn test_float_vectors(&self, random: &mut StdRng) {
    let mut a = vec![0.0; self.size];
    let mut b = vec![0.0; self.size];
    for i in 0..self.size {
      a[i] = random.random::<f32>();
      b[i] = random.random::<f32>();
    }
    let lucene = Self::LUCENE_PROVIDER.get_vector_util_support();
    let panama = Self::PANAMA_SUPPORT;
    self.assert_float_returning_providers(
      lucene.dot_product_f32(&a, &b),
      panama.dot_product_f32(&a, &b),
    );
    self.assert_float_returning_providers(
      lucene.square_distance_f32(&a, &b),
      panama.square_distance_f32(&a, &b),
    );
    self.assert_float_returning_providers(lucene.cosine_f32(&a, &b), panama.cosine_f32(&a, &b));
  }

  fn test_binary_vectors(&self, random: &mut StdRng) {
    let mut a = vec![0; self.size];
    let mut b = vec![0; self.size];
    random.fill_bytes(&mut a);
    random.fill_bytes(&mut b);
    let lucene = Self::LUCENE_PROVIDER.get_vector_util_support();
    let panama = Self::PANAMA_SUPPORT;
    self
      .assert_int_returning_providers(lucene.dot_product_u8(&a, &b), panama.dot_product_u8(&a, &b));
    self.assert_int_returning_providers(
      lucene.square_distance_u8(&a, &b),
      panama.square_distance_u8(&a, &b),
    );
    self.assert_float_returning_providers(lucene.cosine_u8(&a, &b), panama.cosine_u8(&a, &b));
  }

  fn test_binary_vectors_boundaries(&self) {
    let mut a = vec![0; self.size];
    let mut b = vec![0; self.size];
    let lucene = Self::LUCENE_PROVIDER.get_vector_util_support();
    let panama = Self::PANAMA_SUPPORT;

    a.fill(i8::MIN as u8);
    b.fill(i8::MIN as u8);
    self
      .assert_int_returning_providers(lucene.dot_product_u8(&a, &b), panama.dot_product_u8(&a, &b));
    self.assert_int_returning_providers(
      lucene.square_distance_u8(&a, &b),
      panama.square_distance_u8(&a, &b),
    );
    self.assert_float_returning_providers(lucene.cosine_u8(&a, &b), panama.cosine_u8(&a, &b));

    a.fill(i8::MAX as u8);
    b.fill(i8::MAX as u8);
    self
      .assert_int_returning_providers(lucene.dot_product_u8(&a, &b), panama.dot_product_u8(&a, &b));
    self.assert_int_returning_providers(
      lucene.square_distance_u8(&a, &b),
      panama.square_distance_u8(&a, &b),
    );
    self.assert_float_returning_providers(lucene.cosine_u8(&a, &b), panama.cosine_u8(&a, &b));

    a.fill(i8::MIN as u8);
    b.fill(i8::MAX as u8);
    self
      .assert_int_returning_providers(lucene.dot_product_u8(&a, &b), panama.dot_product_u8(&a, &b));
    self.assert_int_returning_providers(
      lucene.square_distance_u8(&a, &b),
      panama.square_distance_u8(&a, &b),
    );
    self.assert_float_returning_providers(lucene.cosine_u8(&a, &b), panama.cosine_u8(&a, &b));

    a.fill(i8::MAX as u8);
    b.fill(i8::MIN as u8);
    self
      .assert_int_returning_providers(lucene.dot_product_u8(&a, &b), panama.dot_product_u8(&a, &b));
    self.assert_int_returning_providers(
      lucene.square_distance_u8(&a, &b),
      panama.square_distance_u8(&a, &b),
    );
    self.assert_float_returning_providers(lucene.cosine_u8(&a, &b), panama.cosine_u8(&a, &b));
  }

  fn test_int4_dot_product(&self, random: &mut StdRng) {
    let mut a = vec![0; self.size];
    let mut b = vec![0; self.size];
    for i in 0..self.size {
      a[i] = random.random_range(0..16);
      b[i] = random.random_range(0..16);
    }
    let lucene = Self::LUCENE_PROVIDER.get_vector_util_support();
    let panama = Self::PANAMA_SUPPORT;
    self.assert_int_returning_providers(
      lucene.int4_dot_product(&a, false, &Self::pack(&b), true),
      panama.int4_dot_product(&a, false, &Self::pack(&b), true),
    );
    self.assert_int_returning_providers(
      lucene.int4_dot_product(&Self::pack(&a), true, &b, false),
      panama.int4_dot_product(&Self::pack(&a), true, &b, false),
    );
    assert_eq!(
      lucene.dot_product_u8(&a, &b),
      panama.int4_dot_product(&a, false, &Self::pack(&b), true),
      "size={}",
      self.size,
    );
  }

  fn test_int4_dot_product_boundaries(&self) {
    let max_value = 15;
    let mut a = vec![0; self.size];
    let mut b = vec![0; self.size];
    let lucene = Self::LUCENE_PROVIDER.get_vector_util_support();
    let panama = Self::PANAMA_SUPPORT;

    a.fill(max_value);
    b.fill(max_value);
    self.assert_int_returning_providers(
      lucene.int4_dot_product(&a, false, &Self::pack(&b), true),
      panama.int4_dot_product(&a, false, &Self::pack(&b), true),
    );
    self.assert_int_returning_providers(
      lucene.int4_dot_product(&Self::pack(&a), true, &b, false),
      panama.int4_dot_product(&Self::pack(&a), true, &b, false),
    );
    assert_eq!(
      lucene.dot_product_u8(&a, &b),
      panama.int4_dot_product(&a, false, &Self::pack(&b), true),
      "size={}",
      self.size,
    );

    let min_value = 0;
    a.fill(min_value);
    b.fill(min_value);
    self.assert_int_returning_providers(
      lucene.int4_dot_product(&a, false, &Self::pack(&b), true),
      panama.int4_dot_product(&a, false, &Self::pack(&b), true),
    );
    self.assert_int_returning_providers(
      lucene.int4_dot_product(&Self::pack(&a), true, &b, false),
      panama.int4_dot_product(&Self::pack(&a), true, &b, false),
    );
    assert_eq!(
      lucene.dot_product_u8(&a, &b),
      panama.int4_dot_product(&a, false, &Self::pack(&b), true),
      "size={}",
      self.size,
    );
  }

  fn pack(unpacked: &[u8]) -> Vec<u8> {
    let len = unpacked.len().div_ceil(2);
    let mut packed = vec![0; len];
    for i in 0..len {
      packed[i] = (unpacked[i] << 4) | unpacked[len + i];
    }
    packed
  }

  // Evaluate the same operation on the two concrete support types at the call
  // site, keeping static dispatch instead of a Java-style interface closure.
  fn assert_float_returning_providers(&self, expected: f32, actual: f32) {
    assert!(
      f64_equals(f64::from(expected), f64::from(actual), DELTA),
      "size={}, default={expected:?}, panama={actual:?}",
      self.size,
    );
  }

  fn assert_int_returning_providers(&self, expected: i32, actual: i32) {
    assert_eq!(expected, actual, "size={}", self.size);
  }
}

#[test]
fn test_float_vectors() {
  let mut random = random();
  for test in TestVectorUtilSupport::parameters_factory() {
    test.test_float_vectors(&mut random);
  }
}

#[test]
fn test_binary_vectors() {
  let mut random = random();
  for test in TestVectorUtilSupport::parameters_factory() {
    test.test_binary_vectors(&mut random);
  }
}

#[test]
fn test_binary_vectors_boundaries() {
  for test in TestVectorUtilSupport::parameters_factory() {
    test.test_binary_vectors_boundaries();
  }
}

#[test]
fn test_int4_dot_product() {
  let mut random = random();
  // Java's int4 tests apply only to even parameter sizes.
  for test in TestVectorUtilSupport::parameters_factory().filter(|test| test.size.is_multiple_of(2))
  {
    test.test_int4_dot_product(&mut random);
  }
}

#[test]
fn test_int4_dot_product_boundaries() {
  for test in TestVectorUtilSupport::parameters_factory().filter(|test| test.size.is_multiple_of(2))
  {
    test.test_int4_dot_product_boundaries();
  }
}
