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
use crate::store::data_output::DataOutput;
use crate::store::{ByteBuffersDataOutput, DataInput, IndexOutput};
use crate::util::error::data_io_error_enum::DataIOError;
use crc32fast::Hasher;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

/// An [`IndexOutput`] writing to a [`ByteBuffersDataOutput`]
pub struct ByteBuffersIndexOutput<'a> {
    last_checksum_position: u64,
    last_checksum: i64,
    delegate: &'a mut ByteBuffersDataOutput,
    name: String,
    resource_description: String,
    checksum: Hasher,
}
impl<'a> ByteBuffersIndexOutput<'a> {
    pub fn new_with_checksum(
        name: &str,
        resource_description: &str,
        delegate: &'a mut ByteBuffersDataOutput,
        checksum: Hasher,
    ) -> Self {
        Self {
            last_checksum_position: 0,
            last_checksum: 0,
            delegate,
            name: name.to_string(),
            resource_description: resource_description.to_string(),
            checksum,
        }
    }
    pub fn new(
        name: &str,
        resource_description: &str,
        delegate: &'a mut ByteBuffersDataOutput,
    ) -> Self {
        Self::new_with_checksum(name, resource_description, delegate, Hasher::new())
    }
    pub fn get_array_copy(&self) -> Vec<u8> {
        self.delegate.get_array_copy()
    }
}

impl<'a> DataOutput for ByteBuffersIndexOutput<'a> {
    fn write_byte(&mut self, b: u8) -> Result<(), DataIOError> {
        self.delegate.write_byte(b)
    }

    fn write_bytes_with_len(&mut self, b: &[u8], len: usize) -> Result<(), DataIOError> {
        self.delegate.write_bytes_with_len(b, len)
    }

    fn write_bytes_range(
        &mut self,
        b: &[u8],
        offset: usize,
        length: usize,
    ) -> Result<(), DataIOError> {
        self.delegate.write_bytes_range(b, offset, length)
    }

    fn write_int(&mut self, i: i32) -> Result<(), DataIOError> {
        self.delegate.write_int(i)
    }

    fn write_short(&mut self, i: i16) -> Result<(), DataIOError> {
        self.delegate.write_short(i)
    }

    fn write_long(&mut self, i: i64) -> Result<(), DataIOError> {
        self.delegate.write_long(i)
    }

    fn write_string(&mut self, s: &str) -> Result<(), DataIOError> {
        self.delegate.write_string(s)
    }

    fn copy_bytes<T: DataInput>(
        &mut self,
        input: &mut T,
        num_bytes: i64,
    ) -> Result<(), DataIOError> {
        self.delegate.copy_bytes(input, num_bytes)
    }

    fn write_map_of_strings(&mut self, map: &HashMap<String, String>) -> Result<(), DataIOError> {
        self.delegate.write_map_of_strings(map)
    }
}

impl<'a> Display for ByteBuffersIndexOutput<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.resource_description)
    }
}

impl<'a> IndexOutput for ByteBuffersIndexOutput<'a> {
    fn get_file_pointer(&self) -> u64 {
        self.delegate.size()
    }

    fn get_check_sum(&mut self) -> i64 {
        if self.last_checksum_position != self.delegate.size() {
            self.last_checksum_position = self.delegate.size();
            self.checksum.reset();
            let (length, mut data) = self.delegate.to_buffer_list();
            if let Some(last_block) = data.pop() {
                //  block length was limited by ByteBuffersDataOutput#LIMIT_MAX_BITS_PER_BLOCK
                debug_assert!(last_block.get_ref().len() <= u32::MAX as usize);
                let mut last_block_len = length;
                for block in data {
                    //Each block has the same data length except for the last block.
                    // Therefore, we need to use last_block_len to get the data length
                    // of the last block.
                    last_block_len -= block.get_ref().len() as u64;
                    self.checksum.update(block.get_ref());
                }
                self.checksum
                    .update(&last_block.get_ref()[0..last_block_len as usize]);
            }
            self.last_checksum = self.checksum.clone().finalize() as i64;
        }
        self.last_checksum
    }

    fn get_name(&self) -> &str {
        self.name.as_str()
    }
}
