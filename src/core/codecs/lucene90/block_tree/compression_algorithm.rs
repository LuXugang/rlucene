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
use crate::core::store::DataInput;
use crate::core::util::compress::lowercase_ascii_compression::LowercaseAsciiCompression;
use crate::core::util::compress::lz4::LZ4;
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Compression algorithm used for suffixes of a block of terms.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CompressionAlgorithm {
    NoCompression,
    LowercaseAscii,
    Lz4,
}

impl CompressionAlgorithm {
    pub fn code(&self) -> u8 {
        match self {
            CompressionAlgorithm::NoCompression => 0x00,
            CompressionAlgorithm::LowercaseAscii => 0x01,
            CompressionAlgorithm::Lz4 => 0x02,
        }
    }

    pub fn by_code(code: u8) -> Result<Self> {
        match code {
            0x00 => Ok(CompressionAlgorithm::NoCompression),
            0x01 => Ok(CompressionAlgorithm::LowercaseAscii),
            0x02 => Ok(CompressionAlgorithm::Lz4),
            _ => Err(LuceneError::illegal_argument(format!(
                "Illegal code for a compression algorithm: {code}"
            ))),
        }
    }

    pub fn read(&self, input: &mut impl DataInput, out: &mut [u8], len: i32) -> Result<()> {
        match self {
            CompressionAlgorithm::NoCompression => {
                input.read_bytes(out, 0, len)?;
            },
            CompressionAlgorithm::LowercaseAscii => {
                debug_assert!(len >= 0);
                LowercaseAsciiCompression::decompress(input, out, len as usize)?;
            },
            CompressionAlgorithm::Lz4 => {
                LZ4::decompress(input, len, out, 0)?;
            },
        }
        Ok(())
    }
}
