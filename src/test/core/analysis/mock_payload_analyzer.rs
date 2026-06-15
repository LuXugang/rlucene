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
use crate::core::analysis::analyzer::{
  Analyzer, AnalyzerEnum, AnalyzerStoredValue, TokenStreamComponents,
};
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_filter::{TokenFilter, TokenFilterBase};
use crate::core::analysis::token_stream::TokenStream;
use crate::core::index::bytes_ref::BytesRef;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_tokenizer::MockTokenizer;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;

/// Wraps a whitespace tokenizer with a filter that sets the first token, and odd tokens to posinc=1,
/// and all others to 0, encoding the position as pos: XXX in the payload.
pub struct MockPayloadAnalyzer {
  stored_value: AnalyzerStoredValue,
}

impl MockPayloadAnalyzer {
  pub fn new() -> Self {
    Self {
      stored_value: AnalyzerStoredValue::per_field(),
    }
  }
}

impl Default for MockPayloadAnalyzer {
  fn default() -> Self {
    Self::new()
  }
}

impl Analyzer for MockPayloadAnalyzer {
  fn create_components(&self, field_name: &str) -> Result<TokenStreamComponents> {
    let tokenizer = MockTokenizer::new(random());
    let filter = MockPayloadFilter::new(tokenizer, field_name.to_string());
    Ok(TokenStreamComponents::new(
      Box::new(filter) as Box<dyn TokenStream + Send + Sync>,
      None,
    ))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

impl From<MockPayloadAnalyzer> for AnalyzerEnum {
  fn from(analyzer: MockPayloadAnalyzer) -> Self {
    AnalyzerEnum::Custom(Box::new(analyzer))
  }
}

struct MockPayloadFilter<TS>
where
  TS: TokenStream,
{
  token_filter_base: TokenFilterBase<TS>,
  _field_name: String,
  pos: i32,
  i: i32,
}

impl<TS> MockPayloadFilter<TS>
where
  TS: TokenStream,
{
  fn new(input: TS, field_name: String) -> Self {
    Self {
      token_filter_base: TokenFilterBase::new(input),
      _field_name: field_name,
      pos: 0,
      i: 0,
    }
  }
}

impl<TS> TokenStream for MockPayloadFilter<TS>
where
  TS: TokenStream,
{
  fn increment_token(&mut self) -> Result<bool> {
    if self.token_filter_base.input.increment_token()? {
      let attr = self.token_filter_base.input.get_attribute_source_mut();
      attr.set_payload(Some(BytesRef::from_string(&format!("pos: {}", self.pos))))?;
      let pos_incr = if self.pos == 0 || self.i % 2 == 1 {
        1
      } else {
        0
      };
      attr.set_position_increment(pos_incr)?;
      self.pos += pos_incr;
      self.i += 1;
      Ok(true)
    } else {
      Ok(false)
    }
  }

  fn end(&mut self) -> Result<()> {
    self.token_filter_base.end()
  }

  fn reset(&mut self) -> Result<()> {
    self.token_filter_base.reset()?;
    self.i = 0;
    self.pos = 0;
    Ok(())
  }

  fn close(&mut self) -> Result<()> {
    self.token_filter_base.close()
  }

  fn set_reader(&mut self, input: ReaderEnum) -> Result<()> {
    self.token_filter_base.input.set_reader(input)
  }

  fn set_reader_test_point(&mut self) -> Result<()> {
    self.token_filter_base.input.set_reader_test_point()
  }

  fn get_attribute_source(&self) -> &Attributes {
    self.token_filter_base.input.get_attribute_source()
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    self.token_filter_base.input.get_attribute_source_mut()
  }
}

impl<TS> TokenFilter for MockPayloadFilter<TS> where TS: TokenStream {}
