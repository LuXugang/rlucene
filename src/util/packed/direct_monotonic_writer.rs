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
use crate::store::IndexOutput;
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::packed::direct_writer::{direct_writer_util, DirectWriter};
/// Write monotonically-increasing sequences of integers. This writer splits
/// data into blocks and then for each block, computes the average slope, the
/// minimum value, and encodes only the delta from the expected value using a
/// `DirectWriter`.
///
/// # See also
/// `DirectMonotonicReader`
///
/// # Internal
pub struct DirectMonotonicWriter<'a, I1, I2>
where
    I1: IndexOutput,
    I2: IndexOutput,
{
    meta: &'a mut I1,
    data: &'a mut I2,
    num_values: i64,
    base_data_pointer: i64,
    buffer: Vec<i64>,
    buffer_size: i32,
    count: i64,
    finished: bool,
    previous: i64,
}
pub mod direct_monotonic_writer_util {
    pub const MIN_BLOCK_SHIFT: i32 = 2;
    pub const MAX_BLOCK_SHIFT: i32 = 22;
}

impl<'a, I1, I2> DirectMonotonicWriter<'a, I1, I2>
where
    I1: IndexOutput,
    I2: IndexOutput,
{
    fn new(
        meta_out: &'a mut I1,
        data_out: &'a mut I2,
        num_values: i64,
        block_shift: i32,
    ) -> Result<Self> {
        if !(direct_monotonic_writer_util::MIN_BLOCK_SHIFT
            ..=direct_monotonic_writer_util::MAX_BLOCK_SHIFT)
            .contains(&block_shift)
        {
            return Err(LuceneError::illegal_argument(format!(
                "blockShift must be in [{}-{}], got {}",
                direct_monotonic_writer_util::MIN_BLOCK_SHIFT,
                direct_monotonic_writer_util::MAX_BLOCK_SHIFT,
                block_shift
            )));
        }

        if num_values < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "numValues can't be negative, got {num_values}"
            )));
        }

        let num_blocks = if num_values == 0 {
            0
        } else {
            ((num_values - 1) >> block_shift) + 1
        };

        if num_blocks > ArrayUtil::MAX_ARRAY_LENGTH as i64 {
            return Err(LuceneError::illegal_argument(format!(
                "blockShift is too low for the provided number of values: blockShift={}, numValues={}, MAX_ARRAY_LENGTH={}",
                block_shift,
                num_values,
                ArrayUtil::MAX_ARRAY_LENGTH
            )));
        }

        let block_size = 1i64 << block_shift;
        let buffer_len = std::cmp::min(num_values, block_size) as i32;
        let buffer = vec![0i64; buffer_len as usize];
        let base_data_pointer = data_out.get_file_pointer();

        Ok(DirectMonotonicWriter {
            meta: meta_out,
            data: data_out,
            num_values,
            base_data_pointer,
            buffer,
            buffer_size: 0,
            count: 0,
            finished: false,
            previous: i64::MIN,
        })
    }
    fn flush(&mut self) -> Result<()> {
        debug_assert!(self.buffer_size != 0);

        let avg_inc = {
            let numerator = self.buffer[(self.buffer_size - 1) as usize] - self.buffer[0];
            let denominator = std::cmp::max(1, self.buffer_size - 1) as f64;
            (numerator as f64 / denominator) as f32
        };

        let mut min = i64::MAX;
        for i in 0..(self.buffer_size as usize) {
            let expected = (avg_inc * (i as f32)) as i64;
            self.buffer[i] -= expected;
            min = std::cmp::min(self.buffer[i], min);
        }

        let mut max_delta = 0;
        for i in 0..(self.buffer_size as usize) {
            self.buffer[i] -= min;
            // use | will change nothing when it comes to computing required
            // bits but has the benefit of working fine with
            // negative values too (in case of overflow)
            max_delta |= self.buffer[i];
        }

        self.meta.write_long(min)?;
        self.meta.write_int(avg_inc.to_bits() as i32)?;
        self.meta
            .write_long(self.data.get_file_pointer() - self.base_data_pointer)?;
        if max_delta == 0 {
            self.meta.write_byte(0)?;
        } else {
            let bits_required = direct_writer_util::unsigned_bits_required(max_delta);
            let mut writer =
                DirectWriter::get_instance(self.data, self.buffer_size as i64, bits_required)?;
            for i in 0..(self.buffer_size as usize) {
                writer.add(self.buffer[i])?;
            }
            writer.finish()?;
            self.meta.write_byte(bits_required as u8)?;
        }
        self.buffer_size = 0;
        Ok(())
    }
    /// Write a new value. Note that data might not be stored until
    /// [`finish()`](DirectMonotonicWriter::finish) is called.
    ///
    /// # Errors
    /// - Returns an error if values are not provided in order.
    pub fn add(&mut self, v: i64) -> Result<()> {
        if v < self.previous {
            return Err(LuceneError::illegal_argument(format!(
                "Values do not come in order: {}, {}",
                self.previous, v
            )));
        }
        if self.buffer_size as usize == self.buffer.len() {
            self.flush()?;
        }
        self.buffer[self.buffer_size as usize] = v;
        self.buffer_size += 1;
        self.previous = v;
        self.count += 1;
        Ok(())
    }
    /// This must be called exactly once after all values have been added using
    /// [`add(i64)`](DirectMonotonicWriter::add).
    pub fn finish(&mut self) -> Result<()> {
        if self.count != self.num_values {
            return Err(LuceneError::illegal_state(format!(
                "Wrong number of values added, expected: {}, got: {}",
                self.num_values, self.count
            )));
        }
        if self.finished {
            return Err(LuceneError::illegal_state(String::from(
                "#finish has been called already",
            )));
        }
        if self.buffer_size > 0 {
            self.flush()?;
        }
        self.finished = true;
        Ok(())
    }
    /// Returns an instance suitable for encoding `num_values` into monotonic
    /// blocks of 2<sup>`block_shift`</sup> values. Metadata will be written
    /// to `meta_out` and actual data to `data_out`.
    pub fn get_instance(
        meta_out: &'a mut I1,
        data_out: &'a mut I2,
        num_values: i64,
        block_shift: i32,
    ) -> Result<Self> {
        Self::new(meta_out, data_out, num_values, block_shift)
    }
}
