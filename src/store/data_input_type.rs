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
use crate::store::{ByteArrayDataInput, DataInput};
use crate::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

pub enum DataInputType<'a> {
    ByteArray(ByteArrayDataInput),
    ByteBuffers(ByteBuffersDataInput<'a>),
}

impl DataInputType<'_> {
    pub fn new_byte_buffers(input: ByteBuffersDataInput) -> DataInputType {
        DataInputType::ByteBuffers(input)
    }
}

impl Display for DataInputType<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DataInputType::ByteArray(input) => write!(f, "{}", input),
            DataInputType::ByteBuffers(input) => write!(f, "{}", input),
        }
    }
}

impl DataInput for DataInputType<'_> {
    fn read_byte(&mut self) -> Result<u8> {
        match self {
            DataInputType::ByteArray(data_input) => data_input.read_byte(),
            DataInputType::ByteBuffers(data_input) => data_input.read_byte(),
        }
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, length: i32) -> Result<()> {
        match self {
            DataInputType::ByteArray(data_input) => data_input.read_bytes(b, offset, length),
            DataInputType::ByteBuffers(data_input) => data_input.read_bytes(b, offset, length),
        }
    }

    fn read_short(&mut self) -> Result<i16> {
        match self {
            DataInputType::ByteArray(data_input) => data_input.read_short(),
            DataInputType::ByteBuffers(data_input) => data_input.read_short(),
        }
    }

    fn read_int(&mut self) -> Result<i32> {
        match self {
            DataInputType::ByteArray(data_input) => data_input.read_int(),
            DataInputType::ByteBuffers(data_input) => data_input.read_int(),
        }
    }

    fn read_vint(&mut self) -> Result<i32> {
        match self {
            DataInputType::ByteArray(data_input) => data_input.read_vint(),
            DataInputType::ByteBuffers(data_input) => data_input.read_vint(),
        }
    }

    fn read_long(&mut self) -> Result<i64> {
        match self {
            DataInputType::ByteArray(data_input) => data_input.read_long(),
            DataInputType::ByteBuffers(data_input) => data_input.read_long(),
        }
    }

    fn read_vlong(&mut self) -> Result<i64> {
        match self {
            DataInputType::ByteArray(data_input) => data_input.read_vlong(),
            DataInputType::ByteBuffers(data_input) => data_input.read_vlong(),
        }
    }

    fn read_string(&mut self) -> Result<String> {
        match self {
            DataInputType::ByteArray(data_input) => data_input.read_string(),
            DataInputType::ByteBuffers(data_input) => data_input.read_string(),
        }
    }

    fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
        match self {
            DataInputType::ByteArray(data_input) => data_input.skip_bytes(num_bytes),
            DataInputType::ByteBuffers(data_input) => data_input.skip_bytes(num_bytes),
        }
    }
}
