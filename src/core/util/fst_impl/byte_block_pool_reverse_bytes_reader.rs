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
use crate::core::util::ByteBlockPool;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fst_impl::fst::BytesReader;

/// Reads in reverse from a ByteBlockPool.
pub struct ByteBlockPoolReverseBytesReader {
    pub(crate) buf: ByteBlockPool,
    // the difference between the FST node address and the hash table copied
    // node address
    pos_delta: i64,
    pos: i64,
}
impl ByteBlockPoolReverseBytesReader {
    pub fn new(buf: ByteBlockPool) -> Self {
        Self {
            buf,
            pos_delta: 0,
            pos: 0,
        }
    }
    pub fn set_pos_delta(&mut self, pos_delta: i64) {
        self.pos_delta = pos_delta;
    }
}

impl DataInput for ByteBlockPoolReverseBytesReader {
    fn read_byte(&mut self) -> Result<u8> {
        let b = self.buf.read_byte(self.pos);
        self.pos -= 1;
        Ok(b)
    }

    fn read_bytes(&mut self, b: &mut [u8], offset: i32, len: i32) -> Result<()> {
        let offset = offset as usize;
        let len = len as usize;
        for i in 0..len {
            b[offset + i] = self.buf.read_byte(self.pos);
            self.pos -= 1;
        }
        Ok(())
    }

    fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
        self.pos -= num_bytes;
        Ok(())
    }
}

impl Display for ByteBlockPoolReverseBytesReader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::any::type_name::<Self>())
    }
}

impl BytesReader for ByteBlockPoolReverseBytesReader {
    fn get_position(&self) -> i64 {
        self.pos + self.pos_delta
    }

    fn set_position(&mut self, pos: i64) {
        self.pos = pos - self.pos_delta;
    }
}
