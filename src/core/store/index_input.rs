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
use crate::core::store::random_access_input::{Either2RandomAccessInput, RandomAccessInput};
use crate::core::store::{DataInput, ReadAdvice};
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

/// Provides random-access input operations for files within a
/// [`Directory`](crate::core::store::directory::Directory).
///
/// `IndexInput` supports reading data from a file and maintains its own
/// internal state, such as the current file position.
///
/// # Thread Safety
///
/// `IndexInput` is **not thread-safe**. If you need to use it in multiple
/// threads, you must **clone** the `IndexInput` instance. Each clone operates
/// on the same underlying resource but maintains an independent position.
///
///
/// # See Also
/// - [`Directory`](crate::core::store::directory::Directory) for file-based
///   operations.
pub trait IndexInput: DataInput + TryClone {
    /// Returns the current position in this file, where the next read will
    /// occur.
    ///
    /// # See Also
    /// [`seek`](IndexInput::seek)
    fn get_file_pointer(&self) -> i64;

    /// Sets the current position in this file, where the next read will occur.
    /// If this position is beyond the end of the file, it will return an
    /// `EOFError`, and the stream will be in an undetermined state.
    ///
    /// # See Also
    /// [`get_file_pointer`](IndexInput::get_file_pointer)
    fn seek(&mut self, pos: i64) -> Result<()>;
    /// Inherits documentation from the parent implementation.
    ///
    /// # Behavior
    /// This is functionally equivalent to seeking to `get_file_pointer() +
    /// num_bytes`.
    ///
    /// # See Also
    /// [`get_file_pointer`](IndexInput::get_file_pointer)
    ///
    /// [`seek`](IndexInput::seek)
    fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
        if num_bytes < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "num_bytes must be >= 0, got {num_bytes}"
            )));
        }
        let skip_to = self.get_file_pointer() + num_bytes;
        self.seek(skip_to)?;
        Ok(())
    }
    /// The number of bytes in the file.
    fn length(&self) -> i64;

    /// Creates a slice of this index input, with the given description, offset,
    /// and length. The slice is positioned at the beginning.
    type Slice: IndexInput;
    fn slice(&self, slice_description: &str, offset: i64, length: i64) -> Result<Self::Slice>;
    /// Creates a slice with a specific [`ReadAdvice`]. This is typically used
    /// by [`CompoundFormat`](crate::core::codecs::compound_format)
    /// implementations to honor the [`ReadAdvice`] of each file within the
    /// compound file.
    ///
    /// # Note
    /// It is only legal to call this method if this `IndexInput` has been
    /// opened with `ReadAdvice::NORMAL`. However, this method accepts any
    /// `ReadAdvice` value except `None` for the slice.
    ///
    /// The default implementation delegates to [`slice`](IndexInput::slice) and
    /// ignores the `ReadAdvice`.
    fn slice_with_read_advice(
        &self,
        description: &str,
        offset: i64,
        length: i64,
        read_advice: &ReadAdvice,
    ) -> Result<Self::Slice> {
        self.default_slice_with_read_advice(description, offset, length, read_advice)
    }
    fn default_slice_with_read_advice(
        &self,
        description: &str,
        offset: i64,
        length: i64,
        _read_advice: &ReadAdvice,
    ) -> Result<Self::Slice> {
        self.slice(description, offset, length)
    }
    type RandomAccessSlice: RandomAccessInput;
    /// Creates a random-access slice of this index input, with the given offset
    /// and length.
    ///
    /// # Note
    /// The default implementation calls [`slice`](IndexInput::slice), and it
    /// doesn't support random access. It implements absolute reads as
    /// seek+read.
    fn random_access_slice(&self, offset: i64, length: i64) -> Result<Self::RandomAccessSlice>;

    /// Optional method: Gives a hint to this input that some bytes will be read
    /// soon. `IndexInput` implementations may take advantage of this hint
    /// to start fetching pages of data immediately from storage.
    ///
    /// # Arguments
    /// * `offset` - The starting offset.
    /// * `length` - The number of bytes to prefetch.
    ///
    /// # Note
    /// The default implementation is a no-op.
    fn prefetch(&mut self, pos: i64, len: i64) -> Result<()> {
        self.default_prefetch(pos, len)
    }

    fn default_prefetch(&mut self, _pos: i64, _len: i64) -> Result<()> {
        Ok(())
    }
}
/// SubStruct call this to get the String for resourceDescription of a slice of
/// this `IndexInput`.
pub fn get_full_slice_description(slice_description: &str) -> String {
    format!(" [slice={slice_description}] ")
}

pub enum Either2IndexInput<A, B> {
    A(A),
    B(B),
}

impl<A, B> DataInput for Either2IndexInput<A, B>
where
    A: IndexInput,
    B: IndexInput,
{
    fn read_byte(&mut self) -> Result<u8> {
        match self {
            Either2IndexInput::A(f) => f.read_byte(),
            Either2IndexInput::B(s) => s.read_byte(),
        }
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<()> {
        match self {
            Either2IndexInput::A(f) => f.read_bytes(b, offset, len),
            Either2IndexInput::B(s) => s.read_bytes(b, offset, len),
        }
    }

    fn read_bytes_with_buffer(
        &mut self,
        b: &mut [u8],
        offset: i32,
        len: i32,
        _use_buffer: bool,
    ) -> Result<()> {
        match self {
            Either2IndexInput::A(f) => f.read_bytes_with_buffer(b, offset, len, _use_buffer),
            Either2IndexInput::B(s) => s.read_bytes_with_buffer(b, offset, len, _use_buffer),
        }
    }

    fn read_short(&mut self) -> Result<i16> {
        match self {
            Either2IndexInput::A(f) => f.read_short(),
            Either2IndexInput::B(s) => s.read_short(),
        }
    }

    fn default_read_short(&mut self) -> Result<i16> {
        match self {
            Either2IndexInput::A(f) => f.default_read_short(),
            Either2IndexInput::B(s) => s.default_read_short(),
        }
    }

    fn read_int(&mut self) -> Result<i32> {
        match self {
            Either2IndexInput::A(f) => f.read_int(),
            Either2IndexInput::B(s) => s.read_int(),
        }
    }

    fn default_read_int(&mut self) -> Result<i32> {
        match self {
            Either2IndexInput::A(f) => f.default_read_int(),
            Either2IndexInput::B(s) => s.default_read_int(),
        }
    }

    fn read_group_vint(&mut self, dst: &mut [i32], offset: i32) -> Result<()> {
        match self {
            Either2IndexInput::A(f) => f.read_group_vint(dst, offset),
            Either2IndexInput::B(s) => s.read_group_vint(dst, offset),
        }
    }

    fn default_read_group_vint(&mut self, dst: &mut [i32], offset: i32) -> Result<()> {
        match self {
            Either2IndexInput::A(f) => f.default_read_group_vint(dst, offset),
            Either2IndexInput::B(s) => s.default_read_group_vint(dst, offset),
        }
    }

    fn read_vint(&mut self) -> Result<i32> {
        match self {
            Either2IndexInput::A(f) => f.read_vint(),
            Either2IndexInput::B(s) => s.read_vint(),
        }
    }

    fn read_zint(&mut self) -> Result<i32> {
        match self {
            Either2IndexInput::A(f) => f.read_zint(),
            Either2IndexInput::B(s) => s.read_zint(),
        }
    }

    fn read_long(&mut self) -> Result<i64> {
        match self {
            Either2IndexInput::A(f) => f.read_long(),
            Either2IndexInput::B(s) => s.read_long(),
        }
    }

    fn default_read_long(&mut self) -> Result<i64> {
        match self {
            Either2IndexInput::A(f) => f.default_read_long(),
            Either2IndexInput::B(s) => s.default_read_long(),
        }
    }

    fn read_longs(&mut self, dst: &mut [i64], offset: i32, len: i32) -> Result<()> {
        match self {
            Either2IndexInput::A(f) => f.read_longs(dst, offset, len),
            Either2IndexInput::B(s) => s.read_longs(dst, offset, len),
        }
    }

    fn read_ints(&mut self, dst: &mut [i32], offset: i32, len: i32) -> Result<()> {
        match self {
            Either2IndexInput::A(f) => f.read_ints(dst, offset, len),
            Either2IndexInput::B(s) => s.read_ints(dst, offset, len),
        }
    }

    fn read_floats(&mut self, dst: &mut [f32], offset: i32, len: i32) -> Result<()> {
        match self {
            Either2IndexInput::A(f) => f.read_floats(dst, offset, len),
            Either2IndexInput::B(s) => s.read_floats(dst, offset, len),
        }
    }

    fn read_vlong(&mut self) -> Result<i64> {
        match self {
            Either2IndexInput::A(f) => f.read_vlong(),
            Either2IndexInput::B(s) => s.read_vlong(),
        }
    }

    fn read_zlong(&mut self) -> Result<i64> {
        match self {
            Either2IndexInput::A(f) => f.read_zlong(),
            Either2IndexInput::B(s) => s.read_zlong(),
        }
    }

    fn read_string(&mut self) -> Result<String> {
        match self {
            Either2IndexInput::A(f) => f.read_string(),
            Either2IndexInput::B(s) => s.read_string(),
        }
    }

    fn read_map_of_strings(&mut self) -> Result<HashMap<String, String>> {
        match self {
            Either2IndexInput::A(f) => f.read_map_of_strings(),
            Either2IndexInput::B(s) => s.read_map_of_strings(),
        }
    }

    fn read_set_of_strings(&mut self) -> Result<HashSet<String>> {
        match self {
            Either2IndexInput::A(f) => f.read_set_of_strings(),
            Either2IndexInput::B(s) => s.read_set_of_strings(),
        }
    }

    fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
        match self {
            Either2IndexInput::A(f) => DataInput::skip_bytes(f, num_bytes),
            Either2IndexInput::B(s) => DataInput::skip_bytes(s, num_bytes),
        }
    }

    fn is_index_input(&self) -> bool {
        match self {
            Either2IndexInput::A(f) => f.is_index_input(),
            Either2IndexInput::B(s) => s.is_index_input(),
        }
    }

    fn seek_in_data_input(&mut self, _pos: i64) -> Result<()> {
        match self {
            Either2IndexInput::A(f) => f.seek_in_data_input(_pos),
            Either2IndexInput::B(s) => s.seek_in_data_input(_pos),
        }
    }

    fn get_file_pointer_in_data_input(&self) -> i64 {
        match self {
            Either2IndexInput::A(f) => f.get_file_pointer_in_data_input(),
            Either2IndexInput::B(s) => s.get_file_pointer_in_data_input(),
        }
    }
}

impl<A, B> Display for Either2IndexInput<A, B>
where
    A: IndexInput,
    B: IndexInput,
{
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<A, B> TryClone for Either2IndexInput<A, B>
where
    A: IndexInput,
    B: IndexInput,
{
    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized,
    {
        match self {
            Either2IndexInput::A(f) => Ok(Either2IndexInput::A(f.try_clone()?)),
            Either2IndexInput::B(s) => Ok(Either2IndexInput::B(s.try_clone()?)),
        }
    }
}

impl<A, B> IndexInput for Either2IndexInput<A, B>
where
    A: IndexInput,
    B: IndexInput,
{
    fn get_file_pointer(&self) -> i64 {
        match self {
            Either2IndexInput::A(f) => f.get_file_pointer(),
            Either2IndexInput::B(s) => s.get_file_pointer(),
        }
    }

    fn seek(&mut self, pos: i64) -> Result<()> {
        match self {
            Either2IndexInput::A(f) => f.seek(pos),
            Either2IndexInput::B(s) => s.seek(pos),
        }
    }

    fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
        match self {
            Either2IndexInput::A(f) => IndexInput::skip_bytes(f, num_bytes),
            Either2IndexInput::B(s) => IndexInput::skip_bytes(s, num_bytes),
        }
    }

    fn length(&self) -> i64 {
        match self {
            Either2IndexInput::A(f) => f.length(),
            Either2IndexInput::B(s) => s.length(),
        }
    }

    type Slice = Either2IndexInput<A::Slice, B::Slice>;

    fn slice(&self, slice_description: &str, offset: i64, length: i64) -> Result<Self::Slice> {
        match self {
            Either2IndexInput::A(f) => Ok(Either2IndexInput::A(f.slice(
                slice_description,
                offset,
                length,
            )?)),
            Either2IndexInput::B(s) => Ok(Either2IndexInput::B(s.slice(
                slice_description,
                offset,
                length,
            )?)),
        }
    }

    fn slice_with_read_advice(
        &self,
        description: &str,
        offset: i64,
        length: i64,
        read_advice: &ReadAdvice,
    ) -> Result<Self::Slice> {
        match self {
            Either2IndexInput::A(f) => Ok(Either2IndexInput::A(f.slice_with_read_advice(
                description,
                offset,
                length,
                read_advice,
            )?)),
            Either2IndexInput::B(s) => Ok(Either2IndexInput::B(s.slice_with_read_advice(
                description,
                offset,
                length,
                read_advice,
            )?)),
        }
    }

    fn default_slice_with_read_advice(
        &self,
        description: &str,
        offset: i64,
        length: i64,
        _read_advice: &ReadAdvice,
    ) -> Result<Self::Slice> {
        match self {
            Either2IndexInput::A(f) => Ok(Either2IndexInput::A(f.default_slice_with_read_advice(
                description,
                offset,
                length,
                _read_advice,
            )?)),
            Either2IndexInput::B(s) => Ok(Either2IndexInput::B(s.default_slice_with_read_advice(
                description,
                offset,
                length,
                _read_advice,
            )?)),
        }
    }

    type RandomAccessSlice = Either2RandomAccessInput<A::RandomAccessSlice, B::RandomAccessSlice>;

    fn random_access_slice(&self, offset: i64, length: i64) -> Result<Self::RandomAccessSlice> {
        match self {
            Either2IndexInput::A(f) => Ok(Either2RandomAccessInput::A(
                f.random_access_slice(offset, length)?,
            )),
            Either2IndexInput::B(s) => Ok(Either2RandomAccessInput::B(
                s.random_access_slice(offset, length)?,
            )),
        }
    }

    fn prefetch(&mut self, pos: i64, len: i64) -> Result<()> {
        match self {
            Either2IndexInput::A(f) => f.prefetch(pos, len),
            Either2IndexInput::B(s) => s.prefetch(pos, len),
        }
    }

    fn default_prefetch(&mut self, _pos: i64, _len: i64) -> Result<()> {
        match self {
            Either2IndexInput::A(f) => f.default_prefetch(_pos, _len),
            Either2IndexInput::B(s) => s.default_prefetch(_pos, _len),
        }
    }
}
