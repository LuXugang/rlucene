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
use crate::store::DataInput;
use crate::util::error::lucene_error::LuceneError;
use crate::util::fst::byte_block_pool_reverse_bytes_reader::ByteBlockPoolReverseBytesReader;
use crate::util::fst::reverse_bytes_reader::ReverseBytesReader;
use crate::util::fst::reverse_random_access_reader::ReverseRandomAccessReader;
use std::fmt::{Display, Formatter};

/// Reads bytes stored in an FST.
pub trait BytesReader: DataInput {
    /// Get current read position.
    fn get_position(&self) -> i64;

    /// Set current read position.
    fn set_position(&mut self, pos: i64);
}

pub(crate) enum BytesReaderEnum<'a, R>
where
    R: RandomAccessInput,
{
    ByteBlockPool(ByteBlockPoolReverseBytesReader),
    Reverse(ReverseBytesReader<'a>),
    ReverseRandomAccess(ReverseRandomAccessReader<R>),
    Dummy(DummyBytesReader),
}

pub struct DummyBytesReader;

impl DataInput for DummyBytesReader {
    fn read_byte(&mut self) -> Result<u8, LuceneError> {
        Err(LuceneError::unsupported_operation(
            "DummyBytesReader does not support reading bytes".to_string(),
        ))
    }

    fn read_bytes(&mut self, _b: &mut [u8], _offset: i32, _len: i32) -> Result<(), LuceneError> {
        Err(LuceneError::unsupported_operation(
            "DummyBytesReader does not support reading bytes".to_string(),
        ))
    }

    fn skip_bytes(&mut self, _num_bytes: i64) -> Result<(), LuceneError> {
        Err(LuceneError::unsupported_operation(
            "DummyBytesReader does not support skipping bytes".to_string(),
        ))
    }
}

impl Display for DummyBytesReader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DummyBytesReader")
    }
}

impl BytesReader for DummyBytesReader {
    fn get_position(&self) -> i64 {
        0
    }

    fn set_position(&mut self, _pos: i64) {}
}
