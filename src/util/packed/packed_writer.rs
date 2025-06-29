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
use crate::store::DataOutput;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::packed::bulk_operation::{bulk_operation_util, BulkOperation};
use crate::util::packed::bulk_operation_packed_enum::BulkOperationPackedEnum;
use crate::util::packed::format_behavior::FormatBehavior;
use crate::util::packed::{Encoder, Format, PackedInts, Writer};

pub(crate) struct PackedWriter<'a, T>
where
    T: DataOutput + 'a,
{
    finished: bool,
    format: Format,
    encoder: &'static BulkOperationPackedEnum,
    next_blocks: Vec<u8>,
    next_values: Vec<i64>,
    iterations: i32,
    off: i32,
    written: i32,
    value_count: i32,
    pub bits_per_value: i32,
    data_output: &'a mut T,
}
impl<'a, T> PackedWriter<'a, T>
where
    T: DataOutput,
{
    #[allow(unused)]
    pub fn new(
        format: Format,
        data_output: &'a mut T,
        value_count: i32,
        bits_per_value: i32,
        mem: i32,
    ) -> Self {
        let encoder = bulk_operation_util::of(format, bits_per_value);
        debug_assert!(value_count >= 0);
        let iterations = encoder.compute_iterations(value_count, mem);
        let next_blocks = vec![0; (iterations * Encoder::byte_block_count(encoder)) as usize];
        let next_values = vec![0; (iterations * Encoder::byte_value_count(encoder)) as usize];

        Self {
            finished: false,
            format,
            encoder,
            next_blocks,
            next_values,
            iterations,
            off: 0,
            written: 0,
            value_count,
            bits_per_value,
            data_output,
        }
    }
    fn flush(&mut self) -> Result<()> {
        self.encoder.encode_i64_to_u8(
            &self.next_values,
            0,
            &mut self.next_blocks,
            0,
            self.iterations,
        );
        let block_count =
            self.format
                .byte_count(PackedInts::VERSION_CURRENT, self.off, self.bits_per_value);

        debug_assert!(block_count <= i32::MAX as i64);
        self.data_output.write_bytes_with_len(
            &self.next_blocks[0..block_count as usize],
            block_count as i32,
        )?;
        self.next_values.fill(0);
        self.off = 0;
        Ok(())
    }
}
impl<T> Writer for PackedWriter<'_, T>
where
    T: DataOutput,
{
    fn get_format(&self) -> &Format {
        &self.format
    }

    fn add(&mut self, v: i64) -> Result<()> {
        debug_assert!(
            PackedInts::unsigned_bits_required(v) <= self.bits_per_value,
            "Value exceeds allowed bits per value"
        );
        debug_assert!(!self.finished, "Cannot add values after finishing writing");
        if self.value_count != -1 && self.written >= self.value_count {
            return Err(LuceneError::eof("Writing past end of stream".to_string()));
        }
        self.next_values[self.off as usize] = v;
        self.off += 1;
        if self.off as usize == self.next_values.len() {
            self.flush()?;
        }
        self.written += 1;
        Ok(())
    }

    fn bits_per_values(&self) -> i32 {
        self.bits_per_value
    }

    fn finish(&mut self) -> Result<()> {
        debug_assert!(!self.finished, "Already finished");
        if self.value_count != -1 {
            while self.written < self.value_count {
                self.add(0)?;
            }
        }
        self.flush()?;
        self.finished = true;
        Ok(())
    }

    fn ord(&self) -> i32 {
        self.written - 1
    }
}
