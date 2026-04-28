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
#[cfg(test)]
use std::collections::HashSet;
use std::fmt::Display;
use std::hash::Hash;

/// Default implementation of [`CharTermAttribute`].
pub struct CharTermAttributeImpl<T>
where
  T: AttributeImpl + CharTermAttributeImplBase,
{
  term_buffer: Vec<char>,
  term_length: usize,
  /// May be used by subclasses to convert to different charsets / encodings for implementing [`get_bytes_ref`](Self::get_bytes_ref).
  pub(crate) builder: BytesRefBuilder<Vec<u8>>,
  pub(crate) sub: T,
  #[cfg(test)]
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
    #[cfg(test)]
    let mut attribute = HashSet::new();
    #[cfg(test)]
    {
      attribute.insert(<Self as CharTermAttribute>::ATTRIBUTE_NAME.to_string());
      attribute.insert(<Self as TermToBytesRefAttribute>::ATTRIBUTE_NAME.to_string());
      attribute.extend(sub.get_attribute_name()?.clone())
    }

    // TODO: IMPORTANT  _bytes_per_element not Specific
    let size = ArrayUtil::oversize(Self::MIN_BUFFER_SIZE, 0);
    Ok(Self {
      term_buffer: vec!['\0'; size],
      term_length: 0,
      builder: BytesRefBuilder::new(),
      sub,
      #[cfg(test)]
      attribute,
    })
  }
  fn grow_term_buffer(&mut self, new_size: usize) {
    if self.term_buffer.len() < new_size {
      // Not big enough; create a new array with slight
      // over allocation:
      // TODO: IMPORTANT  _bytes_per_element not Specific
      let new_capacity = ArrayUtil::oversize(new_size, 0);
      self.term_buffer = vec!['\0'; new_capacity];
    }
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
  fn append_null(&mut self) -> &mut Self {
    self.resize_buffer(self.term_length + 4);
    self.term_buffer[self.term_length] = 'n';
    self.term_buffer[self.term_length + 1] = 'u';
    self.term_buffer[self.term_length + 2] = 'l';
    self.term_buffer[self.term_length + 3] = 'l';
    self.term_length += 4;
    self
  }
}

impl<T> Attribute for CharTermAttributeImpl<T>
where
  T: AttributeImpl + CharTermAttributeImplBase,
{
  #[cfg(test)]
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

  fn copy_buffer(&mut self, buffer: &[char], offset: usize, length: usize) {
    self.grow_term_buffer(length);
    self
      .term_buffer
      .copy_from(&buffer[offset..offset + length], 0);
    self.term_length = length
  }

  fn buffer_mut(&mut self) -> &mut [char] {
    &mut self.term_buffer
  }

  fn buffer(&self) -> &[char] {
    &self.term_buffer
  }

  fn resize_buffer(&mut self, new_size: usize) -> &mut [char] {
    if self.term_buffer.len() < new_size {
      // Not big enough; create a new array with slight
      // over allocation:
      // TODO: IMPORTANT  _bytes_per_element not Specific
      let new_capacity = ArrayUtil::oversize(new_size, std::mem::size_of::<char>());
      ArrayUtil::grow_with_len(&mut self.term_buffer, new_capacity);
    }
    &mut self.term_buffer
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

  fn append_range(&mut self, _csq: &str, _start: usize, _end: usize) -> &mut Self {
    todo!()
  }

  fn append_char(&mut self, c: char) -> &mut Self {
    self.resize_buffer(self.term_length + 1);
    self.term_buffer[self.term_length] = c;
    self.term_length += 1;
    self
  }

  fn append_str(&mut self, s: Option<&str>) -> &mut Self {
    if s.is_none() {
      return self.append_null();
    }
    let s = s.unwrap();
    let chars: Vec<char> = s.chars().collect();
    self.resize_buffer(self.term_length + chars.len());
    self.term_buffer.copy_from(&chars, self.term_length);
    self.term_length += chars.len();
    self
  }

  fn append_term_attribute<C>(&mut self, ta: Option<&mut C>) -> &mut Self
  where
    C: CharTermAttribute,
  {
    if let Some(other) = ta {
      let len = other.length();
      self.resize_buffer(self.term_length + len);
      self
        .term_buffer
        .copy_from(&other.buffer()[0..len], self.term_length);
      self.term_length += len;
      self
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
    let mut builder = BytesRefBuilder::new();
    builder.copy_bytes_from_ref(self.builder.get_bytes_ref());
    copy.builder = builder;
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

  fn copy_to(&self, other: &mut Self::AttributeImpl) {
    other.copy_buffer(&self.term_buffer, 0, self.term_length);
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
    let s: String = self.term_buffer[..self.term_length].iter().collect();
    write!(f, "{s}")
  }
}
#[derive(Clone)]
pub struct EmptyAttributeImpl {
  #[cfg(test)]
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
      #[cfg(test)]
      attribute: HashSet::new(),
    }
  }
}

impl Attribute for EmptyAttributeImpl {
  #[cfg(test)]
  fn get_attribute_name(&self) -> Result<&HashSet<String>> {
    Ok(&self.attribute)
  }
}

impl AttributeImpl for EmptyAttributeImpl {
  fn clear(&mut self) {}

  fn end(&mut self) {}

  type AttributeImpl = EmptyAttributeImpl;

  fn copy_to(&self, _other: &mut Self::AttributeImpl) {}
}
impl CharTermAttributeImplBase for EmptyAttributeImpl {}
pub trait CharTermAttributeImplBase {}
#[cfg(test)]
pub mod tests {
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
  fn test_resize() {
    let mut t = CharTermAttributeImpl::new().unwrap();
    let content: Vec<char> = "hello".chars().collect();
    t.copy_buffer(&content, 0, content.len());

    for i in 0..2000 {
      let buf = t.resize_buffer(i);
      assert!(
        i <= buf.len(),
        "buffer.len() = {}, expected >= {}",
        buf.len(),
        i
      );
      assert_eq!(t.to_string(), "hello");
    }
  }
  #[test]
  fn test_set_length_oob() {
    // this test is not required in Rust Lucene
  }
  #[test]
  fn test_grow() {
    let mut t = CharTermAttributeImpl::new().unwrap();
    let mut buf = String::from("ab");
    for _ in 0..20 {
      let chars: Vec<char> = buf.chars().collect();
      t.copy_buffer(&chars, 0, chars.len());
      assert_eq!(buf.len(), t.length());
      assert_eq!(buf, t.to_string());
      buf.push_str(&buf.clone());
    }
    assert_eq!(1_048_576, t.length());

    let mut t = CharTermAttributeImpl::new().unwrap();
    let mut buf = String::from("ab");
    for _ in 0..20 {
      t.set_empty().append_str(Some(&buf));
      assert_eq!(buf.len(), t.length());
      assert_eq!(buf, t.to_string());
      buf.push_str(&t.to_string());
    }
    assert_eq!(1_048_576, t.length());

    let mut t = CharTermAttributeImpl::new().unwrap();
    let mut buf = String::from("a");
    for _ in 0..20_000 {
      t.set_empty().append_str(Some(&buf));
      assert_eq!(buf.len(), t.length());
      assert_eq!(buf, t.to_string());
      buf.push('a');
    }
    assert_eq!(20_000, t.length());
  }
  #[test]
  fn test_to_string() {
    let mut t = CharTermAttributeImpl::new().unwrap();
    let b: Vec<char> = ['a', 'l', 'o', 'h', 'a'].to_vec();
    t.copy_buffer(&b, 0, 5);
    assert_eq!(t.to_string(), "aloha");

    t.set_empty().append_str(Some("hi there"));
    assert_eq!(t.to_string(), "hi there");
  }

  #[test]
  fn test_clone() {
    let mut t = CharTermAttributeImpl::new().unwrap();
    let content: Vec<char> = "hello".chars().collect();
    t.copy_buffer(&content, 0, 5);

    let copy = assert_clone_is_equal(&t);
    assert_eq!(t.to_string(), copy.to_string());
  }

  #[test]
  fn test_equals() {
    let mut t1a = CharTermAttributeImpl::new().unwrap();
    let content1a: Vec<char> = "hello".chars().collect();
    t1a.copy_buffer(&content1a, 0, 5);

    let mut t1b = CharTermAttributeImpl::new().unwrap();
    let content1b: Vec<char> = "hello".chars().collect();
    t1b.copy_buffer(&content1b, 0, 5);

    let mut t2 = CharTermAttributeImpl::new().unwrap();
    let content2: Vec<char> = "hello2".chars().collect();
    t2.copy_buffer(&content2, 0, 6);

    assert!(t1a == t1b);
    assert!(t1a != t2);
    assert!(t2 != t1b);
  }
  #[test]
  fn test_copy_to() {
    let t = CharTermAttributeImpl::new().unwrap();
    let copy = assert_copy_is_equal(&t);
    assert_eq!(t.to_string(), "");
    assert_eq!(copy.to_string(), "");

    let mut t = CharTermAttributeImpl::new().unwrap();
    let content: Vec<char> = "hello".chars().collect();
    t.copy_buffer(&content, 0, 5);

    let copy = assert_copy_is_equal(&t);
    assert_eq!(t.to_string(), copy.to_string());
  }

  #[test]
  fn test_attribute_reflection() {
    // this test is not required in Rust Lucene
  }
  #[test]
  fn test_char_sequence_interface() -> Result<()> {
    let s = "0123456789";
    let mut t = CharTermAttributeImpl::new().unwrap();
    t.append_str(Some(s));

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
  fn test_appendable_interface() {
    // this test is not required in Rust Lucene
  }
  #[test]
  fn test_appendable_interface_with_longsequences() {
    // this test is not required in Rust Lucene
  }
  #[test]
  fn test_non_char_sequence_append() {
    let mut t = CharTermAttributeImpl::new().unwrap();

    t.append_str(Some("0123456789"))
      .append_str(Some("0123456789"));
    assert_eq!(t.to_string(), "01234567890123456789");

    let sb = String::from("0123456789");
    t.append_str(Some(&sb));
    assert_eq!(t.to_string(), "012345678901234567890123456789");

    let mut t2 = CharTermAttributeImpl::new().unwrap();
    t2.append_str(Some("test"));
    t.append_term_attribute(Some(&mut t2));
    assert_eq!(t.to_string(), "012345678901234567890123456789test");

    t.append_str(None)
      .append_str(None)
      .append_term_attribute::<CharTermAttributeImpl<EmptyAttributeImpl>>(None);
    assert_eq!(
      t.to_string(),
      "012345678901234567890123456789testnullnullnull"
    );
  }

  #[test]
  fn test_exceptions() {
    let mut t = CharTermAttributeImpl::new().unwrap();
    t.append_str(Some("test"));
    assert_eq!(t.to_string(), "test");

    let v = t.char_at(4);
    matches!(v, Err(LuceneError::ArrayIndexOutOfBounds(_)));

    let v = t.sub_sequence(0, 5);
    matches!(v, Err(LuceneError::ArrayIndexOutOfBounds(_)));

    let v = t.sub_sequence(5, 0);
    matches!(v, Err(LuceneError::ArrayIndexOutOfBounds(_)));
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
}
