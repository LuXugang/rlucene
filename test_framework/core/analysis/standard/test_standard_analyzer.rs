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
use crate::core::analysis::reader::{Reader, ReaderEnum};
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)] // for quick search
struct TestStandardAnalyzer;

#[derive(Debug, Clone)]
pub struct SpoonFeedMaxCharsReaderWrapper {
  input: Box<ReaderEnum>,
  max_chars: usize,
}

impl SpoonFeedMaxCharsReaderWrapper {
  pub fn new(max_chars: usize, input: ReaderEnum) -> Self {
    Self {
      input: Box::new(input),
      max_chars,
    }
  }
}

impl Reader for SpoonFeedMaxCharsReaderWrapper {
  fn read_range(&mut self, buf: &mut [char], off: usize, len: usize) -> Result<i32> {
    self.input.read_range(buf, off, self.max_chars.min(len))
  }

  fn close(&mut self) -> Result<()> {
    self.input.close()
  }
}
