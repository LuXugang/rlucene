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
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::base_token_stream_test_case::assert_analyzes_to9;
use crate::test_framework::core::analysis::mock_char_filter::MockCharFilter;
use crate::test_framework::core::analysis::mock_tokenizer::{MockTokenizer, WHITESPACE};
use crate::test_framework::core::util::lucene_test_case::random;
use parking_lot::Mutex;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

#[allow(dead_code)] // for quick search
struct TestMockCharFilter;

#[test]
fn test() -> Result<()> {
  let mut random = random();
  let analyzer = MockCharFilterAnalyzer {
    random: Mutex::new(StdRng::seed_from_u64(random.random())),
    stored_value: AnalyzerStoredValue::global(),
  };

  assert_analyzes_to9(
    &mut random,
    &analyzer,
    "ab",
    &["aab"],
    Some(&[0]),
    Some(&[2]),
  )?;
  assert_analyzes_to9(
    &mut random,
    &analyzer,
    "aba",
    &["aabaa"],
    Some(&[0]),
    Some(&[3]),
  )?;
  assert_analyzes_to9(
    &mut random,
    &analyzer,
    "abcdefga",
    &["aabcdefgaa"],
    Some(&[0]),
    Some(&[8]),
  )
}

struct MockCharFilterAnalyzer {
  random: Mutex<StdRng>,
  stored_value: AnalyzerStoredValue,
}

impl Analyzer for MockCharFilterAnalyzer {
  fn create_components(&self, _field_name: &str) -> Result<TokenStreamComponents> {
    let tokenizer = MockTokenizer::with_default_max_token_length(
      StdRng::seed_from_u64(self.random.lock().random()),
      WHITESPACE.clone(),
      false,
    );
    Ok(TokenStreamComponents::new(
      Box::new(tokenizer) as Box<dyn TokenStream + Send + Sync>,
      None,
    ))
  }

  fn init_reader(&self, _field_name: &str, reader: ReaderEnum) -> ReaderEnum {
    ReaderEnum::MockCharFilter(MockCharFilter::new(reader, 7).expect("valid remainder"))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

crate::impl_analyzer_close!(MockCharFilterAnalyzer);
