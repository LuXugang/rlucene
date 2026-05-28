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
// Migrated from src/core/util/ints_ref.rs

use crate::core::util::ints_ref::IntsRef;

#[allow(dead_code)] // for quick search
struct TestIntsRef;
#[test]
fn test_empty() {
  let i: IntsRef<Vec<i32>> = IntsRef::default();
  assert!(i.ints.is_empty());
  assert_eq!(0, i.offset);
  assert_eq!(0, i.length);
}

#[test]
fn test_from_ints() {
  let ints = vec![1, 2, 3, 4];
  let rc_ints = ints.clone();
  let i = IntsRef::from_slice(rc_ints.clone(), 0, 4);
  assert_eq!(ints, *i.ints);
  assert_eq!(0, i.offset);
  assert_eq!(4, i.length);

  let i2 = IntsRef::from_slice(rc_ints.clone(), 1, 3);
  let expected = IntsRef::from_slice(vec![2, 3, 4], 0, 3);
  assert_eq!(expected, i2);
  assert_ne!(i, i2);
}

#[test]
#[should_panic]
fn test_invalid_deep_copy() {
  let rc_ints = vec![1, 2];
  let mut from = IntsRef::from_slice(rc_ints, 0, 2);
  from.offset += 1; // now invalid
  let _ = IntsRef::deep_copy_of(&from);
}
