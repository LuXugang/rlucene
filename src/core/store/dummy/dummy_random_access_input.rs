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
use crate::core::store::random_access_input::RandomAccessInput;
use crate::core::util::error::lucene_error::Result;

pub struct DummyRandomAccessInput;
impl RandomAccessInput for DummyRandomAccessInput {
  fn length(&self) -> Result<usize> {
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
