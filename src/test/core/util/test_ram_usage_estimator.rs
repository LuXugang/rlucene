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
use crate::core::index::BytesRef;
use crate::core::util::ByteBlockPool;
use crate::core::util::accountable::Accountable;
use crate::core::util::allocator_byte::DirectAllocatorByte;
use crate::core::util::bytes_ref_hash::BytesRefHash;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::ram_usage_estimator::{
  PrimitiveType, primitive_size, size_of_accountable, size_of_string, size_of_string_vec,
  size_of_vec,
};
use crate::test_framework::core::util::lucene_test_case::random;
use rand::RngExt;
use std::sync::LazyLock;

#[allow(dead_code)] // for quick search
struct TestRamUsageEstimator;

static STRINGS: LazyLock<Vec<String>> = LazyLock::new(|| {
  vec![
    "test string".to_string(),
    "hollow".to_string(),
    "catchmaster".to_string(),
  ]
});

#[test]
fn test_static_overloads() -> Result<()> {
  let mut random = random();
  {
    let array = vec![0_u8; random.random_range(0..1024)];
    assert_eq!(
      array.capacity() as i64 * primitive_size(PrimitiveType::Byte),
      size_of_vec(&array)
    );
  }
  {
    let array = vec![false; random.random_range(0..1024)];
    assert_eq!(
      array.capacity() as i64 * primitive_size(PrimitiveType::Boolean),
      size_of_vec(&array)
    );
  }
  {
    let array = vec!['\0'; random.random_range(0..1024)];
    assert_eq!(
      array.capacity() as i64 * primitive_size(PrimitiveType::Char),
      size_of_vec(&array)
    );
  }
  {
    let array = vec![0_i16; random.random_range(0..1024)];
    assert_eq!(
      array.capacity() as i64 * primitive_size(PrimitiveType::Short),
      size_of_vec(&array)
    );
  }
  {
    let array = vec![0_i32; random.random_range(0..1024)];
    assert_eq!(
      array.capacity() as i64 * primitive_size(PrimitiveType::Int),
      size_of_vec(&array)
    );
  }
  {
    let array = vec![0_f32; random.random_range(0..1024)];
    assert_eq!(
      array.capacity() as i64 * primitive_size(PrimitiveType::Float),
      size_of_vec(&array)
    );
  }
  {
    let array = vec![0_i64; random.random_range(0..1024)];
    assert_eq!(
      array.capacity() as i64 * primitive_size(PrimitiveType::Long),
      size_of_vec(&array)
    );
  }
  {
    let array = vec![0_f64; random.random_range(0..1024)];
    assert_eq!(
      array.capacity() as i64 * primitive_size(PrimitiveType::Double),
      size_of_vec(&array)
    );
  }
  Ok(())
}

#[test]
fn test_strings() -> Result<()> {
  let expected = size_of_vec(&STRINGS)
    + STRINGS
      .iter()
      .map(size_of_string)
      .fold(0_i64, i64::saturating_add);
  assert_eq!(expected, size_of_string_vec(&STRINGS));
  Ok(())
}

#[test]
fn test_bytes_ref_hash() -> Result<()> {
  let mut pool = ByteBlockPool::new(DirectAllocatorByte::new());
  let mut bytes = BytesRefHash::new()?;
  for i in 0..100 {
    bytes.add(&BytesRef::from_string(&format!("foo bar {i}")), &mut pool)?;
    bytes.add(&BytesRef::from_string(&format!("baz bam {i}")), &mut pool)?;
  }
  let actual = bytes.ram_bytes_used_with_pool(&pool)?;
  let estimated = size_of_accountable(&bytes)?.saturating_add(pool.ram_bytes_used()?);
  assert_eq!(actual, estimated);
  bytes.close(&mut pool);
  Ok(())
}

#[test]
#[ignore = "Java-only: object-reference width and compressed references are JVM layout properties"]
fn test_reference_size() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: HotSpot management beans and VM flags have no Rust runtime equivalent"]
fn test_hotspot_bean() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
#[ignore = "Java-only: this diagnostic prints JVM object-layout constants"]
fn test_print_values() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
