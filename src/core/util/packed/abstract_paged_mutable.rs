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
use std::fmt::Display;

use crate::core::util::TryIntoInt;
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::long_values::LongValues;
use crate::core::util::packed::mutable_enum::MutableEnum;
use crate::core::util::packed::paged_growable_writer::PagedGrowableWriter;
use crate::core::util::packed::paged_mutable::PagedMutable;
use crate::core::util::packed::{Mutable, PackedInts, Reader};
use crate::core::util::ram_usage_estimator::size_of_vec;

const MIN_BLOCK_SIZE: i32 = 1 << 6;
const MAX_BLOCK_SIZE: i32 = 1 << 30;
/// Base implementation for
/// [`PagedMutable`] and [`PagedGrowableWriter`].
///
///
/// # Lucene Internal
/// This is an internal utility for use within the Lucene system.
pub struct AbstractPagedMutable<T> {
  sub_reader: T,
  size: usize,
  page_shift: i32,
  page_mask: i32,
  pub(crate) sub_mutables: Vec<MutableEnum>,
}

#[allow(private_bounds)] // Models Java's protected AbstractPagedMutable subclass hooks without exposing Rust's internal enum dispatch type.
impl<T> AbstractPagedMutable<T>
where
  T: AbstractPagedMutableBase,
{
  pub fn new(size: usize, page_size: i32, sub_reader: T) -> Result<AbstractPagedMutable<T>> {
    let page_shift = PackedInts::check_block_size(page_size, MIN_BLOCK_SIZE, MAX_BLOCK_SIZE)?;
    let page_mask = page_size - 1;
    let num_pages = PackedInts::num_blocks(size, page_size)?;
    let sub_mutables = Vec::with_capacity(num_pages as usize);
    let mut result = AbstractPagedMutable {
      sub_reader,
      size,
      page_shift,
      page_mask,
      sub_mutables,
    };
    if result.sub_reader.fill_pages() {
      result.fill_pages()?;
    };
    Ok(result)
  }
  fn fill_pages(&mut self) -> Result<()> {
    let num_pages = PackedInts::num_blocks(self.size, self.page_size())?;
    let mut sub_mutables = Vec::with_capacity(num_pages as usize);
    for i in 0..num_pages {
      // do not allocate for more entries than necessary on the last page
      let value_count = if i == num_pages - 1 {
        self.last_page_size(self.size)
      } else {
        self.page_size()
      };
      sub_mutables.push(
        self
          .sub_reader
          .new_mutable(value_count, self.sub_reader.bits_per_value())?,
      );
    }
    self.sub_mutables = sub_mutables;
    Ok(())
  }
  fn last_page_size(&self, size: usize) -> i32 {
    let sz = self.index_in_page(size);
    if sz == 0 { self.page_size() } else { sz }
  }
  fn page_size(&self) -> i32 {
    self.page_mask + 1
  }
  pub fn size(&self) -> usize {
    self.size
  }

  pub(crate) fn take(&mut self) -> Self {
    let replacement = Self {
      sub_reader: self.sub_reader.new_unfilled_copy(),
      size: 0,
      page_shift: self.page_shift,
      page_mask: self.page_mask,
      sub_mutables: Vec::new(),
    };
    std::mem::replace(self, replacement)
  }

  fn page_index(&self, index: usize) -> usize {
    index >> self.page_shift
  }

  fn index_in_page(&self, index: usize) -> i32 {
    (index & self.page_mask as usize) as i32
  }
  /// Sets the value at the specified index.
  pub fn set(&mut self, index: usize, value: i64) -> Result<()> {
    debug_assert!(
      index < self.size,
      "Index out of bounds: index={} size={}",
      index,
      self.size
    );
    let page_index = self.page_index(index);
    let index_in_page = self.index_in_page(index);
    let sub_mutable = self.sub_mutables.get_mut(page_index).ok_or_else(|| {
      LuceneError::array_index_out_of_bounds(format!("page index out of bounds: {page_index}"))
    })?;
    sub_mutable.set(index_in_page, value)
  }
  pub(crate) fn base_ram_bytes_used(&self) -> i64 {
    self.sub_reader.base_ram_bytes_used_base()
  }
  /// Create a new copy of size `new_size` based on the content of
  /// this buffer. This is much more efficient than creating a new
  /// instance and copying values one by one.
  pub fn resize(&self, new_size: usize) -> Result<AbstractPagedMutable<T>> {
    let sub = self.sub_reader.new_unfilled_copy();
    let mut copy = AbstractPagedMutable::new(new_size, self.page_size(), sub)?;
    let num_pages = PackedInts::num_blocks(new_size, self.page_size())? as usize;
    let num_common_pages = std::cmp::min(num_pages, self.sub_mutables.len());
    let mut copy_buffer = vec![0i64; 1024];
    for i in 0..num_pages {
      // Determine the number of values in the current page
      let value_count = if i == num_pages - 1 {
        self.last_page_size(new_size)
      } else {
        self.page_size()
      };
      let bpv = if i < num_common_pages {
        self.sub_mutables[i].get_bits_per_value()
      } else {
        self.sub_reader.bits_per_value()
      };
      let mut sub_mutable = self.sub_reader.new_mutable(value_count, bpv)?;

      if i < num_common_pages {
        let copy_length = std::cmp::min(value_count, self.sub_mutables[i].size());
        PackedInts::copy_with_buffer(
          &self.sub_mutables[i],
          0,
          &mut sub_mutable,
          0,
          copy_length,
          &mut copy_buffer,
        )?;
      }
      copy.sub_mutables.push(sub_mutable);
    }
    Ok(copy)
  }
  pub fn grow_with_size(&self, min_size: usize) -> Result<Option<AbstractPagedMutable<T>>> {
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

  pub fn grow(&self) -> Result<Option<AbstractPagedMutable<T>>> {
    self.grow_with_size(self.size() + 1)
  }
}
#[allow(private_bounds)] // Models Java's protected AbstractPagedMutable subclass hooks without exposing Rust's internal enum dispatch type.
impl<T> LongValues for AbstractPagedMutable<T>
where
  T: AbstractPagedMutableBase,
{
  fn get(&self, index: usize) -> Result<i64> {
    debug_assert!(index < self.size, "index={} size={}", index, self.size);
    let page_index = self.page_index(index);
    let index_in_page = self.index_in_page(index);
    let sub_mutable = self.sub_mutables.get(page_index).ok_or_else(|| {
      LuceneError::array_index_out_of_bounds(format!("page index out of bounds: {page_index}"))
    })?;
    Ok(sub_mutable.get(index_in_page.try_convert()?))
  }
}
#[allow(private_bounds)] // Models Java's protected AbstractPagedMutable subclass hooks without exposing Rust's internal enum dispatch type.
impl<T> Accountable for AbstractPagedMutable<T>
where
  T: AbstractPagedMutableBase,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    let mut byte_used = self
      .base_ram_bytes_used()
      .saturating_add(size_of_vec(&self.sub_mutables));
    for sub_mutable in &self.sub_mutables {
      byte_used = byte_used.saturating_add(sub_mutable.ram_bytes_used()?);
    }
    Ok(byte_used)
  }
}
#[allow(private_bounds)] // Models Java's protected AbstractPagedMutable subclass hooks without exposing Rust's internal enum dispatch type.
impl<T> Display for AbstractPagedMutable<T>
where
  T: AbstractPagedMutableBase + Display,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}(size={},pageSize={})",
      self.sub_reader,
      self.size,
      self.page_size()
    )
  }
}
pub(crate) trait AbstractPagedMutableBase {
  fn new_mutable(&self, value_count: i32, bits_per_value: i32) -> Result<MutableEnum>;
  fn new_unfilled_copy(&self) -> Self
  where
    Self: Sized;
  fn base_ram_bytes_used_base(&self) -> i64;
  fn fill_pages(&self) -> bool;
  fn bits_per_value(&self) -> i32;
}

pub enum AbstractPagedMutableBaseEnum {
  Mutable(PagedMutable),
  GrowableWriter(PagedGrowableWriter),
}
impl Default for AbstractPagedMutableBaseEnum {
  /// for padding using
  fn default() -> Self {
    AbstractPagedMutableBaseEnum::Mutable(PagedMutable::default())
  }
}
impl AbstractPagedMutableBase for AbstractPagedMutableBaseEnum {
  fn new_mutable(&self, value_count: i32, bits_per_value: i32) -> Result<MutableEnum> {
    match self {
      AbstractPagedMutableBaseEnum::Mutable(m) => m.new_mutable(value_count, bits_per_value),
      AbstractPagedMutableBaseEnum::GrowableWriter(g) => g.new_mutable(value_count, bits_per_value),
    }
  }

  fn new_unfilled_copy(&self) -> Self {
    match self {
      AbstractPagedMutableBaseEnum::Mutable(m) => {
        AbstractPagedMutableBaseEnum::Mutable(m.new_unfilled_copy())
      },
      AbstractPagedMutableBaseEnum::GrowableWriter(g) => {
        AbstractPagedMutableBaseEnum::GrowableWriter(g.new_unfilled_copy())
      },
    }
  }

  fn base_ram_bytes_used_base(&self) -> i64 {
    match self {
      AbstractPagedMutableBaseEnum::Mutable(m) => m.base_ram_bytes_used_base(),
      AbstractPagedMutableBaseEnum::GrowableWriter(g) => g.base_ram_bytes_used_base(),
    }
  }

  fn fill_pages(&self) -> bool {
    match self {
      AbstractPagedMutableBaseEnum::Mutable(m) => m.fill_pages(),
      AbstractPagedMutableBaseEnum::GrowableWriter(g) => g.fill_pages(),
    }
  }

  fn bits_per_value(&self) -> i32 {
    match self {
      AbstractPagedMutableBaseEnum::Mutable(m) => m.bits_per_value(),
      AbstractPagedMutableBaseEnum::GrowableWriter(g) => g.bits_per_value(),
    }
  }
}

impl Display for AbstractPagedMutableBaseEnum {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Mutable(m) => m.fmt(f),
      Self::GrowableWriter(g) => g.fmt(f),
    }
  }
}
