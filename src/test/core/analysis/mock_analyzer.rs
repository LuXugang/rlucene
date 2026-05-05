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
  Analyzer, PerFieldReuseStrategy, ReuseStrategyEnum, TokenStreamComponents,
};
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::attribute_source::Attributes;
use crate::core::util::automation::character_run_automaton::CharacterRunAutomaton;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_token_filter::{EMPTY_STOPSET, MockTokenFilter};
pub(crate) use crate::test::core::analysis::mock_tokenizer::{
  DEFAULT_MAX_TOKEN_LENGTH, MockTokenizer, WHITESPACE,
};
use rand::prelude::StdRng;
use rand::{Rng, RngExt, SeedableRng};
use std::sync::Mutex;

/// Analyzer for testing
///
/// This analyzer is a replacement for Whitespace/Simple/KeywordAnalyzers for unit tests. If you
/// are testing a custom component such as a queryparser or analyzer-wrapper that consumes analysis
/// streams, it's a great idea to test it with this analyzer instead. MockAnalyzer has the following
/// behavior:
///
/// - By default, the assertions in [`MockTokenizer`] are turned on for extra checks that the
///   consumer is consuming properly. These checks can be disabled with
///   [`MockAnalyzer::set_enable_checks`].
/// - Payload data is randomly injected into the stream for more thorough testing of payloads.
///
/// See [`MockTokenizer`].
pub struct MockAnalyzer {
  run_automaton: CharacterRunAutomaton,
  lower_case: bool,
  filter: CharacterRunAutomaton,
  position_increment_gap: i32,
  offset_gap: Option<i32>,
  random: Mutex<StdRng>,
  enable_checks: bool,
  max_token_length: i32,
}
impl MockAnalyzer {
  /// Create a Whitespace-lowercasing analyzer with no stopwords removal.
  pub fn new<R>(random: &mut R) -> MockAnalyzer
  where
    R: Rng + ?Sized,
  {
    Self::with_automaton(random, WHITESPACE.clone(), true)
  }

  pub fn with_automaton<R>(
    random: &mut R,
    run_automaton: CharacterRunAutomaton,
    lower_case: bool,
  ) -> MockAnalyzer
  where
    R: Rng + ?Sized,
  {
    Self::with_filter(random, run_automaton, lower_case, EMPTY_STOPSET.clone())
  }
  /// Creates a new [`MockAnalyzer`].
  ///
  /// # Parameters
  ///
  /// - `random` - Random for payloads behavior
  /// - `run_automaton` - DFA describing how tokenization should happen (e.g. `[a-zA-Z]+`)
  /// - `lower_case` - true if the tokenizer should lowercase terms
  /// - `filter` - DFA describing how terms should be filtered (set of stopwords, etc)
  pub fn with_filter<R>(
    random: &mut R,
    run_automaton: CharacterRunAutomaton,
    lower_case: bool,
    filter: CharacterRunAutomaton,
  ) -> MockAnalyzer
  where
    R: Rng + ?Sized,
  {
    MockAnalyzer {
      run_automaton,
      lower_case,
      filter,
      position_increment_gap: 0,
      offset_gap: None,
      random: Mutex::new(StdRng::seed_from_u64(random.random())),
      enable_checks: true,
      max_token_length: DEFAULT_MAX_TOKEN_LENGTH,
    }
  }

  fn next_random(&self) -> StdRng {
    StdRng::seed_from_u64(self.random.lock().expect("random mutex poisoned").random())
  }

  pub fn set_position_increment_gap(&mut self, position_increment_gap: i32) {
    self.position_increment_gap = position_increment_gap;
  }
  /// Set a new offset gap which will then be added to the offset when several fields with the same
  /// name are indexed
  ///
  /// # Parameters
  ///
  /// - `offset_gap` - The offset gap that should be used.
  pub fn set_offset_gap(&mut self, offset_gap: i32) {
    self.offset_gap = Some(offset_gap);
  }
  /// Toggle consumer workflow checking: if your test consumes tokenstreams normally you should leave
  /// this enabled.
  pub fn set_enable_checks(&mut self, enable_checks: bool) {
    self.enable_checks = enable_checks;
  }
  /// Toggle maxTokenLength for MockTokenizer
  pub fn set_max_token_length(&mut self, length: i32) {
    self.max_token_length = length;
  }
}
impl Analyzer for MockAnalyzer {
  fn create_components(&self, field: &str) -> Result<TokenStreamComponents> {
    let mut tokenizer = MockTokenizer::with_automaton(
      self.next_random(),
      self.run_automaton.clone(),
      self.lower_case,
      self.max_token_length,
    );
    tokenizer.set_enable_checks(self.enable_checks);
    let filter = MockTokenFilter::new(tokenizer, self.filter.clone());
    let v = MockFilterWrap::new(filter);
    let _ = field;
    Ok(TokenStreamComponents::new(
      Box::new(v) as Box<dyn TokenStream + Send + Sync>,
      None,
    ))
  }

  fn init_reuse_strategy(&self) -> ReuseStrategyEnum {
    ReuseStrategyEnum::PerField(PerFieldReuseStrategy::default())
  }

  type TokenStream<TS>
    = TS
  where
    TS: TokenStream;

  fn normalize_from_ts<TS>(&self, _field_name: &str, in_: TS) -> Result<Self::TokenStream<TS>>
  where
    TS: TokenStream,
  {
    self.default_normalize_from_ts(_field_name, in_)
  }

  fn default_normalize_from_ts<TS>(&self, _field_name: &str, in_: TS) -> Result<TS>
  where
    TS: TokenStream,
  {
    Ok(in_)
  }

  fn get_position_increment_gap(&self, field_name: &str) -> i32 {
    let _ = field_name;
    self.position_increment_gap
  }
  /// Get the offset gap between tokens in fields if several fields with the same name were added.
  ///
  /// # Parameters
  ///
  /// - `field_name` - Currently not used, the same offset gap is returned for each field.
  fn get_offset_gap(&self, _field_name: &str) -> i32 {
    match self.offset_gap {
      Some(gap) => gap,
      None => self.default_get_offset_gap(_field_name),
    }
  }
}
pub struct MockFilterWrap<TS>
where
  TS: TokenStream,
{
  filter: MockTokenFilter<TS>,
}
impl<TS> MockFilterWrap<TS>
where
  TS: TokenStream,
{
  pub fn new(filter: MockTokenFilter<TS>) -> Self {
    Self { filter }
  }
}
impl<TS> TokenStream for MockFilterWrap<TS>
where
  TS: TokenStream,
{
  fn increment_token(&mut self) -> Result<bool> {
    self.filter.increment_token()
  }

  fn end(&mut self) -> Result<()> {
    self.filter.end()
  }

  fn default_end(&mut self) -> Result<()> {
    self.filter.default_end()
  }

  fn reset(&mut self) -> Result<()> {
    self.filter.reset()
  }

  fn default_reset(&mut self) -> Result<()> {
    self.filter.default_end()
  }

  fn close(&mut self) -> Result<()> {
    self.filter.close()
  }

  fn get_attribute_source(&self) -> &Attributes {
    self.filter.get_attribute_source()
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    self.filter.get_attribute_source_mut()
  }

  fn set_reader(&mut self, input: ReaderEnum) -> Result<()> {
    self.filter.set_reader(input)
  }

  fn set_reader_test_point(&mut self) -> Result<()> {
    self.filter.set_reader_test_point()
  }
}
