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
use crate::core::analysis::token_attributes::bytes_term_attribute::BytesTermAttribute;
use crate::core::analysis::token_attributes::bytes_term_attribute_impl::BytesTermAttributeImpl;
use crate::core::analysis::token_attributes::char_term_attribute::CharTermAttribute;
use crate::core::analysis::token_attributes::char_term_attribute_impl::CharTermAttributeImpl;
#[cfg(test)]
use crate::core::analysis::token_attributes::flags_attribute::FlagsAttribute;
use crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::core::analysis::token_attributes::packed_token_and_binary::BinaryTokenStreamAttributeImpl;
use crate::core::analysis::token_attributes::packed_token_attribute_impl::PackedTokenAttributeImpl;
#[cfg(test)]
use crate::core::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::core::analysis::token_attributes::position_increment_attribute::PositionIncrementAttribute;
use crate::core::analysis::token_attributes::position_length_attribute::PositionLengthAttribute;
use crate::core::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::core::analysis::token_attributes::term_to_bytes_ref_attribute::TermToBytesRefAttribute;
use crate::core::analysis::token_attributes::type_attribute::TypeAttribute;
use crate::core::index::BytesRef;
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::impl_from_for_enum;
#[cfg(test)]
use crate::test::core::analysis::base_token_stream_test_case::CheckClearAttributesAttribute;
use std::borrow::Cow;
#[cfg(debug_assertions)]
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

pub trait AttributeSource {
  // CharTermAttribute
  fn length(&self) -> Result<usize> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn copy_buffer(&mut self, _buffer: &[char], _offset: usize, _length: usize) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn buffer_mut(&mut self) -> Result<&mut [char]> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn buffer(&self) -> Result<&[char]> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn resize_buffer(&mut self, _new_size: usize) -> Result<&mut [char]> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn set_length(&mut self, _length: usize) -> Result<&mut Self> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn set_empty(&mut self) -> Result<&mut Self> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn append_range(&mut self, _csq: Option<&str>, _start: usize, _end: usize) -> Result<&mut Self> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn append_char(&mut self, _c: char) -> Result<&mut Self> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn append_str(&mut self, _s: Option<&str>) -> Result<&mut Self> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn append_term_attribute<C>(&mut self, _term_att: Option<&mut C>) -> Result<&mut Self>
  where
    C: CharTermAttribute,
  {
    Err(LuceneError::unsupported_operation(""))
  }

  // OffsetAttribute
  fn start_offset(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn set_offset(&mut self, _start_offset: i32, _end_offset: i32) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn end_offset(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  // BytesTermAttribute
  fn set_bytes_ref(&mut self, _bytes: Option<BytesRef<Vec<u8>>>) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  // PositionIncrementAttribute
  fn get_position_increment(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn set_position_increment(&mut self, _position_increment: i32) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  // PayloadAttribute;
  fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
    Ok(None)
  }
  fn set_payload(&mut self, _payload: BytesRef<Vec<u8>>) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  // TermToBytesRefAttribute;
  fn get_bytes_ref(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(None)
  }

  // TermFrequencyAttribute;
  fn set_term_frequency(&mut self, _term_frequency: i32) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn get_term_frequency(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  // PositionLengthAttribute
  fn set_position_length(&mut self, _position_length: i32) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn get_position_length(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }
  // KeywordAttribute
  fn is_keyword(&self) -> Result<bool> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn set_keyword(&mut self, _is_keyword: bool) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }
  // FlagsAttribute
  fn get_flags(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn set_flags(&mut self, _flags: i32) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }
  // BoostAttribute
  fn set_boost(&mut self, _boost: f32) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn get_boost(&self) -> Result<f32> {
    Err(LuceneError::unsupported_operation(""))
  }
  // MaxNonCompetitiveBoostAttribute
  fn set_max_non_competitive_boost(&mut self, _max_non_competitive_boost: f32) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn get_max_non_competitive_boost(&self) -> Result<f32> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn set_competitive_term(&mut self, _competitive_term: Option<BytesRef<Vec<u8>>>) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn get_competitive_term(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
    Err(LuceneError::unsupported_operation(""))
  }
  // TypeAttribute;
  fn type_(&self) -> Result<&str> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn set_type(&mut self, _type_: &str) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn end_attributes(&mut self) {}

  fn clear_attributes(&mut self);
}

impl<T> AttributeSource for &T
where
  T: AttributeSource,
{
  fn start_offset(&self) -> Result<i32> {
    (**self).start_offset()
  }

  fn end_offset(&self) -> Result<i32> {
    (**self).end_offset()
  }

  fn get_position_increment(&self) -> Result<i32> {
    (**self).get_position_increment()
  }

  fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
    (**self).get_payload()
  }

  fn get_term_frequency(&self) -> Result<i32> {
    (**self).get_term_frequency()
  }

  fn get_position_length(&self) -> Result<i32> {
    (**self).get_position_length()
  }

  fn is_keyword(&self) -> Result<bool> {
    (**self).is_keyword()
  }

  fn get_flags(&self) -> Result<i32> {
    (**self).get_flags()
  }

  fn get_boost(&self) -> Result<f32> {
    (**self).get_boost()
  }

  fn clear_attributes(&mut self) {
    unreachable!("cannot clear attributes through an immutable reference")
  }
}

impl<T> AttributeSource for &mut T
where
  T: AttributeSource,
{
  fn length(&self) -> Result<usize> {
    (**self).length()
  }

  fn copy_buffer(&mut self, buffer: &[char], offset: usize, length: usize) -> Result<()> {
    (**self).copy_buffer(buffer, offset, length)
  }

  fn buffer_mut(&mut self) -> Result<&mut [char]> {
    (**self).buffer_mut()
  }

  fn buffer(&self) -> Result<&[char]> {
    (**self).buffer()
  }

  fn resize_buffer(&mut self, new_size: usize) -> Result<&mut [char]> {
    (**self).resize_buffer(new_size)
  }

  fn set_length(&mut self, length: usize) -> Result<&mut Self> {
    (**self).set_length(length)?;
    Ok(self)
  }

  fn set_empty(&mut self) -> Result<&mut Self> {
    (**self).set_empty()?;
    Ok(self)
  }

  fn append_range(&mut self, csq: Option<&str>, start: usize, end: usize) -> Result<&mut Self> {
    (**self).append_range(csq, start, end)?;
    Ok(self)
  }

  fn append_char(&mut self, c: char) -> Result<&mut Self> {
    (**self).append_char(c)?;
    Ok(self)
  }

  fn append_str(&mut self, s: Option<&str>) -> Result<&mut Self> {
    (**self).append_str(s)?;
    Ok(self)
  }

  fn append_term_attribute<C>(&mut self, term_att: Option<&mut C>) -> Result<&mut Self>
  where
    C: CharTermAttribute,
  {
    (**self).append_term_attribute(term_att)?;
    Ok(self)
  }

  fn start_offset(&self) -> Result<i32> {
    (**self).start_offset()
  }

  fn set_offset(&mut self, start_offset: i32, end_offset: i32) -> Result<()> {
    (**self).set_offset(start_offset, end_offset)
  }

  fn end_offset(&self) -> Result<i32> {
    (**self).end_offset()
  }

  fn set_bytes_ref(&mut self, bytes: Option<BytesRef<Vec<u8>>>) -> Result<()> {
    (**self).set_bytes_ref(bytes)
  }

  fn get_position_increment(&self) -> Result<i32> {
    (**self).get_position_increment()
  }

  fn set_position_increment(&mut self, position_increment: i32) -> Result<()> {
    (**self).set_position_increment(position_increment)
  }

  fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
    (**self).get_payload()
  }

  fn set_payload(&mut self, payload: BytesRef<Vec<u8>>) -> Result<()> {
    (**self).set_payload(payload)
  }

  fn get_bytes_ref(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    (**self).get_bytes_ref()
  }

  fn get_term_frequency(&self) -> Result<i32> {
    (**self).get_term_frequency()
  }

  fn set_term_frequency(&mut self, term_frequency: i32) -> Result<()> {
    (**self).set_term_frequency(term_frequency)
  }

  fn set_position_length(&mut self, position_length: i32) -> Result<()> {
    (**self).set_position_length(position_length)
  }

  fn get_position_length(&self) -> Result<i32> {
    (**self).get_position_length()
  }

  fn is_keyword(&self) -> Result<bool> {
    (**self).is_keyword()
  }

  fn set_keyword(&mut self, is_keyword: bool) -> Result<()> {
    (**self).set_keyword(is_keyword)
  }

  fn get_flags(&self) -> Result<i32> {
    (**self).get_flags()
  }

  fn set_flags(&mut self, flags: i32) -> Result<()> {
    (**self).set_flags(flags)
  }

  fn set_boost(&mut self, boost: f32) -> Result<()> {
    (**self).set_boost(boost)
  }

  fn get_boost(&self) -> Result<f32> {
    (**self).get_boost()
  }

  fn set_max_non_competitive_boost(&mut self, max_non_competitive_boost: f32) -> Result<()> {
    (**self).set_max_non_competitive_boost(max_non_competitive_boost)
  }

  fn get_max_non_competitive_boost(&self) -> Result<f32> {
    (**self).get_max_non_competitive_boost()
  }

  fn set_competitive_term(&mut self, competitive_term: Option<BytesRef<Vec<u8>>>) -> Result<()> {
    (**self).set_competitive_term(competitive_term)
  }

  fn get_competitive_term(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
    (**self).get_competitive_term()
  }

  fn type_(&self) -> Result<&str> {
    (**self).type_()
  }

  fn set_type(&mut self, type_: &str) -> Result<()> {
    (**self).set_type(type_)
  }

  fn end_attributes(&mut self) {
    (**self).end_attributes()
  }

  fn clear_attributes(&mut self) {
    (**self).clear_attributes()
  }
}

pub enum Attributes {
  PackedToken(CharTermAttributeImpl<PackedTokenAttributeImpl>),
  BytesTerm(BytesTermAttributeImpl),
  BinaryTokenStream(BinaryTokenStreamAttributeImpl),
}
impl Display for Attributes {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Attributes::PackedToken(attr) => attr.fmt(f),
      Attributes::BytesTerm(attr) => attr.fmt(f),
      Attributes::BinaryTokenStream(attr) => attr.fmt(f),
    }
  }
}
#[cfg(test)]
impl Attributes {
  pub fn get_and_reset_clear_called(&mut self) -> Result<bool> {
    match self {
      Attributes::PackedToken(attr) => Ok(attr.get_and_reset_clear_called()),
      Attributes::BytesTerm(attr) => Ok(attr.get_and_reset_clear_called()),
      Attributes::BinaryTokenStream(attr) => Ok(attr.get_and_reset_clear_called()),
    }
  }
}
impl_from_for_enum!(
    Attributes,
    CharTermAttributeImpl<PackedTokenAttributeImpl> => PackedToken,
    BytesTermAttributeImpl=> BytesTerm,
    BinaryTokenStreamAttributeImpl=> BinaryTokenStream,
);
impl Attribute for Attributes {
  #[cfg(debug_assertions)]
  fn get_attribute_name(&self) -> Result<&HashSet<String>> {
    match self {
      Attributes::PackedToken(attr) => attr.get_attribute_name(),
      Attributes::BytesTerm(attr) => attr.get_attribute_name(),
      Attributes::BinaryTokenStream(attr) => attr.get_attribute_name(),
    }
  }
}

impl Default for Attributes {
  fn default() -> Self {
    Attributes::PackedToken(
      PackedTokenAttributeImpl::new().expect("new PackedTokenAttributeImpl fail"),
    )
  }
}

impl AttributeSource for Attributes {
  fn length(&self) -> Result<usize> {
    match self {
      Attributes::PackedToken(attr) => Ok(attr.length()),
      Attributes::BytesTerm(_attr) => Err(LuceneError::unsupported_operation("")),
      Attributes::BinaryTokenStream(attr) => Ok(attr.get_packed_token().length()),
    }
  }

  fn copy_buffer(&mut self, buffer: &[char], offset: usize, length: usize) -> Result<()> {
    match self {
      Attributes::PackedToken(attr) => {
        attr.copy_buffer(buffer, offset, length);
        Ok(())
      },
      Attributes::BinaryTokenStream(attr) => {
        attr
          .get_packed_token_mut()
          .copy_buffer(buffer, offset, length);
        Ok(())
      },
      Attributes::BytesTerm(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn buffer_mut(&mut self) -> Result<&mut [char]> {
    match self {
      Attributes::PackedToken(attr) => Ok(attr.buffer_mut()),
      Attributes::BinaryTokenStream(attr) => Ok(attr.get_packed_token_mut().buffer_mut()),
      Attributes::BytesTerm(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn buffer(&self) -> Result<&[char]> {
    match self {
      Attributes::PackedToken(attr) => Ok(attr.buffer()),
      Attributes::BinaryTokenStream(attr) => Ok(attr.get_packed_token().buffer()),
      Attributes::BytesTerm(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn resize_buffer(&mut self, new_size: usize) -> Result<&mut [char]> {
    match self {
      Attributes::PackedToken(attr) => Ok(attr.resize_buffer(new_size)),
      Attributes::BinaryTokenStream(attr) => {
        Ok(attr.get_packed_token_mut().resize_buffer(new_size))
      },
      Attributes::BytesTerm(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn set_length(&mut self, length: usize) -> Result<&mut Self> {
    match self {
      Attributes::PackedToken(attr) => {
        attr.set_length(length)?;
        Ok(self)
      },
      Attributes::BinaryTokenStream(attr) => {
        attr.get_packed_token_mut().set_length(length)?;
        Ok(self)
      },
      Attributes::BytesTerm(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn set_empty(&mut self) -> Result<&mut Self> {
    match self {
      Attributes::PackedToken(attr) => {
        attr.set_empty();
        Ok(self)
      },
      Attributes::BinaryTokenStream(attr) => {
        attr.get_packed_token_mut().set_empty();
        Ok(self)
      },
      Attributes::BytesTerm(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn append_range(&mut self, csq: Option<&str>, start: usize, end: usize) -> Result<&mut Self> {
    match self {
      Attributes::PackedToken(attr) => {
        attr.append_range(csq, start, end)?;
        Ok(self)
      },
      Attributes::BinaryTokenStream(attr) => {
        attr.get_packed_token_mut().append_range(csq, start, end)?;
        Ok(self)
      },
      Attributes::BytesTerm(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn append_char(&mut self, c: char) -> Result<&mut Self> {
    match self {
      Attributes::PackedToken(attr) => {
        attr.append_char(c);
        Ok(self)
      },
      Attributes::BinaryTokenStream(attr) => {
        attr.get_packed_token_mut().append_char(c);
        Ok(self)
      },
      Attributes::BytesTerm(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn append_str(&mut self, s: Option<&str>) -> Result<&mut Self> {
    match self {
      Attributes::PackedToken(attr) => {
        attr.append_str(s);
        Ok(self)
      },
      Attributes::BinaryTokenStream(attr) => {
        attr.get_packed_token_mut().append_str(s);
        Ok(self)
      },
      Attributes::BytesTerm(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn append_term_attribute<C>(&mut self, term_att: Option<&mut C>) -> Result<&mut Self>
  where
    C: CharTermAttribute,
  {
    match self {
      Attributes::PackedToken(attr) => {
        attr.append_term_attribute(term_att);
        Ok(self)
      },
      Attributes::BinaryTokenStream(attr) => {
        attr.get_packed_token_mut().append_term_attribute(term_att);
        Ok(self)
      },
      Attributes::BytesTerm(_) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn start_offset(&self) -> Result<i32> {
    match self {
      Attributes::PackedToken(attr) => Ok(attr.sub.start_offset()),
      Attributes::BinaryTokenStream(attr) => Ok(attr.get_packed_token().sub.start_offset()),
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn set_offset(&mut self, start_offset: i32, end_offset: i32) -> Result<()> {
    match self {
      Attributes::PackedToken(attr) => attr.sub.set_offset(start_offset, end_offset),
      Attributes::BinaryTokenStream(attr) => attr
        .get_packed_token_mut()
        .sub
        .set_offset(start_offset, end_offset),
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn end_offset(&self) -> Result<i32> {
    match self {
      Attributes::PackedToken(attr) => Ok(attr.sub.end_offset()),
      Attributes::BinaryTokenStream(attr) => Ok(attr.get_packed_token().sub.end_offset()),
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn set_bytes_ref(&mut self, bytes: Option<BytesRef<Vec<u8>>>) -> Result<()> {
    match self {
      Attributes::BytesTerm(attr) => BytesTermAttribute::set_bytes_ref(attr, bytes),
      Attributes::BinaryTokenStream(attr) => {
        BytesTermAttribute::set_bytes_ref(attr.get_binary_mut(), bytes)
      },
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn get_position_increment(&self) -> Result<i32> {
    match self {
      Attributes::PackedToken(attr) => Ok(attr.sub.get_position_increment()),
      Attributes::BinaryTokenStream(attr) => {
        Ok(attr.get_packed_token().sub.get_position_increment())
      },
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn set_position_increment(&mut self, position_increment: i32) -> Result<()> {
    match self {
      Attributes::PackedToken(attr) => attr.sub.set_position_increment(position_increment),
      Attributes::BinaryTokenStream(attr) => attr
        .get_packed_token_mut()
        .sub
        .set_position_increment(position_increment),
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
    match self {
      #[cfg(test)]
      Attributes::PackedToken(v) => Ok(v.sub.token.get_payload()),
      #[cfg(test)]
      Attributes::BinaryTokenStream(v) => Ok(v.get_packed_token().sub.token.get_payload()),
      Attributes::BytesTerm(_) => Err(LuceneError::unsupported_operation("")),

      #[cfg(not(test))]
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn set_payload(&mut self, _payload: BytesRef<Vec<u8>>) -> Result<()> {
    match self {
      #[cfg(test)]
      Attributes::PackedToken(v) => {
        let _: () = v.sub.token.set_payload(Some(_payload));
        Ok(())
      },
      #[cfg(test)]
      Attributes::BinaryTokenStream(v) => {
        let _: () = v
          .get_packed_token_mut()
          .sub
          .token
          .set_payload(Some(_payload));
        Ok(())
      },
      Attributes::BytesTerm(_) => Err(LuceneError::unsupported_operation("")),

      #[cfg(not(test))]
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn get_bytes_ref(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    match self {
      Attributes::PackedToken(attr) => Ok(attr.get_bytes_ref()),
      Attributes::BinaryTokenStream(attr) => Ok(attr.get_binary_mut().get_bytes_ref()),
      Attributes::BytesTerm(attr) => Ok(attr.get_bytes_ref()),
    }
  }

  fn set_term_frequency(&mut self, term_frequency: i32) -> Result<()> {
    match self {
      Attributes::PackedToken(attr) => attr.sub.set_term_frequency(term_frequency),
      Attributes::BinaryTokenStream(attr) => attr
        .get_packed_token_mut()
        .sub
        .set_term_frequency(term_frequency),
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn get_term_frequency(&self) -> Result<i32> {
    match self {
      Attributes::PackedToken(attr) => Ok(attr.sub.get_term_frequency()),
      Attributes::BinaryTokenStream(attr) => Ok(attr.get_packed_token().sub.get_term_frequency()),
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn set_position_length(&mut self, position_length: i32) -> Result<()> {
    match self {
      Attributes::PackedToken(attr) => attr.sub.set_position_length(position_length),
      Attributes::BinaryTokenStream(attr) => attr
        .get_packed_token_mut()
        .sub
        .set_position_length(position_length),
      Attributes::BytesTerm(_attr) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn get_position_length(&self) -> Result<i32> {
    match self {
      Attributes::PackedToken(attr) => Ok(attr.sub.get_position_length()),
      Attributes::BinaryTokenStream(attr) => Ok(attr.get_packed_token().sub.get_position_length()),
      Attributes::BytesTerm(_attr) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn get_flags(&self) -> Result<i32> {
    match self {
      #[cfg(test)]
      Attributes::PackedToken(v) => Ok(v.sub.token.get_flags()),
      #[cfg(test)]
      Attributes::BinaryTokenStream(v) => Ok(v.get_packed_token().sub.token.get_flags()),
      Attributes::BytesTerm(_) => Err(LuceneError::unsupported_operation("")),

      #[cfg(not(test))]
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn set_flags(&mut self, _flags: i32) -> Result<()> {
    match self {
      #[cfg(test)]
      Attributes::PackedToken(v) => {
        let _: () = v.sub.token.set_flags(_flags);
        Ok(())
      },
      #[cfg(test)]
      Attributes::BinaryTokenStream(v) => {
        let _: () = v.get_packed_token_mut().sub.token.set_flags(_flags);
        Ok(())
      },
      Attributes::BytesTerm(_) => Err(LuceneError::unsupported_operation("")),

      #[cfg(not(test))]
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn type_(&self) -> Result<&str> {
    match self {
      Attributes::PackedToken(attr) => Ok(attr.sub.type_()),
      Attributes::BinaryTokenStream(attr) => Ok(attr.get_packed_token().sub.type_()),
      Attributes::BytesTerm(_attr) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn set_type(&mut self, type_: &str) -> Result<()> {
    match self {
      Attributes::PackedToken(attr) => {
        let _: () = attr.sub.set_type(type_);
        Ok(())
      },
      Attributes::BinaryTokenStream(attr) => {
        let _: () = attr.get_packed_token_mut().sub.set_type(type_);
        Ok(())
      },
      Attributes::BytesTerm(_attr) => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn end_attributes(&mut self) {
    match self {
      Attributes::PackedToken(attr) => attr.end(),
      Attributes::BinaryTokenStream(attr) => attr.get_packed_token_mut().end(),
      _ => unimplemented!("not support"),
    }
  }

  fn clear_attributes(&mut self) {
    match self {
      Attributes::PackedToken(attr) => attr.clear(),
      Attributes::BinaryTokenStream(attr) => attr.clear(),
      Attributes::BytesTerm(attr) => attr.clear(),
    }
  }
}

pub struct EmptyAttributeSource;

impl Default for EmptyAttributeSource {
  fn default() -> Self {
    EmptyAttributeSource
  }
}

impl AttributeSource for EmptyAttributeSource {
  fn end_attributes(&mut self) {}

  fn clear_attributes(&mut self) {}
}

macro_rules! define_attribute_source_enum {
    (
        $enum_name:ident,
        [$($V:ident),+ $(,)?]
    ) => {
        pub enum $enum_name<$($V),+> {
            $($V($V)),+
        }

        impl<$($V),+> AttributeSource for $enum_name<$($V),+>
        where
            $($V: AttributeSource,)+
        {
            fn length(&self) -> Result<usize> {
                match self {
                    $(Self::$V(t) => t.length(),)+
                }
            }

            fn copy_buffer(&mut self, buffer: &[char], offset: usize, length: usize) -> Result<()> {
                match self {
                    $(Self::$V(t) => t.copy_buffer(buffer, offset, length),)+
                }
            }

            fn buffer_mut(&mut self) -> Result<&mut [char]> {
                match self {
                    $(Self::$V(t) => t.buffer_mut(),)+
                }
            }

            fn buffer(&self) -> Result<&[char]> {
                match self {
                    $(Self::$V(t) => t.buffer(),)+
                }
            }

            fn resize_buffer(&mut self, new_size: usize) -> Result<&mut [char]> {
                match self {
                    $(Self::$V(t) => t.resize_buffer(new_size),)+
                }
            }

            fn set_length(&mut self, length: usize) -> Result<&mut Self> {
                match self {
                    $(Self::$V(t) => {
                        t.set_length(length)?;
                        Ok(self)
                    },)+
                }
            }

            fn set_empty(&mut self) -> Result<&mut Self> {
                match self {
                    $(Self::$V(t) => {
                        t.set_empty()?;
                        Ok(self)
                    },)+
                }
            }

            fn append_range(&mut self, csq: Option<&str>, start: usize, end: usize) -> Result<&mut Self> {
                match self {
                    $(Self::$V(t) => {
                        t.append_range(csq, start, end)?;
                        Ok(self)
                    },)+
                }
            }

            fn append_char(&mut self, c: char) -> Result<&mut Self> {
                match self {
                    $(Self::$V(t) => {
                        t.append_char(c)?;
                        Ok(self)
                    },)+
                }
            }

            fn append_str(&mut self, s: Option<&str>) -> Result<&mut Self> {
                match self {
                    $(Self::$V(t) => {
                        t.append_str(s)?;
                        Ok(self)
                    },)+
                }
            }

            fn append_term_attribute<TermAtt>(&mut self, term_att: Option<&mut TermAtt>) -> Result<&mut Self>
            where
                TermAtt: CharTermAttribute,
            {
                match self {
                    $(Self::$V(t) => {
                        t.append_term_attribute(term_att)?;
                        Ok(self)
                    },)+
                }
            }

            fn start_offset(&self) -> Result<i32> {
                match self {
                    $(Self::$V(t) => t.start_offset(),)+
                }
            }

            fn set_offset(&mut self, start_offset: i32, end_offset: i32) -> Result<()> {
                match self {
                    $(Self::$V(t) => t.set_offset(start_offset, end_offset),)+
                }
            }

            fn end_offset(&self) -> Result<i32> {
                match self {
                    $(Self::$V(t) => t.end_offset(),)+
                }
            }

            fn set_bytes_ref(&mut self, bytes: Option<BytesRef<Vec<u8>>>) -> Result<()> {
                match self {
                    $(Self::$V(t) => t.set_bytes_ref(bytes),)+
                }
            }

            fn get_position_increment(&self) -> Result<i32> {
                match self {
                    $(Self::$V(t) => t.get_position_increment(),)+
                }
            }

            fn set_position_increment(&mut self, position_increment: i32) -> Result<()> {
                match self {
                    $(Self::$V(t) => t.set_position_increment(position_increment),)+
                }
            }

            fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
                match self {
                    $(Self::$V(t) => t.get_payload(),)+
                }
            }
            fn set_payload(&mut self, payload: BytesRef<Vec<u8>>) -> Result<()> {
                match self {
                    $(Self::$V(t) => t.set_payload(payload),)+
                }
            }

            fn get_bytes_ref(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
                match self {
                    $(Self::$V(t) => t.get_bytes_ref(),)+
                }
            }

            fn get_term_frequency(&self) -> Result<i32> {
                match self {
                    $(Self::$V(t) => t.get_term_frequency(),)+
                }
            }
            fn set_term_frequency(&mut self, term_frequency: i32) -> Result<()> {
                match self {
                    $(Self::$V(t) => t.set_term_frequency(term_frequency),)+
                }
            }
            fn set_position_length(&mut self,position_length: i32) -> Result<()> {
                match self {
                    $(Self::$V(t) => t.set_position_length(position_length),)+
                }
            }
            fn get_position_length(&self) -> Result<i32> {
                match self {
                    $(Self::$V(t) => t.get_position_length(),)+
                }
            }
            fn is_keyword(&self) -> Result<bool>{
                match self {
                    $(Self::$V(t) => t.is_keyword(),)+
                }
            }
            fn set_keyword(&mut self,is_keyword: bool) -> Result<()>{
                match self {
                    $(Self::$V(t) => t.set_keyword(is_keyword),)+
                }
            }
            fn get_flags(&self) -> Result<i32>{
                match self {
                    $(Self::$V(t) => t.get_flags(),)+
                }
            }
            fn set_flags(&mut self,flags: i32) -> Result<()>{
                match self {
                    $(Self::$V(t) => t.set_flags(flags),)+
                }
            }

            fn set_boost(&mut self,boost: f32) -> Result<()>{
                match self {
                    $(Self::$V(t) => t.set_boost(boost),)+
                }
            }
            fn get_boost(&self) -> Result<f32>{
                match self {
                    $(Self::$V(t) => t.get_boost(),)+
                }
            }
            fn set_max_non_competitive_boost(&mut self, max_non_competitive_boost: f32) -> Result<()> {
                match self {
                    $(Self::$V(t) => t.set_max_non_competitive_boost(max_non_competitive_boost),)+
                }
            }
            fn get_max_non_competitive_boost(&self) -> Result<f32> {
                match self {
                    $(Self::$V(t) => t.get_max_non_competitive_boost(),)+
                }
            }
            fn set_competitive_term(&mut self, competitive_term: Option<BytesRef<Vec<u8>>>) -> Result<()> {
                match self {
                    $(Self::$V(t) => t.set_competitive_term(competitive_term),)+
                }
            }
            fn get_competitive_term(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
                match self {
                    $(Self::$V(t) => t.get_competitive_term(),)+
                }
            }
            fn type_(&self) -> Result<&str> {
                match self {
                    $(Self::$V(t) => t.type_(),)+
                }
            }
            fn set_type(&mut self, type_: &str) -> Result<()> {
                match self {
                    $(Self::$V(t) => t.set_type(type_),)+
                }
            }
            fn end_attributes(&mut self) {
                match self {
                    $(Self::$V(t) => t.end_attributes(),)+
                }
            }

            fn clear_attributes(&mut self) {
                match self {
                    $(Self::$V(t) => t.clear_attributes(),)+
                }
            }
        }
    };
}
define_attribute_source_enum!(AttributeSourceEnum2, [A, B]);
define_attribute_source_enum!(AttributeSourceEnum4, [A, B, C, D]);
