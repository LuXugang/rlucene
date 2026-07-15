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
use crate::core::analysis::token_attributes::packed_token_and_binary::BinaryTokenStreamAttributeImpl;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::index::BytesRef;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::error::lucene_error::Result;

/// TokenStream from a canned list of binary (BytesRef-based) tokens.
pub struct CannedBinaryTokenStream {
  attr: Attributes,
  tokens: Vec<BinaryToken>,
  upto: usize,
}

impl CannedBinaryTokenStream {
  pub fn new(tokens: Vec<BinaryToken>) -> Result<Self> {
    Ok(Self {
      attr: BinaryTokenStreamAttributeImpl::new()?.into(),
      tokens,
      upto: 0,
    })
  }
}

impl crate::core::util::close::Closeable for CannedBinaryTokenStream {}

impl TokenStream for CannedBinaryTokenStream {
  fn increment_token(&mut self) -> Result<bool> {
    if self.upto < self.tokens.len() {
      self.clear_attributes()?;

      let token = &self.tokens[self.upto];
      self
        .attr
        .set_bytes_ref(Some(BytesRef::deep_copy_of(&token.term)))?;
      self.attr.set_position_increment(token.pos_inc)?;
      self.attr.set_position_length(token.pos_len)?;
      self.attr.set_offset(token.start_offset, token.end_offset)?;
      self.upto += 1;

      Ok(true)
    } else {
      Ok(false)
    }
  }

  fn end(&mut self) -> Result<()> {
    self.default_end()
  }

  fn reset(&mut self) -> Result<()> {
    self.upto = 0;
    Ok(())
  }

  fn get_attribute_source(&self) -> &Attributes {
    &self.attr
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    &mut self.attr
  }
}

impl AttributeSource for CannedBinaryTokenStream {
  fn clear_attributes(&mut self) -> Result<()> {
    self.attr.clear_attributes()
  }
}

/// Represents a binary token.
pub struct BinaryToken {
  term: BytesRef<Vec<u8>>,
  pos_inc: i32,
  pos_len: i32,
  start_offset: i32,
  end_offset: i32,
}

impl BinaryToken {
  pub fn new(term: BytesRef<Vec<u8>>) -> Self {
    Self {
      term,
      pos_inc: 1,
      pos_len: 1,
      start_offset: 0,
      end_offset: 0,
    }
  }

  pub fn with_pos_inc_pos_len(term: BytesRef<Vec<u8>>, pos_inc: i32, pos_len: i32) -> Self {
    Self {
      term,
      pos_inc,
      pos_len,
      start_offset: 0,
      end_offset: 0,
    }
  }
}
