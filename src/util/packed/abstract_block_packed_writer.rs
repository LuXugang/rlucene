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
use crate::store::DataOutput;
use crate::util::error::lucene_error::LuceneError;
use crate::util::packed::Format::Packed;
use crate::util::packed::{Encoder, FormatBehavior, PackedImpl, PackedInts};

pub(crate) const MIN_BLOCK_SIZE: u32 = 64;
pub(crate) const MAX_BLOCK_SIZE: u32 = 1 << (30 - 3);
pub(crate) const MIN_VALUE_EQUALS_0: u32 = 1 << 0;
pub(crate) const BPV_SHIFT: u32 = 1;
pub struct AbstractBlockPackedWriter<'a, T: DataOutput, D: AbstractBlockPackedWriterBase> {
    out: &'a mut T,
    values: Vec<i64>,
    blocks: Vec<u8>,
    off: usize,
    ord: u64,
    finished: bool,
    sub_writer: D,
}

impl<'a, D: AbstractBlockPackedWriterBase, T: DataOutput> AbstractBlockPackedWriter<'a, T, D> {
    /// Constructs a new `AbstractBlockPackedWriter`.
    ///
    /// # Arguments
    ///
    /// * `out` - The output stream.
    /// * `block_size` - The number of values in a single block, must be a multiple of 64.
    ///
    /// # Errors
    ///
    /// Returns an error if `block_size` is not valid.
    pub fn new(block_size: u32, sub_writer: D, out: &'a mut T) -> Result<Self, LuceneError> {
        PackedInts::check_block_size(block_size, MIN_BLOCK_SIZE, MAX_BLOCK_SIZE)?;

        Ok(Self {
            out,
            values: vec![0; block_size as usize],
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
    pub fn reset(&mut self, out: &'a mut T) {
        self.out = out;
        self.off = 0;
        self.ord = 0;
        self.finished = false;
    }
    fn check_not_finished(&self) -> Result<(), LuceneError> {
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
    /// Returns an error if the writer has already finished or if flushing fails.
    pub fn add(&mut self, value: i64) -> Result<(), LuceneError> {
        self.sub_writer.add(value);
        self.check_not_finished()?;
        if self.off == self.values.len() {
            self.sub_writer
                .flush(self.out, &mut self.off, &mut self.values, &mut self.blocks)?;
        }
        self.values[self.off] = value;
        self.off += 1;
        self.ord += 1;
        Ok(())
    }
    /// Adds a block of zeros to the writer (for testing only).
    ///
    /// # Errors
    ///
    /// Returns an error if the writer has already finished or the offset is invalid.
    #[cfg(feature = "test_only")]
    pub fn add_block_of_zeros(&mut self) -> Result<(), LuceneError> {
        self.check_not_finished()?;
        if self.off != 0 && self.off != self.values.len() {
            return Err(LuceneError::illegal_state(format!("{}", self.off)));
        }
        if self.off == self.values.len() {
            self.sub_writer
                .flush(self.out, &mut self.off, &mut self.values, &mut self.blocks)?;
        }
        self.values.fill(0);
        self.off = self.values.len();
        self.ord += self.values.len() as u64;
        Ok(())
    }
    /// Flushes all buffered data to the output stream. After calling this method,
    /// this instance is no longer usable until `reset` is called.
    ///
    /// # Errors
    ///
    /// Returns an error if the writer has already finished or if flushing fails.
    pub fn finish(&mut self) -> Result<(), LuceneError> {
        self.check_not_finished()?;
        if self.off > 0 {
            self.sub_writer
                .flush(self.out, &mut self.off, &mut self.values, &mut self.blocks)?;
        }
        self.finished = true;
        Ok(())
    }
    /// Returns the number of values that have been added.
    pub fn ord(&self) -> u64 {
        self.ord
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
    pub fn write_values<O: DataOutput>(
        bits_required: u32,
        out: &mut O,
        blocks: &mut Vec<u8>,
        values: &mut [i64],
        off: usize,
    ) -> Result<(), LuceneError> {
        let encoder = PackedInts::get_encoder(
            Packed(PackedImpl::new(0)),
            PackedInts::VERSION_CURRENT,
            bits_required,
        )?;
        let iterations = values.len() / Encoder::byte_value_count(encoder) as usize;
        let block_size = Encoder::byte_value_count(encoder) as usize * iterations;
        if blocks.len() < block_size {
            *blocks = vec![0u8; block_size];
        }
        if off < values.len() {
            for value in values.iter_mut().skip(off) {
                *value = 0;
            }
        }
        debug_assert!(iterations <= u32::MAX as usize);
        encoder.encode_i64_to_u8(values, 0, blocks, 0, iterations as u32);
        debug_assert!(off <= u32::MAX as usize);
        let block_count = Packed(PackedImpl::new(0)).byte_count(
            PackedInts::VERSION_CURRENT,
            off as u32,
            bits_required,
        );
        debug_assert!(block_count <= u32::MAX as u64);
        out.write_bytes_with_len(blocks, block_count as u32)?;
        Ok(())
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
pub(crate) fn write_values<O: DataOutput>(
    bits_required: u32,
    out: &mut O,
    blocks: &mut Vec<u8>,
    values: &mut [i64],
    off: usize,
) -> Result<(), LuceneError> {
    let encoder = PackedInts::get_encoder(
        Packed(PackedImpl::new(0)),
        PackedInts::VERSION_CURRENT,
        bits_required,
    )?;
    let iterations = values.len() / Encoder::byte_value_count(encoder) as usize;
    let block_size = Encoder::byte_block_count(encoder) as usize * iterations;
    if blocks.len() < block_size {
        *blocks = vec![0u8; block_size];
    }
    if off < values.len() {
        for value in values.iter_mut().skip(off) {
            *value = 0;
        }
    }
    debug_assert!(iterations <= u32::MAX as usize);
    encoder.encode_i64_to_u8(values, 0, blocks, 0, iterations as u32);
    debug_assert!(off <= u32::MAX as usize);
    let block_count = Packed(PackedImpl::new(0)).byte_count(
        PackedInts::VERSION_CURRENT,
        off as u32,
        bits_required,
    );
    debug_assert!(block_count <= u32::MAX as u64);
    out.write_bytes_with_len(blocks, block_count as u32)?;
    Ok(())
}
/// Same as DataOutput::writeVLong but accepts negative values.
pub(crate) fn write_vlong<T: DataOutput>(out: &mut T, mut i: i64) -> Result<(), LuceneError> {
    let mut k = 0;
    while (i & !0x7F) != 0 && k < 8 {
        out.write_byte(((i & 0x7F) | 0x80) as u8)?;
        i >>= 7;
        k += 1;
    }
    out.write_byte(i as u8)?;
    Ok(())
}
pub trait AbstractBlockPackedWriterBase {
    fn flush<T: DataOutput>(
        &mut self,
        out: &mut T,
        off: &mut usize,
        values: &mut [i64],
        blocks: &mut Vec<u8>,
    ) -> Result<(), LuceneError>;
    fn add(&mut self, _value: i64) {}
}
