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
use std::fmt::{Display, Formatter};

use crate::core::store::random_access_input::RandomAccessInput;
use crate::core::store::{DataInput, IndexInput};
use crate::core::util::error::lucene_error::Result;

pub struct DummyIndexInput;

impl DataInput for DummyIndexInput {
  fn read_byte(&mut self) -> Result<u8> {
    dummy_unreachable!()
  }

  fn read_bytes(&mut self, _b: &mut [u8], _offset: usize, _len: usize) -> Result<()> {
    dummy_unreachable!()
  }

  fn read_group_vint(&mut self, _dst: &mut [i32], _offset: usize) -> Result<()> {
    dummy_unreachable!()
  }

  fn skip_bytes(&mut self, _num_bytes: i64) -> Result<()> {
    dummy_unreachable!()
  }
}

impl Display for DummyIndexInput {
  fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
    dummy_unreachable!()
  }
}

impl crate::core::util::clone::TryClone for DummyIndexInput {
  fn try_clone(&self) -> Result<Self>
  where
    Self: Sized,
  {
    dummy_unreachable!()
  }
}

impl IndexInput for DummyIndexInput {
  type IndexInput = DummyIndexInput;

  fn get_file_pointer(&self) -> Result<usize> {
    dummy_unreachable!()
  }

  fn seek(&mut self, _pos: usize) -> Result<()> {
    dummy_unreachable!()
  }

  fn length(&self) -> usize {
    dummy_unreachable!()
  }

  fn slice(
    &self,
    _slice_description: &str,
    __offset: usize,
    _length: usize,
  ) -> Result<Self::IndexInput> {
    dummy_unreachable!()
  }

  type RandomAccessSlice = DummyIndexInput;

  fn random_access_slice(
    &self,
    __offset: usize,
    _length: usize,
  ) -> Result<Self::RandomAccessSlice> {
    dummy_unreachable!()
  }
}
impl RandomAccessInput for DummyIndexInput {
  fn length(&self) -> usize {
    dummy_unreachable!()
  }

  fn read_byte(&mut self, _pos: usize) -> Result<u8> {
    dummy_unreachable!()
  }

  fn read_short(&mut self, _pos: usize) -> Result<i16> {
    dummy_unreachable!()
  }

  fn read_int(&mut self, _pos: usize) -> Result<i32> {
    dummy_unreachable!()
  }

  fn read_long(&mut self, _pos: usize) -> Result<i64> {
    dummy_unreachable!()
  }

  fn prefetch(&mut self, _pos: usize, _len: usize) -> Result<()> {
    dummy_unreachable!()
  }
}
