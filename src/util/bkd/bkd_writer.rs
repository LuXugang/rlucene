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
use crate::index::BytesRef;
use crate::store::{DataOutput, IndexOutput};
use crate::util::array_util::{ArrayUtil, ByteArrayComparator, ByteArrayComparatorEnum};
use crate::util::bkd::bkd_config::BKDConfig;
use crate::util::bkd::bkd_util::{ByteArrayPredicate, ByteArrayPredicateEnum};
use crate::util::bkd::doc_ids_writer::DocIdsWriter;
use crate::util::error::lucene_error::LuceneError;
use crate::util::io_runnable::IORunnable;
use crate::util::ToInt;
use std::cell::RefCell;
use std::rc::Rc;
use crate::codecs::mutable_point_tree::{MutablePointTree, MutablePointTreeEnum};
use crate::store::directory::Directory;
use crate::util::bkd::point_writer::PointWriterEnum;

pub struct BKDWriter<D> where D: Directory {
    config: Rc<BKDConfig>,
    common_prefix_comparator: ByteArrayComparatorEnum,
    scratch_bytes_ref1: BytesRef,
    scratch_bytes_ref2: BytesRef,
    common_prefix_lengths: Vec<i32>,
    point_writer: Rc<RefCell<PointWriterEnum<D>>>,
    finish: bool,
    min_packed_value: Vec<u8>,
    max_packed_value: Vec<u8>,
    total_point_count: i64,
    equals_predicate: Rc<ByteArrayPredicateEnum>,
    doc_ids_writer: DocIdsWriter,
}
impl<D> BKDWriter<D> where D: Directory {
    fn check_max_leaf_node_count(&self, num_leaves: usize) -> Result<(), LuceneError> {
        if (self.config.bytes_per_dim as u64) * (num_leaves as u64)
            > ArrayUtil::MAX_ARRAY_LENGTH as u64
        {
            return Err(LuceneError::illegal_state(format!(
                "too many nodes; increase config.maxPointsInLeafNode() (currently {}) and reindex",
                self.config.max_points_in_leaf_node
            )));
        }
        Ok(())
    }
    fn write_common_prefixes(
        &self,
        out: &mut impl DataOutput,
        common_prefixes: &[i32],
        packed_value: &[u8],
    ) -> Result<(), LuceneError> {
        let num_dims = self.config.num_dims as usize;
        for dim in 0..num_dims {
            out.write_vint(common_prefixes[dim])?;
            out.write_bytes_range(
                packed_value,
                dim as i32 * self.config.bytes_per_dim,
                common_prefixes[dim],
            )?;
        }
        Ok(())
    }
    fn write_leaf_block_docs(
        &mut self,
        out: &mut D::IndexOutputType,
        doc_ids: &[i32],
        start: i32,
        count: i32,
    ) -> Result<(), LuceneError>
    {
        debug_assert!(
            count > 0,
            "config.max_points_in_leaf_node()={}",
            self.config.max_points_in_leaf_node
        );
        out.write_vint(count)?;
        self.doc_ids_writer
            .write_doc_ids(doc_ids, start, count, out)?;
        Ok(())
    }
    fn write_leaf_block_packed_values<F>(
        &self,
        out: &mut impl DataOutput,
        common_prefix_lengths: &[i32],
        count: i32,
        sorted_dim: i32,
        packed_values: F,
        leaf_cardinality: i32,
    ) -> Result<(), LuceneError>
    where
        F: Fn(i32) -> BytesRef,
    {
        Ok(())
    }
}
pub struct OneDimensionBKDWriter<'a,D> where D:Directory{
    meta_out: Rc<RefCell<D::IndexOutputType>>,
    index_out: Rc<RefCell<D::IndexOutputType>>,
    data_out: Rc<RefCell<D::IndexOutputType>>,
    data_start_fp: i64,
    leaf_block_fps: Vec<i64>,
    leaf_block_start_values: Vec<Vec<u8>>,
    leaf_values: Vec<u8>,
    leaf_docs: Vec<i32>,
    value_count: i64,
    leaf_count: i32,
    leaf_cardinality: i32,
    // for asserts
    last_packed_value: Vec<u8>,
    last_doc_id: i32,
    bkd_writer: &'a mut BKDWriter<D>,
}

impl<'a, D> OneDimensionBKDWriter<'a, D>
where
    D: Directory,
{
    pub fn new(
        meta_out: Rc<RefCell<D::IndexOutputType>>,
        index_out: Rc<RefCell<D::IndexOutputType>>,
        data_out: Rc<RefCell<D::IndexOutputType>>,
        config: Rc<BKDConfig>,
        point_count: i64,
        bkd_writer: &'a mut BKDWriter<D>,
    ) -> Result<Self, LuceneError> {
        if config.num_index_dims != 1 {
            return Err(LuceneError::unsupported_operation(format!(
                "config.numIndexDims() must be 1 but got {}",
                config.num_index_dims
            )));
        }
        if point_count != 0 {
            return Err(LuceneError::illegal_state(
                "cannot mix add and merge".to_string(),
            ));
        }

        // Catch user silliness:
        if bkd_writer.finish {
            return Err(LuceneError::illegal_state("already finished".to_string()));
        }

        // Mark that we already finished:
        bkd_writer.finish = true;

        let data_start_fp = data_out.borrow().get_file_pointer();
        let leaf_values =
            vec![0u8; (config.max_points_in_leaf_node * config.packed_bytes_length()) as usize];
        let leaf_docs = vec![0i32; config.max_points_in_leaf_node as usize];
        let last_packed_value = vec![0u8; config.packed_bytes_length() as usize];

        Ok(OneDimensionBKDWriter {
            meta_out,
            index_out,
            data_out,
            data_start_fp,
            leaf_block_fps: Vec::new(),
            leaf_block_start_values: Vec::new(),
            leaf_values,
            leaf_docs,
            value_count: 0,
            leaf_count: 0,
            leaf_cardinality: 0,
            last_packed_value,
            last_doc_id: 0,
            bkd_writer,
        })
    }
    pub fn add(&mut self, packed_value: &[u8], doc_id: i32) -> Result<(), LuceneError> {
        debug_assert!(Self::value_in_order(
            self.bkd_writer.config.clone(),
            self.value_count + self.leaf_count as i64,
            0,
            self.last_packed_value.as_mut_slice(),
            packed_value,
            0,
            doc_id,
            self.last_doc_id
        ));

        if self.leaf_count == 0
            || !self.bkd_writer.equals_predicate.test(
                &self.leaf_values,
                ((self.leaf_count - 1) * self.bkd_writer.config.bytes_per_dim) as usize,
                packed_value,
                0,
            )
        {
            self.leaf_cardinality += 1;
        }

        let offset = (self.leaf_count * self.bkd_writer.config.packed_bytes_length()) as usize;
        let length = self.bkd_writer.config.packed_bytes_length() as usize;
        self.leaf_values[offset..offset + length].copy_from_slice(&packed_value[0..length]);

        self.leaf_docs[self.leaf_count as usize] = doc_id;
        // docsSeen.set(doc_id);
        self.leaf_count += 1;

        if self.value_count + self.leaf_count as i64 > self.bkd_writer.total_point_count {
            return Err(LuceneError::illegal_state(format!(
                "totalPointCount={} was passed when we were created, but we just hit {} values",
                self.bkd_writer.total_point_count,
                self.value_count + self.leaf_count as i64
            )));
        }

        if self.leaf_count == self.bkd_writer.config.max_points_in_leaf_node {
            self.write_leaf_block(self.leaf_cardinality)?;
            self.leaf_cardinality = 0;
            self.leaf_count = 0;
        }

        debug_assert!(doc_id >= 0);
        self.last_doc_id = doc_id;

        Ok(())
    }
    // only called from assert
    pub fn write_leaf_block(&mut self, leaf_cardinality: i32) -> Result<(), LuceneError> {
        debug_assert!(self.leaf_count != 0);

        if self.value_count == 0 {
            self.bkd_writer.min_packed_value
                [0..(self.bkd_writer.config.packed_index_bytes_length() as usize)]
                .copy_from_slice(
                    &self.leaf_values
                        [0..(self.bkd_writer.config.packed_index_bytes_length() as usize)],
                );
        }
        {
            let start = ((self.leaf_count - 1) * self.bkd_writer.config.packed_index_bytes_length())
                as usize;
            self.bkd_writer.max_packed_value
                [0..(self.bkd_writer.config.packed_index_bytes_length() as usize)]
                .copy_from_slice(
                    &self.leaf_values[start
                        ..start + self.bkd_writer.config.packed_index_bytes_length() as usize],
                );
        }

        self.value_count += self.leaf_count as i64;

        if !self.leaf_block_fps.is_empty() {
            // Save the first (minimum) value in each leaf block except the first, to build the split
            // value index in the end:
            self.leaf_block_start_values.push(
                self.leaf_values[0..(self.bkd_writer.config.packed_index_bytes_length() as usize)]
                    .to_vec(),
            );
        }
        self.leaf_block_fps
            .push(self.data_out.borrow().get_file_pointer());
        self.bkd_writer
            .check_max_leaf_node_count(self.leaf_block_fps.len())?;

        // Find per-dim common prefix:
        self.bkd_writer.common_prefix_lengths[0] =
            self.bkd_writer.common_prefix_comparator.compare(
                &self.leaf_values,
                0,
                &self.leaf_values,
                ((self.leaf_count - 1) * self.bkd_writer.config.packed_index_bytes_length())
                    as usize,
            );

        self.bkd_writer.write_leaf_block_docs(
            &mut *self.data_out.borrow_mut(),
            &self.leaf_docs,
            0,
            self.leaf_count,
        )?;
        self.bkd_writer.write_common_prefixes(
            &mut *self.data_out.borrow_mut(),
            &self.bkd_writer.common_prefix_lengths,
            &self.leaf_values,
        )?;

        self.bkd_writer.scratch_bytes_ref1.length =
            self.bkd_writer.config.packed_index_bytes_length();
        self.bkd_writer.scratch_bytes_ref1.bytes = self.leaf_values.clone();

        // let byte = &self.bkd_writer.scratch_bytes_ref1.bytes;
        // let offset = self.bkd_writer.config.packed_index_bytes_length();
        // let packed_values = move |i: i32| -> (&[u8], i32, i32) {
        //     let start = (offset * i) as usize;
        //     let end = start + offset as usize;
        //     (&byte[start..end], offset * i, offset)
        // };

        // debug_assert!(Self::values_in_order_and_bounds(
        //     self.bkd_writer.config.clone(),
        //     self.leaf_count,
        //     0,
        //     &self.leaf_values[0..(self.bkd_writer.config.packed_index_bytes_length() as usize)],
        //     &self.leaf_values[((self.leaf_count - 1)
        //         * self.bkd_writer.config.packed_index_bytes_length())
        //         as usize
        //         ..((self.leaf_count) * self.bkd_writer.config.packed_index_bytes_length())
        //             as usize],
        //     &packed_values,
        //     &self.leaf_docs,
        //     0
        // ));

        // self.bkd_writer.write_leaf_block_packed_values(
        //     &mut *self.data_out.borrow_mut(),
        //     &self.bkd_writer.common_prefix_lengths,
        //     self.leaf_count,
        //     0,
        //     &packed_values,
        //     leaf_cardinality,
        // )?;

        Ok(())
    }

    // only called from assert
    #[allow(clippy::too_many_arguments)]
    fn values_in_order_and_bounds<F>(
        config: Rc<BKDConfig>,
        count: i32,
        sorted_dim: i32,
        min_packed_value: &[u8],
        max_packed_value: &[u8],
        values: F,
        docs: &[i32],
        docs_offset: usize,
    ) -> bool
    where
        F: Fn(i32) -> (&'a [u8], i32, i32),
    {
        let mut last_packed_value = vec![0u8; config.packed_bytes_length() as usize];
        let mut last_doc = -1;
        for i in 0..count {
            let (bytes, offset, length) = values(i);
            debug_assert_eq!(length, config.packed_bytes_length());
            debug_assert!(Self::value_in_order(
                config.clone(),
                i as i64,
                sorted_dim,
                &mut last_packed_value,
                bytes,
                offset,
                docs[docs_offset + i as usize],
                last_doc
            ));
            last_doc = docs[docs_offset + i as usize];
            // Make sure this value does in fact fall within this leaf cell:
            debug_assert!(Self::value_in_bounds(
                config.clone(),
                bytes,
                offset,
                min_packed_value,
                max_packed_value
            ));
        }
        true
    }

    // only called from assert
    fn value_in_order(
        config: Rc<BKDConfig>,
        ord: i64,
        sorted_dim: i32,
        last_packed_value: &mut [u8],
        packed_value: &[u8],
        packed_value_offset: i32,
        doc: i32,
        last_doc: i32,
    ) -> bool {
        let dim_offset = (sorted_dim * config.bytes_per_dim) as usize;
        if ord > 0 {
            let cmp = last_packed_value[dim_offset..dim_offset + config.bytes_per_dim as usize]
                .cmp(
                    &packed_value[packed_value_offset as usize + dim_offset
                        ..(packed_value_offset + config.bytes_per_dim) as usize + dim_offset],
                )
                .to_int();
            if cmp > 0 {
                debug_assert!(
                    false,
                    "values out of order: last value={:?} current value={:?} ord={}",
                    BytesRef::from_bytes(last_packed_value.to_vec()),
                    BytesRef::from_vec(
                        packed_value.to_vec(),
                        packed_value_offset,
                        config.packed_index_bytes_length()
                    ),
                    ord
                );
            }
            if cmp == 0 && config.num_dims > config.num_index_dims {
                let cmp = last_packed_value[config.packed_index_bytes_length() as usize
                    ..config.packed_bytes_length() as usize]
                    .cmp(
                        &packed_value[(packed_value_offset + config.packed_index_bytes_length())
                            as usize
                            ..(packed_value_offset + config.packed_bytes_length()) as usize],
                    )
                    .to_int();

                if cmp > 0 {
                    debug_assert!(
                        false,
                        "data values out of order: last value={:?} current value={:?} ord={}",
                        BytesRef::from_bytes(last_packed_value.to_vec()),
                        BytesRef::from_vec(
                            packed_value.to_vec(),
                            packed_value_offset,
                            config.packed_index_bytes_length()
                        ),
                        ord
                    );
                }
            }
            if cmp == 0 && doc < last_doc {
                debug_assert!(
                    false,
                    "docs out of order: last doc={} current doc={} ord={}",
                    last_doc, doc, ord
                );
            }
        }
        last_packed_value[..(config.packed_bytes_length() as usize)].copy_from_slice(
            &packed_value[packed_value_offset as usize
                ..(packed_value_offset + config.packed_bytes_length()) as usize],
        );
        true
    }

    // only called from assert
    fn value_in_bounds(
        config: Rc<BKDConfig>,
        bytes: &[u8],
        bytes_offset: i32,
        min_packed_value: &[u8],
        max_packed_value: &[u8],
    ) -> bool {
        for dim in 0..config.num_index_dims {
            let offset = (config.bytes_per_dim * dim) as usize;
            let start = bytes_offset as usize + offset;
            let end = start + config.bytes_per_dim as usize;
            if bytes[start..end]
                .cmp(&min_packed_value[offset..offset + config.bytes_per_dim as usize])
                .to_int()
                < 0
            {
                return false;
            }
            if bytes[start..end]
                .cmp(&max_packed_value[offset..offset + config.bytes_per_dim as usize])
                .to_int()
                > 0
            {
                return false;
            }
        }
        true
    }
}

/// flat representation of a kd-tree
trait BKDTreeLeafNodes {
    /// number of leaf nodes
    fn num_leaves(&self) -> i32;

    /// pointer to the leaf node previously written. Leaves are order from left to right, so leaf at
    /// `index` 0 is the leftmost leaf and the leaf at `num_leaves()` - 1 is the rightmost
    fn get_leaf_lp(&self, index: i32) -> i64;

    /// split value between two leaves. The split value at position n corresponds to the leaves at (n
    /// -1) and n.
    fn get_split_value(&self, index: i32) -> (&[u8], i32, i32);

    /// split dimension between two leaves. The split dimension at position n corresponds to the
    /// leaves at (n -1) and n.
    fn get_split_dimension(&self, index: i32) -> i32;
}
struct BKDTreeLeafNodesOneDimension {
    scratch_bytes_ref1: BytesRef,
    leaf_block_fps: Rc<RefCell<Vec<i64>>>,
}
impl BKDTreeLeafNodes for BKDTreeLeafNodesOneDimension {
    fn num_leaves(&self) -> i32 {
        self.leaf_block_fps.borrow().len() as i32
    }

    fn get_leaf_lp(&self, index: i32) -> i64 {
        self.leaf_block_fps.borrow()[index as usize]
    }

    fn get_split_value(&self, index: i32) -> (&[u8], i32, i32) {
        (
            &self.scratch_bytes_ref1.bytes,
            self.scratch_bytes_ref1.offset,
            self.scratch_bytes_ref1.length,
        )
    }

    fn get_split_dimension(&self, _index: i32) -> i32 {
        0
    }
}
struct BKDTreeLeafNodesImpl {
    scratch_bytes_ref1: BytesRef,
    leaf_block_fps: Vec<i64>,
    split_dimension_values: Vec<u8>,
    config: Rc<BKDConfig>,
}
impl BKDTreeLeafNodes for BKDTreeLeafNodesImpl {
    fn num_leaves(&self) -> i32 {
        self.leaf_block_fps.len() as i32
    }

    fn get_leaf_lp(&self, index: i32) -> i64 {
        self.leaf_block_fps[index as usize]
    }

    fn get_split_value(&self, index: i32) -> (&[u8], i32, i32) {
        (
            &self.scratch_bytes_ref1.bytes,
            index * self.config.bytes_per_dim,
            self.scratch_bytes_ref1.length,
        )
    }

    fn get_split_dimension(&self, index: i32) -> i32 {
        self.split_dimension_values[index as usize] as i32
    }
}

enum BKDTreeLeafNodesEnum {
    OneDimension(BKDTreeLeafNodesOneDimension),
    MultiDimensions(BKDTreeLeafNodesImpl),
}
impl BKDTreeLeafNodes for BKDTreeLeafNodesEnum {
    fn num_leaves(&self) -> i32 {
        match self {
            BKDTreeLeafNodesEnum::OneDimension(leaf) => leaf.num_leaves(),
            BKDTreeLeafNodesEnum::MultiDimensions(leaf) => leaf.num_leaves(),
        }
    }

    fn get_leaf_lp(&self, index: i32) -> i64 {
        match self {
            BKDTreeLeafNodesEnum::OneDimension(leaf) => leaf.get_leaf_lp(index),
            BKDTreeLeafNodesEnum::MultiDimensions(leaf) => leaf.get_leaf_lp(index),
        }
    }

    fn get_split_value(&self, index: i32) -> (&[u8], i32, i32) {
        match self {
            BKDTreeLeafNodesEnum::OneDimension(leaf) => leaf.get_split_value(index),
            BKDTreeLeafNodesEnum::MultiDimensions(leaf) => leaf.get_split_value(index),
        }
    }

    fn get_split_dimension(&self, index: i32) -> i32 {
        match self {
            BKDTreeLeafNodesEnum::OneDimension(leaf) => leaf.get_split_dimension(index),
            BKDTreeLeafNodesEnum::MultiDimensions(leaf) => leaf.get_split_dimension(index),
        }
    }
}

pub(crate) struct IORunnableOneDimension;
impl IORunnable for IORunnableOneDimension {
    fn run(&self) -> Result<(), LuceneError> {
        todo!()
    }
}
pub(crate) struct IORunnableImpl;
impl IORunnable for IORunnableImpl {
    fn run(&self) -> Result<(), LuceneError> {
        todo!()
    }
}
pub(crate) enum IORunnableEnum {
    OneDimension(IORunnableOneDimension),
    MultiDimensions(IORunnableImpl),
}
impl IORunnable for IORunnableEnum {
    fn run(&self) -> Result<(), LuceneError> {
        match self {
            IORunnableEnum::OneDimension(runnable) => runnable.run(),
            IORunnableEnum::MultiDimensions(runnable) => runnable.run(),
        }
    }
}

trait PackedValues{
    fn apply(&mut self, i: i32) -> (&[u8],i32,i32);
}
struct ScratchBytesRefPackedValues{
    scratch_bytes_ref: BytesRef,
    config: Rc<BKDConfig>,
}
impl PackedValues for ScratchBytesRefPackedValues {
    fn apply(&mut self, i: i32) -> (&[u8],i32,i32) {
        self.scratch_bytes_ref.offset = self.config.packed_bytes_length() * i;
        (&self.scratch_bytes_ref.bytes, self.scratch_bytes_ref.offset, self.scratch_bytes_ref.length)
    }
}
struct MutablePointTreePackedValues{
    reader: Rc<RefCell<MutablePointTreeEnum>>,
    from:i32,
    scratch_bytes_ref1: BytesRef,
}
impl PackedValues for MutablePointTreePackedValues {
    fn apply(&mut self, i: i32) ->(&[u8],i32,i32) {
        {
            self.reader.borrow().get_value(i + self.from, &mut self.scratch_bytes_ref1);
        }
        (&self.scratch_bytes_ref1.bytes, self.scratch_bytes_ref1.offset, self.scratch_bytes_ref1.length)
    }
}
struct PointWriterPackedValues<D> where D:Directory{
    heap_source: Rc<RefCell<PointWriterEnum<D>>>,
    from: i32,
}
impl<D> PackedValues for PointWriterPackedValues<D> where D:Directory {
    fn apply(&mut self, i: i32) -> (&[u8], i32, i32) {
       todo!()
    }
}