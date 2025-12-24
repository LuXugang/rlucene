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
use crate::core::store::{DataOutput, IndexInput};
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fst_impl::fst_reader::FstReader;
use crate::core::util::fst_impl::reverse_random_access_reader::ReverseRandomAccessReader;
use std::sync::Arc;
/// Provides off heap storage of finite state machine (FST), using underlying
/// index input instead of  byte store on heap
pub struct OffHeapFSTStore<I>
where
    I: IndexInput,
{
    input: Arc<I>,
    offset: i64,
    num_bytes: i64,
}
impl<I> OffHeapFSTStore<I>
where
    I: IndexInput,
{
    pub fn new(input: Arc<I>, offset: i64, num_bytes: i64) -> Self {
        Self {
            input,
            offset,
            num_bytes,
        }
    }
    pub fn size(&self) -> i64 {
        self.num_bytes
    }
}

impl<I> Accountable for OffHeapFSTStore<I>
where
    I: IndexInput,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

impl<I> FstReader for OffHeapFSTStore<I>
where
    I: IndexInput,
{
    type FstBytesReader = ReverseRandomAccessReader<I::RandomAccessSlice>;

    fn get_reverse_bytes_reader(&self) -> Result<Self::FstBytesReader> {
        let slice = self
            .input
            .random_access_slice(self.offset, self.num_bytes)?;
        Ok(ReverseRandomAccessReader::new(slice))
    }

    fn write_to(&self, _out: &mut impl DataOutput) -> Result<()> {
        Err(LuceneError::unsupported_operation(
            "write_to is not supported for OffHeapFSTStore",
        ))
    }
}
