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
use crate::core::util::error::lucene_error::Result;
use crate::core::util::unicode_util::{UTF8CodePoint, UnicodeUtil};
use crate::test_framework::core::util::lucene_test_case::{at_least, random};
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestUnicodeUtil;

#[test]
fn test_code_point_count() -> Result<()> {
  // TODO: UnicodeUtil::code_point_count has not been migrated.
  Ok(())
}

#[test]
fn test_utf8_to_utf32() -> Result<()> {
  // TODO: UnicodeUtil::utf8_to_utf32 has not been migrated.
  Ok(())
}

#[test]
fn test_utf8_code_point_at() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 50_000);
  let mut reuse = UTF8CodePoint::default();
  for _ in 0..num {
    let s = TestUtil::random_unicode_string(&mut random);
    let utf8 = s.as_bytes();
    let expected: Vec<i32> = s.chars().map(|ch| ch as i32).collect();
    let mut pos = 0;
    let mut expected_upto = 0;
    while pos < utf8.len() {
      UnicodeUtil::code_point_at(utf8, pos, &mut reuse)?;
      assert_eq!(expected[expected_upto], reuse.code_point);
      expected_upto += 1;
      pos += reuse.num_bytes;
    }
    assert_eq!(utf8.len(), pos);
    assert_eq!(expected.len(), expected_upto);
  }
  Ok(())
}

#[test]
fn test_utf8_span_multiple_bytes() -> Result<()> {
  // TODO: The UTF-8 automaton conversion used by CompiledAutomaton has not been migrated.
  Ok(())
}

#[test]
fn test_new_string() -> Result<()> {
  let code_points = [0x103FF, 0x10FC00, 0xDBFF, 'A' as i32, -1];
  let first = char::from_u32(code_points[0] as u32).unwrap().to_string();
  let second = char::from_u32(code_points[1] as u32).unwrap().to_string();
  let first_second = format!("{first}{second}");

  assert_eq!(first, UnicodeUtil::new_string(&code_points, 0, 1)?);
  assert_eq!(first_second, UnicodeUtil::new_string(&code_points, 0, 2)?);
  assert_eq!(second, UnicodeUtil::new_string(&code_points, 1, 1)?);

  // Rust String cannot contain the unpaired UTF-16 surrogate that Java String accepts, so the
  // Java-success cases containing code point 0xDBFF must fail in Rust.
  assert!(UnicodeUtil::new_string(&code_points, 1, 2).is_err());
  assert!(UnicodeUtil::new_string(&code_points, 1, 3).is_err());
  assert!(UnicodeUtil::new_string(&code_points, 2, 2).is_err());
  assert!(UnicodeUtil::new_string(&code_points, 2, 3).is_err());
  assert!(UnicodeUtil::new_string(&code_points, 4, 5).is_err());
  // TODO: Java's negative count case cannot be expressed by Rust's usize count parameter.
  Ok(())
}

#[test]
fn test_utf8_utf16_chars_ref() -> Result<()> {
  // TODO: UnicodeUtil::utf8_to_utf16 with CharsRef output has not been migrated.
  Ok(())
}

#[test]
fn test_calc_utf16_to_utf8_length() -> Result<()> {
  // TODO: UnicodeUtil::calc_utf16_to_utf8_length has not been migrated.
  Ok(())
}
