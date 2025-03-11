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
use crate::codecs::mutable_point_tree::{MutablePointTree, MutablePointTreeEnum};
use crate::index::BytesRef;
use crate::util::array_util::{ArrayUtil, ByteArrayComparator, ByteArrayComparatorEnum};
use crate::util::bkd::bkd_config::BKDConfig;
use crate::util::error::lucene_error::LuceneError;
use crate::util::intro_sorter::IntroSorter;
use crate::util::radix_selector::RadixSelectorBase;
use crate::util::selector::Selector;
use crate::util::{
    IntroSelector, IntroSelectorBase, IntroSelectorBaseDefault, MSBRadixSorterBase, Sorter,
    StableMSBRadixSorterBase,
};
use std::cell::RefCell;
use std::cmp::max;
use std::rc::Rc;

/// Utility APIs for sorting and partitioning buffered points.
pub struct MutablePointTreeReaderUtils;

struct StableMSBRadixSorterImpl<'a> {
    reader: &'a mut MutablePointTreeEnum,
    config: Rc<BKDConfig>,
    bits_per_doc_id: i32,
}

impl MSBRadixSorterBase for StableMSBRadixSorterImpl<'_> {
    fn byte_at(&mut self, i: i32, k: i32) -> Result<i32, LuceneError> {
        if k < self.config.packed_bytes_length() {
            Ok(self.reader.get_byte_at(i, k) as i32)
        } else {
            let shift = self.bits_per_doc_id - ((k - self.config.packed_bytes_length() + 1) << 3);
            let effective_shift = max(0, shift) as u32;
            Ok(((self.reader.get_doc_id(i) as u32 >> effective_shift) & 0xff) as i32)
        }
    }
}

impl Sorter for StableMSBRadixSorterImpl<'_> {
    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        self.reader.swap(i, j);
        Ok(())
    }
}

impl StableMSBRadixSorterBase for StableMSBRadixSorterImpl<'_> {
    fn save(&mut self, i: i32, j: i32) {
        self.reader.save(i, j);
    }

    fn restore(&mut self, i: i32, j: i32) {
        self.reader.restore(i, j);
    }
}

struct IntroSorterImpl<'a> {
    reader: &'a mut MutablePointTreeEnum,
    config: Rc<BKDConfig>,
    bits_per_doc_id: i32,
    pivot: BytesRef,
    scratch2: BytesRef,
    pivot_doc: i32,
    comparator: ByteArrayComparatorEnum,
    start: i32,
}
impl Sorter for IntroSorterImpl<'_> {
    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        self.reader.swap(i, j);
        Ok(())
    }

    fn set_pivot(&mut self, i: i32) -> Result<(), LuceneError> {
        self.reader.get_value(i, &mut self.pivot);
        Ok(())
    }

    fn compare_pivot(&mut self, j: i32) -> Result<i32, LuceneError> {
        self.reader.get_value(j, &mut self.scratch2);

        let cmp = self.comparator.compare(
            &self.pivot.bytes,
            (self.pivot.offset + self.start) as usize,
            &self.scratch2.bytes,
            (self.scratch2.offset + self.start) as usize,
        );

        if cmp == 0 {
            let pivot_index_start =
                (self.pivot.offset + self.config.packed_index_bytes_length()) as usize;
            let pivot_index_end = (self.pivot.offset + self.config.packed_bytes_length()) as usize;
            let scratch_index_start =
                (self.scratch2.offset + self.config.packed_index_bytes_length()) as usize;
            let scratch_index_end =
                (self.scratch2.offset + self.config.packed_bytes_length()) as usize;

            let pivot_slice = &self.pivot.bytes[pivot_index_start..pivot_index_end];
            let scratch_slice = &self.scratch2.bytes[scratch_index_start..scratch_index_end];

            let cmp = match pivot_slice.cmp(scratch_slice) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };

            if cmp == 0 {
                return Ok(self.pivot_doc - self.reader.get_doc_id(j));
            } else {
                return Ok(cmp);
            }
        }
        Ok(cmp)
    }
}

impl IntroSorter for IntroSorterImpl<'_> {}

struct RadixSelectorImpl {
    split_dim: i32,
    config: Rc<BKDConfig>,
    dim_cmp_bytes: i32,
    reader: Rc<RefCell<MutablePointTreeEnum>>,
    dim_offset: i32,
    data_cmp_bytes: i32,
    bits_per_doc_id: i32,
}

impl Selector for RadixSelectorImpl {
    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        self.reader.borrow_mut().swap(i, j);
        Ok(())
    }
}

impl RadixSelectorBase for RadixSelectorImpl {
    fn byte_at(&self, i: i32, k: i32) -> i32 {
        let reader = self.reader.borrow();
        if k < self.dim_cmp_bytes {
            reader.get_byte_at(i, self.dim_offset + k) as i32
        } else if k < self.data_cmp_bytes {
            reader.get_byte_at(
                i,
                self.config.packed_index_bytes_length() + k - self.dim_cmp_bytes,
            ) as i32
        } else {
            let shift = self.bits_per_doc_id - ((k - self.data_cmp_bytes + 1) << 3);
            let effective_shift = std::cmp::max(0, shift) as u32;
            ((reader.get_doc_id(i) as u32 >> effective_shift) & 0xff) as i32
        }
    }

    fn get_fallback_selector(&mut self, k: i32, _max_length: i32) -> impl Selector
    where
        Self: Sized,
    {
        let dim_start = self.split_dim * self.config.get_bytes_per_dim();
        let data_start = if k < self.dim_cmp_bytes {
            self.config.packed_index_bytes_length()
        } else {
            self.config.packed_index_bytes_length() + k - self.dim_cmp_bytes
        };
        let data_end = self.config.get_num_dims() * self.config.get_bytes_per_dim();
        let dim_comparator = ArrayUtil::get_unsigned_comparator(self.config.bytes_per_dim as usize);

        let sub_selector = IntroSelectorImpl {
            split_dim: self.split_dim,
            config: self.config.clone(),
            dim_cmp_bytes: self.dim_cmp_bytes,
            data_cmp_bytes: self.data_cmp_bytes,
            pivot: BytesRef::new(),
            reader: self.reader.clone(),
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

struct IntroSelectorImpl {
    split_dim: i32,
    config: Rc<BKDConfig>,
    dim_cmp_bytes: i32,
    data_cmp_bytes: i32,
    pivot: BytesRef,
    reader: Rc<RefCell<MutablePointTreeEnum>>,
    pivot_doc: i32,
    k: i32,
    scratch2: BytesRef,
    dim_comparator: ByteArrayComparatorEnum,
    dim_start: i32,
    data_start: i32,
    data_end: i32,
}

impl IntroSelectorBaseDefault for IntroSelectorImpl {
    fn set_pivot(&mut self, i: i32) {
        let reader = self.reader.borrow_mut();
        reader.get_value(i, &mut self.pivot);
        self.pivot_doc = reader.get_doc_id(i);
    }

    fn compare_pivot(&mut self, j: i32) -> i32 {
        let reader = self.reader.borrow();
        if self.k < self.dim_cmp_bytes {
            reader.get_value(j, &mut self.scratch2);
            let cmp = self.dim_comparator.compare(
                &self.pivot.bytes,
                (self.pivot.offset + self.dim_start) as usize,
                &self.scratch2.bytes,
                (self.scratch2.offset + self.dim_start) as usize,
            );
            if cmp != 0 {
                return cmp;
            }
        }
        if self.k < self.data_cmp_bytes {
            reader.get_value(j, &mut self.scratch2);
            let pivot_slice = &self.pivot.bytes[(self.pivot.offset + self.data_start) as usize
                ..(self.pivot.offset + self.data_end) as usize];
            let scratch_slice = &self.scratch2.bytes[(self.scratch2.offset + self.data_start)
                as usize
                ..(self.scratch2.offset + self.data_end) as usize];
            let cmp = match pivot_slice.cmp(scratch_slice) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            if cmp != 0 {
                return cmp;
            }
        }
        self.pivot_doc - reader.get_doc_id(j)
    }
}

impl Selector for IntroSelectorImpl {
    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        self.reader.borrow_mut().swap(i, j);
        Ok(())
    }
}

impl IntroSelectorBase for IntroSelectorImpl {}
