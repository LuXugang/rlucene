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
use crate::test::core::index::base_field_info_format_test_case::BaseFieldInfoFormatTestCase;
use crate::test::core::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use rand::Rng;
#[allow(dead_code)] // for quick search
pub struct TestLucene94FieldInfosFormat;

impl BaseIndexFileFormatTestCase for TestLucene94FieldInfosFormat {
  fn add_random_fields<R>(_random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    todo!()
  }
}

impl BaseFieldInfoFormatTestCase for TestLucene94FieldInfosFormat {}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestLucene94FieldInfosFormat, &mut rand::prelude::StdRng) -> Result<()>,
{
  let mut random = random();
  let test = TestLucene94FieldInfosFormat;
  f(&test, &mut random)
}

#[test]
fn test_one_field() -> Result<()> {
  run_case(|test, random| test.test_one_field(random))
}

#[test]
fn test_immutable_attributes() -> Result<()> {
  run_case(|test, random| test.test_immutable_attributes(random))
}

#[test]
fn test_exception_on_create_output() -> Result<()> {
  run_case(|test, random| test.test_exception_on_create_output(random))
}

#[test]
fn test_exception_on_close_output() -> Result<()> {
  run_case(|test, random| test.test_exception_on_close_output(random))
}

#[test]
fn test_exception_on_open_input() -> Result<()> {
  run_case(|test, random| test.test_exception_on_open_input(random))
}

#[test]
fn test_exception_on_close_input() -> Result<()> {
  run_case(|test, random| test.test_exception_on_close_input(random))
}

#[test]
fn test_random() -> Result<()> {
  run_case(|test, random| test.test_random(random))
}
