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
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::base_token_stream_test_case::assert_analyzes_to6;
use rand::rng;

#[allow(dead_code)] // for quick search
struct TestWhitespaceAnalyzer;

const LONGTOKEN: &str = concat!(
  "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
  "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
  "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz"
);

#[test]
fn test_default_maximum_token_length() -> Result<()> {
  let a = WhitespaceAnalyzer::new();
  let mut random = rng();
  let long_token_input = format!("{LONGTOKEN} extra");
  let expected_first = concat!(
    "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
    "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
    "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstu"
  );
  let expected = [
    expected_first,
    "vwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz",
    "extra",
  ];
  assert_analyzes_to6(&mut random, &a, &long_token_input, &expected)
}

#[test]
fn test_custom_maximum_token_length() -> Result<()> {
  let a = WhitespaceAnalyzer::with_max_token_length(1024);
  let mut random = rng();
  let long_token_input = format!("{LONGTOKEN} extra");
  let expected = [LONGTOKEN, "extra"];
  assert_analyzes_to6(&mut random, &a, &long_token_input, &expected)
}
