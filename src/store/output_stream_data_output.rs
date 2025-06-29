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
use std::io::{BufWriter, Write};

use byteorder::WriteBytesExt;

use crate::store::data_output::DataOutput;
use crate::util::error::lucene_error::Result;
/// A [`DataOutput`] wrapping a plain [`OutputStream`](Write).
pub struct OutputStreamDataOutput<W: Write> {
    pub os: BufWriter<W>,
}
impl<W: Write> OutputStreamDataOutput<W> {
    pub fn new(os: W) -> OutputStreamDataOutput<W> {
        OutputStreamDataOutput {
            os: BufWriter::new(os),
        }
    }
}
impl<W: Write> DataOutput for OutputStreamDataOutput<W> {
    fn write_byte(&mut self, b: u8) -> Result<()> {
        Ok(self.os.write_u8(b)?)
    }

    fn write_bytes_range(&mut self, b: &[u8], offset: i32, length: i32) -> Result<()> {
        let end = offset + length;
        Ok(self.os.write_all(&b[offset as usize..end as usize])?)
    }
}
