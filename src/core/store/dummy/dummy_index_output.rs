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

use crate::core::store::{DataOutput, IndexOutput};
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;

pub struct DummyIndexOutput;

impl DataOutput for DummyIndexOutput {
    fn write_byte(&mut self, _b: u8) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn write_bytes_range(&mut self, _b: &[u8], _offset: i32, _length: i32) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl Display for DummyIndexOutput {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl Closeable for DummyIndexOutput {
    fn close(&mut self) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl IndexOutput for DummyIndexOutput {
    fn get_file_pointer(&self) -> i64 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_checksum(&mut self) -> u64 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_name(&self) -> &str {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
