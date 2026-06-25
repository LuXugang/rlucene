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

use crate::core::analysis::token_attributes::char_term_attribute::CharTermAttribute;
use crate::core::analysis::token_attributes::char_term_attribute_impl::{
  CharTermAttributeImpl, EmptyAttributeImpl,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use regex::Regex;
use std::hash::{DefaultHasher, Hash, Hasher};

#[allow(deprecated)] // for quick search
struct TestCharTermAttributeImpl;

#[test]
fn test_resize() -> Result<()> {
  let mut t = CharTermAttributeImpl::new().unwrap();
  let content: Vec<char> = "hello".chars().collect();
  t.copy_buffer(&content, 0, content.len())?;

  for i in 0..2000 {
    let buf = t.resize_buffer(i)?;
    assert!(
      i <= buf.len(),
      "buffer.len() = {}, expected >= {}",
      buf.len(),
      i
    );
    assert_eq!(t.to_string(), "hello");
  }
  Ok(())
}
#[test]
fn test_set_length_oob() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
#[test]
fn test_grow() -> Result<()> {
  let mut t = CharTermAttributeImpl::new().unwrap();
  let mut buf = String::from("ab");
  for _ in 0..20 {
    let chars: Vec<char> = buf.chars().collect();
    t.copy_buffer(&chars, 0, chars.len())?;
    assert_eq!(buf.len(), t.length());
    assert_eq!(buf, t.to_string());
    buf.push_str(&buf.clone());
  }
  assert_eq!(1_048_576, t.length());

  let mut t = CharTermAttributeImpl::new().unwrap();
  let mut buf = String::from("ab");
  for _ in 0..20 {
    t.set_empty().append_str(Some(&buf))?;
    assert_eq!(buf.len(), t.length());
    assert_eq!(buf, t.to_string());
    buf.push_str(&t.to_string());
  }
  assert_eq!(1_048_576, t.length());

  let mut t = CharTermAttributeImpl::new().unwrap();
  let mut buf = String::from("a");
  for _ in 0..20_000 {
    t.set_empty().append_str(Some(&buf))?;
    assert_eq!(buf.len(), t.length());
    assert_eq!(buf, t.to_string());
    buf.push('a');
  }
  assert_eq!(20_000, t.length());
  Ok(())
}
#[test]
fn test_to_string() -> Result<()> {
  let mut t = CharTermAttributeImpl::new().unwrap();
  let b: Vec<char> = ['a', 'l', 'o', 'h', 'a'].to_vec();
  t.copy_buffer(&b, 0, 5)?;
  assert_eq!(t.to_string(), "aloha");

  t.set_empty().append_str(Some("hi there"))?;
  assert_eq!(t.to_string(), "hi there");
  Ok(())
}

#[test]
fn test_clone() -> Result<()> {
  let mut t = CharTermAttributeImpl::new().unwrap();
  let content: Vec<char> = "hello".chars().collect();
  t.copy_buffer(&content, 0, 5)?;

  let copy = assert_clone_is_equal(&t);
  assert_eq!(t.to_string(), copy.to_string());
  Ok(())
}

#[test]
fn test_equals() -> Result<()> {
  let mut t1a = CharTermAttributeImpl::new().unwrap();
  let content1a: Vec<char> = "hello".chars().collect();
  t1a.copy_buffer(&content1a, 0, 5)?;

  let mut t1b = CharTermAttributeImpl::new().unwrap();
  let content1b: Vec<char> = "hello".chars().collect();
  t1b.copy_buffer(&content1b, 0, 5)?;

  let mut t2 = CharTermAttributeImpl::new().unwrap();
  let content2: Vec<char> = "hello2".chars().collect();
  t2.copy_buffer(&content2, 0, 6)?;

  assert!(t1a == t1b);
  assert!(t1a != t2);
  assert!(t2 != t1b);
  Ok(())
}
#[test]
fn test_copy_to() -> Result<()> {
  let t = CharTermAttributeImpl::new().unwrap();
  let copy = assert_copy_is_equal(&t);
  assert_eq!(t.to_string(), "");
  assert_eq!(copy.to_string(), "");

  let mut t = CharTermAttributeImpl::new().unwrap();
  let content: Vec<char> = "hello".chars().collect();
  t.copy_buffer(&content, 0, 5)?;

  let copy = assert_copy_is_equal(&t);
  assert_eq!(t.to_string(), copy.to_string());
  Ok(())
}

#[test]
fn test_attribute_reflection() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
#[test]
fn test_char_sequence_interface() -> Result<()> {
  let s = "0123456789";
  let mut t = CharTermAttributeImpl::new().unwrap();
  t.append_str(Some(s))?;

  assert_eq!(s.len(), t.length());

  let sub_sub_sequence: String = t.sub_sequence(1, 3)?.iter().collect();
  assert_eq!("12", sub_sub_sequence);
  let sub_sub_sequence: String = t.sub_sequence(0, s.len())?.iter().collect();
  assert_eq!(s.to_string(), sub_sub_sequence);

  let re_full = Regex::new(r"^01\d+$").unwrap();
  assert!(re_full.is_match(&t.to_string()));

  let re_sub = Regex::new(r"^34$").unwrap();
  let sub_sub_sequence: String = t.sub_sequence(3, 5)?.iter().collect();
  assert!(re_sub.is_match(&sub_sub_sequence));

  let sub_sub_sequence: String = t.sub_sequence(3, 7)?.iter().collect();
  assert_eq!(s[3..7].to_string(), sub_sub_sequence);

  for (i, ch) in s.chars().enumerate() {
    assert_eq!(ch, t.char_at(i)?)
  }
  Ok(())
}
#[test]
fn test_appendable_interface() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
#[test]
fn test_appendable_interface_with_longsequences() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
#[test]
fn test_non_char_sequence_append() -> Result<()> {
  let mut t = CharTermAttributeImpl::new().unwrap();

  t.append_str(Some("0123456789"))?
    .append_str(Some("0123456789"))?;
  assert_eq!(t.to_string(), "01234567890123456789");

  let sb = String::from("0123456789");
  t.append_str(Some(&sb))?;
  assert_eq!(t.to_string(), "012345678901234567890123456789");

  let mut t2 = CharTermAttributeImpl::new().unwrap();
  t2.append_str(Some("test"))?;
  t.append_term_attribute(Some(&mut t2))?;
  assert_eq!(t.to_string(), "012345678901234567890123456789test");

  t.append_str(None)?
    .append_str(None)?
    .append_term_attribute::<CharTermAttributeImpl<EmptyAttributeImpl>>(None)?;
  assert_eq!(
    t.to_string(),
    "012345678901234567890123456789testnullnullnull"
  );
  Ok(())
}

#[test]
fn test_exceptions() -> Result<()> {
  let mut t = CharTermAttributeImpl::new().unwrap();
  t.append_str(Some("test"))?;
  assert_eq!(t.to_string(), "test");

  let v = t.char_at(4);
  matches!(v, Err(LuceneError::ArrayIndexOutOfBounds(_)));

  let v = t.sub_sequence(0, 5);
  matches!(v, Err(LuceneError::ArrayIndexOutOfBounds(_)));

  let v = t.sub_sequence(5, 0);
  matches!(v, Err(LuceneError::ArrayIndexOutOfBounds(_)));
  Ok(())
}
pub fn assert_clone_is_equal<T>(att: &T) -> T
where
  T: Clone + PartialEq + Hash,
{
  let cloned = att.clone();
  assert!(att == &cloned);
  let mut hash1 = DefaultHasher::new();
  att.hash(&mut hash1);
  let mut hash2 = DefaultHasher::new();
  cloned.hash(&mut hash2);
  assert_eq!(
    hash1.finish(),
    hash2.finish(),
    "Clone's hashcode must be equal"
  );
  cloned
}
pub fn assert_copy_is_equal<T>(att: &T) -> T
where
  T: Clone + PartialEq + Hash,
{
  let copy = att.clone();

  assert!(att == &copy);

  let mut hash1 = DefaultHasher::new();
  att.hash(&mut hash1);
  let mut hash2 = DefaultHasher::new();
  copy.hash(&mut hash2);
  assert_eq!(
    hash1.finish(),
    hash2.finish(),
    "Copid instance's hashcode must be equal"
  );

  copy
}
