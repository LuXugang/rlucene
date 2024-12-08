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
use std::io::Cursor;
use crate::store::byte_buffers_data_input::ByteBuffersDataInput;
use crate::store::{BufferedIndexInputBase, ByteBuffersIndexInput, Context, DataInput, IOContext};
use crate::store::index_input::IndexInput;
use crate::store::random_access_input::RandomAccessInput;
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::error::runtime_error::RuntimeError;

/// Default buffer size set to `BUFFER_SIZE`.
pub const BUFFER_SIZE: u32= 1024;
/// Minimum buffer size allowed
pub const MIN_BUFFER_SIZE: u32 = 8;

/// A buffer size for merges set to `MERGE_BUFFER_SIZE`. */
/// The normal read buffer size defaults to 1024, but
/// increasing this during merging seems to yield
/// performance gains.  However, we don't want to increase
/// it too much because there are quite a few
/// BufferedIndexInputs created during merging.  See
/// LUCENE-888 for details.
pub const MERGE_BUFFER_SIZE: u32 = 4096;

/// Base implementation class for buffered [`IndexInput`]. */
pub struct BufferedIndexInput<T> where T:IndexInput + BufferedIndexInputBase{
   buffer_size: u32,
    resource_desc: String,
    buffer: Cursor<Vec<u8>>,
    sub_index_input: T
}
impl <T>BufferedIndexInput<T> where T:IndexInput + BufferedIndexInputBase{
    pub fn new_with_buffer_size(sub_index_input:T, resource_desc:&str, buffer_size: u32) -> BufferedIndexInput<T> {
        let buffer = Cursor::new(vec![0u8; buffer_size as usize]);
        BufferedIndexInput {
            buffer_size,
            resource_desc: resource_desc.to_string(),
            buffer,
            sub_index_input
            
        }
    }
    pub fn new_with_resource_desc(sub_index_input:T, resource_desc: &str) -> BufferedIndexInput<T> {
       Self::new_with_buffer_size(sub_index_input, resource_desc, BUFFER_SIZE)
    }
   
    pub fn new_io_context(sub_index_input:T, resource_desc: &str, context: IOContext) -> BufferedIndexInput<T> {
        Self::new_with_buffer_size(sub_index_input,resource_desc, Self::buffer_size(context))
    }

    /// Returns default buffer sizes for the given [`IOContext`].
    pub fn buffer_size(io_context: IOContext) -> u32{
        match io_context.context {
            Context::Merge => MERGE_BUFFER_SIZE,
            Context::Default | Context::Flush => BUFFER_SIZE,
        }
    }
    
    fn check_buffer_size(buffer_size: u32) -> Result<(), RuntimeError>{
        if buffer_size < MIN_BUFFER_SIZE {
            return Err(RuntimeError::illegal_argument(format!("bufferSize must be at least MIN_BUFFER_SIZE (got {})", buffer_size)));
        } 
        Ok(())
    }
}

impl <T>DataInput for BufferedIndexInput<T>
where T:IndexInput + BufferedIndexInputBase{
    fn read_byte(&mut self) -> Result<u8, DataIOError> {
        todo!()
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<(), DataIOError> {
        self.read_bytes_with_buffer(b, offset, len, true)
    }

    fn read_bytes_with_buffer(&mut self, b: &mut [u8], offset: usize, len: usize, _use_buffer: bool) -> Result<(), DataIOError> {
        todo!()
    }


    fn skip_bytes(&mut self, num_bytes: u64) -> Result<(), DataIOError> {
        todo!()
    }
}

impl <T>Display for BufferedIndexInput<T>where T:IndexInput + BufferedIndexInputBase{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl <T>Clone for BufferedIndexInput<T> where T:IndexInput + BufferedIndexInputBase{
    fn clone(&self) -> Self {
        todo!()
    }
}

impl <T>IndexInput for BufferedIndexInput<T> where T:IndexInput + BufferedIndexInputBase{
    fn get_file_pointer(&self) -> u64 {
        todo!()
    }

    fn seek(&mut self, pos: u64) -> Result<(), DataIOError> {
        todo!()
    }

    fn length(&self) -> u64 {
        todo!()
    }

    fn slice(&self, slice_description: &str, offset: u64, length: u64) -> Result<ByteBuffersIndexInput, DataIOError> {
        unreachable!()
    }

    fn is_random_access(&self) -> bool {
        todo!()
    }

    fn get_random_access_slice(&self, offset: u64, length: u64) -> Result<ByteBuffersDataInput, DataIOError> {
        unreachable!()
    }
}

impl <T>RandomAccessInput for BufferedIndexInput<T> where T:IndexInput + BufferedIndexInputBase{
    fn length(&self) -> u64 {
        todo!()
    }

    fn read_byte(&mut self, pos: u64) -> Result<u8, DataIOError> {
        todo!()
    }

    fn read_short(&mut self, pos: u64) -> Result<i16, DataIOError> {
        todo!()
    }

    fn read_int(&mut self, pos: u64) -> Result<i32, DataIOError> {
        todo!()
    }

    fn read_long(&mut self, pos: u64) -> Result<i64, DataIOError> {
        todo!()
    }

    fn pre_fetch(&mut self, pos: u64, len: u64) -> Result<(), DataIOError> {
        todo!()
    }
}
