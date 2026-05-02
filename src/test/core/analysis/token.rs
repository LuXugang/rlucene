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
use crate::core::analysis::token_attributes::char_term_attribute_impl::CharTermAttributeImpl;
use crate::core::analysis::token_attributes::flags_attribute::FlagsAttribute;
use crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::core::analysis::token_attributes::packed_token_attribute_impl::PackedTokenAttributeImpl;
use crate::core::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::core::analysis::token_attributes::position_increment_attribute::PositionIncrementAttribute;
use crate::core::analysis::token_attributes::position_length_attribute::PositionLengthAttribute;
use crate::core::index::BytesRef;
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
/// A [`TokenBase`] is an occurrence of a term from the text of a field. It consists of a term's text, the
/// start and end offset of the term in the text of the field, and a type string.
pub type Token = CharTermAttributeImpl<PackedTokenAttributeImpl>;
#[derive(Clone)]
pub struct TokenBase {
  flags: i32,
  payload: Option<BytesRef<Vec<u8>>>,
  attribute: HashSet<String>,
}
pub fn new() -> Result<CharTermAttributeImpl<PackedTokenAttributeImpl>> {
  PackedTokenAttributeImpl::new()
}
/// Constructs a [`TokenBase`] with the given term text, start and end offsets. The type defaults to
/// "word." **NOTE:** for better indexing speed you should instead use the `char[] termBuffer`
/// methods to set the term text.
///
/// # Parameters
///
/// - `text` - term text
/// - `start` - start offset in the source text
/// - `end` - end offset in the source text
pub fn with_range(
  text: Option<&str>,
  start: i32,
  end: i32,
) -> Result<CharTermAttributeImpl<PackedTokenAttributeImpl>> {
  let mut base = PackedTokenAttributeImpl::new()?;
  base.append_str(text);
  base.sub.set_offset(start, end)?;
  Ok(base)
}
/// Constructs a [`TokenBase`] with the given term text, position increment, start and end offsets.
pub fn with_pos_inc(
  text: &str,
  pos_inc: i32,
  start: i32,
  end: i32,
) -> Result<CharTermAttributeImpl<PackedTokenAttributeImpl>> {
  let mut base = PackedTokenAttributeImpl::new()?;
  base.append_str(Some(text));
  base.sub.set_offset(start, end)?;
  base.sub.set_position_increment(pos_inc)?;
  Ok(base)
}

pub fn with_all(
  text: &str,
  pos_inc: i32,
  start: i32,
  end: i32,
  pos_length: i32,
) -> Result<CharTermAttributeImpl<PackedTokenAttributeImpl>> {
  let mut base = PackedTokenAttributeImpl::new()?;
  base.append_str(Some(text));
  base.sub.set_offset(start, end)?;
  base.sub.set_position_increment(pos_inc)?;
  base.sub.set_position_length(pos_length)?;
  Ok(base)
}
impl Default for TokenBase {
  fn default() -> Self {
    let mut v = TokenBase {
      flags: 0,
      payload: None,
      attribute: HashSet::new(),
    };
    v.attribute
      .insert(<TokenBase as FlagsAttribute>::ATTRIBUTE_NAME.to_string());
    v.attribute
      .insert(<TokenBase as PayloadAttribute>::ATTRIBUTE_NAME.to_string());
    v
  }
}

impl Attribute for TokenBase {
  #[cfg(debug_assertions)]
  fn get_attribute_name(&self) -> Result<&HashSet<String>> {
    Ok(&self.attribute)
  }
}

impl FlagsAttribute for TokenBase {
  fn get_flags(&self) -> i32 {
    self.flags
  }

  fn set_flags(&mut self, flags: i32) {
    self.flags = flags;
  }
}
impl PayloadAttribute for TokenBase {
  fn get_payload(&self) -> Option<&BytesRef<Vec<u8>>> {
    self.payload.as_ref()
  }

  fn set_payload(&mut self, payload: Option<BytesRef<Vec<u8>>>) {
    self.payload = payload;
  }
}

impl AttributeImpl for TokenBase {
  fn clear(&mut self) {
    self.flags = 0;
    self.payload = None;
  }

  type AttributeImpl = Self;

  fn copy_to(&self, other: &mut Self::AttributeImpl) -> Result<()> {
    other.set_flags(self.flags);
    other.set_payload(self.payload.as_ref().map(BytesRef::deep_copy_of));
    Ok(())
  }
}
impl PartialEq for TokenBase {
  fn eq(&self, other: &Self) -> bool {
    self.flags == other.flags && self.payload == other.payload
  }
}
impl Eq for TokenBase {}
impl Hash for TokenBase {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.flags.hash(state);
    self.payload.hash(state)
  }
}
