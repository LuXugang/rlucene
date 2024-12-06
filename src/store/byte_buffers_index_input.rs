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
use crate::store::byte_buffers_data_input::ByteBuffersDataInput;
use crate::store::index_input::IndexInput;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::DataInput;
use crate::util::error::data_io_error_enum::DataIOError;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

/// An [`IndexInput`] implementing [`RandomAccessInput`]
/// and backed by a [`ByteBuffersDataInput`].
pub struct ByteBuffersIndexInput<'a> {
    data_input: ByteBuffersDataInput<'a>,
    resource_description: String,
}
impl<'a> ByteBuffersIndexInput<'a> {
    pub fn new(data_input: ByteBuffersDataInput<'a>, resource_description: &str) -> Self {
        Self {
            data_input,
            resource_description: resource_description.to_string(),
        }
    }
}

impl DataInput for ByteBuffersIndexInput<'_> {
    fn read_byte(&mut self) -> Result<u8, DataIOError> {
        DataInput::read_byte(&mut self.data_input)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<(), DataIOError> {
        DataInput::read_bytes(&mut self.data_input, b, offset, len)
    }

    fn read_bytes_with_buffer(
        &mut self,
        b: &mut [u8],
        offset: usize,
        len: usize,
        _use_buffer: bool,
    ) -> Result<(), DataIOError> {
        self.data_input
            .read_bytes_with_buffer(b, offset, len, false)
    }

    fn read_short(&mut self) -> Result<i16, DataIOError> {
        DataInput::read_short(&mut self.data_input)
    }

    fn read_int(&mut self) -> Result<i32, DataIOError> {
        DataInput::read_int(&mut self.data_input)
    }

    fn read_group_vint(&mut self, dst: &mut [i64], offset: usize) -> Result<(), DataIOError> {
        self.data_input.read_group_vint(dst, offset)
    }

    fn read_vint(&mut self) -> Result<i32, DataIOError> {
        DataInput::read_vint(&mut self.data_input)
    }

    fn read_zint(&mut self) -> Result<i32, DataIOError> {
        DataInput::read_zint(&mut self.data_input)
    }

    fn read_long(&mut self) -> Result<i64, DataIOError> {
        DataInput::read_long(&mut self.data_input)
    }

    fn read_longs(
        &mut self,
        dst: &mut [i64],
        offset: usize,
        len: usize,
    ) -> Result<(), DataIOError> {
        self.data_input.read_longs(dst, offset, len)
    }

    fn read_floats(
        &mut self,
        dst: &mut [f32],
        offset: usize,
        len: usize,
    ) -> Result<(), DataIOError> {
        self.data_input.read_floats(dst, offset, len)
    }

    fn read_vlong(&mut self) -> Result<i64, DataIOError> {
        self.data_input.read_vlong()
    }

    fn read_zlong(&mut self) -> Result<i64, DataIOError> {
        self.data_input.read_zlong()
    }

    fn read_string(&mut self) -> Result<String, DataIOError> {
        self.data_input.read_string()
    }

    fn read_map_of_strings(&mut self) -> Result<HashMap<String, String>, DataIOError> {
        self.data_input.read_map_of_strings()
    }

    fn read_set_of_strings(&mut self) -> Result<HashSet<String>, DataIOError> {
        self.data_input.read_set_of_strings()
    }

    fn skip_bytes(&mut self, num_bytes: u64) -> Result<(), DataIOError> {
        DataInput::skip_bytes(&mut self.data_input, num_bytes)
    }
}
impl RandomAccessInput for ByteBuffersIndexInput<'_> {
    fn length(&self) -> u64 {
        RandomAccessInput::length(&self.data_input)
    }

    fn read_byte(&mut self, pos: u64) -> Result<u8, DataIOError> {
        RandomAccessInput::read_byte(&mut self.data_input, pos)
    }

    fn read_short(&mut self, pos: u64) -> Result<i16, DataIOError> {
        RandomAccessInput::read_short(&mut self.data_input, pos)
    }

    fn read_int(&mut self, pos: u64) -> Result<i32, DataIOError> {
        RandomAccessInput::read_int(&mut self.data_input, pos)
    }

    fn read_long(&mut self, pos: u64) -> Result<i64, DataIOError> {
        RandomAccessInput::read_long(&mut self.data_input, pos)
    }

    fn pre_fetch(&mut self, pos: u64, len: u64) -> Result<(), DataIOError> {
        Ok(())
    }
}

impl Display for ByteBuffersIndexInput<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.resource_description)
    }
}

impl Clone for ByteBuffersIndexInput<'_> {
    fn clone(&self) -> Self {
        let slice = self.data_input.slice(0, self.data_input.length()).unwrap();
        ByteBuffersIndexInput::new(slice, format!("(clone of) {}", self).as_str())
    }
}

impl IndexInput for ByteBuffersIndexInput<'_> {
    fn get_file_pointer(&self) -> u64 {
        self.data_input.position()
    }

    fn seek(&mut self, pos: u64) -> Result<(), DataIOError> {
        self.data_input.seek(pos)
    }

    fn length(&self) -> u64 {
        self.data_input.length()
    }

    fn slice(
        &self,
        slice_description: &str,
        offset: u64,
        length: u64,
    ) -> Result<ByteBuffersIndexInput, DataIOError> {
        Ok(ByteBuffersIndexInput::new(
            self.data_input.slice(offset, length)?,
            slice_description,
        ))
    }

    fn random_access_slice(
        &self,
        offset: u64,
        length: u64,
    ) -> Result<ByteBuffersIndexInput, DataIOError> {
        self.slice("", offset, length)
    }

    fn is_random_access(&self) -> bool {
        true
    }

    fn get_random_access_slice(
        &self,
        offset: u64,
        length: u64,
    ) -> Result<ByteBuffersIndexInput, DataIOError> {
        self.slice("", offset, length)
    }
}
