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
use crate::core::util::compress::lz4::{HashTableEnum, HighCompressionHashTable};
use crate::test::core::util::compress::lz4_test_case::{AssertingHashTable, LZ4TestCase};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;

#[allow(dead_code)] // for quick search
struct TestHighLZ4;
impl LZ4TestCase for TestHighLZ4 {
  fn new_hash_table(&self) -> AssertingHashTable {
    AssertingHashTable::new(HashTableEnum::High(HighCompressionHashTable::new()))
  }
}
#[test]
fn test_empty_high() -> crate::core::util::error::lucene_error::Result<()> {
  let mut random = random();
  let case = TestHighLZ4;
  case.test_empty(&mut random)
}
#[test]
fn test_short_literals_and_matches_high() -> crate::core::util::error::lucene_error::Result<()> {
  let mut random = random();
  let case = TestHighLZ4;
  case.test_short_literals_and_matches(&mut random)
}
#[test]
fn test_long_matches_high() -> crate::core::util::error::lucene_error::Result<()> {
  let mut random = random();
  let case = TestHighLZ4;
  case.test_long_matches(&mut random)
}
#[test]
fn test_long_literals_high() -> crate::core::util::error::lucene_error::Result<()> {
  let mut random = random();
  let case = TestHighLZ4;
  case.test_long_literals(&mut random)
}
#[test]
fn test_match_right_before_last_literals_high() -> crate::core::util::error::lucene_error::Result<()>
{
  let mut random = random();
  let case = TestHighLZ4;
  case.test_match_right_before_last_literals(&mut random)
}
#[test]
fn test_incompressible_random_high() -> crate::core::util::error::lucene_error::Result<()> {
  let mut random = random();
  let case = TestHighLZ4;
  case.test_incompressible_random(&mut random)
}
// TODO IMPORTANT 这个测试太慢了
#[cfg(feature = "nightly")]
#[test]
#[ignore = "nightly"]
fn test_compressible_random_high() -> crate::core::util::error::lucene_error::Result<()> {
  let mut random = random();
  let case = TestHighLZ4;
  case.test_compressible_random(&mut random)
}
#[test]
fn test_lucene5201_high() -> crate::core::util::error::lucene_error::Result<()> {
  let mut random = random();
  let case = TestHighLZ4;
  case.test_lucene5201(&mut random)
}
#[test]
fn test_use_dictionary_high() -> crate::core::util::error::lucene_error::Result<()> {
  let mut random = random();
  let case = TestHighLZ4;
  case.test_use_dictionary(&mut random)
}
