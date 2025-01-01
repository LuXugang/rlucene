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
use crate::store::data_output::DataOutput;
use crate::util::error::lucene_error::LuceneError;
use byteorder::WriteBytesExt;
use std::io::{BufWriter, Write};
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
    fn write_byte(&mut self, b: u8) -> Result<(), LuceneError> {
        Ok(self.os.write_u8(b)?)
    }

    fn write_bytes_range(
        &mut self,
        b: &[u8],
        offset: u32,
        length: u32,
    ) -> Result<(), LuceneError> {
        let end = offset + length;
        Ok(self.os.write_all(&b[offset as usize..end as usize])?)
    }
}
