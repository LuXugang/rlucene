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
use crate::index::base_compound_format_test_case::BaseCompoundFormatTestCase;
use crate::util::test_error::TestError;

pub struct TestLucene90CompoundFormat;
impl BaseCompoundFormatTestCase for TestLucene90CompoundFormat {}
#[test]
fn test_empty() -> Result<(), TestError> {
    let mut random = my_random("test_empty".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_empty(&mut random)
}
#[test]
fn test_single_file() -> Result<(), TestError> {
    let mut random = my_random("test_single_file".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_single_file(&mut random)
}
#[test]
fn test_two_files() -> Result<(), TestError> {
    let mut random = my_random("test_two_files".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_two_files(&mut random)
}
#[test]
fn test_double_close() -> Result<(), TestError> {
    let case = TestLucene90CompoundFormat;
    case.test_double_close()
}
#[test]
fn test_pass_io_context() -> Result<(), TestError> {
    let case = TestLucene90CompoundFormat;
    case.test_pass_io_context()
}
#[test]
fn test_large_cfs() -> Result<(), TestError> {
    let case = TestLucene90CompoundFormat;
    case.test_large_cfs()
}
#[test]
fn test_list_all() -> Result<(), TestError> {
    let case = TestLucene90CompoundFormat;
    case.test_list_all()
}
#[test]
fn test_create_output_disabled() -> Result<(), TestError> {
    let mut random = my_random("test_create_output_disabled".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_create_output_disabled(&mut random)
}
#[test]
fn test_delete_file_disabled() -> Result<(), TestError> {
    let mut random = my_random("test_delete_file_disabled".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_delete_file_disabled(&mut random)
}
#[test]
fn test_rename_file_disabled() -> Result<(), TestError> {
    let mut random = my_random("test_rename_file_disabled".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_rename_file_disabled(&mut random)
}
#[test]
fn test_sync_disabled() -> Result<(), TestError> {
    let mut random = my_random("test_sync_disabled".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_sync_disabled(&mut random)
}
#[test]
fn test_make_lock_disabled() -> Result<(), TestError> {
    let mut random = my_random("test_make_lock_disabled".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_make_lock_disabled(&mut random)
}
#[test]
fn test_random_files() -> Result<(), TestError> {
    let mut random = my_random("test_random_files".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_random_files(&mut random)
}
#[test]
fn test_many_sub_files() -> Result<(), TestError> {
    let mut random = my_random("test_many_sub_files".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_many_sub_files(&mut random)
}
