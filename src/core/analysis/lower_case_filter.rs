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
use crate::core::analysis::character_utils::CharacterUtils;
use crate::core::analysis::token_attributes::char_term_attribute::CharTermAttribute;
use crate::core::analysis::token_attributes::packed_token_attribute_impl::PackedTokenAttributeImpl;
use crate::core::analysis::token_filter::{TokenFilter, TokenFilterBase};
use crate::core::analysis::token_stream::{TokenStream, TokenStreamBase};
use crate::core::util::attribute_source::Attributes;
use crate::core::util::error::lucene_error::Result;
/// Normalizes token text to lower case.
pub struct LowerCaseFilter<TS>
where
  TS: TokenStream,
{
  token_filter_base: TokenFilterBase<TS>,
  token_stream_base: TokenStreamBase,
}
impl<TS> LowerCaseFilter<TS>
where
  TS: TokenStream,
{
  /// Create a new `LowerCaseFilter` that normalizes token text to lower case.
  ///
  /// # Parameters
  ///
  /// - `in_`: `TokenStream` to filter.
  pub fn new(input: TS) -> Self {
    let token_filter_base = TokenFilterBase::new(input);
    let token_stream_base = TokenStreamBase::new(PackedTokenAttributeImpl::default().into());
    Self {
      token_filter_base,
      token_stream_base,
    }
  }
}

impl<TS> TokenStream for LowerCaseFilter<TS>
where
  TS: TokenStream,
{
  fn increment_token(&mut self) -> Result<bool> {
    if self.token_filter_base.input.increment_token()? {
      let attr = self.get_attribute_source_mut();
      let len = attr.length();
      CharacterUtils::convert_to_lower_case(attr.buffer_mut(), 0, len);
      Ok(true)
    } else {
      Ok(false)
    }
  }

  fn end(&mut self) -> Result<()> {
    self.token_filter_base.end()
  }

  fn reset(&mut self) -> Result<()> {
    self.token_filter_base.reset()
  }

  fn close(&mut self) -> Result<()> {
    self.token_filter_base.close()
  }

  fn get_attribute_source(&self) -> &Attributes {
    &self.token_stream_base.att
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    &mut self.token_stream_base.att
  }
}

impl<TS> TokenFilter for LowerCaseFilter<TS> where TS: TokenStream {}
