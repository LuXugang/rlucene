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
use crate::util::error::lucene_error::LuceneError;
use std::fmt::{Display, Formatter};

pub struct DummyIndexInput;

impl DataInput for DummyIndexInput {
    fn read_byte(&mut self) -> Result<u8, LuceneError> {
        unreachable!("DummyIndexInput should not be called")
    }

    fn read_bytes(&mut self, _b: &mut [u8], _offset: u32, _len: u32) -> Result<(), LuceneError> {
        unreachable!("DummyIndexInput should not be called")
    }

    fn skip_bytes(&mut self, _num_bytes: u64) -> Result<(), LuceneError> {
        unreachable!("DummyIndexInput should not be called")
    }
}

impl Display for DummyIndexInput {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("DummyIndexInput should not be called")
    }
}

impl Clone for DummyIndexInput {
    fn clone(&self) -> Self {
        unreachable!("DummyIndexInput should not be called")
    }
}

impl IndexInput for DummyIndexInput {
    fn get_file_pointer(&self) -> u64 {
        unreachable!("DummyIndexInput should not be called")
    }

    fn seek(&mut self, _pos: u64) -> Result<(), LuceneError> {
        unreachable!("DummyIndexInput should not be called")
    }

    fn length(&self) -> u64 {
        unreachable!("DummyIndexInput should not be called")
    }

    #[allow(refining_impl_trait)]
    fn slice(
        &self,
        _slice_description: &str,
        _offset: u64,
        _length: u64,
    ) -> Result<DummyIndexInput, LuceneError> {
        unreachable!("DummyIndexInput should not be called");
    }

    #[allow(refining_impl_trait)]
    fn random_access_slice(
        &self,
        _offset: u64,
        _length: u64,
    ) -> Result<DummyIndexInput, LuceneError> {
        unreachable!("DummyIndexInput should not be called");
    }
}
impl RandomAccessInput for DummyIndexInput {
    fn length(&self) -> u64 {
        unreachable!("DummyIndexInput should not be called")
    }

    fn read_byte(&mut self, _pos: u64) -> Result<u8, LuceneError> {
        unreachable!("DummyIndexInput should not be called")
    }

    fn read_short(&mut self, _pos: u64) -> Result<i16, LuceneError> {
        unreachable!("DummyIndexInput should not be called")
    }

    fn read_int(&mut self, _pos: u64) -> Result<i32, LuceneError> {
        unreachable!("DummyIndexInput should not be called")
    }

    fn read_long(&mut self, _pos: u64) -> Result<i64, LuceneError> {
        unreachable!("DummyIndexInput should not be called")
    }

    fn pre_fetch(&mut self, _pos: u64, _len: u64) -> Result<(), LuceneError> {
        unreachable!("DummyIndexInput should not be called")
    }
}
