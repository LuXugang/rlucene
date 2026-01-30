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

use crc32fast::Hasher;

use crate::core::store::check_sum_index_input::ChecksumIndexInput;
use crate::core::store::dummy::dummy_index_input::DummyIndexInput;
use crate::core::store::index_input::IndexInput;
use crate::core::store::{BufferedChecksum, Checksum, DataInput, HasherChecksum};
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Simple implementation of [`ChecksumIndexInput`] that wraps another input and
/// delegates calls.
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

impl<T> crate::core::util::clone::TryClone for BufferedChecksumIndexInput<T>
where
    T: IndexInput,
{
    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized,
    {
        unreachable!("unsupported operation")
    }
}

impl<T> IndexInput for BufferedChecksumIndexInput<T>
where
    T: IndexInput,
{
    fn get_file_pointer(&self) -> Result<usize> {
        self.main.get_file_pointer()
    }

    fn seek(&mut self, pos: usize) -> Result<()> {
        ChecksumIndexInput::seek(self, pos)
    }

    fn length(&self) -> usize {
        self.main.length()
    }

    type RandomAccessSlice = DummyIndexInput;

    fn random_access_slice(
        &self,
        _offset: usize,
        _length: usize,
    ) -> Result<Self::RandomAccessSlice> {
        Err(LuceneError::unsupported_operation(
            "BufferedChecksumIndexInput does not support random access slicing",
        ))
    }
}

// TODO: readInt/Long not implemented
impl<T> DataInput for BufferedChecksumIndexInput<T>
where
    T: IndexInput,
{
    fn read_byte(&mut self) -> Result<u8> {
        let b = self.main.read_byte()?;
        self.digest.update(b);
        Ok(b)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
        self.main.read_bytes(b, offset, len)?;
        self.digest.update_bytes(b, offset, len);
        Ok(())
    }

    fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
        IndexInput::skip_bytes(self, num_bytes)
    }

    fn is_index_input(&self) -> bool {
        true
    }

    fn seek_in_data_input(&mut self, _pos: usize) -> Result<()> {
        debug_assert!(self.is_index_input());
        IndexInput::seek(self, _pos)
    }

    fn get_file_pointer_in_data_input(&self) -> Result<usize> {
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

impl<T> ChecksumIndexInput for BufferedChecksumIndexInput<T>
where
    T: IndexInput,
{
    fn get_checksum(&mut self) -> i64 {
        self.digest.get_value()
    }
}
