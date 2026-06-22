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
use crate::core::util::error::lucene_error::Result;
use crate::core::util::{LATEST, Version};
use crate::test::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::core::index::base_segment_info_format_test_case::BaseSegmentInfoFormatTestCase;
use crate::test::core::util::lucene_test_case::random;
use rand::Rng;
#[allow(dead_code)] // for quick search
pub struct TestLucene99SegmentInfoFormat;

mod base_segment_info_format_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::codecs::lucene99::test_lucene99_segment_info_format::run_case;
  use crate::test::core::index::base_segment_info_format_test_case::BaseSegmentInfoFormatTestCase;

  #[test]
  fn test_files() -> Result<()> {
    run_case(|case, random| case.test_files(random))
  }

  #[test]
  fn test_has_blocks() -> Result<()> {
    run_case(|case, random| case.test_has_blocks(random))
  }

  #[test]
  fn test_adds_self_to_files() -> Result<()> {
    run_case(|case, random| case.test_adds_self_to_files(random))
  }

  #[test]
  fn test_diagnostics() -> Result<()> {
    run_case(|case, random| case.test_diagnostics(random))
  }

  #[test]
  fn test_attributes() -> Result<()> {
    run_case(|case, random| case.test_attributes(random))
  }

  #[test]
  fn test_unique_id() -> Result<()> {
    run_case(|case, random| case.test_unique_id(random))
  }

  #[test]
  fn test_versions() -> Result<()> {
    run_case(|case, random| case.test_versions(random))
  }

  #[test]
  fn test_sort() -> Result<()> {
    run_case(|case, random| case.test_sort(random))
  }

  #[test]
  fn test_exception_on_create_output() -> Result<()> {
    run_case(|case, _random| case.test_exception_on_create_output())
  }

  #[test]
  fn test_exception_on_close_output() -> Result<()> {
    run_case(|case, _random| case.test_exception_on_close_output())
  }

  #[test]
  fn test_exception_on_open_input() -> Result<()> {
    run_case(|case, _random| case.test_exception_on_open_input())
  }

  #[test]
  fn test_exception_on_close_input() -> Result<()> {
    run_case(|case, _random| case.test_exception_on_close_input())
  }

  #[test]
  fn test_random() -> Result<()> {
    run_case(|case, random| case.test_random(random))
  }
}

impl BaseIndexFileFormatTestCase for TestLucene99SegmentInfoFormat {
  fn add_random_fields<R>(_random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    todo!()
  }
}

impl BaseSegmentInfoFormatTestCase for TestLucene99SegmentInfoFormat {
  fn get_versions(&self) -> Vec<Version> {
    Vec::from([LATEST.clone()])
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestLucene99SegmentInfoFormat, &mut rand::prelude::StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestLucene99SegmentInfoFormat;
  f(&case, &mut random)
}
