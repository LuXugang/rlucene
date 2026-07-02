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
use crate::test_framework::core::util::lucene_test_case::random;
use rand::prelude::StdRng;

use crate::codecs_tests::compressing::abstract_test_compression_mod::AbstractTestCompressionMode;
use crate::core::codecs::compression::compression_mode::CompressionModeEnum;
use crate::core::codecs::lz4_with_preset_dict_compression_mode::LZ4WithPresetDictCompressionMode;
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)]
pub struct TestLZ4WithPresetDictCompressionMode;

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestLZ4WithPresetDictCompressionMode, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestLZ4WithPresetDictCompressionMode;
  f(&case, &mut random)
}

impl AbstractTestCompressionMode for TestLZ4WithPresetDictCompressionMode {
  fn get_mode(&self) -> CompressionModeEnum {
    CompressionModeEnum::LZ4Dict(LZ4WithPresetDictCompressionMode)
  }
}

mod abstract_test_compression_mode {
  use super::*;

  #[test]
  fn test_decompress() -> Result<()> {
    run_case(|case, random| case.test_decompress(random))
  }

  #[test]
  fn test_partial_decompress() -> Result<()> {
    run_case(|case, random| case.test_partial_decompress(random))
  }

  #[test]
  fn test_empty_sequence() -> Result<()> {
    run_case(|case, _random| case.test_empty_sequence())
  }

  #[test]
  fn test_short_sequence() -> Result<()> {
    run_case(|case, random| case.test_short_sequence(random))
  }

  #[test]
  fn test_incompressible() -> Result<()> {
    run_case(|case, random| case.test_incompressible(random))
  }

  #[test]
  fn test_constant() -> Result<()> {
    run_case(|case, random| case.test_constant(random))
  }

  #[test]
  fn test_extremely_large_input() -> Result<()> {
    run_case(|case, _random| case.test_extremely_large_input())
  }
}
