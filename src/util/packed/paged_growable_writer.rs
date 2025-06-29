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
use crate::util::packed::growable_writer::GrowableWriter;
use crate::util::packed::mutable_enum::MutableEnum;
/// A [`PagedGrowableWriter`]. This structure slices data into fixed-size blocks
/// which have independent numbers of bits per value and grow on-demand.
///
/// # Note
/// You should use this structure instead of the
/// [`PackedLongValues`](crate::util::packed::packed_long_values::PackedLongValues)
/// related ones only when you need random write-access. Otherwise, this
/// structure will likely be slower and less memory-efficient.
///
/// # Lucene Internal
/// This is an internal utility for use within the Lucene system.
#[derive(Default)]
pub(crate) struct PagedGrowableWriter {
    acceptable_overhead_ratio: f32,
    bits_per_value: i32,
    fill_page: bool,
}
impl PagedGrowableWriter {
    pub fn new(start_bits_per_value: i32, acceptable_overhead_ratio: f32, fill_page: bool) -> Self {
        PagedGrowableWriter {
            acceptable_overhead_ratio,
            bits_per_value: start_bits_per_value,
            fill_page,
        }
    }
    pub fn with_fill_page(start_bits_per_value: i32, acceptable_overhead_ratio: f32) -> Self {
        PagedGrowableWriter::new(start_bits_per_value, acceptable_overhead_ratio, true)
    }
}
impl AbstractPagedMutableBase for PagedGrowableWriter {
    fn new_mutable(&self, value_count: i32, bits_per_value: i32) -> MutableEnum {
        MutableEnum::GrowableW(GrowableWriter::new(
            bits_per_value,
            value_count,
            self.acceptable_overhead_ratio,
        ))
    }

    type PagedMutableBase = PagedGrowableWriter;
    fn new_unfilled_copy(
        &self,
        new_size: i64,
        page_size: i32,
    ) -> Result<AbstractPagedMutable<Self::PagedMutableBase>> {
        let sub_read =
            PagedGrowableWriter::new(self.bits_per_value, self.acceptable_overhead_ratio, false);
        AbstractPagedMutable::new(new_size, page_size, sub_read)
    }

    fn base_ram_bytes_used_base(&self) -> i64 {
        0
    }

    fn fill_pages(&self) -> bool {
        self.fill_page
    }

    fn bits_per_value(&self) -> i32 {
        self.bits_per_value
    }
}
