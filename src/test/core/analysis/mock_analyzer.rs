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

use crate::analysis::common::analysis_impl::core::whitespace_analyzer::WhitespaceAnalyzer;
use crate::core::analysis::analyzer::{Analyzer, ReuseStrategyEnum, TokenStreamComponents};
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_stream::{InnerTokenStreams, TokenStream, TokenStreams};
use crate::core::index::BytesRef;
use crate::core::util::attribute_source::Attributes;
use crate::core::util::error::lucene_error::Result;
use rand::Rng;

pub struct MockAnalyzer {
  in_: WhitespaceAnalyzer,
}
impl MockAnalyzer {
  pub fn new<R>(_random: &mut R) -> MockAnalyzer
  where
    R: Rng + ?Sized,
  {
    // TODO IMPORTANT only support WhitespaceAnalyzer now
    MockAnalyzer {
      in_: WhitespaceAnalyzer::new(),
    }
  }
  pub fn set_enable_checks(&mut self, _enable_checks: bool) {}
}
impl Analyzer for MockAnalyzer {
  fn create_components(&self, field: &str) -> Result<TokenStreamComponents<InnerTokenStreams>> {
    self.in_.create_components(field)
  }

  fn init_reuse_strategy(&self) -> ReuseStrategyEnum {
    self.in_.init_reuse_strategy()
  }

  type TokenStream<TS>
    = TS
  where
    TS: TokenStream;

  fn normalize_from_ts<TS>(&self, _field_name: &str, in_: TS) -> Result<Self::TokenStream<TS>>
  where
    TS: TokenStream + Into<TokenStreams>,
  {
    self.in_.normalize_from_ts(_field_name, in_)
  }

  fn default_normalize_from_ts<TS>(&self, _field_name: &str, in_: TS) -> Result<TS>
  where
    TS: TokenStream,
  {
    self.in_.default_normalize_from_ts(_field_name, in_)
  }

  fn ensure_reuse_strategy<'a>(
    &'a self,
    slot: &'a mut Option<ReuseStrategyEnum>,
  ) -> &'a mut ReuseStrategyEnum {
    self.in_.ensure_reuse_strategy(slot)
  }

  fn token_stream<R>(&self, field_name: &str, input: R) -> Result<()>
  where
    R: Into<ReaderEnum>,
  {
    self.in_.token_stream(field_name, input)
  }

  fn normalize(&self, field_name: &str, text: &str) -> Result<BytesRef<Vec<u8>>> {
    self.in_.normalize(field_name, text)
  }

  fn init_reader(&self, _filed_name: &str, reader: ReaderEnum) -> ReaderEnum {
    self.in_.init_reader(_filed_name, reader)
  }

  fn init_reader_for_normalization(&self, filed_name: &str, reader: ReaderEnum) -> ReaderEnum {
    self.in_.init_reader_for_normalization(filed_name, reader)
  }

  fn attribute_factory(&self, field_name: &str) -> Attributes {
    self.in_.attribute_factory(field_name)
  }

  fn get_position_increment_gap(&self, field_name: &str) -> i32 {
    self.in_.get_position_increment_gap(field_name)
  }

  fn get_offset_gap(&self, _field_name: &str) -> i32 {
    self.in_.get_offset_gap(_field_name)
  }
}
