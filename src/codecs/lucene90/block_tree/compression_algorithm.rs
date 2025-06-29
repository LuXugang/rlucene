/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/
use crate::store::DataInput;
use crate::util::compress::lowercase_ascii_compression::LowercaseAsciiCompression;
use crate::util::compress::lz4::LZ4;
use crate::util::error::lucene_error::{LuceneError, Result};

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
                "Illegal code for a compression algorithm: {}",
                code
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
