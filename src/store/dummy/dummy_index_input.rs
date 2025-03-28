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
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{DataInput, IndexInput};
use crate::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};

pub struct DummyIndexInput;

impl DataInput for DummyIndexInput {
    fn read_byte(&mut self) -> Result<u8> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn read_bytes(&mut self, _b: &mut [u8], _offset: i32, _len: i32) -> Result<()> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn skip_bytes(&mut self, _num_bytes: i64) -> Result<()> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }
}

impl Display for DummyIndexInput {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("this method should never be called")
    }
}

impl Clone for DummyIndexInput {
    fn clone(&self) -> Self {
        unreachable!("this method should never be called")
    }
}

impl IndexInput for DummyIndexInput {
    fn get_file_pointer(&self) -> i64 {
        unreachable!("this method should never be called")
    }

    fn seek(&mut self, _pos: i64) -> Result<()> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn length(&self) -> i64 {
        unreachable!("this method should never be called")
    }

    type Slice = DummyIndexInput;

    fn slice(
        &self,
        _slice_description: &str,
        _offset: i64,
        _length: i64,
    ) -> Result<DummyIndexInput> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    type RandomAccessSlice = DummyIndexInput;

    fn random_access_slice(&self, _offset: i64, _length: i64) -> Result<DummyIndexInput> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }
}
impl RandomAccessInput for DummyIndexInput {
    fn length(&self) -> i64 {
        unreachable!("this method should never be called")
    }

    fn read_byte(&mut self, _pos: i64) -> Result<u8> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn read_short(&mut self, _pos: i64) -> Result<i16> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn read_int(&mut self, _pos: i64) -> Result<i32> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn read_long(&mut self, _pos: i64) -> Result<i64> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn prefetch(&mut self, _pos: i64, _len: i64) -> Result<()> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }
}
