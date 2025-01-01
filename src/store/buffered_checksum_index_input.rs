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
use crate::store::check_sum_index_input::ChecksumIndexInput;
use crate::store::dummy_index_input::DummyIndexInput;
use crate::store::index_input::IndexInput;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{BufferedChecksum, Checksum, DataInput, HasherChecksum};
use crate::util::error::data_io_error_enum::RuntimeError;
use crc32fast::Hasher;
use std::fmt::{Display, Formatter};

/// Simple implementation of [`ChecksumIndexInput`] that wraps another input and delegates calls.
pub struct BufferedChecksumIndexInput<T: IndexInput> {
    main: T,
    digest: BufferedChecksum<HasherChecksum>,
}
impl<T> BufferedChecksumIndexInput<T>
where
    T: IndexInput,
{
    pub fn new(main: T) -> BufferedChecksumIndexInput<T> {
        let digest = BufferedChecksum::new(HasherChecksum::new(Hasher::new()));
        BufferedChecksumIndexInput { main, digest }
    }
}

impl<T> IndexInput for BufferedChecksumIndexInput<T>
where
    T: IndexInput,
{
    fn get_file_pointer(&self) -> u64 {
        self.main.get_file_pointer()
    }

    fn seek(&mut self, pos: u64) -> Result<(), RuntimeError> {
        ChecksumIndexInput::seek(self, pos)
    }

    fn length(&self) -> u64 {
        self.main.length()
    }

    #[allow(unreachable_code)]
    fn slice(
        &self,
        _slice_description: &str,
        _offset: u64,
        _length: u64,
    ) -> Result<impl IndexInput + RandomAccessInput, RuntimeError> {
        // Used by the compiler to infer the returned type
        if false {
            return Ok(DummyIndexInput);
        }
        Err(RuntimeError::unsupported_operation(
            "BufferedChecksumIndexInput does not support slicing",
        ))
    }

    #[allow(unreachable_code)]
    fn random_access_slice(
        &self,
        _offset: u64,
        _length: u64,
    ) -> Result<impl IndexInput + RandomAccessInput, RuntimeError> {
        // Used by the compiler to infer the returned type
        if false {
            return Ok(DummyIndexInput);
        }
        Err(RuntimeError::unsupported_operation(
            "BufferedChecksumIndexInput does not support random access slicing",
        ))
    }
}

impl<T> DataInput for BufferedChecksumIndexInput<T>
where
    T: IndexInput,
{
    fn read_byte(&mut self) -> Result<u8, RuntimeError> {
        let b = self.main.read_byte()?;
        self.digest.update(b);
        Ok(b)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: u32, len: u32) -> Result<(), RuntimeError> {
        self.main.read_bytes(b, offset, len)?;
        self.digest.update_bytes(b, offset, len);
        Ok(())
    }

    fn skip_bytes(&mut self, num_bytes: u64) -> Result<(), RuntimeError> {
        IndexInput::skip_bytes(self, num_bytes)
    }

    fn is_index_input(&self) -> bool {
        true
    }

    fn seek_in_data_input(&mut self, pos: u64) -> Result<(), RuntimeError> {
        debug_assert!(self.is_index_input());
        IndexInput::seek(self, pos)
    }

    fn get_file_pointer_in_data_input(&self) -> u64 {
        debug_assert!(self.is_index_input());
        IndexInput::get_file_pointer(self)
    }
}

impl<T> Display for BufferedChecksumIndexInput<T>
where
    T: IndexInput,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BufferedChecksumIndexInput({})", self.main)
    }
}

impl<T> Clone for BufferedChecksumIndexInput<T>
where
    T: IndexInput,
{
    fn clone(&self) -> Self {
        unreachable!("unsupported operation")
    }
}

impl<T> ChecksumIndexInput for BufferedChecksumIndexInput<T>
where
    T: IndexInput,
{
    fn get_checksum(&mut self) -> u64 {
        self.digest.get_value()
    }
}
