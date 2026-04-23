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
use crate::core::analysis::token_attributes::flags_attribute::FlagsAttribute;
use crate::core::analysis::token_attributes::keyword_attribute::KeywordAttribute;
use crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::core::analysis::token_attributes::packed_token_and_binary::BinaryTokenStreamAttributeImpl;
use crate::core::analysis::token_attributes::packed_token_attribute_impl::PackedTokenAttributeImpl;
use crate::core::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::core::analysis::token_attributes::position_increment_attribute::PositionIncrementAttribute;
use crate::core::analysis::token_attributes::position_length_attribute::PositionLengthAttribute;
use crate::core::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::core::analysis::token_attributes::term_to_bytes_ref_attribute::TermToBytesRefAttribute;
use crate::core::analysis::token_attributes::type_attribute::TypeAttribute;
use crate::core::index::BytesRef;
use crate::core::search::boost_attribute::BoostAttribute;
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::impl_from_for_enum;
#[cfg(test)]
use crate::test::core::analysis::base_token_stream_test_case::CheckClearAttributesAttribute;
use std::borrow::Cow;
#[cfg(test)]
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

pub trait AttributeSource {
  // OffsetAttribute
  fn start_offset(&self) -> Result<i32> {
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

  fn set_boost(&mut self, _boost: f32) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_boost(&self) -> Result<f32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn end_attributes(&mut self) {}

  fn clear_attributes(&mut self);
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
  #[cfg(test)]
  fn get_attribute_name(&self) -> Result<&HashSet<String>> {
    match self {
      Attributes::PackedToken(attr) => attr.get_attribute_name(),
      Attributes::BytesTerm(attr) => attr.get_attribute_name(),
      Attributes::BinaryTokenStream(attr) => attr.get_attribute_name(),
    }
  }
}

impl CharTermAttribute for Attributes {
  fn length(&self) -> usize {
    match self {
      Attributes::PackedToken(attr) => attr.length(),
      Attributes::BinaryTokenStream(attr) => attr.get_packed_token().length(),
      _ => unimplemented!("not support"),
    }
  }

  fn copy_buffer(&mut self, buffer: &[char], offset: usize, length: usize) {
    match self {
      Attributes::PackedToken(attr) => attr.copy_buffer(buffer, offset, length),
      Attributes::BinaryTokenStream(attr) => attr
        .get_packed_token_mut()
        .copy_buffer(buffer, offset, length),
      _ => unimplemented!("not support"),
    }
  }

  fn buffer_mut(&mut self) -> &mut [char] {
    match self {
      Attributes::PackedToken(attr) => attr.buffer_mut(),
      Attributes::BinaryTokenStream(attr) => attr.get_packed_token_mut().buffer_mut(),
      _ => unimplemented!("not support"),
    }
  }

  fn buffer(&self) -> &[char] {
    match self {
      Attributes::PackedToken(attr) => attr.buffer(),
      Attributes::BinaryTokenStream(attr) => attr.get_packed_token().buffer(),
      _ => unimplemented!("not support"),
    }
  }

  fn resize_buffer(&mut self, new_size: usize) -> &mut [char] {
    match self {
      Attributes::PackedToken(attr) => attr.resize_buffer(new_size),
      Attributes::BinaryTokenStream(attr) => attr.get_packed_token_mut().resize_buffer(new_size),
      _ => unimplemented!("not support"),
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
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn set_empty(&mut self) -> &mut Self {
    match self {
      Attributes::PackedToken(attr) => {
        attr.set_empty();
        self
      },
      Attributes::BinaryTokenStream(attr) => {
        attr.get_packed_token_mut().set_empty();
        self
      },
      _ => unimplemented!("not support"),
    }
  }

  fn append_range(&mut self, csq: &str, start: usize, end: usize) -> &mut Self {
    match self {
      Attributes::PackedToken(attr) => {
        attr.append_range(csq, start, end);
        self
      },
      Attributes::BinaryTokenStream(attr) => {
        attr.get_packed_token_mut().append_range(csq, start, end);
        self
      },
      _ => unimplemented!("not support"),
    }
  }

  fn append_char(&mut self, c: char) -> &mut Self {
    match self {
      Attributes::PackedToken(attr) => {
        attr.append_char(c);
        self
      },
      Attributes::BinaryTokenStream(attr) => {
        attr.get_packed_token_mut().append_char(c);
        self
      },
      _ => unimplemented!("not support"),
    }
  }

  fn append_str(&mut self, s: Option<&str>) -> &mut Self {
    match self {
      Attributes::PackedToken(attr) => {
        attr.append_str(s);
        self
      },
      Attributes::BinaryTokenStream(attr) => {
        attr.get_packed_token_mut().append_str(s);
        self
      },
      _ => unimplemented!("not support"),
    }
  }

  fn append_term_attribute<C>(&mut self, term_att: Option<&mut C>) -> &mut Self
  where
    C: CharTermAttribute,
  {
    match self {
      Attributes::PackedToken(attr) => {
        attr.append_term_attribute(term_att);
        self
      },
      Attributes::BinaryTokenStream(attr) => {
        attr.get_packed_token_mut().append_term_attribute(term_att);
        self
      },
      _ => unimplemented!("not support"),
    }
  }
}
impl PositionIncrementAttribute for Attributes {
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

  fn get_position_increment(&self) -> i32 {
    match self {
      Attributes::PackedToken(attr) => attr.sub.get_position_increment(),
      Attributes::BinaryTokenStream(attr) => attr.get_packed_token().sub.get_position_increment(),
      _ => unimplemented!("not support"),
    }
  }
}
impl PositionLengthAttribute for Attributes {
  fn set_position_length(&mut self, position_length: i32) -> Result<()> {
    match self {
      Attributes::PackedToken(attr) => attr.sub.set_position_length(position_length),
      Attributes::BinaryTokenStream(attr) => attr
        .get_packed_token_mut()
        .sub
        .set_position_length(position_length),
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }

  fn get_position_length(&self) -> i32 {
    match self {
      Attributes::PackedToken(attr) => attr.sub.get_position_length(),
      Attributes::BinaryTokenStream(attr) => attr.get_packed_token().sub.get_position_length(),
      _ => unimplemented!("not support"),
    }
  }
}
impl FlagsAttribute for Attributes {
  fn get_flags(&self) -> i32 {
    match self {
      Attributes::PackedToken(_attr) => unimplemented!("not support"),
      Attributes::BinaryTokenStream(_attr) => unimplemented!("not support"),
      Attributes::BytesTerm(_attr) => unimplemented!("not support"),
    }
  }

  fn set_flags(&mut self, _flags: i32) {
    match self {
      Attributes::PackedToken(_attr) => unimplemented!("not support"),
      Attributes::BinaryTokenStream(_attr) => unimplemented!("not support"),
      Attributes::BytesTerm(_attr) => unimplemented!("not support"),
    }
  }
}
impl KeywordAttribute for Attributes {
  fn is_keyword(&self) -> Result<bool> {
    match self {
      Attributes::PackedToken(_attr) => unimplemented!("not support"),
      Attributes::BinaryTokenStream(_attr) => unimplemented!("not support"),
      Attributes::BytesTerm(_attr) => unimplemented!("not support"),
    }
  }

  fn set_keyword(&mut self, _is_keyword: bool) -> Result<()> {
    match self {
      Attributes::PackedToken(_attr) => unimplemented!("not support"),
      Attributes::BinaryTokenStream(_attr) => unimplemented!("not support"),
      Attributes::BytesTerm(_attr) => unimplemented!("not support"),
    }
  }
}
impl BoostAttribute for Attributes {
  fn set_boost(&mut self, _boost: f32) {
    match self {
      Attributes::PackedToken(_attr) => unimplemented!("not support"),
      Attributes::BinaryTokenStream(_attr) => unimplemented!("not support"),
      Attributes::BytesTerm(_attr) => unimplemented!("not support"),
    }
  }

  fn get_boost(&self) -> f32 {
    match self {
      Attributes::PackedToken(_attr) => unimplemented!("not support"),
      Attributes::BinaryTokenStream(_attr) => unimplemented!("not support"),
      Attributes::BytesTerm(_attr) => unimplemented!("not support"),
    }
  }
}
impl PayloadAttribute for Attributes {
  fn get_payload(&self) -> &BytesRef<Vec<u8>> {
    match self {
      Attributes::PackedToken(_attr) => unimplemented!("not support"),
      Attributes::BinaryTokenStream(_attr) => unimplemented!("not support"),
      Attributes::BytesTerm(_attr) => unimplemented!("not support"),
    }
  }

  fn set_payload(&mut self, _payload: BytesRef<Vec<u8>>) {
    match self {
      Attributes::PackedToken(_attr) => unimplemented!("not support"),
      Attributes::BinaryTokenStream(_attr) => unimplemented!("not support"),
      Attributes::BytesTerm(_attr) => unimplemented!("not support"),
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
  fn start_offset(&self) -> Result<i32> {
    match self {
      Attributes::PackedToken(attr) => Ok(attr.sub.start_offset()),
      Attributes::BinaryTokenStream(attr) => Ok(attr.get_packed_token().sub.start_offset()),
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
      Attributes::PackedToken(_) => Ok(None),
      Attributes::BinaryTokenStream(_) => Ok(None),
      Attributes::BytesTerm(_) => Ok(None),
    }
  }

  fn get_bytes_ref(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    match self {
      Attributes::PackedToken(attr) => Ok(attr.get_bytes_ref()),
      Attributes::BinaryTokenStream(attr) => Ok(attr.get_binary_mut().get_bytes_ref()),
      Attributes::BytesTerm(attr) => Ok(attr.get_bytes_ref()),
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

impl OffsetAttribute for Attributes {
  fn start_offset(&self) -> i32 {
    match self {
      Attributes::PackedToken(attr) => attr.sub.start_offset(),
      Attributes::BinaryTokenStream(attr) => attr.get_packed_token().sub.start_offset(),
      _ => unimplemented!("not support"),
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

  fn end_offset(&self) -> i32 {
    match self {
      Attributes::PackedToken(attr) => attr.sub.end_offset(),
      Attributes::BinaryTokenStream(attr) => attr.get_packed_token().sub.end_offset(),
      _ => unimplemented!("not support"),
    }
  }
}

impl TypeAttribute for Attributes {
  fn type_value(&self) -> &str {
    match self {
      Attributes::PackedToken(attr) => attr.sub.type_value(),
      Attributes::BinaryTokenStream(attr) => attr.get_packed_token().sub.type_value(),
      _ => unimplemented!("not support"),
    }
  }

  fn set_type(&mut self, type_: &str) {
    match self {
      Attributes::PackedToken(attr) => attr.sub.set_type(type_),
      Attributes::BinaryTokenStream(attr) => attr.get_packed_token_mut().sub.set_type(type_),
      _ => unimplemented!("not support"),
    }
  }
}

impl TermToBytesRefAttribute for Attributes {
  fn get_bytes_ref(&mut self) -> Option<Cow<'_, BytesRef<Vec<u8>>>> {
    match self {
      Attributes::BytesTerm(attr) => TermToBytesRefAttribute::get_bytes_ref(attr),
      Attributes::BinaryTokenStream(attr) => {
        TermToBytesRefAttribute::get_bytes_ref(attr.get_binary_mut())
      },
      _ => unimplemented!("not support"),
    }
  }
}

impl BytesTermAttribute for Attributes {
  fn set_bytes_ref(&mut self, bytes: Option<BytesRef<Vec<u8>>>) -> Result<()> {
    match self {
      Attributes::BytesTerm(attr) => BytesTermAttribute::set_bytes_ref(attr, bytes),
      Attributes::BinaryTokenStream(attr) => {
        BytesTermAttribute::set_bytes_ref(attr.get_binary_mut(), bytes)
      },
      _ => Err(LuceneError::unsupported_operation("")),
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
            fn start_offset(&self) -> Result<i32> {
                match self {
                    $(Self::$V(t) => t.start_offset(),)+
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
