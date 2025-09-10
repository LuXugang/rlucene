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
use crate::core::analysis::standard::standard_tokenizer::MAX_TOKEN_LENGTH_LIMIT;
use crate::core::analysis::token_attributes::char_term_attribute::CharTermAttribute;
use crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::core::analysis::token_stream::{TokenStream, default_attribute};
use crate::core::analysis::tokenizer::{Tokenizer, TokenizerBase};
use crate::core::util::attribute_source::Attributes;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// An trait for simple, character-oriented tokenizers.
pub trait CharTokenizer: Tokenizer {
    fn get_char_tokenizer_base(&mut self) -> &mut CharTokenizerBase;
    /// Returns true iff a codepoint should be included in a token.
    fn is_token_char(&self, c: &char) -> bool;
    fn end(&mut self) -> Result<()> {
        TokenStream::end(self)?;
        // set final offset
        let base = self.get_char_tokenizer_base();
        let final_offset = base.final_offset;
        self.get_char_tokenizer_base()
            .att
            .set_offset(final_offset, final_offset)
    }
    fn reset(&mut self) -> Result<()> {
        TokenStream::reset(self)?;
        let base = self.get_char_tokenizer_base();
        base.buffer_index = 0;
        base.offset = 0;
        base.data_len = 0;
        base.final_offset = 0;
        base.io_buffer.reset();
        Ok(())
    }
    fn increment_token(&mut self) -> Result<bool> {
        // TODO: clear_attributes 未实现
        // self.clear_attributes();
        let mut length: usize = 0;
        let mut start: i32 = 0;
        let mut end: i32 = 0;
        loop {
            let base = self.get_char_tokenizer_base();
            if base.buffer_index >= base.data_len {
                base.offset += base.data_len;
                // // read supplementary char aware with CharacterUtils
                CharacterUtils::fill(&mut base.io_buffer, &mut base.tokenizer_base.input)?;
                if base.io_buffer.get_length() == 0 {
                    base.data_len = 0;
                    if length > 0 {
                        break;
                    } else {
                        let offset = base.offset;
                        self.get_char_tokenizer_base().final_offset = self.correct_offset(offset);
                        return Ok(false);
                    }
                }
                base.data_len = base.io_buffer.get_length() as i32;
                base.buffer_index = 0;
            }
            let c = base.io_buffer.get_buffer()[base.buffer_index as usize];
            base.buffer_index += 1;
            if self.is_token_char(&c) {
                let base = self.get_char_tokenizer_base();
                if length == 0 {
                    // start of token
                    debug_assert_eq!(start, -1);
                    start = base.offset + base.buffer_index - 1;
                    end = start;
                } else if length >= base.att.buffer().len() - 1 {
                    base.att.resize_buffer(2 + length);
                }

                base.att.buffer()[length] = c;
                length += 1;
                end += 1;

                if length >= base.max_token_len as usize {
                    break;
                }
            } else if length > 0 {
                break;
            }
        }
        let correct_start = self.correct_offset(start);
        let correct_end = self.correct_offset(end);
        let base = self.get_char_tokenizer_base();
        base.att.set_length(length)?;
        debug_assert_ne!(start, -1);
        base.final_offset = correct_end;
        base.att.set_offset(correct_start, base.final_offset)?;

        Ok(true)
    }
}
/// Creates a new instance of `CharTokenizer` using a custom predicate, supplied as a method
/// reference or lambda expression.
/// The predicate should return `true` for all valid token characters.
pub fn from_token_char_predicate(
    token_char_predicate: fn(i32) -> bool,
) -> Result<CharTokenizerImpl> {
    from_token_char_predicate_with_attr(default_attribute(), token_char_predicate)
}

/// Creates a new instance of CharTokenizer with the supplied attribute factory using a custom predicate, supplied as method reference or lambda expression. The predicate should return true for all valid token characters.
pub fn from_token_char_predicate_with_attr(
    att: Attributes,
    f: fn(i32) -> bool,
) -> Result<CharTokenizerImpl> {
    CharTokenizerImpl::new(att, f)
}
/// Creates a new instance of CharTokenizer using a custom predicate,
/// supplied as method reference or lambda expression.
/// The predicate should return true for all valid token separator characters.
/// This method is provided for convenience to easily use predicates that are negated (they match the separator characters, not the token characters).
pub fn from_separator_char_predicate(
    separator_char_predicate: fn(i32) -> bool,
) -> Result<CharTokenizerImpl> {
    from_separator_char_predicate_with_attr(default_attribute(), separator_char_predicate)
}
/// Creates a new instance of CharTokenizer with the supplied attribute factory using a custom predicate,
/// supplied as method reference or lambda expression.
/// The predicate should return true for all valid token separator characters.
pub fn from_separator_char_predicate_with_attr(
    att: Attributes,
    separator_char_predicate: fn(i32) -> bool,
) -> Result<CharTokenizerImpl> {
    from_token_char_predicate_with_attr(att, separator_char_predicate)
}

pub const DEFAULT_MAX_WORD_LEN: i32 = 255;
const I_BUFFER_SIZE: i32 = 4096;
pub struct CharTokenizerBase {
    offset: i32,
    buffer_index: i32,
    data_len: i32,
    final_offset: i32,
    max_token_len: i32,
    io_buffer: CharacterBuffer,
    pub(crate) att: Attributes,
    pub(crate) tokenizer_base: TokenizerBase,
}
impl CharTokenizerBase {
    pub fn new() -> Result<Self> {
        Self::with_max_token_len(default_attribute(), DEFAULT_MAX_WORD_LEN)
    }
    pub fn with_att(att: Attributes) -> Result<Self> {
        Self::with_max_token_len(att, DEFAULT_MAX_WORD_LEN)
    }
    pub fn with_max_token_len(att: Attributes, max_token_len: i32) -> Result<Self> {
        if max_token_len > MAX_TOKEN_LENGTH_LIMIT || max_token_len == 0 {
            return Err(LuceneError::illegal_argument(format!(
                "maxTokenLen must be greater than 0 and less than {}, passed: {}",
                MAX_TOKEN_LENGTH_LIMIT, max_token_len
            )));
        }
        Ok(CharTokenizerBase {
            offset: 0,
            buffer_index: 0,
            data_len: 0,
            final_offset: 0,
            max_token_len,
            io_buffer: CharacterUtils::new_character_buffer(I_BUFFER_SIZE as usize)?,
            att,
            tokenizer_base: TokenizerBase::new(),
        })
    }
}

pub struct CharTokenizerImpl {
    base: CharTokenizerBase,
    token_char_predicate: fn(i32) -> bool,
}
impl CharTokenizerImpl {
    fn new(att: Attributes, token_char_predicate: fn(i32) -> bool) -> Result<Self> {
        Ok(CharTokenizerImpl {
            base: CharTokenizerBase::with_att(att)?,
            token_char_predicate,
        })
    }
}

impl Tokenizer for CharTokenizerImpl {
    fn get_tokenizer_base_mut(&mut self) -> &mut TokenizerBase {
        &mut self.base.tokenizer_base
    }

    fn get_tokenizer_base(&self) -> &TokenizerBase {
        &self.base.tokenizer_base
    }
}

impl TokenStream for CharTokenizerImpl {
    fn increment_token(&mut self) -> Result<bool> {
        CharTokenizer::increment_token(self)
    }

    fn end(&mut self) -> Result<()> {
        CharTokenizer::end(self)
    }

    fn reset(&mut self) -> Result<()> {
        CharTokenizer::reset(self)
    }

    fn close(&mut self) -> Result<()> {
        Tokenizer::close(self)
    }

    type AttributeSource = Attributes;

    fn get_attribute_source(&self) -> &Self::AttributeSource {
        &self.base.att
    }

    fn get_attribute_source_mut(&mut self) -> &mut Self::AttributeSource {
        &mut self.base.att
    }
}

impl CharTokenizer for CharTokenizerImpl {
    fn get_char_tokenizer_base(&mut self) -> &mut CharTokenizerBase {
        &mut self.base
    }

    fn is_token_char(&self, c: &char) -> bool {
        (self.token_char_predicate)(*c as i32)
    }
}
