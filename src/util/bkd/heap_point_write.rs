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
use crate::util::array_util::{ArrayUtil, ByteArrayComparator, ByteArrayComparatorEnum};
use crate::util::bit_util::BitUtil;
use crate::util::bkd::bkd_config::BKDConfig;
use crate::util::bkd::heap_point_reader::HeapPointReader;
use crate::util::bkd::point_reader::PointReaderEnum;
use crate::util::bkd::point_value::{PointValue, PointValueEnum};
use crate::util::bkd::point_writer::PointWriter;
use crate::util::error::lucene_error::LuceneError;
use crate::util::{CommonUtil, VecCopyOps};
use std::cell::RefCell;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

/// Utility class to write new points into in-heap arrays.
pub struct HeapPointWriter {
    pub block: Vec<u8>,
    pub size: i32,
    pub config: Rc<BKDConfig>,
    pub scratch: Vec<u8>,
    pub dim_comparator: ByteArrayComparatorEnum,
    // length is composed by the data dimensions plus the docID
    pub data_dims_and_doc_length: i32,
    pub next_write: i32,
    pub closed: bool,
    pub point_value: Option<Rc<RefCell<PointValueEnum>>>,
}
impl HeapPointWriter {
    pub fn new(config: Rc<BKDConfig>, size: i32) -> Self {
        let data_dims_and_doc_length = config.bytes_per_doc() + config.packed_index_bytes_length();
        let bytes_per_doc = config.bytes_per_doc() as usize;
        let point_value = if size > 0 {
            Some(Rc::new(RefCell::new(PointValueEnum::Heap(
                HeapPointValue::new(&config),
            ))))
        } else {
            None
        };
        let bytes_per_dim = config.bytes_per_dim as usize;
        Self {
            block: vec![0u8; bytes_per_doc * (size as usize)],
            size,
            config,
            scratch: vec![0u8; bytes_per_doc],
            dim_comparator: ArrayUtil::get_unsigned_comparator(bytes_per_dim),
            data_dims_and_doc_length,
            next_write: 0,
            closed: false,
            point_value,
        }
    }
    pub fn get_packed_value_slice(&mut self, index: i32) -> Rc<RefCell<PointValueEnum>> {
        debug_assert!(
            index < self.next_write,
            "next_write={} vs index={}",
            self.next_write,
            index
        );
        self.point_value
            .as_mut()
            .unwrap()
            .borrow_mut()
            .set_offset(index * self.config.bytes_per_doc());
        self.point_value.as_ref().unwrap().clone()
    }
    /// Swaps the point at point `i` with the point at position `j`
    pub(crate) fn swap(&mut self, i: i32, j: i32) {
        let bytes_per_doc = self.config.bytes_per_doc() as usize;
        let index_i = bytes_per_doc * i as usize;
        let index_j = bytes_per_doc * j as usize;
        self.scratch
            .copy_from(&mut self.block[index_i..index_i + bytes_per_doc], 0);
        self.block
            .copy_within(index_j..index_j + bytes_per_doc, index_i);
        self.block.copy_from(&self.scratch, index_j);
    }

    /// Return the byte at position `k` of the point at position `i`
    pub fn byte_at(&self, i: i32, k: i32) -> i32 {
        self.block[(i * self.config.bytes_per_doc() + k) as usize] as i32
    }

    /// Copy the dimension `dim` of the point at position `i` in the provided `bytes`
    /// at the given offset
    pub fn copy_dim(&self, i: i32, dim: i32, bytes: &mut [u8], offset: usize) {
        let start = (i * self.config.bytes_per_doc() + dim) as usize;
        let len = self.config.get_bytes_per_dim() as usize;
        bytes[offset..offset + len].copy_from_slice(&self.block[start..start + len]);
    }

    /// Copy the data dimensions and doc value of the point at position `i` in the provided
    /// `bytes` at the given offset
    pub fn copy_data_dims_and_doc(&self, i: i32, bytes: &mut [u8], offset: usize) {
        let start =
            (i * self.config.bytes_per_doc() + self.config.packed_index_bytes_length()) as usize;
        let len = self.data_dims_and_doc_length as usize;
        bytes[offset..offset + len].copy_from_slice(&self.block[start..start + len]);
    }

    /// Compares the dimension `dim` value of the point at position `i` with the point at
    /// position `j`
    pub fn compare_dim(&self, i: i32, j: i32, dim: i32) -> i32 {
        let i_offset = (i * self.config.bytes_per_doc() + dim) as usize;
        let j_offset = (j * self.config.bytes_per_doc() + dim) as usize;
        self.compare_dim_slice(&self.block, i_offset, &self.block, j_offset)
    }

    /// Compares the dimension `dim` value of the point at position `j` with the provided
    /// value
    pub fn compare_dim_with(&self, j: i32, dim_value: &[u8], offset: usize, dim: i32) -> i32 {
        let j_offset = (j * self.config.bytes_per_doc() + dim) as usize;
        self.compare_dim_slice(dim_value, offset, &self.block, j_offset)
    }

    fn compare_dim_slice(
        &self,
        block_i: &[u8],
        offset_i: usize,
        block_j: &[u8],
        offset_j: usize,
    ) -> i32 {
        self.dim_comparator
            .compare(block_i, offset_i, block_j, offset_j)
    }

    /// Compares the data dimensions and doc values of the point at position `i` with the point
    /// at position `j`
    pub fn compare_data_dims_and_doc(&self, i: i32, j: i32) -> i32 {
        let i_offset =
            (i * self.config.bytes_per_doc() + self.config.packed_index_bytes_length()) as usize;
        let j_offset =
            (j * self.config.bytes_per_doc() + self.config.packed_index_bytes_length()) as usize;
        self.compare_data_dims_and_doc_slice(&self.block, i_offset, &self.block, j_offset)
    }

    /// Compares the data dimensions and doc values of the point at position `j` with the
    /// provided value
    pub fn compare_data_dims_and_doc_with(
        &self,
        j: i32,
        data_dims_and_docs: &[u8],
        offset: usize,
    ) -> i32 {
        let j_offset =
            (j * self.config.bytes_per_doc() + self.config.packed_index_bytes_length()) as usize;
        self.compare_data_dims_and_doc_slice(data_dims_and_docs, offset, &self.block, j_offset)
    }

    fn compare_data_dims_and_doc_slice(
        &self,
        block_i: &[u8],
        offset_i: usize,
        block_j: &[u8],
        offset_j: usize,
    ) -> i32 {
        let len = self.data_dims_and_doc_length as usize;
        let slice_i = &block_i[offset_i..offset_i + len];
        let slice_j = &block_j[offset_j..offset_j + len];
        match slice_i.cmp(slice_j) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }

    /// Computes the cardinality of the points between `from` tp `to`
    pub fn compute_cardinality(&self, from: i32, to: i32, common_prefix_lengths: &[i32]) -> i32 {
        let mut leaf_cardinality = 1;
        for i in (from + 1)..to {
            let point_offset = ((i - 1) * self.config.bytes_per_doc()) as usize;
            let next_point_offset = point_offset + self.config.bytes_per_doc() as usize;
            for dim in 0..self.config.get_num_dims() {
                let start = (dim * self.config.get_bytes_per_dim()
                    + common_prefix_lengths[dim as usize]) as usize;
                let end = (dim * self.config.get_bytes_per_dim() + self.config.get_bytes_per_dim())
                    as usize;
                if CommonUtil::miss_match(
                    &self.block[next_point_offset + start..next_point_offset + end],
                    &self.block[point_offset + start..point_offset + end],
                ) != -1
                {
                    leaf_cardinality += 1;
                    break;
                }
            }
        }
        leaf_cardinality
    }
}
impl PointWriter for HeapPointWriter {
    fn append_bytes(&mut self, packed_value: &[u8], doc_id: i32) -> Result<(), LuceneError> {
        debug_assert!(!self.closed, "point writer is already closed");
        assert_eq!(
            packed_value.len(),
            self.config.packed_bytes_length() as usize,
            "[packedValue] must have length {} but was {}",
            self.config.packed_bytes_length(),
            packed_value.len()
        );
        debug_assert!(
            self.next_write < self.size,
            "nextWrite={} vs size={}",
            self.next_write + 1,
            self.size
        );
        let position = self.next_write * self.config.bytes_per_doc();
        self.block.copy_from(
            &packed_value[0..self.config.packed_bytes_length() as usize],
            position as usize,
        );
        BitUtil::set_i32_be(
            &mut self.block,
            (position + self.config.packed_bytes_length()) as usize,
            doc_id,
        );
        self.next_write += 1;
        Ok(())
    }

    fn append_point_value(&mut self, point_value: &PointValueEnum) -> Result<(), LuceneError> {
        debug_assert!(!self.closed, "point writer is already closed");
        debug_assert!(
            self.next_write < self.size,
            "nextWrite={} vs size={}",
            self.next_write + 1,
            self.size
        );
        let (offset, length) = point_value.packed_value_doc_id_bytes();
        assert_eq!(
            length,
            self.config.bytes_per_doc(),
            "[packedValue] must have length {} but was {}",
            self.config.bytes_per_doc(),
            length
        );
        let position = self.next_write * self.config.bytes_per_doc();
        self.block.copy_within(
            offset as usize..(offset + self.config.bytes_per_doc()) as usize,
            position as usize,
        );
        self.next_write += 1;
        Ok(())
    }

    fn get_reader(&mut self, start: i64, length: i64) -> Result<PointReaderEnum, LuceneError> {
        debug_assert!(
            self.closed,
            "point writer is still open and trying to get a reader"
        );
        debug_assert!(
            start + length <= self.size as i64,
            "start={} length={} docIDs.length={}",
            start,
            length,
            self.size
        );
        debug_assert!(
            start + length <= self.next_write as i64,
            "start={} length={} nextWrite={}",
            start,
            length,
            self.next_write
        );
        let value = start + length;
        if value > i32::MAX as i64 {
            return Err(LuceneError::illegal_argument(format!(
                "start + length must be <= {}",
                i32::MAX
            )));
        }
        Ok(PointReaderEnum::Heap(HeapPointReader::new(
            self.point_value.as_ref().unwrap().clone(),
            start as i32,
            value as i32,
            self.config.bytes_per_doc(),
        )))
    }

    fn count(&self) -> i64 {
        self.next_write as i64
    }

    fn destroy(&mut self) -> Result<(), LuceneError> {
        Ok(())
    }
}
impl Display for HeapPointWriter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HeapPointWriter(count={} size={})",
            self.next_write, self.size
        )
    }
}

/// Reusable implementation for a point value on-heap.
#[derive(Debug, Clone)]
pub(crate) struct HeapPointValue {
    pub(crate) offset: i32,
    pub(crate) packed_value_length: i32,
    pub(crate) packed_value_doc_id_length: i32,
}
impl HeapPointValue {
    pub fn new(config: &BKDConfig) -> Self {
        Self {
            offset: 0,
            packed_value_length: config.packed_bytes_length(),
            packed_value_doc_id_length: config.bytes_per_doc(),
        }
    }
}
impl PointValue for HeapPointValue {
    fn set_offset(&mut self, offset: i32) {
        self.offset = offset;
    }

    fn packed_value(&self) -> (i32, i32) {
        (self.offset, self.packed_value_length)
    }

    fn doc_id(&self, bytes: &[u8]) -> i32 {
        let position = (self.offset + self.packed_value_length) as usize;
        BitUtil::get_i32_be(&bytes[position..], 0)
    }

    fn packed_value_doc_id_bytes(&self) -> (i32, i32) {
        (self.offset, self.packed_value_doc_id_length)
    }
}
