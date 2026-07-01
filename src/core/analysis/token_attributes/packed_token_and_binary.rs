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
use crate::core::analysis::token_attributes::bytes_term_attribute_impl::BytesTermAttributeImpl;
use crate::core::analysis::token_attributes::char_term_attribute::CharTermAttribute;
use crate::core::analysis::token_attributes::char_term_attribute_impl::CharTermAttributeImpl;
use crate::core::analysis::token_attributes::packed_token_attribute_impl::PackedTokenAttributeImpl;
use crate::core::index::BytesRef;
#[cfg(debug_assertions)]
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::attribute_source::AttributeSource;
use crate::core::util::error::lucene_error::Result;
#[cfg(test)]
use crate::test::support::core::analysis::base_token_stream_test_case::CheckClearAttributesAttribute;
#[cfg(test)]
use crate::test::support::core::analysis::base_token_stream_test_case::CheckClearAttributesAttributeImpl;
use std::borrow::Cow;
#[cfg(debug_assertions)]
use std::collections::HashSet;
use std::fmt::Display;

pub struct BinaryTokenStreamAttributeImpl {
  packed_token: CharTermAttributeImpl<PackedTokenAttributeImpl>,
  binary: BytesTermAttributeImpl,
  #[cfg(test)]
  check_clear_attributes: CheckClearAttributesAttributeImpl,
  #[cfg(debug_assertions)]
  attribute: HashSet<String>,
}

impl BinaryTokenStreamAttributeImpl {
  pub fn new() -> Result<Self> {
    let packed_token = PackedTokenAttributeImpl::new()?;
    let binary = BytesTermAttributeImpl::default();
    // TODO is there a better way to do this?
    #[cfg(debug_assertions)]
    let mut attribute = HashSet::new();
    #[cfg(debug_assertions)]
    {
      attribute.extend(packed_token.get_attribute_name()?.clone());
      attribute.extend(binary.get_attribute_name()?.clone());
    }
    Ok(Self {
      packed_token,
      binary,
      #[cfg(test)]
      check_clear_attributes: CheckClearAttributesAttributeImpl::new(),
      #[cfg(debug_assertions)]
      attribute,
    })
  }
}
impl BinaryTokenStreamAttributeImpl {
  pub fn get_packed_token(&self) -> &CharTermAttributeImpl<PackedTokenAttributeImpl> {
    &self.packed_token
  }
  pub fn get_packed_token_mut(&mut self) -> &mut CharTermAttributeImpl<PackedTokenAttributeImpl> {
    &mut self.packed_token
  }
  pub fn get_binary(&self) -> &BytesTermAttributeImpl {
    &self.binary
  }
  pub fn get_binary_mut(&mut self) -> &mut BytesTermAttributeImpl {
    &mut self.binary
  }
  pub fn clear(&mut self) {
    self.binary.clear();
    self.packed_token.clear()
  }
}
impl Display for BinaryTokenStreamAttributeImpl {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.packed_token.fmt(f)
  }
}

impl AttributeSource for BinaryTokenStreamAttributeImpl {
  fn length(&self) -> Result<usize> {
    AttributeSource::length(&self.packed_token)
  }

  fn copy_buffer(&mut self, buffer: &[char], offset: usize, length: usize) -> Result<()> {
    AttributeSource::copy_buffer(&mut self.packed_token, buffer, offset, length)
  }

  fn buffer_mut(&mut self) -> Result<&mut [char]> {
    AttributeSource::buffer_mut(&mut self.packed_token)
  }

  fn buffer(&self) -> Result<&[char]> {
    AttributeSource::buffer(&self.packed_token)
  }

  fn resize_buffer(&mut self, new_size: usize) -> Result<&mut [char]> {
    AttributeSource::resize_buffer(&mut self.packed_token, new_size)
  }

  fn set_length(&mut self, length: usize) -> Result<&mut Self> {
    AttributeSource::set_length(&mut self.packed_token, length)?;
    Ok(self)
  }

  fn set_empty(&mut self) -> Result<&mut Self> {
    AttributeSource::set_empty(&mut self.packed_token)?;
    Ok(self)
  }

  fn append_range(&mut self, csq: Option<&str>, start: usize, end: usize) -> Result<&mut Self> {
    AttributeSource::append_range(&mut self.packed_token, csq, start, end)?;
    Ok(self)
  }

  fn append_char(&mut self, c: char) -> Result<&mut Self> {
    AttributeSource::append_char(&mut self.packed_token, c)?;
    Ok(self)
  }

  fn append_str(&mut self, s: Option<&str>) -> Result<&mut Self> {
    AttributeSource::append_str(&mut self.packed_token, s)?;
    Ok(self)
  }

  fn append_term_attribute<C>(&mut self, term_att: Option<&mut C>) -> Result<&mut Self>
  where
    C: CharTermAttribute,
  {
    AttributeSource::append_term_attribute(&mut self.packed_token, term_att)?;
    Ok(self)
  }

  fn start_offset(&self) -> Result<i32> {
    self.packed_token.start_offset()
  }

  fn set_offset(&mut self, start_offset: i32, end_offset: i32) -> Result<()> {
    self.packed_token.set_offset(start_offset, end_offset)
  }

  fn end_offset(&self) -> Result<i32> {
    self.packed_token.end_offset()
  }

  fn set_bytes_ref(&mut self, bytes: Option<BytesRef<Vec<u8>>>) -> Result<()> {
    AttributeSource::set_bytes_ref(&mut self.binary, bytes)
  }

  fn get_position_increment(&self) -> Result<i32> {
    self.packed_token.get_position_increment()
  }

  fn set_position_increment(&mut self, position_increment: i32) -> Result<()> {
    self.packed_token.set_position_increment(position_increment)
  }

  fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
    self.packed_token.get_payload()
  }

  fn set_payload(&mut self, payload: Option<BytesRef<Vec<u8>>>) -> Result<()> {
    self.packed_token.set_payload(payload)
  }

  fn get_bytes_ref(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    AttributeSource::get_bytes_ref(&mut self.binary)
  }

  fn set_term_frequency(&mut self, term_frequency: i32) -> Result<()> {
    self.packed_token.set_term_frequency(term_frequency)
  }

  fn get_term_frequency(&self) -> Result<i32> {
    self.packed_token.get_term_frequency()
  }

  fn set_position_length(&mut self, position_length: i32) -> Result<()> {
    self.packed_token.set_position_length(position_length)
  }

  fn get_position_length(&self) -> Result<i32> {
    self.packed_token.get_position_length()
  }

  fn get_flags(&self) -> Result<i32> {
    self.packed_token.get_flags()
  }

  fn set_flags(&mut self, flags: i32) -> Result<()> {
    self.packed_token.set_flags(flags)
  }

  fn type_(&self) -> Result<&str> {
    self.packed_token.type_()
  }

  fn set_type(&mut self, type_: &str) -> Result<()> {
    self.packed_token.set_type(type_)
  }

  fn end_attributes(&mut self) {
    self.packed_token.end_attributes()
  }

  fn clear_attributes(&mut self) {
    self.clear()
  }
}

#[cfg(test)]
impl AttributeImpl for BinaryTokenStreamAttributeImpl {
  fn clear(&mut self) {
    self.check_clear_attributes.clear();
  }

  type AttributeImpl = CheckClearAttributesAttributeImpl;

  fn copy_to(&self, other: &mut Self::AttributeImpl) -> Result<()> {
    self.check_clear_attributes.copy_to(other)
  }
}

#[cfg(debug_assertions)]
impl Attribute for BinaryTokenStreamAttributeImpl {
  fn get_attribute_name(&self) -> Result<&HashSet<String>> {
    Ok(&self.attribute)
  }
}

#[cfg(test)]
impl Clone for BinaryTokenStreamAttributeImpl {
  fn clone(&self) -> Self {
    unreachable!("")
  }
}

#[cfg(test)]
impl CheckClearAttributesAttribute for BinaryTokenStreamAttributeImpl {
  fn get_and_reset_clear_called(&mut self) -> bool {
    self.check_clear_attributes.get_and_reset_clear_called()
  }
}
