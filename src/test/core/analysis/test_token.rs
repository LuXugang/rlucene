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
use crate::core::analysis::token_attributes::flags_attribute::FlagsAttribute;
use crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::core::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::core::analysis::token_attributes::position_increment_attribute::PositionIncrementAttribute;
use crate::core::analysis::token_attributes::position_length_attribute::PositionLengthAttribute;
use crate::core::analysis::token_attributes::type_attribute::TypeAttribute;
use crate::core::index::BytesRef;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::token;

#[allow(dead_code)] // for quick search
pub struct TestToken;

#[test]
fn test_ctor() -> Result<()> {
  let t = token::with_range(Some("hello"), 0, 0)?;

  assert_eq!(0, t.sub.start_offset());
  assert_eq!(0, t.sub.end_offset());
  assert_eq!(1, t.sub.get_position_increment());
  assert_eq!(1, t.sub.get_position_length());
  assert_eq!("hello", t.to_string());
  assert_eq!("word", t.sub.type_());
  assert_eq!(0, t.sub.token.get_flags());
  assert!(t.sub.token.get_payload().is_none());

  Ok(())
}
#[test]
fn test_clone() -> Result<()> {
  let mut t = token::new()?;
  t.sub.set_offset(0, 5)?;

  let content: Vec<char> = "hello".chars().collect();
  t.copy_buffer(&content, 0, 5)?;

  let copy = t.clone();
  assert_eq!(t.to_string(), copy.to_string());

  let pl = BytesRef::from_bytes(vec![1, 2, 3, 4]);
  t.sub.token.set_payload(Some(pl.clone()));

  let copy = t.clone();
  assert_eq!(&pl, copy.sub.token.get_payload().unwrap(),);

  Ok(())
}
#[test]
fn test_copy_to() -> Result<()> {
  let mut t = token::new()?;
  let mut copy = token::new()?;
  t.copy_to(&mut copy)?;

  assert_eq!("", t.to_string());
  assert_eq!("", copy.to_string());

  t = token::new()?;
  t.sub.set_offset(0, 5)?;

  let content: Vec<char> = "hello".chars().collect();
  t.copy_buffer(&content, 0, 5)?;
  copy = token::new()?;
  t.copy_to(&mut copy)?;
  assert_eq!(t.to_string(), copy.to_string());

  let pl = BytesRef::from_bytes(vec![1, 2, 3, 4]);
  t.sub.token.set_payload(Some(pl.clone()));

  copy = token::new()?;
  t.copy_to(&mut copy)?;
  assert_eq!(&pl, copy.sub.token.get_payload().unwrap());

  Ok(())
}
