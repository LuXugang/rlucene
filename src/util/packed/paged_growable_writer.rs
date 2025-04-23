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
    fn new_mutable(&self, value_count: i32, bits_per_value: i32) -> Result<MutableEnum> {
        Ok(MutableEnum::GrowableW(GrowableWriter::new(
            bits_per_value,
            value_count,
            self.acceptable_overhead_ratio,
        )?))
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
