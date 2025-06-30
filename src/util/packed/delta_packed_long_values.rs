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
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::Result;
use crate::util::packed::monotonic_long_values::{MonotonicLongValues, MonotonicLongValuesBuilder};
use crate::util::packed::packed_long_values::INITIAL_PAGE_COUNT;

pub(crate) struct DeltaPackedLongValues {
    pub(crate) sub_long_value: Option<MonotonicLongValues>,
    pub(crate) mins: Vec<i64>,
}

impl DeltaPackedLongValues {
    #[allow(dead_code)]
    const BASE_RAM_BYTES_USED: u64 = 0;
    pub(crate) fn new(mins: Vec<i64>, sub_reader: Option<MonotonicLongValues>) -> Self {
        Self {
            sub_long_value: sub_reader,
            mins,
        }
    }
    pub(crate) fn decode_block(&self, block: i32, dest: &mut [i64], count: i32) -> i32 {
        let min = self.mins[block as usize];
        for item in dest.iter_mut().take(count as usize) {
            *item += min;
        }
        match self.sub_long_value {
            Some(ref sub) => sub.decode_block(block, dest, count),
            _ => count,
        }
    }

    pub(crate) fn get_value(&self, block: i32, element: i32, _value: u64) -> i64 {
        let current = self.mins[block as usize];
        match self.sub_long_value {
            Some(ref reader) => reader.get_value(block, element, current as u64),
            None => current,
        }
    }
}

pub struct DeltaPackedLongValuesBuilder {
    pub(crate) sub_builder: Option<MonotonicLongValuesBuilder>,
    pub(crate) mins: Vec<i64>,
}
impl Default for DeltaPackedLongValuesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DeltaPackedLongValuesBuilder {
    // TODO
    #[allow(dead_code)]
    const BASE_RAM_BYTES_USED: u64 = 0;
    pub(crate) fn new() -> DeltaPackedLongValuesBuilder {
        Self::with_sub_builder(None)
    }
    pub(crate) fn with_sub_builder(
        sub_builder: Option<MonotonicLongValuesBuilder>,
    ) -> DeltaPackedLongValuesBuilder {
        Self {
            sub_builder,
            mins: vec![0; INITIAL_PAGE_COUNT as usize],
        }
    }

    pub(crate) fn build(mut self, values_off: i32) -> Result<DeltaPackedLongValues> {
        let sub_reader = if self.sub_builder.is_some() {
            Some(self.sub_builder.take().unwrap().build(values_off)?)
        } else {
            None
        };
        let _ = self.mins.split_off(values_off as usize);
        // TODO:
        let _ram_bytes_used = 0;
        Ok(DeltaPackedLongValues::new(
            std::mem::take(&mut self.mins),
            sub_reader,
        ))
    }
    pub(crate) fn pack(&mut self, values: &mut [i64], num_values: i32, block: i32) {
        if self.sub_builder.is_some() {
            self.sub_builder
                .as_mut()
                .unwrap()
                .pack(values, num_values, block);
        }
        let mut min = values[0];
        for &value in values.iter().take(num_values as usize).skip(1) {
            min = min.min(value);
        }
        for value in values.iter_mut().take(num_values as usize) {
            *value -= min;
        }
        self.mins[block as usize] = min;
    }

    pub(crate) fn grow(&mut self, new_block_count: i32) -> Result<()> {
        if let Some(ref mut builder) = self.sub_builder {
            builder.grow(new_block_count)?
        }
        ArrayUtil::grow_exact(&mut self.mins, new_block_count as usize)?;
        // TODO: memory calculation not implemented
        Ok(())
    }
    #[allow(dead_code)]
    fn base_ram_bytes_used(&self) -> u64 {
        todo!()
    }
}
