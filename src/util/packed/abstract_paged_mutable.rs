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
use crate::util::error::lucene_error::LuceneError;
use crate::util::long_values::LongValues;
use crate::util::packed::mutable_enum::MutableEnum;
use crate::util::packed::{DummyMutable, Mutable, PackedInts, Reader};
use std::cmp::min;
use std::fmt::Display;

const MIN_BLOCK_SIZE: u32 = 1 << 6;
const MAX_BLOCK_SIZE: u32 = 1 << 30;
/// Base implementation for [`PagedMutable`](crate::util::packed::paged_mutable::PagedMutable) and [`PagedGrowableWriter`](crate::util::packed::paged_growable_writer::PagedGrowableWriter).
///
/// # Lucene Internal
/// This is an internal utility for use within the Lucene system.
#[derive(Default)]
pub struct AbstractPagedMutable<T>
where
    T: AbstractPagedMutableBase,
{
    sub_reader: T,
    size: u64,
    page_shift: u32,
    page_mask: u32,
    sub_mutables: Vec<MutableEnum>,
    bits_per_value: u32,
}

impl<T> AbstractPagedMutable<T>
where
    T: AbstractPagedMutableBase<PagedMutableBase = T>,
{
    pub fn new(
        bits_per_value: u32,
        size: u64,
        page_size: u32,
        sub_reader: T,
    ) -> Result<AbstractPagedMutable<T>, LuceneError> {
        let page_shift = PackedInts::check_block_size(page_size, MIN_BLOCK_SIZE, MAX_BLOCK_SIZE)?;
        let page_mask = page_size - 1;
        let num_pages = PackedInts::num_blocks(size, page_size)?;
        let mut sub_mutables = Vec::with_capacity(num_pages as usize);
        // We use index-based access to sub_mutables, so we can initialize it as DummyMutable.
        for _ in 0..num_pages as usize {
            sub_mutables.push(MutableEnum::Dummy(DummyMutable));
        }
        let mut result = AbstractPagedMutable {
            sub_reader,
            size,
            page_shift,
            page_mask,
            sub_mutables,
            bits_per_value,
        };
        if result.sub_reader.fill_pages() {
            result.fill_pages()?;
        };
        Ok(result)
    }
    pub fn fill_pages(&mut self) -> Result<(), LuceneError> {
        let num_pages = PackedInts::num_blocks(self.size, self.page_size())?;
        for i in 0..num_pages {
            // do not allocate for more entries than necessary on the last page
            let value_count = if i == num_pages - 1 {
                self.last_page_size(self.size)
            } else {
                self.page_size()
            };
            self.sub_mutables[i as usize] = self
                .sub_reader
                .new_mutable(value_count, self.bits_per_value)?;
        }
        Ok(())
    }
    fn last_page_size(&self, size: u64) -> u32 {
        let sz = self.index_in_page(size);
        if sz == 0 {
            self.page_size()
        } else {
            sz
        }
    }
    fn page_size(&self) -> u32 {
        self.page_mask + 1
    }
    pub fn size(&self) -> u64 {
        self.size
    }
    fn page_index(&self, index: u64) -> usize {
        (index >> self.page_shift) as usize
    }

    fn index_in_page(&self, index: u64) -> u32 {
        (index & self.page_mask as u64) as u32
    }
    /// Sets the value at the specified index.
    pub fn set(&mut self, index: u64, value: i64) -> Result<(), LuceneError> {
        debug_assert!(
            index < self.size,
            "Index out of bounds: index={} size={}",
            index,
            self.size
        );
        let page_index = self.page_index(index);
        let index_in_page = self.index_in_page(index);
        self.sub_mutables[page_index].set(index_in_page as usize, value)
    }
    pub(crate) fn base_ram_bytes_used(&self) -> u64 {
        self.sub_reader.base_ram_bytes_used_base()
    }
    /// Create a new copy of size <code>newSize</code> based on the content of this buffer. This
    /// is much more efficient than creating a new instance and copying values one by one.
    pub fn resize(&mut self, new_size: u64) -> Result<AbstractPagedMutable<T>, LuceneError> {
        let mut copy = self
            .sub_reader
            .new_unfilled_copy(new_size, self.page_size())?;
        let num_common_pages = min(copy.sub_mutables.len(), self.sub_mutables.len());
        let mut copy_buffer = vec![0i64; 1024];
        for i in 0..copy.sub_mutables.len() {
            // Determine the number of values in the current page
            let value_count = if i == copy.sub_mutables.len() - 1 {
                self.last_page_size(new_size)
            } else {
                self.page_size()
            };
            let bpv = if i < num_common_pages {
                self.sub_mutables[i].get_bits_per_value()
            } else {
                self.bits_per_value
            };
            copy.sub_mutables[i] = self.sub_reader.new_mutable(value_count, bpv)?;

            if i < num_common_pages {
                let copy_length = min(value_count, self.sub_mutables[i].size());
                PackedInts::copy_with_buffer(
                    &mut self.sub_mutables[i],
                    0,
                    &mut copy.sub_mutables[i],
                    0,
                    copy_length as usize,
                    &mut copy_buffer,
                )?;
            }
        }
        Ok(copy)
    }
    pub fn grow_with_size(
        &mut self,
        min_size: u64,
    ) -> Result<Option<AbstractPagedMutable<T>>, LuceneError> {
        if min_size <= self.size {
            return Ok(None);
        }
        let mut extra = min_size >> 3;
        if extra < 3 {
            extra = 3;
        }
        let new_size = min_size + extra;
        Ok(Some(self.resize(new_size)?))
    }
    pub fn grow(&mut self) -> Result<Option<AbstractPagedMutable<T>>, LuceneError> {
        self.grow_with_size(self.size() << 1)
    }
}
impl<T> LongValues for AbstractPagedMutable<T>
where
    T: AbstractPagedMutableBase<PagedMutableBase = T>,
{
    fn get(&mut self, index: u64) -> Result<i64, LuceneError> {
        debug_assert!(index < self.size, "index={} size={}", index, self.size);
        let page_index = self.page_index(index);
        let index_in_page = self.index_in_page(index);
        self.sub_mutables[page_index].get(index_in_page as usize)
    }
}
impl<T> Accountable for AbstractPagedMutable<T>
where
    T: AbstractPagedMutableBase<PagedMutableBase = T>,
{
    fn ram_bytes_used(&self) -> u64 {
        let mut byte_used = self.base_ram_bytes_used();
        for sub_mutable in &self.sub_mutables {
            byte_used += sub_mutable.ram_bytes_used();
        }
        byte_used
    }
}
impl<T> Display for AbstractPagedMutable<T>
where
    T: AbstractPagedMutableBase<PagedMutableBase = T> + Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}(size={}, pageSize={})",
            self.sub_reader,
            self.size,
            self.page_size()
        )
    }
}
pub trait AbstractPagedMutableBase: Default {
    fn new_mutable(
        &self,
        value_count: u32,
        bits_per_value: u32,
    ) -> Result<MutableEnum, LuceneError>;
    type PagedMutableBase: AbstractPagedMutableBase;
    fn new_unfilled_copy(
        &self,
        new_size: u64,
        page_size: u32,
    ) -> Result<AbstractPagedMutable<Self::PagedMutableBase>, LuceneError>;
    fn base_ram_bytes_used_base(&self) -> u64;
    fn fill_pages(&self) -> bool;
}
