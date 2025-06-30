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
use std::fmt::{Display, Formatter};

use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::Result;
use crate::util::packed::mutable_packed64_enum::MutablePacked64Enum;
use crate::util::packed::{Mutable, PackedInts, Reader};

/// Implements [`Mutable`], but grows the bit count of the underlying packed
/// ints on-demand.
///
/// # Note
/// - Beware that this struct will accept setting negative values. However, in
///   order to do this, it will grow the number of bits per value to 64.
///
/// # Internal
/// This is an internal API and may change in future versions.
pub(crate) struct GrowableWriter {
    current_mask: i64,
    current: MutablePacked64Enum,
    acceptable_overhead_ratio: f32,
}
impl GrowableWriter {
    pub fn new(
        start_bits_per_value: i32,
        value_count: i32,
        acceptable_overhead_ratio: f32,
    ) -> GrowableWriter {
        let current =
            PackedInts::get_mutable(value_count, start_bits_per_value, acceptable_overhead_ratio);
        let current_mask = Self::mask(current.get_bits_per_value());
        GrowableWriter {
            current_mask,
            current,
            acceptable_overhead_ratio,
        }
    }
    fn mask(bits_per_value: i32) -> i64 {
        if bits_per_value == 64 {
            !0i64
        } else {
            PackedInts::max_value(bits_per_value)
        }
    }
    #[allow(unused)]
    pub fn get_mutable(&self) -> &MutablePacked64Enum {
        &self.current
    }
    fn ensure_capacity(&mut self, value: i64) {
        if (value & self.current_mask) == value {
            return;
        }
        let bits_required = PackedInts::unsigned_bits_required(value);
        debug_assert!(bits_required > self.current.get_bits_per_value());
        let value_count = self.size();
        let mut next =
            PackedInts::get_mutable(value_count, bits_required, self.acceptable_overhead_ratio);

        PackedInts::copy(
            &mut self.current,
            0,
            &mut next,
            0,
            value_count,
            PackedInts::DEFAULT_BUFFER_SIZE,
        );

        self.current = next;
        self.current_mask = Self::mask(self.current.get_bits_per_value());
    }
    #[allow(unused)]
    pub fn resize(&mut self, new_size: i32) -> GrowableWriter {
        let mut next = GrowableWriter::new(
            self.current.get_bits_per_value(),
            new_size,
            self.acceptable_overhead_ratio,
        );
        let limit = std::cmp::min(self.size(), new_size);
        PackedInts::copy(
            &mut self.current,
            0,
            &mut next,
            0,
            limit,
            PackedInts::DEFAULT_BUFFER_SIZE,
        );
        next
    }
}

impl Reader for GrowableWriter {
    fn get(&self, index: i32) -> i64 {
        self.current.get(index)
    }

    fn get_bulk(&self, index: i32, arr: &mut [i64], off: i32, len: i32) -> i32 {
        self.current.get_bulk(index, arr, off, len)
    }

    fn size(&self) -> i32 {
        self.current.size()
    }
}

impl Accountable for GrowableWriter {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

impl Display for GrowableWriter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "GrowableWriter")
    }
}

impl Mutable for GrowableWriter {
    fn get_bits_per_value(&self) -> i32 {
        self.current.get_bits_per_value()
    }

    fn set(&mut self, index: i32, value: i64) {
        self.ensure_capacity(value);
        self.current.set(index, value);
    }

    fn set_bulk(&mut self, index: i32, arr: &[i64], off: i32, len: i32) -> i32 {
        let mut max = 0i64;
        max |= arr
            .iter()
            .skip(off as usize)
            .take(len as usize)
            .fold(0, |acc, &value| acc | value);
        self.ensure_capacity(max);
        self.current.set_bulk(index, arr, off, len)
    }

    fn fill(&mut self, from_index: i32, to_index: i32, val: i64) {
        self.ensure_capacity(val);
        self.current.fill(from_index, to_index, val)
    }

    fn clear(&mut self) {
        self.current.clear()
    }
}
