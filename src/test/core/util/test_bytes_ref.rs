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

use crate::test_framework::core::util::lucene_test_case::random;

use crate::core::index::BytesRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestBytesRef;

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
  for _i in 0..100 {
    let s = TestUtil::random_unicode_string(&mut random);
    let s2: String = BytesRef::<Vec<u8>>::from_string(&s).utf8_to_string()?;
    assert_eq!(s, s2);
  }
  // only for 4.x
  assert_eq!(
    "\u{ffff}",
    BytesRef::<Vec<u8>>::from_string("\u{ffff}").utf8_to_string()?
  );
  Ok(())
}

#[test]
fn test_invalid_deep_copy() -> Result<()> {
  let mut from = BytesRef::from_bytes(vec![1, 2]);
  from.offset += 1;
  let result = BytesRef::deep_copy_of(&from);
  assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));
  Ok(())
}
