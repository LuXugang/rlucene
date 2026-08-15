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
use crate::core::codecs::mutable_point_tree::MutablePointTree;
use crate::core::index::BytesRef;
use crate::core::util::array_util::{ArrayUtil, ByteArrayComparator, ByteArrayComparatorEnum};
use crate::core::util::bkd::bkd_config::BKDConfig;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::intro_sorter::IntroSorter;
use crate::core::util::packed::PackedInts;
use crate::core::util::radix_selector::{RadixSelector, RadixSelectorBase};
use crate::core::util::selector::Selector;
use crate::core::util::{
  IntroSelector, IntroSelectorBase, IntroSelectorBaseDefault, MSBRadixSorter, MSBRadixSorterBase,
  Sorter, StableMSBRadixSorter, StableMSBRadixSorterBase, ToInt, TryIntoInt,
};

/// Utility APIs for sorting and partitioning buffered points.
pub struct MutablePointTreeReaderUtils;

impl MutablePointTreeReaderUtils {
  /// Sort the given [`MutablePointTree`] based on its packed value then doc
  /// ID.
  pub fn sort<M>(
    config: &BKDConfig,
    max_doc: i32,
    reader: &mut M,
    from: usize,
    to: usize,
  ) -> Result<()>
  where
    M: MutablePointTree,
  {
    let mut sorted_by_doc_id = true;
    let mut prev_doc = 0;
    for i in from..to {
      let doc = reader.get_doc_id(i)?;
      if doc < prev_doc {
        sorted_by_doc_id = false;
        break;
      }
      prev_doc = doc;
    }

    // No need to tie break on doc IDs if already sorted by doc ID, since we
    // use a stable sort. This should be a common situation as
    // IndexWriter accumulates data in doc ID order when
    // index sorting is not enabled.
    let bits_per_doc_id: usize = if sorted_by_doc_id {
      0
    } else {
      PackedInts::bits_required((max_doc - 1) as i64)?.try_convert()?
    };
    let max_length = config.packed_bytes_length() + bits_per_doc_id.div_ceil(8);
    let delegate = StableMSBRadixSorterImpl {
      reader,
      config,
      bits_per_doc_id,
    };
    let stable_msb_radix_sorter = StableMSBRadixSorter::new(delegate, max_length);
    let mut sorter = MSBRadixSorter::new(max_length, stable_msb_radix_sorter);
    sorter.sort(from, to)
  }

  /// Sort points on the given dimension.
  #[allow(clippy::too_many_arguments)]
  pub fn sort_by_dim<M>(
    config: &BKDConfig,
    sorted_dim: usize,
    _common_prefix_lengths: &[usize],
    reader: &mut M,
    from: usize,
    to: usize,
    _scratch1: &mut BytesRef<Vec<u8>>,
    _scratch2: &mut BytesRef<Vec<u8>>,
  ) -> Result<()>
  where
    M: MutablePointTree,
  {
    // Get an unsigned comparator for the byte arrays.
    let comparator = ArrayUtil::get_unsigned_comparator(config.bytes_per_dim);
    let start = sorted_dim * config.bytes_per_dim;
    // No need for a fancy radix sort here, this is called on the leaves
    // only so there are not many values to sort.
    let mut intro_sorter = IntroSorterImpl {
      reader,
      config,
      pivot: BytesRef::new(),
      scratch2: BytesRef::new(),
      pivot_doc: 0,
      comparator,
      start,
    };
    intro_sorter.sort(from, to)?;
    Ok(())
  }
  /// Partition points around `mid`. All values on the left must be less than
  /// or equal to it and all values on the right must be greater than or
  /// equal to it.
  #[allow(clippy::too_many_arguments)]
  pub fn partition<M>(
    config: &BKDConfig,
    max_doc: i32,
    split_dim: usize,
    common_prefix_len: usize,
    reader: &mut M,
    from: usize,
    to: usize,
    mid: usize,
    _scratch1: &mut BytesRef<Vec<u8>>,
    _scratch2: &mut BytesRef<Vec<u8>>,
  ) -> Result<()>
  where
    M: MutablePointTree,
  {
    let dim_offset = split_dim * config.bytes_per_dim + common_prefix_len;
    let dim_cmp_bytes = config.bytes_per_dim - common_prefix_len;
    debug_assert!(config.num_dims >= config.num_index_dims);
    let data_cmp_bytes =
      (config.num_dims - config.num_index_dims) * config.bytes_per_dim + dim_cmp_bytes;
    let bits_per_doc_id = PackedInts::bits_required((max_doc - 1) as i64)? as usize;
    let max_length = data_cmp_bytes + bits_per_doc_id.div_ceil(8);

    let sub_selector = RadixSelectorImpl {
      split_dim,
      config,
      dim_cmp_bytes,
      reader,
      dim_offset,
      data_cmp_bytes,
      bits_per_doc_id,
    };
    let mut radix_selector = RadixSelector::new(max_length, sub_selector);
    radix_selector.select(from, to, mid)
  }
}

struct StableMSBRadixSorterImpl<'a, M> {
  reader: &'a mut M,
  config: &'a BKDConfig,
  bits_per_doc_id: usize,
}

impl<M> MSBRadixSorterBase for StableMSBRadixSorterImpl<'_, M>
where
  M: MutablePointTree,
{
  fn byte_at(&mut self, i: usize, k: usize) -> Result<i32> {
    if k < self.config.packed_bytes_length() {
      Ok(self.reader.get_byte_at(i, k) as i32)
    } else {
      let rhs = (k - self.config.packed_bytes_length() + 1) << 3;

      let effective_shift = self.bits_per_doc_id.saturating_sub(rhs);
      Ok(((self.reader.get_doc_id(i)? as u32 >> effective_shift) & 0xff) as i32)
    }
  }
}

impl<M> Sorter for StableMSBRadixSorterImpl<'_, M>
where
  M: MutablePointTree,
{
  fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    self.reader.swap(i, j);
    Ok(())
  }
}

impl<M> StableMSBRadixSorterBase for StableMSBRadixSorterImpl<'_, M>
where
  M: MutablePointTree,
{
  fn save(&mut self, i: usize, j: usize) {
    self.reader.save(i, j);
  }

  fn restore(&mut self, i: usize, j: usize) {
    self.reader.restore(i, j);
  }
}

struct IntroSorterImpl<'a, M> {
  reader: &'a mut M,
  config: &'a BKDConfig,
  pivot: BytesRef<Vec<u8>>,
  scratch2: BytesRef<Vec<u8>>,
  pivot_doc: i32,
  comparator: ByteArrayComparatorEnum,
  start: usize,
}
impl<M> Sorter for IntroSorterImpl<'_, M>
where
  M: MutablePointTree,
{
  fn compare(&mut self, i: usize, j: usize) -> Result<i32> {
    self.set_pivot(i)?;
    self.compare_pivot(j)
  }

  fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    self.reader.swap(i, j);
    Ok(())
  }

  fn set_pivot(&mut self, i: usize) -> Result<()> {
    self.reader.get_value(i, &mut self.pivot)?;
    self.pivot_doc = self.reader.get_doc_id(i)?;
    Ok(())
  }

  fn compare_pivot(&mut self, j: usize) -> Result<i32> {
    self.reader.get_value(j, &mut self.scratch2)?;

    let cmp = self.comparator.compare(
      &self.pivot.bytes,
      self.pivot.offset + self.start,
      &self.scratch2.bytes,
      self.scratch2.offset + self.start,
    );

    if cmp == 0 {
      let pivot_index_start = self.pivot.offset + self.config.packed_index_bytes_length();
      let pivot_index_end = self.pivot.offset + self.config.packed_bytes_length();
      let scratch_index_start = self.scratch2.offset + self.config.packed_index_bytes_length();
      let scratch_index_end = self.scratch2.offset + self.config.packed_bytes_length();

      let pivot_slice = &self.pivot.bytes[pivot_index_start..pivot_index_end];
      let scratch_slice = &self.scratch2.bytes[scratch_index_start..scratch_index_end];

      let cmp = pivot_slice.cmp(scratch_slice).to_int();
      return if cmp == 0 {
        Ok(self.pivot_doc - self.reader.get_doc_id(j)?)
      } else {
        Ok(cmp)
      };
    }
    Ok(cmp)
  }

  fn sort(&mut self, from: usize, to: usize) -> Result<()> {
    IntroSorter::sort_range(self, from, to)?;
    Ok(())
  }
}

impl<M> IntroSorter for IntroSorterImpl<'_, M> where M: MutablePointTree {}

struct RadixSelectorImpl<'a, M> {
  split_dim: usize,
  config: &'a BKDConfig,
  dim_cmp_bytes: usize,
  reader: &'a mut M,
  dim_offset: usize,
  data_cmp_bytes: usize,
  bits_per_doc_id: usize,
}

impl<M> Selector for RadixSelectorImpl<'_, M>
where
  M: MutablePointTree,
{
  fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    self.reader.swap(i, j);
    Ok(())
  }
}

impl<M> RadixSelectorBase for RadixSelectorImpl<'_, M>
where
  M: MutablePointTree,
{
  fn byte_at(&mut self, i: usize, k: usize) -> Result<i32> {
    if k < self.dim_cmp_bytes {
      Ok(self.reader.get_byte_at(i, self.dim_offset + k) as i32)
    } else if k < self.data_cmp_bytes {
      Ok(self.reader.get_byte_at(
        i,
        self.config.packed_index_bytes_length() + k - self.dim_cmp_bytes,
      ) as i32)
    } else {
      let rhs = (k - self.data_cmp_bytes + 1) << 3;

      let effective_shift = self.bits_per_doc_id.saturating_sub(rhs);
      Ok(((self.reader.get_doc_id(i)? as u32 >> effective_shift) & 0xff) as i32)
    }
  }

  fn get_fallback_selector(&mut self, k: usize, _max_length: usize) -> impl Selector
  where
    Self: Sized,
  {
    let dim_start = self.split_dim * self.config.bytes_per_dim;
    let data_start = if k < self.dim_cmp_bytes {
      self.config.packed_index_bytes_length()
    } else {
      self.config.packed_index_bytes_length() + k - self.dim_cmp_bytes
    };
    let data_end = self.config.num_dims * self.config.bytes_per_dim;
    let dim_comparator = ArrayUtil::get_unsigned_comparator(self.config.bytes_per_dim);

    let sub_selector = IntroSelectorImpl {
      dim_cmp_bytes: self.dim_cmp_bytes,
      data_cmp_bytes: self.data_cmp_bytes,
      pivot: BytesRef::new(),
      reader: self.reader,
      pivot_doc: 0,
      k,
      scratch2: BytesRef::new(),
      dim_comparator,
      dim_start,
      data_start,
      data_end,
    };
    IntroSelector::new(sub_selector)
  }
}

struct IntroSelectorImpl<'a, M> {
  dim_cmp_bytes: usize,
  data_cmp_bytes: usize,
  pivot: BytesRef<Vec<u8>>,
  reader: &'a mut M,
  pivot_doc: i32,
  k: usize,
  scratch2: BytesRef<Vec<u8>>,
  dim_comparator: ByteArrayComparatorEnum,
  dim_start: usize,
  data_start: usize,
  data_end: usize,
}

impl<M> IntroSelectorBaseDefault for IntroSelectorImpl<'_, M>
where
  M: MutablePointTree,
{
  fn set_pivot(&mut self, i: usize) -> Result<()> {
    self.reader.get_value(i, &mut self.pivot)?;
    self.pivot_doc = self.reader.get_doc_id(i)?;
    Ok(())
  }

  fn compare_pivot(&mut self, j: usize) -> Result<i32> {
    if self.k < self.dim_cmp_bytes {
      self.reader.get_value(j, &mut self.scratch2)?;
      let cmp = self.dim_comparator.compare(
        &self.pivot.bytes,
        self.pivot.offset + self.dim_start,
        &self.scratch2.bytes,
        self.scratch2.offset + self.dim_start,
      );
      if cmp != 0 {
        return Ok(cmp);
      }
    }
    if self.k < self.data_cmp_bytes {
      self.reader.get_value(j, &mut self.scratch2)?;
      let pivot_slice =
        &self.pivot.bytes[self.pivot.offset + self.data_start..self.pivot.offset + self.data_end];
      let scratch_slice = &self.scratch2.bytes
        [self.scratch2.offset + self.data_start..self.scratch2.offset + self.data_end];
      let cmp = pivot_slice.cmp(scratch_slice).to_int();
      if cmp != 0 {
        return Ok(cmp);
      }
    }
    Ok(self.pivot_doc - self.reader.get_doc_id(j)?)
  }
}

impl<M> Selector for IntroSelectorImpl<'_, M>
where
  M: MutablePointTree,
{
  fn swap(&mut self, i: usize, j: usize) -> Result<()> {
    self.reader.swap(i, j);
    Ok(())
  }
}

impl<M> IntroSelectorBase for IntroSelectorImpl<'_, M> where M: MutablePointTree {}
