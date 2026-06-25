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
use crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::core::analysis::token_attributes::packed_token_attribute_impl::PackedTokenAttributeImpl;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::token_attributes::test_char_term_attribute_impl::{
  assert_clone_is_equal, assert_copy_is_equal,
};

#[allow(dead_code)] // for quick search
struct TestPackedTokenAttributeImpl;
#[test]
fn test_clone() -> Result<()> {
  let mut t = PackedTokenAttributeImpl::new()?;
  t.sub.set_offset(0, 5)?;
  let content: Vec<char> = "hello".chars().collect();
  t.copy_buffer(&content, 0, 5)?;
  let copy = assert_clone_is_equal(&t);
  assert_eq!(t.to_string(), copy.to_string());
  Ok(())
}
#[test]
fn test_copy_to() -> Result<()> {
  let t = PackedTokenAttributeImpl::new()?;
  let mut copy = assert_copy_is_equal(&t);
  assert_eq!(t.to_string(), "");
  assert_eq!(copy.to_string(), "");

  let mut t = PackedTokenAttributeImpl::new()?;
  t.sub.set_offset(0, 5)?;
  let content: Vec<char> = "hello".chars().collect();
  t.copy_buffer(&content, 0, 5)?;

  copy = assert_copy_is_equal(&t);
  assert_eq!(t.to_string(), copy.to_string());

  Ok(())
}
#[test]
fn test_packed_token_attribute_factory() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
#[test]
fn test_attribute_reflection() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
