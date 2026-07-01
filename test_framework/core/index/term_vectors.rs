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
use crate::analysis::token_attributes::payload_attribute_impl::PayloadAttributeImpl;
use crate::core::analysis::token_attributes::char_term_attribute::CharTermAttribute;
use crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::core::analysis::token_attributes::packed_token_attribute_impl::{
  PackedTokenAttribute, PackedTokenAttributeImpl,
};
use crate::core::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::core::analysis::token_attributes::term_to_bytes_ref_attribute::TermToBytesRefAttribute;
use crate::core::index::BytesRef;
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::attribute_source::AttributeSource;
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;
#[cfg(debug_assertions)]
use std::collections::HashSet;

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct PermissiveOffsetAttributeImpl {
  start: i32,
  end: i32,
}

impl PermissiveOffsetAttributeImpl {
  fn new() -> Self {
    PermissiveOffsetAttributeImpl { start: 0, end: 0 }
  }
}

impl Attribute for PermissiveOffsetAttributeImpl {}

impl OffsetAttribute for PermissiveOffsetAttributeImpl {
  fn start_offset(&self) -> i32 {
    self.start
  }

  fn set_offset(&mut self, start_offset: i32, end_offset: i32) -> Result<()> {
    self.start = start_offset;
    self.end = end_offset;
    Ok(())
  }

  fn end_offset(&self) -> i32 {
    self.end
  }
}

impl AttributeImpl for PermissiveOffsetAttributeImpl {
  fn clear(&mut self) {
    self.start = 0;
    self.end = 0;
  }

  type AttributeImpl = Self;

  fn copy_to(&self, other: &mut Self::AttributeImpl) -> Result<()> {
    other.set_offset(self.start, self.end)
  }
}

pub struct RandomTokenStreamAttr {
  packed: PackedTokenAttribute,
  o_att: PermissiveOffsetAttributeImpl,
  p_att: PayloadAttributeImpl,
  #[cfg(debug_assertions)]
  attribute: HashSet<String>,
}

impl RandomTokenStreamAttr {
  pub(crate) fn new() -> Result<Self> {
    let packed = PackedTokenAttributeImpl::new()?;
    let p_att = PayloadAttributeImpl::new();
    #[cfg(debug_assertions)]
    let mut attribute = HashSet::new();
    #[cfg(debug_assertions)]
    {
      attribute.extend(packed.get_attribute_name()?.clone());
      attribute
        .insert(<PermissiveOffsetAttributeImpl as OffsetAttribute>::ATTRIBUTE_NAME.to_string());
      attribute.extend(p_att.get_attribute_name()?.clone());
    }
    Ok(Self {
      packed,
      o_att: PermissiveOffsetAttributeImpl::new(),
      p_att,
      #[cfg(debug_assertions)]
      attribute,
    })
  }
}

impl Attribute for RandomTokenStreamAttr {
  #[cfg(debug_assertions)]
  fn get_attribute_name(&self) -> Result<&HashSet<String>> {
    Ok(&self.attribute)
  }
}

impl std::fmt::Display for RandomTokenStreamAttr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.packed.fmt(f)
  }
}

impl AttributeSource for RandomTokenStreamAttr {
  fn length(&self) -> Result<usize> {
    Ok(CharTermAttribute::length(&self.packed))
  }

  fn copy_buffer(&mut self, buffer: &[char], offset: usize, length: usize) -> Result<()> {
    CharTermAttribute::copy_buffer(&mut self.packed, buffer, offset, length)
  }

  fn buffer_mut(&mut self) -> Result<&mut [char]> {
    Ok(CharTermAttribute::buffer_mut(&mut self.packed))
  }

  fn buffer(&self) -> Result<&[char]> {
    Ok(CharTermAttribute::buffer(&self.packed))
  }

  fn resize_buffer(&mut self, new_size: usize) -> Result<&mut [char]> {
    CharTermAttribute::resize_buffer(&mut self.packed, new_size)
  }

  fn set_length(&mut self, length: usize) -> Result<&mut Self> {
    CharTermAttribute::set_length(&mut self.packed, length)?;
    Ok(self)
  }

  fn set_empty(&mut self) -> Result<&mut Self> {
    CharTermAttribute::set_empty(&mut self.packed);
    Ok(self)
  }

  fn append_range(&mut self, csq: Option<&str>, start: usize, end: usize) -> Result<&mut Self> {
    CharTermAttribute::append_range(&mut self.packed, csq, start, end)?;
    Ok(self)
  }

  fn append_char(&mut self, c: char) -> Result<&mut Self> {
    CharTermAttribute::append_char(&mut self.packed, c)?;
    Ok(self)
  }

  fn append_str(&mut self, s: Option<&str>) -> Result<&mut Self> {
    CharTermAttribute::append_str(&mut self.packed, s)?;
    Ok(self)
  }

  fn append_term_attribute<C>(&mut self, term_att: Option<&mut C>) -> Result<&mut Self>
  where
    C: CharTermAttribute,
  {
    CharTermAttribute::append_term_attribute(&mut self.packed, term_att)?;
    Ok(self)
  }

  fn start_offset(&self) -> Result<i32> {
    self.packed.start_offset()
  }

  fn set_offset(&mut self, start_offset: i32, end_offset: i32) -> Result<()> {
    self.packed.set_offset(start_offset, end_offset)
  }

  fn end_offset(&self) -> Result<i32> {
    self.packed.end_offset()
  }

  fn get_position_increment(&self) -> Result<i32> {
    self.packed.get_position_increment()
  }

  fn set_position_increment(&mut self, position_increment: i32) -> Result<()> {
    self.packed.set_position_increment(position_increment)
  }

  fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
    Ok(self.p_att.get_payload())
  }

  fn set_payload(&mut self, payload: Option<BytesRef<Vec<u8>>>) -> Result<()> {
    self.p_att.set_payload(payload);
    Ok(())
  }

  fn get_bytes_ref(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(TermToBytesRefAttribute::get_bytes_ref(&mut self.packed))
  }

  fn set_term_frequency(&mut self, term_frequency: i32) -> Result<()> {
    self.packed.set_term_frequency(term_frequency)
  }

  fn get_term_frequency(&self) -> Result<i32> {
    self.packed.get_term_frequency()
  }

  fn set_position_length(&mut self, position_length: i32) -> Result<()> {
    self.packed.set_position_length(position_length)
  }

  fn get_position_length(&self) -> Result<i32> {
    self.packed.get_position_length()
  }

  fn get_flags(&self) -> Result<i32> {
    self.packed.get_flags()
  }

  #[cfg(test)]
  fn set_flags(&mut self, flags: i32) -> Result<()> {
    self.packed.set_flags(flags)
  }

  fn type_(&self) -> Result<&str> {
    self.packed.type_()
  }

  fn set_type(&mut self, type_: &str) -> Result<()> {
    self.packed.set_type(type_)
  }

  fn end_attributes(&mut self) {
    self.packed.end_attributes();
    self.o_att.end();
    self.p_att.end();
  }

  fn clear_attributes(&mut self) {
    self.packed.clear_attributes();
    self.o_att.clear();
    self.p_att.clear();
  }
}
