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
// Migrated from src/core/util/longs_ref.rs

use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::longs_ref::LongsRef;

#[allow(dead_code)] // for quick search
struct TestLongsRef;
#[test]
fn test_empty() {
  let i = LongsRef::new();
  assert_eq!(i.longs, LongsRef::new().longs);
  assert_eq!(i.offset, 0);
  assert_eq!(i.length, 0);
}

#[test]
fn test_from_longs() {
  let longs = vec![1, 2, 3, 4];
  let i = LongsRef::from_slice(longs.clone(), 0, 4);
  assert_eq!(i.longs, i.longs);
  assert_eq!(i.offset, 0);
  assert_eq!(i.length, 4);

  let i2 = LongsRef::from_slice(longs.clone(), 1, 3);
  let expected_longs = vec![2, 3, 4];
  let expected = LongsRef::from_slice(expected_longs, 0, 3);
  assert!(i2.eq(&expected));

  assert_ne!(i, i2);
}

#[test]
fn test_invalid_deep_copy() -> Result<()> {
  let mut from = LongsRef::from_slice(vec![1, 2], 0, 2);
  from.offset += 1;
  let result = LongsRef::deep_copy_of(&from);
  assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));
  Ok(())
}
