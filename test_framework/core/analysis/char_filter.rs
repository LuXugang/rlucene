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
use crate::core::analysis::reader::{Reader, ReaderEnum};
use crate::core::util::error::lucene_error::Result;

#[derive(Clone, Debug)]
pub struct CharFilter1 {
  input: Box<ReaderEnum>,
}

impl CharFilter1 {
  pub fn new(input: ReaderEnum) -> Self {
    Self {
      input: Box::new(input),
    }
  }
}

impl Reader for CharFilter1 {
  fn read_range(&mut self, buf: &mut [char], off: usize, len: usize) -> Result<i32> {
    self.input.read_range(buf, off, len)
  }

  fn close(&mut self) -> Result<()> {
    CharFilter::close(self)
  }
}

impl CharFilter for CharFilter1 {
  fn get_reader(&self) -> &ReaderEnum {
    &self.input
  }

  fn get_reader_mut(&mut self) -> &mut ReaderEnum {
    &mut self.input
  }

  fn correct(&self, current_off: i32) -> i32 {
    current_off + 1
  }
}

#[derive(Clone, Debug)]
pub struct CharFilter2 {
  input: Box<ReaderEnum>,
}

impl CharFilter2 {
  pub fn new(input: ReaderEnum) -> Self {
    Self {
      input: Box::new(input),
    }
  }
}

impl Reader for CharFilter2 {
  fn read_range(&mut self, buf: &mut [char], off: usize, len: usize) -> Result<i32> {
    self.input.read_range(buf, off, len)
  }

  fn close(&mut self) -> Result<()> {
    CharFilter::close(self)
  }
}

impl CharFilter for CharFilter2 {
  fn get_reader(&self) -> &ReaderEnum {
    &self.input
  }

  fn get_reader_mut(&mut self) -> &mut ReaderEnum {
    &mut self.input
  }

  fn correct(&self, current_off: i32) -> i32 {
    current_off + 2
  }
}
