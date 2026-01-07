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

use crate::core::store::DataInput;
use crate::core::store::random_access_input::RandomAccessInput;
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fst_impl::fst::BytesReader;

/// Implements reverse read from a RandomAccessInput.
pub struct ReverseRandomAccessReader<R>
where
    R: RandomAccessInput,
{
    input: R,
    pos: usize,
}

impl<R> ReverseRandomAccessReader<R>
where
    R: RandomAccessInput,
{
    pub fn new(input: R) -> Self {
        Self { input, pos: 0 }
    }
}

impl<R> DataInput for ReverseRandomAccessReader<R>
where
    R: RandomAccessInput,
{
    fn read_byte(&mut self) -> Result<u8> {
        let b = self.input.read_byte(self.pos)?;
        self.pos -= 1;
        Ok(b)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
        let mut i = offset;
        let end = offset + len;
        while i < end {
            b[i] = self.input.read_byte(self.pos)?;
            self.pos -= 1;
            i += 1;
        }
        Ok(())
    }

    fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
        let num_bytes = num_bytes.try_convert()?;
        self.pos = self.pos.checked_sub(num_bytes).ok_or_else(|| {
            LuceneError::illegal_state(format!(
                "underflow, pos {}, num_bytes {} ",
                self.pos, num_bytes
            ))
        })?;
        Ok(())
    }
}

impl<R> BytesReader for ReverseRandomAccessReader<R>
where
    R: RandomAccessInput,
{
    fn get_position(&self) -> usize {
        self.pos
    }

    fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }
}

impl<R> Display for ReverseRandomAccessReader<R>
where
    R: RandomAccessInput,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::any::type_name::<Self>())
    }
}
