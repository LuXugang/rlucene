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
use crate::core::index::terms_enum_index::prefix8_to_comparable_unsigned_long;

#[allow(dead_code)] // for quick search
struct TestTermsEnumIndex;

#[test]
fn test_prefix8_to_comparable_unsigned_long() {
  let b = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

  assert_eq!(
    0u64,
    prefix8_to_comparable_unsigned_long(&BytesRef {
      bytes: b.clone(),
      offset: 1,
      length: 0,
    })
  );

  assert_eq!(
    4u64 << 56,
    prefix8_to_comparable_unsigned_long(&BytesRef {
      bytes: b.clone(),
      offset: 3,
      length: 1,
    })
  );

  assert_eq!(
    (4u64 << 56) | (5u64 << 48),
    prefix8_to_comparable_unsigned_long(&BytesRef {
      bytes: b.clone(),
      offset: 3,
      length: 2,
    })
  );

  assert_eq!(
    (4u64 << 56) | (5u64 << 48) | (6u64 << 40),
    prefix8_to_comparable_unsigned_long(&BytesRef {
      bytes: b.clone(),
      offset: 3,
      length: 3,
    })
  );

  assert_eq!(
    (4u64 << 56) | (5u64 << 48) | (6u64 << 40) | (7u64 << 32),
    prefix8_to_comparable_unsigned_long(&BytesRef {
      bytes: b.clone(),
      offset: 3,
      length: 4,
    })
  );

  assert_eq!(
    (4u64 << 56) | (5u64 << 48) | (6u64 << 40) | (7u64 << 32) | (8u64 << 24),
    prefix8_to_comparable_unsigned_long(&BytesRef {
      bytes: b.clone(),
      offset: 3,
      length: 5,
    })
  );

  assert_eq!(
    (4u64 << 56) | (5u64 << 48) | (6u64 << 40) | (7u64 << 32) | (8u64 << 24) | (9u64 << 16),
    prefix8_to_comparable_unsigned_long(&BytesRef {
      bytes: b.clone(),
      offset: 3,
      length: 6,
    })
  );

  assert_eq!(
    (4u64 << 56)
      | (5u64 << 48)
      | (6u64 << 40)
      | (7u64 << 32)
      | (8u64 << 24)
      | (9u64 << 16)
      | (10u64 << 8),
    prefix8_to_comparable_unsigned_long(&BytesRef {
      bytes: b.clone(),
      offset: 3,
      length: 7,
    })
  );

  assert_eq!(
    (4u64 << 56)
      | (5u64 << 48)
      | (6u64 << 40)
      | (7u64 << 32)
      | (8u64 << 24)
      | (9u64 << 16)
      | (10u64 << 8)
      | 11u64,
    prefix8_to_comparable_unsigned_long(&BytesRef {
      bytes: b.clone(),
      offset: 3,
      length: 8,
    })
  );

  assert_eq!(
    (4u64 << 56)
      | (5u64 << 48)
      | (6u64 << 40)
      | (7u64 << 32)
      | (8u64 << 24)
      | (9u64 << 16)
      | (10u64 << 8)
      | 11u64,
    prefix8_to_comparable_unsigned_long(&BytesRef {
      bytes: b,
      offset: 3,
      length: 9,
    })
  );
}
