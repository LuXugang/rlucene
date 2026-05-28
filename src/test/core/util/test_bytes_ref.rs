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
// Migrated from src/core/index/bytes_ref.rs

use rand::RngExt;
use rand::distr::Alphanumeric;

use crate::core::index::BytesRef;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use crate::test::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestBytesRef {}

#[test]
fn test_empty() {
  let b: BytesRef<Vec<u8>> = BytesRef::new();
  assert_eq!(b.bytes.len(), 0);
  assert_eq!(b.length, 0);
  assert_eq!(b.offset, 0);
}
#[test]
fn test_from_bytes() -> Result<()> {
  let mut bytes: Vec<u8> = "abcd".as_bytes().to_vec();
  let b = BytesRef::from_bytes(bytes.clone());
  assert_eq!(bytes, b.bytes);
  assert_eq!(b.length, 4);
  assert_eq!(b.offset, 0);

  bytes = "abcd".as_bytes().to_vec();
  let b2 = BytesRef::from_slice(bytes, 1, 3);
  let b2_value = b2.utf8_to_string()?;
  assert_eq!("bcd", b2_value);

  assert!(!b.eq(&b2));
  Ok(())
}
#[test]
fn test_from_chars() -> Result<()> {
  let mut random = random();
  let length = random.random_range(1000..100000);
  for _i in 0..100 {
    let s = (&mut random)
      .sample_iter(&Alphanumeric)
      .take(length)
      .map(char::from)
      .collect::<String>();
    let s2: String = BytesRef::<Vec<u8>>::from_string(&s).utf8_to_string()?;
    assert_eq!(s, s2);
  }
  let s = TestUtil::random_unicode_string(&mut random);
  let s2 = BytesRef::<Vec<u8>>::from_string(&s).utf8_to_string()?;
  assert_eq!(s, s2);
  Ok(())
}

#[test]
fn test_invalid_deep_copy() -> Result<()> {
  let mut from = BytesRef::from_bytes(vec![1, 2]);
  from.offset += 1;
  let result = std::panic::catch_unwind(|| {
    BytesRef::deep_copy_of(&from);
  });
  assert!(result.is_err());
  Ok(())
}
