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
use std::io::{BufWriter, Write};

use byteorder::WriteBytesExt;

use crate::core::store::data_output::DataOutput;
use crate::core::util::close::{Closeable, CloseableWrite};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
/// A [`DataOutput`] wrapping a plain [`OutputStream`](Write).
pub struct OutputStreamDataOutput<W: CloseableWrite> {
  pub os: Option<BufWriter<W>>,
}
impl<W: CloseableWrite> OutputStreamDataOutput<W> {
  pub fn new(os: W) -> OutputStreamDataOutput<W> {
    OutputStreamDataOutput {
      os: Some(BufWriter::new(os)),
    }
  }

  fn output_stream(&mut self) -> Result<&mut BufWriter<W>> {
    self
      .os
      .as_mut()
      .ok_or_else(|| LuceneError::already_closed("this DataOutput is closed"))
  }
}

impl<W: CloseableWrite> Closeable for OutputStreamDataOutput<W> {
  fn close(&mut self) -> Result<()> {
    if let Some(mut output_stream) = self.os.take() {
      let flush_result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| output_stream.flush()));
      let (output_stream, _) = output_stream.into_parts();
      let close_result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| output_stream.close()));
      IOUtils::use_or_suppress_caught_result(
        flush_result.map(|result| result.map_err(Into::into)),
        close_result,
      )?;
    }
    Ok(())
  }
}

impl<W: CloseableWrite> DataOutput for OutputStreamDataOutput<W> {
  fn write_byte(&mut self, b: u8) -> Result<()> {
    Ok(self.output_stream()?.write_u8(b)?)
  }

  fn write_bytes_range(&mut self, b: &[u8], offset: usize, length: usize) -> Result<()> {
    let end = offset + length;
    Ok(self.output_stream()?.write_all(&b[offset..end])?)
  }
}
