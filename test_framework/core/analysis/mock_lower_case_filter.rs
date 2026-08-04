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
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_filter::{TokenFilter, TokenFilterBase};
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;

/// A lowercasing [`TokenFilter`].
pub struct MockLowerCaseFilter<TS>
where
  TS: TokenStream,
{
  token_filter_base: TokenFilterBase<TS>,
}

impl<TS> MockLowerCaseFilter<TS>
where
  TS: TokenStream,
{
  /// Sole constructor.
  pub fn new(input: TS) -> Self {
    Self {
      token_filter_base: TokenFilterBase::new(input),
    }
  }
}

impl<TS> Closeable for MockLowerCaseFilter<TS>
where
  TS: TokenStream,
{
  fn close(&mut self) -> Result<()> {
    self.token_filter_base.close()
  }
}

impl<TS> TokenStream for MockLowerCaseFilter<TS>
where
  TS: TokenStream,
{
  fn increment_token(&mut self) -> Result<bool> {
    if self.token_filter_base.input.increment_token()? {
      let attr = self.get_attribute_source_mut();
      let length = attr.length()?;
      CharacterUtils::convert_to_lower_case(attr.buffer_mut()?, 0, length);
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

  fn set_reader(&mut self, input: ReaderEnum) -> Result<()> {
    self.token_filter_base.set_reader(input)
  }

  fn set_reader_test_point(&mut self) -> Result<()> {
    self.token_filter_base.set_reader_test_point()
  }

  fn get_attribute_source(&self) -> &Attributes {
    self.token_filter_base.input.get_attribute_source()
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    self.token_filter_base.input.get_attribute_source_mut()
  }
}

impl<TS> TokenFilter for MockLowerCaseFilter<TS> where TS: TokenStream {}
