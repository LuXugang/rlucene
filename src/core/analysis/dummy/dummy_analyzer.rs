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
use crate::core::analysis::analyzer::{Analyzer, ReuseStrategyEnum, TokenStreamComponents};
use crate::core::analysis::dummy::dummy_token_stream::DummyTokenStream;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::error::lucene_error::Result;

pub struct DummyAnalyzer;
impl Analyzer for DummyAnalyzer {
  fn create_components(&self, _field: &str) -> Result<TokenStreamComponents> {
    dummy_unreachable!()
  }

  fn init_reuse_strategy(&self) -> ReuseStrategyEnum {
    dummy_unreachable!()
  }

  type TokenStream<TS>
    = DummyTokenStream
  where
    TS: TokenStream;

  fn normalize_from_ts<TS>(&self, _field_name: &str, _in_: TS) -> Result<Self::TokenStream<TS>>
  where
    TS: TokenStream,
  {
    dummy_unreachable!()
  }

  fn default_normalize_from_ts<TS>(&self, _field_name: &str, _in_: TS) -> Result<TS>
  where
    TS: TokenStream,
  {
    dummy_unreachable!()
  }

  fn get_position_increment_gap(&self, _field_name: &str) -> i32 {
    dummy_unreachable!()
  }

  fn get_offset_gap(&self, _field_name: &str) -> i32 {
    dummy_unreachable!()
  }
}
