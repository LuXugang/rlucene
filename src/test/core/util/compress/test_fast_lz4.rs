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

use crate::core::util::compress::lz4::FastCompressionHashTable;
use crate::core::util::compress::lz4::HashTableEnum;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::compress::lz4_test_case::{AssertingHashTable, LZ4TestCase};
use crate::test_framework::core::util::lucene_test_case::random;
use rand::prelude::StdRng;

struct TestFastLZ4;
impl LZ4TestCase for TestFastLZ4 {
  fn new_hash_table(&self) -> AssertingHashTable {
    AssertingHashTable::new(HashTableEnum::Fast(FastCompressionHashTable::new()))
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestFastLZ4, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestFastLZ4;
  f(&case, &mut random)
}

mod lz4_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::util::compress::lz4_test_case::LZ4TestCase;
  use crate::test::core::util::compress::test_fast_lz4::run_case;

  #[test]
  fn test_empty_fast() -> Result<()> {
    run_case(|case, random| case.test_empty(random))
  }

  #[test]
  fn test_short_literals_and_matches_fast() -> Result<()> {
    run_case(|case, random| case.test_short_literals_and_matches(random))
  }

  #[test]
  fn test_long_matches_fast() -> Result<()> {
    run_case(|case, random| case.test_long_matches(random))
  }

  #[test]
  fn test_long_literals_fast() -> Result<()> {
    run_case(|case, random| case.test_long_literals(random))
  }

  #[test]
  fn test_match_right_before_last_literals_fast() -> Result<()> {
    run_case(|case, random| case.test_match_right_before_last_literals(random))
  }

  #[test]
  fn test_incompressible_random_fast() -> Result<()> {
    run_case(|case, random| case.test_incompressible_random(random))
  }

  #[test]
  fn test_compressible_random_fast() -> Result<()> {
    run_case(|case, random| case.test_compressible_random(random))
  }

  #[test]
  fn test_lucene5201_fast() -> Result<()> {
    run_case(|case, random| case.test_lucene5201(random))
  }

  #[test]
  fn test_use_dictionary_fast() -> Result<()> {
    run_case(|case, random| case.test_use_dictionary(random))
  }
}
