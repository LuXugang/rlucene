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
use crate::util::error::lucene_error::Result;
use crate::util::packed::abstract_paged_mutable::{AbstractPagedMutable, AbstractPagedMutableBase};
use crate::util::packed::mutable_enum::MutableEnum;
use crate::util::packed::{fastest_format_and_bits, Format, FormatAndBits, PackedInts};
/// A `PagedMutable`. This structure slices data into fixed-size blocks which
/// have the same number of bits per value. It can be a useful replacement for
/// `PackedIntsMutable` to store more than 2 billion values.
///
/// # Lucene Internal
/// This is an internal utility for use within the Lucene system.
#[derive(Default)]
pub struct PagedMutable {
    format: Format,
    bits_per_value: i32,
}
impl PagedMutable {
    pub fn with_overhead_ratio(
        page_size: i32,
        bits_per_value: i32,
        acceptable_overhead_ratio: f32,
    ) -> Self {
        let format_and_bits =
            fastest_format_and_bits(page_size, bits_per_value, acceptable_overhead_ratio);
        Self::with_format_and_bits(format_and_bits)
    }
    fn with_format_and_bits(format_and_bits: FormatAndBits) -> Self {
        Self::with_bits_and_format(format_and_bits.bits_per_value, format_and_bits.format)
    }
    fn with_bits_and_format(bits_per_value: i32, format: Format) -> Self {
        Self {
            format,
            bits_per_value,
        }
    }
}
impl AbstractPagedMutableBase for PagedMutable {
    fn new_mutable(&self, value_count: i32, bits_per_value: i32) -> MutableEnum {
        debug_assert!(self.bits_per_value >= bits_per_value);
        let sub_mutable =
            PackedInts::get_mutable_impl(value_count, self.bits_per_value, self.format);
        MutableEnum::Packed(sub_mutable)
    }

    type PagedMutableBase = PagedMutable;

    fn new_unfilled_copy(
        &self,
        new_size: i64,
        page_size: i32,
    ) -> Result<AbstractPagedMutable<Self::PagedMutableBase>> {
        let sub_reader = PagedMutable::with_bits_and_format(self.bits_per_value, self.format);
        AbstractPagedMutable::new(new_size, page_size, sub_reader)
    }

    fn base_ram_bytes_used_base(&self) -> i64 {
        0
    }

    fn fill_pages(&self) -> bool {
        true
    }

    fn bits_per_value(&self) -> i32 {
        self.bits_per_value
    }
}
