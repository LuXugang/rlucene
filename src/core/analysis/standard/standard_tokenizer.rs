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

use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::standard::standard_tokenizer_impl::{StandardTokenizerImpl, YYEOF};
use crate::core::analysis::token_attributes::char_term_attribute::CharTermAttribute;
use crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::core::analysis::token_attributes::type_attribute::TypeAttribute;
use crate::core::analysis::token_stream::{TokenStream, default_attribute};
use crate::core::analysis::tokenizer::{Tokenizer, TokenizerBase};
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub struct StandardTokenizer {
  scanner: StandardTokenizerImpl,
  skipped_positions: i32,
  max_token_length: i32,
  tokenizer_base: TokenizerBase,
}

pub const ALPHANUM: i32 = 0;
pub const NUM: i32 = 1;
pub const SOUTHEAST_ASIAN: i32 = 2;
pub const IDEOGRAPHIC: i32 = 3;
pub const HIRAGANA: i32 = 4;
pub const KATAKANA: i32 = 5;
pub const HANGUL: i32 = 6;
pub const EMOJI: i32 = 7;

pub const DEFAULT_MAX_TOKEN_LENGTH: i32 = 255;
pub const MAX_TOKEN_LENGTH_LIMIT: i32 = 1024 * 1024;

pub const TOKEN_TYPES: [&str; 8] = [
  "<ALPHANUM>",
  "<NUM>",
  "<SOUTHEAST_ASIAN>",
  "<IDEOGRAPHIC>",
  "<HIRAGANA>",
  "<KATAKANA>",
  "<HANGUL>",
  "<EMOJI>",
];

impl StandardTokenizer {
  pub fn new() -> Self {
    Self::with_att(default_attribute())
  }

  pub fn with_att(att: Attributes) -> Self {
    let tokenizer_base = TokenizerBase::new(att);
    let scanner = StandardTokenizerImpl::new(ReaderEnum::default());
    Self {
      scanner,
      skipped_positions: 0,
      max_token_length: DEFAULT_MAX_TOKEN_LENGTH,
      tokenizer_base,
    }
  }

  pub fn set_max_token_length(&mut self, length: i32) -> Result<()> {
    if length < 1 {
      return Err(LuceneError::illegal_argument(
        "maxTokenLength must be greater than zero",
      ));
    } else if length > MAX_TOKEN_LENGTH_LIMIT {
      return Err(LuceneError::illegal_argument(format!(
        "maxTokenLength may not exceed {MAX_TOKEN_LENGTH_LIMIT}"
      )));
    }
    if length != self.max_token_length {
      self.max_token_length = length;
      self.scanner.set_buffer_size(length as usize);
    }
    Ok(())
  }

  pub fn get_max_token_length(&self) -> i32 {
    self.max_token_length
  }
}

impl Default for StandardTokenizer {
  fn default() -> Self {
    Self::new()
  }
}

impl TokenStream for StandardTokenizer {
  fn increment_token(&mut self) -> Result<bool> {
    self.tokenizer_base.token_stream_base.att.clear_attributes();
    self.skipped_positions = 0;

    loop {
      let token_type = self.scanner.get_next_token()?;
      if token_type == YYEOF {
        return Ok(false);
      }

      if self.scanner.yylength() <= self.max_token_length {
        self
          .tokenizer_base
          .token_stream_base
          .att
          .set_position_increment(self.skipped_positions + 1)?;
        self
          .scanner
          .get_text(&mut self.tokenizer_base.token_stream_base.att);
        let start = self.scanner.yychar();
        let end = start + self.tokenizer_base.token_stream_base.att.length() as i32;
        let start = self.correct_offset(start);
        let end = self.correct_offset(end);
        self
          .tokenizer_base
          .token_stream_base
          .att
          .set_offset(start, end)?;
        self.tokenizer_base.token_stream_base.att.set_type(
          TOKEN_TYPES
            .get(token_type as usize)
            .ok_or_else(|| LuceneError::illegal_state("invalid token type"))?,
        );
        return Ok(true);
      }

      self.skipped_positions += 1;
    }
  }

  fn end(&mut self) -> Result<()> {
    self.tokenizer_base.end()?;
    let final_offset = self.correct_offset(self.scanner.yychar() + self.scanner.yylength());
    self
      .tokenizer_base
      .token_stream_base
      .att
      .set_offset(final_offset, final_offset)?;
    let position_increment = self
      .tokenizer_base
      .token_stream_base
      .att
      .get_position_increment()
      .unwrap_or(0);
    self
      .tokenizer_base
      .token_stream_base
      .att
      .set_position_increment(position_increment + self.skipped_positions)
  }

  fn reset(&mut self) -> Result<()> {
    self.tokenizer_base.reset()?;
    self.scanner.yyreset(self.tokenizer_base.input.clone());
    self.skipped_positions = 0;
    Ok(())
  }

  fn close(&mut self) -> Result<()> {
    self.tokenizer_base.close()?;
    self.scanner.yyreset(self.tokenizer_base.input.clone());
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

impl Tokenizer for StandardTokenizer {
  fn get_tokenizer_base_mut(&mut self) -> &mut TokenizerBase {
    &mut self.tokenizer_base
  }

  fn get_tokenizer_base(&self) -> &TokenizerBase {
    &self.tokenizer_base
  }
}
