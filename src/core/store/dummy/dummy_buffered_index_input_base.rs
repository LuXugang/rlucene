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
use std::io::Cursor;

use crate::core::store::{BufferedIndexInput, BufferedIndexInputBase};
use crate::core::util::error::lucene_error::Result;

pub struct DummyBufferedIndexInputBase;

impl crate::core::util::clone::TryClone for DummyBufferedIndexInputBase {
    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl BufferedIndexInputBase for DummyBufferedIndexInputBase {
    fn seek_internal(&mut self, _pos: i64) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn read_internal(
        &mut self,
        _b: &mut Cursor<Vec<u8>>,
        _len: i64,
        _file_pointer: i64,
    ) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type Slice = BufferedIndexInput<DummyBufferedIndexInputBase>;

    fn slice(&self, _slice_description: &str, _offset: i64, _length: i64) -> Result<Self::Slice> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn length(&self) -> i64 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
