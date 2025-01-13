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
use crate::common::my_random;
use crate::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::index::base_segment_info_format_test_case::BaseSegmentInfoFormatTestCase;
use crate::util::test_error::TestError;
use rlucene::codecs::lucene101_codec::Lucene101Codec;
use rlucene::codecs::LATEST_CODEC;
use rlucene::util::{Version, LATEST};

pub struct TestLucene99SegmentInfoFormat;

impl BaseIndexFileFormatTestCase for TestLucene99SegmentInfoFormat {}

impl BaseSegmentInfoFormatTestCase for TestLucene99SegmentInfoFormat {
    fn get_versions(&self) -> Vec<Version> {
        Vec::from([LATEST.clone()])
    }
}

#[test]
fn test_files() -> Result<(), TestError> {
    let mut random = my_random("test_files".to_string());
    let test = TestLucene99SegmentInfoFormat;
    test.test_files(&mut random)
}
#[test]
fn test_has_blocks() -> Result<(), TestError> {
    let mut random = my_random("test_has_blocks".to_string());
    let test = TestLucene99SegmentInfoFormat;
    test.test_has_blocks(&mut random)
}
#[test]
fn test_adds_self_to_files() -> Result<(), TestError> {
    let mut random = my_random("test_adds_self_to_files".to_string());
    let test = TestLucene99SegmentInfoFormat;
    test.test_adds_self_to_files(&mut random)
}
#[test]
fn test_diagnostics() -> Result<(), TestError> {
    let mut random = my_random("test_diagnostics".to_string());
    let test = TestLucene99SegmentInfoFormat;
    test.test_diagnostics(&mut random)
}
#[test]
fn test_attributes() -> Result<(), TestError> {
    let mut random = my_random("test_attributes".to_string());
    let test = TestLucene99SegmentInfoFormat;
    test.test_attributes(&mut random)
}
#[test]
fn test_unique_id() -> Result<(), TestError> {
    let mut random = my_random("test_unique_id".to_string());
    let test = TestLucene99SegmentInfoFormat;
    test.test_unique_id(&mut random)
}
#[test]
fn test_versions() -> Result<(), TestError> {
    let mut random = my_random("test_versions".to_string());
    let test = TestLucene99SegmentInfoFormat;
    test.test_versions(&mut random)
}
