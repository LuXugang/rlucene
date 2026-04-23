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
use crate::core::analysis::char_array_set::CharArraySet;
use crate::core::analysis::filtering_token_filter::FilteringTokenFilter;
use crate::core::analysis::lower_case_filter::LowerCaseFilter;
use crate::core::analysis::standard::standard_tokenizer::{
  MAX_TOKEN_LENGTH_LIMIT, StandardTokenizer,
};
use crate::core::analysis::stop_filter::StopFilter;
use crate::core::analysis::stop_word_analyzer_base::{StopWordAnalyzerBase, init_stop_wors};
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

/// Default maximum allowed token length
pub const DEFAULT_MAX_TOKEN_LENGTH: usize = 255;
/// Filters [`StandardTokenizer`] with [`LowerCaseFilter`] and [`StopFilter`],
/// using a configurable list of stop words.
pub struct StandardAnalyzer {
  base: AnalyzerBase<GlobalReuseStrategy>,
  max_token_length: usize,
  stop_words: Arc<CharArraySet>,
}

pub type StandardAnalyzerTS = FilteringTokenFilter<LowerCaseFilter<StandardTokenizer>, StopFilter>;
impl Default for StandardAnalyzer {
  fn default() -> Self {
    Self::new()
  }
}

impl StandardAnalyzer {
  pub fn new() -> Self {
    let stop_words = Arc::new(init_stop_wors(None));
    Self {
      base: AnalyzerBase::new(),
      max_token_length: DEFAULT_MAX_TOKEN_LENGTH,
      stop_words,
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

impl Analyzer for StandardAnalyzer {
  fn create_components(&self, _field: &str) -> Result<TokenStreamComponents> {
    let mut src = StandardTokenizer::new();
    src.set_max_token_length(self.max_token_length)?;
    let tok = StopFilter::new(LowerCaseFilter::new(src), self.stop_words.clone());
    // TODO IMPORTANT
    Ok(TokenStreamComponents::new(tok))
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
impl StopWordAnalyzerBase for StandardAnalyzer {
  fn get_stop_words(&self) -> &CharArraySet {
    self.stop_words.as_ref()
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::analysis::common::analysis_impl::core::whitespace_analyzer::WhitespaceAnalyzer;
  use crate::test::core::analysis::base_token_stream_test_case::assert_analyzes_to6;
  use rand::rng;
  #[allow(dead_code)]
  struct TestStandardAnalyzer;

  #[test]
  fn test_farsi() -> Result<()> {
    let a = WhitespaceAnalyzer::new();
    let mut random = rng();
    let input =
      "ویکی پدیای انگلیسی در تاریخ ۲۵ دی ۱۳۷۹ به صورت مکملی برای دانشنامهٔ تخصصی نوپدیا نوشته شد.";
    let expected = [
      "ویکی",
      "پدیای",
      "انگلیسی",
      "در",
      "تاریخ",
      "۲۵",
      "دی",
      "۱۳۷۹",
      "به",
      "صورت",
      "مکملی",
      "برای",
      "دانشنامهٔ",
      "تخصصی",
      "نوپدیا",
      "نوشته",
      "شد",
    ];
    assert_analyzes_to6(&mut random, &a, input, &expected)
  }

  #[test]
  fn test_numeric_sa() -> Result<()> {
    let a = StandardAnalyzer::new();
    let mut random = rng();

    assert_analyzes_to6(&mut random, &a, "21.35", &["21.35"])?;
    assert_analyzes_to6(&mut random, &a, "R2D2 C3PO", &["r2d2", "c3po"])?;
    assert_analyzes_to6(&mut random, &a, "216.239.63.104", &["216.239.63.104"])?;
    assert_analyzes_to6(&mut random, &a, "216.239.63.104", &["216.239.63.104"])?;
    Ok(())
  }
}
