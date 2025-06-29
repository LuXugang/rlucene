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
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

use crate::store::byte_buffers_data_input::ByteBuffersDataInputRef;
use crate::store::index_input::IndexInput;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::DataInput;
use crate::util::error::lucene_error::Result;

/// An [`IndexInput`] implementing [`RandomAccessInput`]
/// and backed by a [`ByteBuffersDataInput`](crate::store::byte_buffers_data_input::ByteBuffersDataInput).
pub struct ByteBuffersIndexInput<'a> {
    data_input: ByteBuffersDataInputRef<'a>,
    resource_description: String,
}
impl<'a> ByteBuffersIndexInput<'a> {
    pub fn new(data_input: ByteBuffersDataInputRef<'a>, resource_description: &str) -> Self {
        Self {
            data_input,
            resource_description: resource_description.to_string(),
        }
    }
}

impl DataInput for ByteBuffersIndexInput<'_> {
    fn read_byte(&mut self) -> Result<u8> {
        DataInput::read_byte(&mut self.data_input)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<()> {
        DataInput::read_bytes(&mut self.data_input, b, offset, len)
    }

    fn read_bytes_with_buffer(
        &mut self,
        b: &mut [u8],
        offset: i32,
        len: i32,
        _use_buffer: bool,
    ) -> Result<()> {
        self.data_input
            .read_bytes_with_buffer(b, offset, len, false)
    }

    fn read_short(&mut self) -> Result<i16> {
        DataInput::read_short(&mut self.data_input)
    }

    fn read_int(&mut self) -> Result<i32> {
        DataInput::read_int(&mut self.data_input)
    }

    fn read_group_vint(&mut self, dst: &mut [i32], offset: i32) -> Result<()> {
        self.data_input.read_group_vint(dst, offset)
    }

    fn read_vint(&mut self) -> Result<i32> {
        DataInput::read_vint(&mut self.data_input)
    }

    fn read_zint(&mut self) -> Result<i32> {
        DataInput::read_zint(&mut self.data_input)
    }

    fn read_long(&mut self) -> Result<i64> {
        DataInput::read_long(&mut self.data_input)
    }

    fn read_longs(&mut self, dst: &mut [i64], offset: i32, len: i32) -> Result<()> {
        self.data_input.read_longs(dst, offset, len)
    }

    fn read_floats(&mut self, dst: &mut [f32], offset: i32, len: i32) -> Result<()> {
        self.data_input.read_floats(dst, offset, len)
    }

    fn read_vlong(&mut self) -> Result<i64> {
        self.data_input.read_vlong()
    }

    fn read_zlong(&mut self) -> Result<i64> {
        self.data_input.read_zlong()
    }

    fn read_string(&mut self) -> Result<String> {
        self.data_input.read_string()
    }

    fn read_map_of_strings(&mut self) -> Result<HashMap<String, String>> {
        self.data_input.read_map_of_strings()
    }

    fn read_set_of_strings(&mut self) -> Result<HashSet<String>> {
        self.data_input.read_set_of_strings()
    }

    fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
        DataInput::skip_bytes(&mut self.data_input, num_bytes)
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
impl RandomAccessInput for ByteBuffersIndexInput<'_> {
    fn length(&self) -> i64 {
        RandomAccessInput::length(&self.data_input)
    }

    fn read_byte(&mut self, pos: i64) -> Result<u8> {
        RandomAccessInput::read_byte(&mut self.data_input, pos)
    }

    fn read_short(&mut self, pos: i64) -> Result<i16> {
        RandomAccessInput::read_short(&mut self.data_input, pos)
    }

    fn read_int(&mut self, pos: i64) -> Result<i32> {
        RandomAccessInput::read_int(&mut self.data_input, pos)
    }

    fn read_long(&mut self, pos: i64) -> Result<i64> {
        RandomAccessInput::read_long(&mut self.data_input, pos)
    }

    fn prefetch(&mut self, _pos: i64, _len: i64) -> Result<()> {
        Ok(())
    }
}

impl Display for ByteBuffersIndexInput<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.resource_description)
    }
}

impl crate::util::clone::TryClone for ByteBuffersIndexInput<'_> {
    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized,
    {
        let slice = self.data_input.slice(0, self.data_input.length())?;
        Ok(ByteBuffersIndexInput::new(
            slice,
            format!("(clone of) {}", self).as_str(),
        ))
    }
}

impl<'a> IndexInput for ByteBuffersIndexInput<'a> {
    fn get_file_pointer(&self) -> i64 {
        self.data_input.position()
    }

    fn seek(&mut self, pos: i64) -> Result<()> {
        self.data_input.seek(pos)
    }

    fn length(&self) -> i64 {
        self.data_input.length()
    }

    type Slice = ByteBuffersIndexInput<'a>;

    fn slice(&self, slice_description: &str, offset: i64, length: i64) -> Result<Self::Slice> {
        Ok(ByteBuffersIndexInput::new(
            self.data_input.slice(offset, length)?,
            slice_description,
        ))
    }

    type RandomAccessSlice = Self::Slice;

    fn random_access_slice(&self, offset: i64, length: i64) -> Result<Self::Slice> {
        self.slice("", offset, length)
    }
}
