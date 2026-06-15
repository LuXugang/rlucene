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
use crate::core::analysis::analyzer::{Analyzer, AnalyzerStoredValue, TokenStreamComponents};
use crate::core::analysis::char_array_set::CharArraySet;
use crate::core::analysis::filtering_token_filter::FilteringTokenFilter;
use crate::core::analysis::lower_case_filter::LowerCaseFilter;
use crate::core::analysis::standard::standard_tokenizer::{
  MAX_TOKEN_LENGTH_LIMIT, StandardTokenizer,
};
use crate::core::analysis::stop_filter::StopFilter;
use crate::core::analysis::stop_word_analyzer_base::{StopWordAnalyzerBase, init_stop_wors};
use crate::core::analysis::token_stream::NormalizeTokenStream;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

/// Default maximum allowed token length
pub const DEFAULT_MAX_TOKEN_LENGTH: usize = 255;
/// Filters `StandardTokenizer` with `LowerCaseFilter` and `StopFilter`,
/// using a configurable list of stop words.
pub struct StandardAnalyzer {
  max_token_length: usize,
  stop_words: Arc<CharArraySet>,
  stored_value: AnalyzerStoredValue,
}

impl Default for StandardAnalyzer {
  fn default() -> Self {
    Self::new()
  }
}

impl StandardAnalyzer {
  pub fn new() -> Self {
    let stop_words = Arc::new(init_stop_wors(None));
    Self {
      max_token_length: DEFAULT_MAX_TOKEN_LENGTH,
      stop_words,
      stored_value: AnalyzerStoredValue::new(),
    }
  }
  /// Sets the maximum allowed token length.
  ///
  /// Tokens longer than this value will be split at this length and emitted as
  /// multiple tokens. To skip such large tokens instead, you can increase this
  /// limit and then use `LengthFilter` to remove long tokens. The default value
  /// is `StandardAnalyzer::DEFAULT_MAX_TOKEN_LENGTH`.
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

pub type StandardAnalyzerTS = FilteringTokenFilter<LowerCaseFilter<StandardTokenizer>, StopFilter>;
impl Analyzer for StandardAnalyzer {
  fn create_components(&self, _field: &str) -> Result<TokenStreamComponents> {
    let mut src = StandardTokenizer::new();
    src.set_max_token_length(self.max_token_length)?;
    let tok = StopFilter::new(LowerCaseFilter::new(src), self.stop_words.clone());
    Ok(TokenStreamComponents::new(tok, Some(self.max_token_length)))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }

  fn normalize_from_ts(
    &self,
    _field_name: &str,
    in_: NormalizeTokenStream,
  ) -> Result<NormalizeTokenStream> {
    Ok(LowerCaseFilter::new(Box::new(in_)).into())
  }

  fn get_offset_gap(&self, field_name: &str) -> i32 {
    self.default_get_offset_gap(field_name)
  }
}
impl StopWordAnalyzerBase for StandardAnalyzer {
  fn get_stop_words(&self) -> &CharArraySet {
    self.stop_words.as_ref()
  }
}
