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
  Analyzer, AnalyzerBase, GlobalReuseStrategy, TokenStreamComponents,
};
use crate::core::analysis::lower_case_filter::LowerCaseFilter;
use crate::core::analysis::standard::standard_tokenizer::{
  MAX_TOKEN_LENGTH_LIMIT, StandardTokenizer,
};
use crate::core::analysis::token_stream::{InnerTokenStreams, StandardAnalyzerTS, TokenStream};
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
pub const DEFAULT_MAX_TOKEN_LENGTH: usize = 255;
pub struct StandardAnalyzer {
  base: AnalyzerBase<StandardAnalyzerTS, GlobalReuseStrategy<StandardAnalyzerTS>>,
  max_token_length: usize,
}

impl Default for StandardAnalyzer {
  fn default() -> Self {
    Self::new()
  }
}

impl StandardAnalyzer {
  pub fn new() -> Self {
    Self {
      base: AnalyzerBase::new(),
      max_token_length: DEFAULT_MAX_TOKEN_LENGTH,
    }
  }

  pub fn set_max_token_length(&mut self, length: usize) -> Result<()> {
    if length < 1 {
      return Err(LuceneError::illegal_argument(
        "maxTokenLength must be greater than zero",
      ));
    } else if length > MAX_TOKEN_LENGTH_LIMIT {
      return Err(LuceneError::illegal_argument(format!(
        "maxTokenLength may not exceed {MAX_TOKEN_LENGTH_LIMIT}"
      )));
    }
    self.max_token_length = length;
    Ok(())
  }

  pub fn get_max_token_length(&self) -> usize {
    self.max_token_length
  }
}

impl Analyzer for StandardAnalyzer {
  fn create_components(&self, _field: &str) -> Result<TokenStreamComponents<InnerTokenStreams>> {
    let mut src = StandardTokenizer::new();
    src.set_max_token_length(self.max_token_length)?;
    Ok(TokenStreamComponents::new(InnerTokenStreams::Standard(
      LowerCaseFilter::new(src),
    )))
  }

  type TokenStream<TS>
    = LowerCaseFilter<TS>
  where
    TS: TokenStream;

  fn normalize_from_ts<TS>(&self, _field_name: &str, in_: TS) -> Result<Self::TokenStream<TS>>
  where
    TS: TokenStream,
  {
    Ok(LowerCaseFilter::new(in_))
  }
}
