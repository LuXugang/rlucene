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
use crate::util::error::data_io_error_enum::DataIOError;
use std::fmt::{Display, Formatter};

pub struct DummyIndexInput;

impl DataInput for DummyIndexInput {
    fn read_byte(&mut self) -> Result<u8, DataIOError> {
        unreachable!("DummyIndexInput")
    }

    fn read_bytes(&mut self, _b: &mut [u8], _offset: u32, _len: u32) -> Result<(), DataIOError> {
        unreachable!("DummyIndexInput")
    }

    fn skip_bytes(&mut self, _num_bytes: u64) -> Result<(), DataIOError> {
        unreachable!("DummyIndexInput")
    }
}

impl Display for DummyIndexInput {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("DummyIndexInput")
    }
}

impl Clone for DummyIndexInput {
    fn clone(&self) -> Self {
        unreachable!("DummyIndexInput")
    }
}

impl IndexInput for DummyIndexInput {
    fn get_file_pointer(&self) -> u64 {
        unreachable!("DummyIndexInput")
    }

    fn seek(&mut self, _pos: u64) -> Result<(), DataIOError> {
        unreachable!("DummyIndexInput")
    }

    fn length(&self) -> u64 {
        unreachable!("DummyIndexInput")
    }

    #[allow(unreachable_code)]
    fn slice(
        &self,
        _slice_description: &str,
        _offset: u64,
        _length: u64,
    ) -> Result<impl IndexInput + RandomAccessInput, DataIOError> {
        // Used by the compiler to infer the returned type
        if false {
            return Ok(DummyIndexInput);
        }
        Err(DataIOError::unsupported_operation(
            "slice method is not supported".to_string(),
        ))
    }

    #[allow(unreachable_code)]
    fn random_access_slice(
        &self,
        _offset: u64,
        _length: u64,
    ) -> Result<impl IndexInput + RandomAccessInput, DataIOError> {
        // Used by the compiler to infer the returned type
        if false {
            return Ok(DummyIndexInput);
        }
        Err(DataIOError::unsupported_operation(
            "random_access_slice method is not supported".to_string(),
        ))
    }
}
impl RandomAccessInput for DummyIndexInput {
    fn length(&self) -> u64 {
        unreachable!("DummyIndexInput")
    }

    fn read_byte(&mut self, _pos: u64) -> Result<u8, DataIOError> {
        unreachable!("DummyIndexInput")
    }

    fn read_short(&mut self, _pos: u64) -> Result<i16, DataIOError> {
        unreachable!("DummyIndexInput")
    }

    fn read_int(&mut self, _pos: u64) -> Result<i32, DataIOError> {
        unreachable!("DummyIndexInput")
    }

    fn read_long(&mut self, _pos: u64) -> Result<i64, DataIOError> {
        unreachable!("DummyIndexInput")
    }

    fn pre_fetch(&mut self, __pos: u64, _len: u64) -> Result<(), DataIOError> {
        unreachable!("DummyIndexInput")
    }
}
