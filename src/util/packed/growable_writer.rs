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
use crate::util::packed::{Mutable, MutablePacked64Enum, PackedInts, Reader};
use std::cmp::min;
use std::fmt::{Display, Formatter};
/// Implements [`Mutable`], but grows the bit count of the underlying packed ints
/// on-demand.
///
/// # Notes
/// - Beware that this struct will accept setting negative values. However, in order to do this,
///   it will grow the number of bits per value to 64.
///
/// # Internal
/// This is an internal API and may change in future versions.
pub struct GrowableWriter {
    current_mask: u64,
    current: MutablePacked64Enum,
    acceptable_overhead_ratio: f32,
}
impl GrowableWriter {
    pub fn new(
        start_bits_per_value: u32,
        value_count: u32,
        acceptable_overhead_ratio: f32,
    ) -> Result<GrowableWriter, DataIOError> {
        let current =
            PackedInts::get_mutable(value_count, start_bits_per_value, acceptable_overhead_ratio)?;
        let current_mask = Self::mask(current.get_bits_per_value());
        Ok(GrowableWriter {
            current_mask,
            current,
            acceptable_overhead_ratio,
        })
    }
    fn mask(bits_per_value: u32) -> u64 {
        if bits_per_value == 64 {
            !0u64
        } else {
            PackedInts::max_value(bits_per_value) as u64
        }
    }
    pub fn get_mutable(&self) -> &MutablePacked64Enum {
        &self.current
    }
    fn ensure_capacity(&mut self, value: i64) -> Result<(), DataIOError> {
        if (value & self.current_mask as i64) == value {
            return Ok(());
        }
        let bits_required = PackedInts::unsigned_bits_required(value);
        assert!(bits_required > self.current.get_bits_per_value());
        let value_count = self.size();
        let mut next =
            PackedInts::get_mutable(value_count, bits_required, self.acceptable_overhead_ratio)?;

        PackedInts::copy(
            &mut self.current,
            0,
            &mut next,
            0,
            value_count as usize,
            PackedInts::DEFAULT_BUFFER_SIZE,
        )?;

        self.current = next;
        self.current_mask = Self::mask(self.current.get_bits_per_value());
        Ok(())
    }
    pub fn resize(&mut self, new_size: u32) -> Result<GrowableWriter, DataIOError> {
        let mut next = GrowableWriter::new(
            self.current.get_bits_per_value(),
            new_size,
            self.acceptable_overhead_ratio,
        )?;
        let limit = min(self.size(), new_size);
        PackedInts::copy(
            &mut self.current,
            0,
            &mut next,
            0,
            limit as usize,
            PackedInts::DEFAULT_BUFFER_SIZE,
        )?;
        Ok(next)
    }
}

impl Reader for GrowableWriter {
    fn get(&mut self, index: usize) -> Result<i64, DataIOError> {
        self.current.get(index)
    }

    fn get_bulk(
        &mut self,
        index: usize,
        arr: &mut [i64],
        off: usize,
        len: usize,
    ) -> Result<u32, DataIOError> {
        self.current.get_bulk(index, arr, off, len)
    }

    fn size(&self) -> u32 {
        self.current.size()
    }
}

impl Accountable for GrowableWriter {
    fn ram_bytes_used(&self) -> u64 {
        todo!()
    }
}

impl Display for GrowableWriter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "GrowableWriter")
    }
}

impl Mutable for GrowableWriter {
    fn get_bits_per_value(&self) -> u32 {
        self.current.get_bits_per_value()
    }

    fn set(&mut self, index: usize, value: i64) -> Result<(), DataIOError> {
        self.ensure_capacity(value)?;
        self.current.set(index, value)?;
        Ok(())
    }

    fn set_bulk(
        &mut self,
        index: usize,
        arr: &[i64],
        off: usize,
        len: usize,
    ) -> Result<u32, DataIOError> {
        let mut max = 0i64;
        max |= arr
            .iter()
            .skip(off)
            .take(len)
            .fold(0, |acc, &value| acc | value);
        self.ensure_capacity(max)?;
        self.current.set_bulk(index, arr, off, len)
    }

    fn fill(&mut self, from_index: usize, to_index: usize, val: i64) -> Result<(), DataIOError> {
        self.ensure_capacity(val)?;
        self.current.fill(from_index, to_index, val)?;
        Ok(())
    }

    fn clear(&mut self) -> Result<(), DataIOError> {
        self.current.clear()
    }
}
