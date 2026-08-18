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
use crate::core::store::DataOutput;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::packed::Format::Packed;
use crate::core::util::packed::{Encoder, FormatBehavior, PackedImpl, PackedInts};

pub(crate) const MIN_BLOCK_SIZE: i32 = 64;
pub(crate) const MAX_BLOCK_SIZE: i32 = 1 << (30 - 3);
pub(crate) const MIN_VALUE_EQUALS_0: i32 = 1 << 0;
pub(crate) const BPV_SHIFT: i32 = 1;
pub(crate) struct AbstractBlockPackedWriter<D> {
  values: Vec<i64>,
  blocks: Vec<u8>,
  off: i32,
  ord: usize,
  finished: bool,
  sub_writer: D,
}

impl<D: AbstractBlockPackedWriterBase> AbstractBlockPackedWriter<D> {
  /// Constructs a new `AbstractBlockPackedWriter`.
  ///
  /// # Arguments
  ///
  /// * `out` - The output stream.
  /// * `block_size` - The number of values in a single block, must be a
  ///   multiple of 64.
  ///
  /// # Errors
  ///
  /// Returns an error if `block_size` is not valid.
  pub fn new(block_size: usize, sub_writer: D) -> Result<Self> {
    PackedInts::check_block_size(block_size as i32, MIN_BLOCK_SIZE, MAX_BLOCK_SIZE)?;

    Ok(Self {
      values: vec![0; block_size],
      blocks: Vec::new(),
      off: 0,
      ord: 0,
      finished: false,
      sub_writer,
    })
  }

  /// Resets the writer with a new output.
  ///
  /// # Arguments
  ///
  /// * `out` - The new output stream.
  pub fn reset(&mut self) {
    self.off = 0;
    self.ord = 0;
    self.finished = false;
  }
  fn check_not_finished(&self) -> Result<()> {
    if self.finished {
      return Err(LuceneError::illegal_state("Already finished"));
    }
    Ok(())
  }

  /// Appends a new `i64` value to the writer.
  ///
  /// # Arguments
  ///
  /// * `value` - The value to append.
  ///
  /// # Errors
  ///
  /// Returns an error if the writer has already finished or if flushing
  /// fails.
  pub fn add(&mut self, value: i64, out: &mut impl DataOutput) -> Result<()> {
    self.sub_writer.add(value);
    self.check_not_finished()?;
    if self.off as usize == self.values.len() {
      self
        .sub_writer
        .flush(out, &mut self.off, &mut self.values, &mut self.blocks)?;
    }
    self.values[self.off as usize] = value;
    self.off += 1;
    self.ord += 1;
    Ok(())
  }
  /// Adds a block of zeros to the writer (for testing only).
  ///
  /// # Errors
  ///
  /// Returns an error if the writer has already finished or the offset is
  /// invalid.
  #[cfg(debug_assertions)]
  pub(crate) fn add_block_of_zeros(&mut self, out: &mut impl DataOutput) -> Result<()> {
    self.check_not_finished()?;
    if self.off != 0 && self.off as usize != self.values.len() {
      return Err(LuceneError::illegal_state(format!("{}", self.off)));
    }
    if self.off as usize == self.values.len() {
      self
        .sub_writer
        .flush(out, &mut self.off, &mut self.values, &mut self.blocks)?;
    }
    self.values.fill(0);
    debug_assert!(self.values.len() <= i32::MAX as usize);
    self.off = self.values.len() as i32;
    self.ord += self.values.len();
    Ok(())
  }
  /// Flushes all buffered data to the output stream. After calling this
  /// method, this instance is no longer usable until `reset` is called.
  ///
  /// # Errors
  ///
  /// Returns an error if the writer has already finished or if flushing
  /// fails.
  pub fn finish(&mut self, out: &mut impl DataOutput) -> Result<()> {
    self.check_not_finished()?;
    if self.off > 0 {
      self
        .sub_writer
        .flush(out, &mut self.off, &mut self.values, &mut self.blocks)?;
    }
    self.finished = true;
    Ok(())
  }
  /// Returns the number of values that have been added.
  #[cfg(test)]
  pub fn ord(&self) -> usize {
    self.ord
  }
}
/// Encodes and writes the current values to the output stream.
///
/// # Arguments
///
/// * `bits_required` - The number of bits required for encoding the values.
///
/// # Errors
///
/// Returns an error if writing to the output stream fails.
pub(crate) fn write_values(
  bits_required: i32,
  out: &mut impl DataOutput,
  blocks: &mut Vec<u8>,
  values: &mut [i64],
  off: i32,
) -> Result<()> {
  let encoder = PackedInts::get_encoder(
    Packed(PackedImpl::new(0)),
    PackedInts::VERSION_CURRENT,
    bits_required,
  )?;
  let iterations = values.len() / Encoder::byte_value_count(encoder) as usize;
  let block_size = Encoder::byte_block_count(encoder) as usize * iterations;
  ArrayUtil::grow_no_copy(blocks, block_size)?;
  if (off as usize) < values.len() {
    for value in values.iter_mut().skip(off as usize) {
      *value = 0;
    }
  }
  debug_assert!(iterations <= i32::MAX as usize);
  encoder.encode_i64_to_u8(values, 0, blocks, 0, iterations as i32);
  let block_count =
    Packed(PackedImpl::new(0)).byte_count(PackedInts::VERSION_CURRENT, off, bits_required);
  out.write_bytes_with_len(blocks, block_count as usize)?;
  Ok(())
}
/// Same as DataOutput::writeVLong but accepts negative values.
pub(crate) fn write_vlong(out: &mut impl DataOutput, mut i: i64) -> Result<()> {
  let mut k = 0;
  while (i & !0x7F) != 0 && k < 8 {
    out.write_byte(((i & 0x7F) | 0x80) as u8)?;
    i >>= 7;
    k += 1;
  }
  out.write_byte(i as u8)?;
  Ok(())
}
pub(crate) trait AbstractBlockPackedWriterBase {
  fn flush(
    &mut self,
    out: &mut impl DataOutput,
    off: &mut i32,
    values: &mut [i64],
    blocks: &mut Vec<u8>,
  ) -> Result<()>;
  fn add(&mut self, _value: i64) {}
}
