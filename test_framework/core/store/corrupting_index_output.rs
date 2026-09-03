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
use crate::core::store::IO_CONTEXT_DEFAULT;
use crate::core::store::data_input::DataInput;
use crate::core::store::data_output::DataOutput;
use crate::core::store::directory::Directory;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
use std::fmt::{Display, Formatter};

/// Corrupts on bit of a file after close.
pub(crate) struct CorruptingIndexOutput<'a, D>
where
  D: Directory,
{
  dir: &'a D,
  byte_to_corrupt: usize,
  out: D::IndexOutput,
  closed: bool,
}

impl<'a, D> CorruptingIndexOutput<'a, D>
where
  D: Directory,
{
  pub(crate) fn new(dir: &'a D, byte_to_corrupt: usize, out: D::IndexOutput) -> Self {
    Self {
      dir,
      byte_to_corrupt,
      out,
      closed: false,
    }
  }
}

impl<D> IndexOutput for CorruptingIndexOutput<'_, D>
where
  D: Directory,
{
  fn get_name(&self) -> &str {
    self.out.get_name()
  }

  fn get_file_pointer(&self) -> Result<usize> {
    self.out.get_file_pointer()
  }

  fn get_checksum(&mut self) -> Result<u64> {
    Ok(self.out.get_checksum()? ^ 1)
  }
}

impl<D> Closeable for CorruptingIndexOutput<'_, D>
where
  D: Directory,
{
  fn close(&mut self) -> Result<()> {
    if !self.closed {
      self.out.close()?;
      // NOTE: must corrupt after file is closed, because if we corrupt "inlined" (as bytes are
      // being written) the checksum sees the wrong
      // bytes and is "correct"!!
      self.corrupt_file()?;
      self.closed = true;
    }
    Ok(())
  }
}
impl<D> Drop for CorruptingIndexOutput<'_, D>
where
  D: Directory,
{
  fn drop(&mut self) {
    let _ = self.close();
  }
}

impl<'a, D> CorruptingIndexOutput<'a, D>
where
  D: Directory,
{
  fn corrupt_file(&mut self) -> Result<()> {
    // Now corrupt the specified byte:
    let name = self.out.get_name().to_string();
    let new_temp_name;
    {
      let mut tmp_out = self.dir.create_temp_output(
        "tmp",
        "tmp",
        IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?,
      )?;
      new_temp_name = tmp_out.get_name().to_string();
      let input_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        self
          .dir
          .open_input(&name, IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?)
      }));
      let mut input = match input_result {
        Ok(Ok(input)) => input,
        result => {
          let close_result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tmp_out.close()));
          return IOUtils::use_or_suppress_caught_result(result, close_result).map(|_| ());
        },
      };

      let body_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> {
        let input_length = input.length()?;

        if self.byte_to_corrupt >= input_length {
          return Err(LuceneError::illegal_argument(format!(
            "byteToCorrupt={} but file \"{}\" is only length={}",
            self.byte_to_corrupt, name, input_length
          )));
        }

        tmp_out.copy_bytes(&mut input, self.byte_to_corrupt)?;
        // Flip the 0th bit:
        tmp_out.write_byte(input.read_byte()? ^ 1)?;
        tmp_out.copy_bytes(&mut input, input_length - self.byte_to_corrupt - 1)?;
        Ok(())
      }));
      let close_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        IOUtils::close_with(0..2, |operation| match operation {
          0 => input.close(),
          _ => tmp_out.close(),
        })
      }));
      IOUtils::use_or_suppress_caught_result(body_result, close_result)?;
    }

    // Delete original and copy corrupt version back:
    self.dir.delete_file(&name)?;
    self.dir.copy_from(
      self.dir,
      &new_temp_name,
      &name,
      IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?,
    )?;
    self.dir.delete_file(&new_temp_name)?;
    Ok(())
  }
}

impl<D> DataOutput for CorruptingIndexOutput<'_, D>
where
  D: Directory,
{
  fn write_byte(&mut self, b: u8) -> Result<()> {
    self.out.write_byte(b)
  }

  fn write_bytes_range(&mut self, b: &[u8], offset: usize, length: usize) -> Result<()> {
    for i in 0..length {
      self.write_byte(b[offset + i])?;
    }
    Ok(())
  }
}

impl<D> Display for CorruptingIndexOutput<'_, D>
where
  D: Directory,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "CorruptingIndexOutput({})", self.out)
  }
}
