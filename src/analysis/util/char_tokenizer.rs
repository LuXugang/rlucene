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
use crate::analysis::character_utils::{CharacterBuffer, CharacterUtils};
use crate::analysis::standard::standard_tokenizer::MAX_TOKEN_LENGTH_LIMIT;
use crate::analysis::token_attributes::char_term_attribute::CharTermAttribute;
use crate::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::analysis::token_attributes::packed_token_attribute_impl::PackedTokenAttributeImpl;
use crate::analysis::token_stream::TokenStream;
use crate::analysis::tokenizer::{Tokenizer, TokenizerBase};
use crate::util::attribute::Attribute;
use crate::util::error::lucene_error::{LuceneError, Result};

pub trait CharTokenizer: Tokenizer {
    fn get_char_tokenizer_base(&mut self) -> &mut CharTokenizerBase;
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

pub const DEFAULT_MAX_WORD_LEN: i32 = 255;
const I_BUFFER_SIZE: i32 = 4096;
pub struct CharTokenizerBase {
    offset: i32,
    buffer_index: i32,
    data_len: i32,
    final_offset: i32,
    max_token_len: i32,
    io_buffer: CharacterBuffer,
    att: Attributes,
    tokenizer_base: TokenizerBase,
}
impl CharTokenizerBase {
    pub fn new() -> Result<Self> {
        Self::with_max_token_len(DEFAULT_MAX_WORD_LEN)
    }
    pub fn with_max_token_len(max_token_len: i32) -> Result<Self> {
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
            att: Attributes::PackedToken(PackedTokenAttributeImpl::new()),
            tokenizer_base: TokenizerBase::new(),
        })
    }
}

enum Attributes {
    PackedToken(PackedTokenAttributeImpl),
}

impl Attribute for Attributes {}

impl CharTermAttribute for Attributes {
    fn length(&self) -> usize {
        match self {
            Attributes::PackedToken(attr) => attr.length(),
        }
    }

    fn copy_buffer(&mut self, buffer: &[char], offset: usize, length: usize) {
        match self {
            Attributes::PackedToken(attr) => attr.copy_buffer(buffer, offset, length),
        }
    }

    fn buffer(&mut self) -> &mut [char] {
        match self {
            Attributes::PackedToken(attr) => attr.buffer(),
        }
    }

    fn resize_buffer(&mut self, new_size: usize) -> &mut [char] {
        match self {
            Attributes::PackedToken(attr) => attr.resize_buffer(new_size),
        }
    }

    fn set_length(&mut self, length: usize) -> Result<&mut Self> {
        match self {
            Attributes::PackedToken(attr) => {
                attr.set_length(length)?;
                Ok(self)
            },
        }
    }

    fn set_empty(&mut self) -> &mut Self {
        match self {
            Attributes::PackedToken(attr) => {
                attr.set_empty();
                self
            },
        }
    }

    fn append_range(&mut self, csq: &str, start: usize, end: usize) -> &mut Self {
        match self {
            Attributes::PackedToken(attr) => {
                attr.append_range(csq, start, end);
                self
            },
        }
    }

    fn append_char(&mut self, c: char) -> &mut Self {
        match self {
            Attributes::PackedToken(attr) => {
                attr.append_char(c);
                self
            },
        }
    }

    fn append_str(&mut self, s: Option<&str>) -> &mut Self {
        match self {
            Attributes::PackedToken(attr) => {
                attr.append_str(s);
                self
            },
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
        }
    }
}
impl OffsetAttribute for Attributes {
    fn start_offset(&self) -> i32 {
        match self {
            Attributes::PackedToken(attr) => attr.start_offset(),
        }
    }

    fn set_offset(&mut self, start_offset: i32, end_offset: i32) -> Result<()> {
        match self {
            Attributes::PackedToken(attr) => attr.set_offset(start_offset, end_offset),
        }
    }

    fn end_offset(&self) -> i32 {
        match self {
            Attributes::PackedToken(attr) => attr.end_offset(),
        }
    }
}
