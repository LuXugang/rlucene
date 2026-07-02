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
use crate::core::analysis::char_filter::CharFilter;
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::reusable_string_reader::ReusableStringReader;
use crate::core::util::error::lucene_error::Result;
pub use crate::test_framework::core::analysis::char_filter::{CharFilter1, CharFilter2};

#[allow(dead_code)]
struct TestCharFilter;
#[test]
fn test_char_filter1() -> Result<()> {
  let mut reader = ReusableStringReader::new();
  reader.set_value("");
  let cs = CharFilter1::new(ReaderEnum::ReusedString(reader));
  assert_eq!(1, cs.correct_offset(0), "corrected offset is invalid");
  Ok(())
}
#[test]
fn test_char_filter2() -> Result<()> {
  let mut reader = ReusableStringReader::new();
  reader.set_value("");
  let cs = CharFilter2::new(ReaderEnum::ReusedString(reader));
  assert_eq!(2, cs.correct_offset(0), "corrected offset is invalid");
  Ok(())
}

#[test]
fn test_char_filter12() -> Result<()> {
  let mut reader = ReusableStringReader::new();
  reader.set_value("");
  let cs = CharFilter2::new(ReaderEnum::CharFilter1(CharFilter1::new(
    ReaderEnum::ReusedString(reader),
  )));
  assert_eq!(3, cs.correct_offset(0), "corrected offset is invalid");
  Ok(())
}

#[test]
fn test_char_filter11() -> Result<()> {
  let mut reader = ReusableStringReader::new();
  reader.set_value("");
  let cs = CharFilter1::new(ReaderEnum::CharFilter1(CharFilter1::new(
    ReaderEnum::ReusedString(reader),
  )));
  assert_eq!(2, cs.correct_offset(0), "corrected offset is invalid");
  Ok(())
}
