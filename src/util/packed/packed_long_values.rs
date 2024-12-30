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
use crate::util::accountable::Accountable;
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::long_values::LongValues;
use crate::util::packed::delta_packed_long_values::{
    DeltaPackedLongValues, DeltaPackedLongValuesBuilder,
};
use crate::util::packed::monotonic_long_values::MonotonicLongValuesBuilder;
use crate::util::packed::read_enum::PackedIntsReadEnum;
use crate::util::packed::{Mutable, NullReader, PackedInts, Reader};

/// Utility class to compress integers into a [`LongValues`] instance.
pub struct PackedLongValues {
    page_shift: u32,
    page_mask: u32,
    pub(crate) values: Vec<PackedIntsReadEnum>,
    size: u64,
    ram_bytes_used: u64,
}

impl PackedLongValues {
    pub const DEFAULT_PAGE_SIZE: u32 = 256;
    const MIN_PAGE_SIZE: u32 = 64;
    // More than 1M doesn't really makes sense with these appending buffers
    // since their goal is to try to have small numbers of bits per value
    const MAX_PAGE_SIZE: u32 = 1 << 20;
    /// Return a new [`PackedLongValuesBuilder`](PackedLongValuesBuilder) that will compress efficiently positive integers.
    pub fn packed_long_values_builder(
        page_size: u32,
        acceptable_overhead_ratio: f32,
    ) -> Result<PackedLongValuesBuilder, DataIOError> {
        PackedLongValuesBuilder::new(page_size, acceptable_overhead_ratio)
    }
    /// See [`PackedLongValuesBuilder`](PackedLongValuesBuilder).
    pub fn packed_long_values_builder_default(
        acceptable_overhead_ratio: f32,
    ) -> Result<PackedLongValuesBuilder, DataIOError> {
        Self::packed_long_values_builder(
            PackedLongValues::DEFAULT_PAGE_SIZE,
            acceptable_overhead_ratio,
        )
    }

    /// Return a new [`DeltaPackedLongValuesBuilder`](DeltaPackedLongValuesBuilder) that will compress efficiently integers that are close to each other.
    pub fn delta_packed_long_values_builder(
        page_size: u32,
        acceptable_overhead_ratio: f32,
    ) -> Result<DeltaPackedLongValuesBuilder, DataIOError> {
        DeltaPackedLongValuesBuilder::new(page_size, acceptable_overhead_ratio)
    }

    /// See [`delta_packed_long_values_builder`].
    pub fn delta_packed_long_values_builder_default(
        acceptable_overhead_ratio: f32,
    ) -> Result<DeltaPackedLongValuesBuilder, DataIOError> {
        Self::delta_packed_long_values_builder(
            PackedLongValues::DEFAULT_PAGE_SIZE,
            acceptable_overhead_ratio,
        )
    }

    /// Return a new [`MonotonicLongValuesBuilder`](MonotonicLongValuesBuilder) that will compress efficiently integers that would be a monotonic function of their index.
    pub fn monotonic_long_values_builder(
        page_size: u32,
        acceptable_overhead_ratio: f32,
    ) -> Result<MonotonicLongValuesBuilder, DataIOError> {
        MonotonicLongValuesBuilder::new(page_size, acceptable_overhead_ratio)
    }

    /// See [`monotonic_long_values_builder`].
    pub fn monotonic_long_values_builder_default(
        acceptable_overhead_ratio: f32,
    ) -> Result<MonotonicLongValuesBuilder, DataIOError> {
        PackedLongValues::monotonic_long_values_builder(
            PackedLongValues::DEFAULT_PAGE_SIZE,
            acceptable_overhead_ratio,
        )
    }
    pub(crate) fn new(
        page_shift: u32,
        page_mask: u32,
        values: Vec<PackedIntsReadEnum>,
        size: u64,
        ram_bytes_used: u64,
    ) -> Self {
        Self {
            page_shift,
            page_mask,
            values,
            size,
            ram_bytes_used,
        }
    }
    pub fn size(&self) -> u64 {
        self.size
    }
}
impl Accountable for PackedLongValues {
    fn ram_bytes_used(&self) -> u64 {
        //TODO
        self.ram_bytes_used
    }
}
impl LongValues for PackedLongValues {
    fn get(&mut self, index: u64) -> Result<i64, DataIOError> {
        debug_assert!(index < self.size());
        let block = (index >> self.page_shift) as usize;
        let element = (index & self.page_mask as u64) as usize;

        self.get_value(block, element)
    }
}
impl PackedLongValuesBase1 for PackedLongValues {
    fn decode_block(&mut self, block: usize, dest: &mut [i64]) -> Result<u32, DataIOError> {
        let vals = &mut self.values[block];
        let size = vals.size();
        let mut k = 0;
        while k < size {
            k += vals.get_bulk(k as usize, dest, k as usize, (size - k) as usize)?;
        }
        Ok(size)
    }

    fn get_value(&mut self, block: usize, element: usize) -> Result<i64, DataIOError> {
        self.values[block].get(element)
    }
}

/// A Builder for a {@link PackedLongValues} instance.
pub struct PackedLongValuesBuilder {
    pub(crate) page_shift: u32,
    pub(crate) page_mask: u32,
    acceptable_overhead_ratio: f32,
    pending: Option<Vec<i64>>,
    pub(crate) size: u64,
    pub(crate) values: Vec<PackedIntsReadEnum>,
    pub(crate) ram_bytes_used: u64,
    pub(crate) values_off: usize,
    pending_off: u32,
}

/// A Builder for a [`PackedLongValues`] instance.
impl PackedLongValuesBuilder {
    const INITIAL_PAGE_COUNT: usize = 16;
    // TODO
    const BASE_RAM_BYTES_USED: u64 = 0;
    pub fn new(
        page_size: u32,
        acceptable_overhead_ratio: f32,
    ) -> Result<PackedLongValuesBuilder, DataIOError> {
        let page_shift = PackedInts::check_block_size(
            page_size,
            PackedLongValues::MIN_PAGE_SIZE,
            PackedLongValues::MAX_PAGE_SIZE,
        )?;
        let page_mask = page_size - 1;
        let pending = Some(vec![0; page_size as usize]);
        let values = Vec::with_capacity(Self::INITIAL_PAGE_COUNT);
        Ok(Self {
            page_shift,
            page_mask,
            acceptable_overhead_ratio,
            pending,
            size: 0,
            values,
            ram_bytes_used: 0, // TODO
            values_off: 0,
            pending_off: 0,
        })
    }
    /**
     * Build a [`PackedLongValues`] instance that contains values that have been added to this
     * builder. This operation is destructive.
     */
    pub fn build(mut self) -> Result<PackedLongValues, DataIOError> {
        self.finish()?;
        // TODO
        let ram_bytes_used = 0;
        let mut values = std::mem::take(&mut self.values);
        let _ = values.split_off(self.values_off as usize);
        Ok(PackedLongValues::new(
            self.page_shift,
            self.page_mask,
            values,
            self.size,
            ram_bytes_used,
        ))
    }

    /**
     * Add a new element to this builder.
     */
    pub fn add(&mut self, l: i64) -> Result<&mut Self, DataIOError> {
        if self.pending.is_none() {
            return Err(DataIOError::illegal_state("Cannot be reused after build()"));
        }

        if self.pending_off as usize == self.pending.as_ref().unwrap().len() {
            let current_value_len = self.values.len();
            if current_value_len == self.values_off as usize {
                // Not consistent with the Java version implementation, we increase by half of the current length
                let new_length = current_value_len + current_value_len / 2;
                debug_assert!(new_length <= u32::MAX as usize);
                self.grow(new_length as u32);
            }
            self.pack_impl()?;
        }

        self.pending.as_mut().unwrap()[self.pending_off as usize] = l;
        self.pending_off += 1;
        self.size += 1;
        Ok(self)
    }
    pub(crate) fn finish(&mut self) -> Result<(), DataIOError> {
        if self.pending_off > 0 {
            if self.values.len() == self.values_off {
                debug_assert!(self.values_off <= u32::MAX as usize);
                self.grow(self.values_off as u32);
            }
            self.pack_impl()?;
        }
        Ok(())
    }
    fn pack_impl(&mut self) -> Result<(), DataIOError> {
        let mut pending = self.pending.take().unwrap();
        self.pack(
            &mut pending,
            self.pending_off,
            self.values_off as usize,
            self.acceptable_overhead_ratio,
        )?;
        // TODO
        self.ram_bytes_used = 0;
        self.values_off += 1;
        // Reset pending buffer
        self.pending_off = 0;
        Ok(())
    }
}

impl PackedLongValuesBase2 for PackedLongValuesBuilder {
    fn base_ram_bytes_used(&self) -> u64 {
        // TODO
        PackedLongValuesBuilder::BASE_RAM_BYTES_USED
    }

    fn pack(
        &mut self,
        values: &mut [i64],
        num_values: u32,
        block: usize,
        acceptable_overhead_ratio: f32,
    ) -> Result<(), DataIOError> {
        let mut min_value = values[0];
        let mut max_value = values[0];

        for &value in values.iter().take(num_values as usize).skip(1) {
            min_value = min_value.min(value);
            max_value = max_value.max(value);
        }

        // Build a new packed reader
        if min_value == 0 && max_value == 0 {
            let reader = NullReader::new(num_values);
            self.values[block] = PackedIntsReadEnum::NullReader(reader);
            Ok(())
        } else {
            let bits_required = if min_value < 0 {
                64
            } else {
                PackedInts::bits_required(max_value)?
            };

            let mut mutable =
                PackedInts::get_mutable(num_values, bits_required, acceptable_overhead_ratio)?;
            let mut i = 0;
            while i < num_values {
                i += mutable.set_bulk(i as usize, values, i as usize, (num_values - i) as usize)?;
            }

            self.values[block] = PackedIntsReadEnum::PackedReader(mutable);
            Ok(())
        }
    }

    fn grow(&mut self, new_block_count: u32) {
        // TODO
        self.ram_bytes_used = 0;
        let current_len = self.values.len();
        if new_block_count <= current_len as u32 {
            return;
        }
        for _i in 0..(new_block_count as usize - current_len) {
            // PackedIntsReadEnum::NullReader as padding value
            self.values
                .push(PackedIntsReadEnum::NullReader(NullReader::new(0)));
        }
    }
}

pub trait PackedLongValuesBase1 {
    fn decode_block(&mut self, block: usize, dest: &mut [i64]) -> Result<u32, DataIOError>;
    fn get_value(&mut self, block: usize, element: usize) -> Result<i64, DataIOError>;
}

pub trait PackedLongValuesBase2 {
    fn base_ram_bytes_used(&self) -> u64;
    fn pack(
        &mut self,
        values: &mut [i64],
        num_values: u32,
        block: usize,
        acceptable_overhead_ratio: f32,
    ) -> Result<(), DataIOError>;
    fn grow(&mut self, new_block_count: u32);
}
