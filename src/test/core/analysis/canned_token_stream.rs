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
use crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::token::Token;

pub struct CannedTokenStream {
  attr: Attributes,
  tokens: Vec<Token>,
  upto: usize,
  final_offset: i32,
  final_pos_inc: i32,
}
impl CannedTokenStream {
  pub fn new(tokens: Vec<Token>) -> CannedTokenStream {
    Self::with_offset_pos_inc(0, 0, tokens)
  }
  pub fn with_offset_pos_inc(
    final_offset: i32,
    final_pos_inc: i32,
    tokens: Vec<Token>,
  ) -> CannedTokenStream {
    let attr = Attributes::default();
    CannedTokenStream {
      attr,
      tokens,
      upto: 0,
      final_offset,
      final_pos_inc,
    }
  }
}
impl TokenStream for CannedTokenStream {
  fn increment_token(&mut self) -> Result<bool> {
    if self.upto < self.tokens.len() {
      self.clear_attributes();

      match self.attr {
        Attributes::PackedToken(ref mut token_attr) => self.tokens[self.upto].copy_to(token_attr),
        _ => unreachable!("PackedTokenAttribute not found in CannedTokenStream"),
      };

      self.upto += 1;

      Ok(true)
    } else {
      Ok(false)
    }
  }

  fn end(&mut self) -> Result<()> {
    AttributeSource::set_position_increment(&mut self.attr, self.final_pos_inc)?;
    self.attr.set_offset(self.final_offset, self.final_offset)?;
    Ok(())
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
impl AttributeSource for CannedTokenStream {
  fn clear_attributes(&mut self) {
    self.attr.clear_attributes()
  }
}
