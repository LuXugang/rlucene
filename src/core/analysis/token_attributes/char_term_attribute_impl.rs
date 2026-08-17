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
use crate::core::analysis::token_attributes::term_to_bytes_ref_attribute::TermToBytesRefAttribute;
use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::{CoreHelper, SliceCopyOps};
use std::borrow::Cow;
#[cfg(any(test, debug_assertions))]
use std::collections::HashSet;
use std::fmt::Display;
use std::hash::Hash;

/// Default implementation of [`CharTermAttribute`].
pub struct CharTermAttributeImpl<T> {
  term_buffer: Vec<char>,
  term_length: usize,
  /// Implementations may use this to convert to other character sets or encodings when implementing [`get_bytes_ref`](Self::get_bytes_ref).
  pub(crate) builder: BytesRefBuilder<Vec<u8>>,
  pub sub: T,
  #[cfg(any(test, debug_assertions))]
  attribute: HashSet<String>,
}
impl CharTermAttributeImpl<EmptyAttributeImpl> {
  pub fn new() -> Result<Self> {
    Self::with_sub(EmptyAttributeImpl::default())
  }
}

impl<T> CharTermAttributeImpl<T>
where
  T: AttributeImpl + CharTermAttributeImplBase,
{
  const MIN_BUFFER_SIZE: usize = 10;

  pub fn with_sub(sub: T) -> Result<Self> {
    #[cfg(any(test, debug_assertions))]
    let mut attribute = HashSet::new();
    #[cfg(any(test, debug_assertions))]
    {
      attribute.insert(<Self as CharTermAttribute>::ATTRIBUTE_NAME.to_string());
      attribute.insert(<Self as TermToBytesRefAttribute>::ATTRIBUTE_NAME.to_string());
      attribute.extend(sub.get_attribute_name()?.clone())
    }

    let size = ArrayUtil::oversize(Self::MIN_BUFFER_SIZE, std::mem::size_of::<char>())?;
    Ok(Self {
      term_buffer: vec!['\0'; size],
      term_length: 0,
      builder: BytesRefBuilder::new(),
      sub,
      #[cfg(any(test, debug_assertions))]
      attribute,
    })
  }
  fn grow_term_buffer(&mut self, new_size: usize) -> Result<()> {
    if self.term_buffer.len() < new_size {
      ArrayUtil::grow_no_copy(&mut self.term_buffer, new_size)?;
    }
    Ok(())
  }

  pub fn char_at(&self, index: usize) -> Result<char> {
    debug_assert!(index <= i32::MAX as usize);
    debug_assert!(self.term_length <= i32::MAX as usize);
    CoreHelper::check_index(index, self.term_length)?;
    Ok(self.term_buffer[index])
  }
  pub fn sub_sequence(&self, start: usize, end: usize) -> Result<&[char]> {
    CoreHelper::check_from_to_index(start, end, self.term_length)?;
    Ok(&self.term_buffer[start..end])
  }
  fn append_null(&mut self) -> Result<&mut Self> {
    self.resize_buffer(self.term_length + 4)?;
    self.term_buffer[self.term_length] = 'n';
    self.term_buffer[self.term_length + 1] = 'u';
    self.term_buffer[self.term_length + 2] = 'l';
    self.term_buffer[self.term_length + 3] = 'l';
    self.term_length += 4;
    Ok(self)
  }
}

impl<T> Attribute for CharTermAttributeImpl<T>
where
  T: AttributeImpl + CharTermAttributeImplBase,
{
  #[cfg(any(test, debug_assertions))]
  fn get_attribute_name(&self) -> Result<&HashSet<String>> {
    Ok(&self.attribute)
  }
}

impl<T> CharTermAttribute for CharTermAttributeImpl<T>
where
  T: AttributeImpl + CharTermAttributeImplBase,
{
  fn length(&self) -> usize {
    self.term_length
  }

  fn copy_buffer(&mut self, buffer: &[char], offset: usize, length: usize) -> Result<()> {
    self.grow_term_buffer(length)?;
    self
      .term_buffer
      .copy_from(&buffer[offset..offset + length], 0);
    self.term_length = length;
    Ok(())
  }

  fn buffer_mut(&mut self) -> &mut [char] {
    &mut self.term_buffer
  }

  fn buffer(&self) -> &[char] {
    &self.term_buffer
  }

  fn resize_buffer(&mut self, new_size: usize) -> Result<&mut [char]> {
    if self.term_buffer.len() < new_size {
      // Not big enough; create a new array with slight
      // over allocation:
      ArrayUtil::grow_with_len(&mut self.term_buffer, new_size)?;
    }
    Ok(&mut self.term_buffer)
  }

  fn set_length(&mut self, length: usize) -> Result<&mut Self> {
    debug_assert!(self.term_buffer.len() <= i32::MAX as usize);
    CoreHelper::check_from_index_size(0, length, self.term_buffer.len())?;
    self.term_length = length;
    Ok(self)
  }

  fn set_empty(&mut self) -> &mut Self {
    self.term_length = 0;
    self
  }

  fn append_range(&mut self, csq: Option<&str>, start: usize, end: usize) -> Result<&mut Self> {
    let csq = csq.unwrap_or("null");
    let csq_len = csq.chars().count();
    CoreHelper::check_from_to_index(start, end, csq_len)?;

    let len = end - start;
    if len == 0 {
      return Ok(self);
    }

    self.resize_buffer(self.term_length + len)?;
    if len > 4 {
      if csq.is_ascii() {
        for (slot, c) in self.term_buffer[self.term_length..self.term_length + len]
          .iter_mut()
          .zip(&csq.as_bytes()[start..end])
        {
          *slot = *c as char;
        }
      } else {
        for (slot, c) in self.term_buffer[self.term_length..self.term_length + len]
          .iter_mut()
          .zip(csq.chars().skip(start).take(len))
        {
          *slot = c;
        }
      }
      self.term_length += len;
    } else {
      for c in csq.chars().skip(start).take(len) {
        self.term_buffer[self.term_length] = c;
        self.term_length += 1;
      }
    }
    Ok(self)
  }

  fn append_char(&mut self, c: char) -> Result<&mut Self> {
    self.resize_buffer(self.term_length + 1)?;
    self.term_buffer[self.term_length] = c;
    self.term_length += 1;
    Ok(self)
  }

  fn append_str(&mut self, s: Option<&str>) -> Result<&mut Self> {
    if s.is_none() {
      return self.append_null();
    }
    let s = s.unwrap();
    let len = s.chars().count();
    self.resize_buffer(self.term_length + len)?;
    if len == s.len() {
      for (slot, c) in self.term_buffer[self.term_length..self.term_length + len]
        .iter_mut()
        .zip(s.bytes())
      {
        *slot = c as char;
      }
    } else {
      for (slot, c) in self.term_buffer[self.term_length..self.term_length + len]
        .iter_mut()
        .zip(s.chars())
      {
        *slot = c;
      }
    }
    self.term_length += len;
    Ok(self)
  }

  fn append_term_attribute<C>(&mut self, ta: Option<&mut C>) -> Result<&mut Self>
  where
    C: CharTermAttribute,
  {
    if let Some(other) = ta {
      let len = other.length();
      self.resize_buffer(self.term_length + len)?;
      self
        .term_buffer
        .copy_from(&other.buffer()[0..len], self.term_length);
      self.term_length += len;
      Ok(self)
    } else {
      self.append_null()
    }
  }
}
impl<T> TermToBytesRefAttribute for CharTermAttributeImpl<T>
where
  T: AttributeImpl + CharTermAttributeImplBase,
{
  fn get_bytes_ref(&mut self) -> Option<Cow<'_, BytesRef<Vec<u8>>>> {
    self
      .builder
      .copy_chars_from_chars(&self.term_buffer, 0, self.term_length);
    Some(Cow::Borrowed(&self.builder.bytes_ref))
  }
}

impl<T> Clone for CharTermAttributeImpl<T>
where
  T: AttributeImpl + CharTermAttributeImplBase,
{
  fn clone(&self) -> Self {
    let mut copy = CharTermAttributeImpl::with_sub(self.sub.clone()).expect("should not failed");
    copy.term_buffer = self.term_buffer.clone();
    copy.term_length = self.term_length;
    copy.builder.bytes_ref = self.builder.bytes_ref.clone();
    copy
  }
}

impl<T> AttributeImpl for CharTermAttributeImpl<T>
where
  T: AttributeImpl<AttributeImpl = T> + CharTermAttributeImplBase,
{
  fn clear(&mut self) {
    self.term_length = 0;
    self.sub.clear()
  }

  fn end(&mut self) {
    self.clear();
    self.sub.end()
  }

  type AttributeImpl = CharTermAttributeImpl<T>;

  fn copy_to(&self, other: &mut Self::AttributeImpl) -> Result<()> {
    other.copy_buffer(&self.term_buffer, 0, self.term_length)?;
    self.sub.copy_to(&mut other.sub)
  }
}
impl<T> Hash for CharTermAttributeImpl<T>
where
  T: AttributeImpl + CharTermAttributeImplBase,
{
  fn hash<H>(&self, state: &mut H)
  where
    H: std::hash::Hasher,
  {
    self.term_length.hash(state);
    self.term_buffer.hash(state);
  }
}
impl<T> PartialEq for CharTermAttributeImpl<T>
where
  T: AttributeImpl + CharTermAttributeImplBase,
{
  fn eq(&self, other: &Self) -> bool {
    if self.term_length != other.term_length {
      return false;
    }
    self.term_buffer[..self.term_length] == other.term_buffer[..other.term_length]
  }
}
impl<T> Display for CharTermAttributeImpl<T>
where
  T: AttributeImpl + CharTermAttributeImplBase,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut buffer = [0u8; 1024];
    let mut length = 0;
    for c in &self.term_buffer[..self.term_length] {
      let char_length = c.len_utf8();
      if length + char_length > buffer.len() {
        f.write_str(std::str::from_utf8(&buffer[..length]).expect("chars are valid UTF-8"))?;
        length = 0;
      }
      length += c.encode_utf8(&mut buffer[length..]).len();
    }
    if length > 0 {
      f.write_str(std::str::from_utf8(&buffer[..length]).expect("chars are valid UTF-8"))?;
    }
    Ok(())
  }
}
#[derive(Clone)]
pub struct EmptyAttributeImpl {
  #[cfg(any(test, debug_assertions))]
  attribute: HashSet<String>,
}
impl Default for EmptyAttributeImpl {
  fn default() -> Self {
    EmptyAttributeImpl::new()
  }
}
impl EmptyAttributeImpl {
  fn new() -> Self {
    EmptyAttributeImpl {
      #[cfg(any(test, debug_assertions))]
      attribute: HashSet::new(),
    }
  }
}

impl Attribute for EmptyAttributeImpl {
  #[cfg(any(test, debug_assertions))]
  fn get_attribute_name(&self) -> Result<&HashSet<String>> {
    Ok(&self.attribute)
  }
}

impl AttributeImpl for EmptyAttributeImpl {
  fn clear(&mut self) {}

  fn end(&mut self) {}

  type AttributeImpl = EmptyAttributeImpl;

  fn copy_to(&self, _other: &mut Self::AttributeImpl) -> Result<()> {
    Ok(())
  }
}
impl CharTermAttributeImplBase for EmptyAttributeImpl {}
pub trait CharTermAttributeImplBase {}
