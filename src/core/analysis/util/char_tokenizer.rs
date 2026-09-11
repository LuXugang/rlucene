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
use crate::core::analysis::character_utils::{CharacterBuffer, CharacterUtils};
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::standard::standard_tokenizer::MAX_TOKEN_LENGTH_LIMIT;
use crate::core::analysis::token_stream::{TokenStream, default_attribute};
use crate::core::analysis::tokenizer::{Tokenizer, TokenizerBase};
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub struct CharTokenizer<S> {
  offset: i32,
  buffer_index: usize,
  data_len: usize,
  final_offset: i32,
  max_token_len: usize,
  io_buffer: CharacterBuffer,
  pub(crate) tokenizer_base: TokenizerBase,
  sub: S,
}
impl<S> CharTokenizer<S>
where
  S: CharTokenizerBase,
{
  pub fn new(sub: S) -> Result<Self> {
    Self::with_max_token_len(default_attribute()?, DEFAULT_MAX_WORD_LEN, sub)
  }
  pub fn with_att(att: Attributes, sub: S) -> Result<Self> {
    Self::with_max_token_len(att, DEFAULT_MAX_WORD_LEN, sub)
  }
  pub fn with_max_token_len(att: Attributes, max_token_len: usize, sub: S) -> Result<Self> {
    if max_token_len > MAX_TOKEN_LENGTH_LIMIT {
      return Err(LuceneError::illegal_argument(format!(
        "maxTokenLen must be greater than 0 and less than {}, passed: {}",
        MAX_TOKEN_LENGTH_LIMIT, max_token_len
      )));
    }
    Ok(CharTokenizer {
      offset: 0,
      buffer_index: 0,
      data_len: 0,
      final_offset: 0,
      max_token_len,
      io_buffer: CharacterUtils::new_character_buffer(I_BUFFER_SIZE)?,
      tokenizer_base: TokenizerBase::new(att),
      sub,
    })
  }
}

impl<S> Drop for CharTokenizer<S> {
  fn drop(&mut self) {
    let _ = self.tokenizer_base.close();
  }
}

impl<S> Closeable for CharTokenizer<S>
where
  S: CharTokenizerBase,
{
  fn close(&mut self) -> Result<()> {
    self.tokenizer_base.close()
  }
}

impl<S> TokenStream for CharTokenizer<S>
where
  S: CharTokenizerBase,
{
  fn increment_token(&mut self) -> Result<bool> {
    self
      .tokenizer_base
      .token_stream_base
      .att
      .clear_attributes()?;
    let mut length: usize = 0;
    let mut start: i32 = -1;
    let mut end: i32 = -1;
    loop {
      if self.buffer_index >= self.data_len {
        self.offset += self.data_len as i32;
        // // read supplementary char aware with CharacterUtils
        CharacterUtils::fill(&mut self.io_buffer, &mut self.tokenizer_base.input)?;
        if self.io_buffer.get_length() == 0 {
          self.data_len = 0;
          if length > 0 {
            break;
          } else {
            let offset = self.offset;
            self.final_offset = self.correct_offset(offset);
            return Ok(false);
          }
        }
        self.data_len = self.io_buffer.get_length();
        self.buffer_index = 0;
      }
      let c = self.io_buffer.get_buffer()[self.buffer_index];
      self.buffer_index += 1;
      if self.sub.is_token_char(&c) {
        let att = &mut self.tokenizer_base.token_stream_base.att;
        let mut buffer = att.buffer_mut()?;
        if length == 0 {
          // start of token
          debug_assert_eq!(start, -1);
          start = self.offset + self.buffer_index as i32 - 1;
          end = start;
        } else if length >= buffer.len() - 1 {
          buffer = att.resize_buffer(2 + length)?;
        }

        buffer[length] = c;
        length += 1;
        end += 1;

        if length >= self.max_token_len {
          break;
        }
      } else if length > 0 {
        break;
      }
    }
    self
      .tokenizer_base
      .token_stream_base
      .att
      .set_length(length)?;
    debug_assert_ne!(start, -1);
    self.final_offset = self.correct_offset(end);
    self
      .tokenizer_base
      .token_stream_base
      .att
      .set_offset(self.correct_offset(start), self.final_offset)?;
    Ok(true)
  }

  fn end(&mut self) -> Result<()> {
    self.tokenizer_base.end()?;
    // set final offset
    self
      .tokenizer_base
      .token_stream_base
      .att
      .set_offset(self.final_offset, self.final_offset)
  }

  fn reset(&mut self) -> Result<()> {
    self.tokenizer_base.reset()?;
    self.buffer_index = 0;
    self.offset = 0;
    self.data_len = 0;
    self.final_offset = 0;
    self.io_buffer.reset();
    Ok(())
  }

  fn get_attribute_source(&self) -> &Attributes {
    self.tokenizer_base.get_attribute_source()
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    self.tokenizer_base.get_attribute_source_mut()
  }

  fn set_reader(&mut self, input: ReaderEnum) -> Result<()> {
    self.tokenizer_base.set_reader(input)
  }
}

impl<S> Tokenizer for CharTokenizer<S>
where
  S: CharTokenizerBase,
{
  fn get_tokenizer_base_mut(&mut self) -> &mut TokenizerBase {
    &mut self.tokenizer_base
  }

  fn get_tokenizer_base(&self) -> &TokenizerBase {
    &self.tokenizer_base
  }
}
/// An trait for simple, character-oriented tokenizers.
pub trait CharTokenizerBase {
  fn is_token_char(&self, c: &char) -> bool;
}
/// Creates a new instance of [`CharTokenizer`] using a custom predicate, supplied as a method
/// reference or lambda expression.
/// The predicate should return `true` for all valid token characters.
/// Function pointers and closures with captured state are both accepted.
pub fn from_token_char_predicate<P>(
  token_char_predicate: P,
) -> Result<CharTokenizer<CharTokenizerImpl<P>>>
where
  P: Fn(i32) -> bool,
{
  from_token_char_predicate_with_attr(default_attribute()?, token_char_predicate)
}

/// Creates a new instance of CharTokenizer with the supplied attribute factory using a custom predicate, supplied as method reference or lambda expression. The predicate should return true for all valid token characters.
pub fn from_token_char_predicate_with_attr<P>(
  att: Attributes,
  f: P,
) -> Result<CharTokenizer<CharTokenizerImpl<P>>>
where
  P: Fn(i32) -> bool,
{
  CharTokenizerImpl::new(att, f)
}
/// Creates a new instance of CharTokenizer using a custom predicate,
/// supplied as method reference or lambda expression.
/// The predicate should return true for all valid token separator characters.
/// This method is provided for convenience to easily use predicates that are negated (they match the separator characters, not the token characters).
pub fn from_separator_char_predicate<P>(
  separator_char_predicate: P,
) -> Result<CharTokenizer<CharTokenizerImpl<P>>>
where
  P: Fn(i32) -> bool,
{
  from_separator_char_predicate_with_attr(default_attribute()?, separator_char_predicate)
}
/// Creates a new instance of CharTokenizer with the supplied attribute factory using a custom predicate,
/// supplied as method reference or lambda expression.
/// The predicate should return true for all valid token separator characters.
pub fn from_separator_char_predicate_with_attr<P>(
  att: Attributes,
  separator_char_predicate: P,
) -> Result<CharTokenizer<CharTokenizerImpl<P>>>
where
  P: Fn(i32) -> bool,
{
  from_token_char_predicate_with_attr(att, separator_char_predicate)
}

pub const DEFAULT_MAX_WORD_LEN: usize = 255;
const I_BUFFER_SIZE: usize = 4096;

pub struct CharTokenizerImpl<P = fn(i32) -> bool> {
  token_char_predicate: P,
}
impl<P> CharTokenizerImpl<P>
where
  P: Fn(i32) -> bool,
{
  fn new(att: Attributes, token_char_predicate: P) -> Result<CharTokenizer<CharTokenizerImpl<P>>> {
    let v = CharTokenizerImpl {
      token_char_predicate,
    };
    CharTokenizer::with_att(att, v)
  }
}
impl<P> CharTokenizerBase for CharTokenizerImpl<P>
where
  P: Fn(i32) -> bool,
{
  fn is_token_char(&self, c: &char) -> bool {
    (self.token_char_predicate)(*c as i32)
  }
}
