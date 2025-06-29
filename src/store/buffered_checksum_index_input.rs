/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/
use std::fmt::{Display, Formatter};

use crc32fast::Hasher;

use crate::store::check_sum_index_input::ChecksumIndexInput;
use crate::store::dummy::dummy_index_input::DummyIndexInput;
use crate::store::index_input::IndexInput;
use crate::store::{BufferedChecksum, Checksum, DataInput, HasherChecksum};
use crate::util::error::lucene_error::{LuceneError, Result};

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

impl<T> crate::util::clone::TryClone for BufferedChecksumIndexInput<T>
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
    fn get_file_pointer(&self) -> i64 {
        self.main.get_file_pointer()
    }

    fn seek(&mut self, pos: i64) -> Result<()> {
        ChecksumIndexInput::seek(self, pos)
    }

    fn length(&self) -> i64 {
        self.main.length()
    }

    type Slice = DummyIndexInput;
    fn slice(&self, _slice_description: &str, _offset: i64, _length: i64) -> Result<Self::Slice> {
        Err(LuceneError::unsupported_operation(
            "BufferedChecksumIndexInput does not support slicing",
        ))
    }

    type RandomAccessSlice = DummyIndexInput;

    fn random_access_slice(&self, _offset: i64, _length: i64) -> Result<DummyIndexInput> {
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

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<()> {
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

    fn seek_in_data_input(&mut self, pos: i64) -> Result<()> {
        debug_assert!(self.is_index_input());
        IndexInput::seek(self, pos)
    }

    fn get_file_pointer_in_data_input(&self) -> i64 {
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
