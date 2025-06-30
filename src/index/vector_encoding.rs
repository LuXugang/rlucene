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
use strum_macros::{Display, EnumCount, FromRepr};
/// The numeric datatype of the vector values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr, EnumCount, Display)]
#[repr(u8)]
pub enum VectorEncoding {
    /**
     * Encodes vector using 8 bits of precision per sample. Values provided
     * with higher precision (e.g., queries provided as float) *must*
     * be in the range [-128, 127]. NOTE: this can enable significant
     * storage savings and faster searches, at the cost of some possible
     * loss of precision.
     */
    BYTE(i32),

    /// Encodes vector using 32 bits of precision per sample in IEEE floating
    /// point format.
    FLOAT32(i32),
}

impl VectorEncoding {
    /// The number of bytes required to encode a scalar in this format.
    /// A vector will nominally require dimension * byteSize bytes of storage.
    pub fn byte_size(&self) -> i32 {
        match self {
            VectorEncoding::BYTE(size) => *size,
            VectorEncoding::FLOAT32(size) => *size,
        }
    }
}

impl Default for VectorEncoding {
    fn default() -> Self {
        VectorEncoding::BYTE(1)
    }
}
