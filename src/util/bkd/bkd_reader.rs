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
use crate::codecs::CodecUtil;
use crate::index::point_values::{IntersectVisitor, PointTree, PointValuesBase, Relation};
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::store::{DataInput, IndexInput};
use crate::util::array_util::{ArrayUtil, ByteArrayComparator};
use crate::util::bkd::bkd_config::BKDConfig;
use crate::util::bkd::bkd_writer::BKDWriter;
use crate::util::bkd::doc_ids_writer::DocIdsWriter;
use crate::util::error::lucene_error::LuceneError;
use crate::util::math_util::MathUtil;
use crate::util::VecCopyOps;
use std::cell::RefCell;
use std::rc::Rc;

/// Handles reading a block KD-tree in byte[] space previously written with `BKDWriter`
pub struct BKDReader<I>
where
    I: IndexInput,
{
    config: Rc<BKDConfig>,
    num_leaves: i32,
    index_in: Rc<RefCell<I>>,
    data_in: Rc<RefCell<I>>,
    min_packed_value: Vec<u8>,
    max_packed_value: Vec<u8>,
    point_count: i64,
    doc_count: i32,
    version: i32,
    min_leaf_block_fp: i64,
    index_start_pointer: i64,
    num_index_bytes: i32,
    is_tree_balanced: bool,
}

impl<I: IndexInput> BKDReader<I>
where
    I: IndexInput,
{
    /// Caller must pre-seek the provided `IndexInput` to the index location
    /// that `BKDWriter::finish()` returned. BKD tree is always stored off-heap.
    pub fn new(
        meta_in: Rc<RefCell<I>>,
        index_in: Rc<RefCell<I>>,
        data_in: Rc<RefCell<I>>,
    ) -> Result<Self, LuceneError> {
        let meta_in = &mut *meta_in.borrow_mut();
        let version = CodecUtil::check_header(
            meta_in,
            BKDWriter::CODEC_NAME,
            BKDWriter::VERSION_START,
            BKDWriter::VERSION_CURRENT,
        )?;

        let num_dims = meta_in.read_vint()?;
        let num_index_dims = if version >= BKDWriter::VERSION_SELECTIVE_INDEXING {
            meta_in.read_vint()?
        } else {
            num_dims
        };

        let max_points_in_leaf_node = meta_in.read_vint()?;
        let bytes_per_dim = meta_in.read_vint()?;
        let config = Rc::new(BKDConfig::new(
            num_dims,
            num_index_dims,
            bytes_per_dim,
            max_points_in_leaf_node,
        )?);

        // Read index:
        let num_leaves = meta_in.read_vint()?;
        debug_assert!(num_leaves > 0);
        let packed_index_bytes_length = config.packed_index_bytes_length();
        let mut min_packed_value = vec![0; packed_index_bytes_length as usize];
        let mut max_packed_value = vec![0; packed_index_bytes_length as usize];

        DataInput::read_bytes(meta_in, &mut min_packed_value, 0, packed_index_bytes_length)?;
        DataInput::read_bytes(meta_in, &mut max_packed_value, 0, packed_index_bytes_length)?;

        let bytes_per_dim = config.bytes_per_dim as usize;
        let comparator = ArrayUtil::get_unsigned_comparator(bytes_per_dim);
        for dim in 0..config.num_index_dims as usize {
            let offset = dim * bytes_per_dim;
            if comparator.compare(&min_packed_value, offset, &max_packed_value, offset) > 0 {
                return Err(LuceneError::corrupt_index(format!(
                    "minPackedValue {} is > maxPackedValue {} for dim={}, (resource={})",
                    BytesRef::from_bytes(min_packed_value),
                    BytesRef::from_bytes(max_packed_value),
                    dim,
                    meta_in
                )));
            }
        }

        let point_count = meta_in.read_vlong()?;
        let doc_count = meta_in.read_vint()?;
        let num_index_bytes = meta_in.read_vint()?;

        let (min_leaf_block_fp, index_start_pointer) = if version >= BKDWriter::VERSION_META_FILE {
            (
                DataInput::read_long(meta_in)?,
                DataInput::read_long(meta_in)?,
            )
        } else {
            let mut index_in = index_in.borrow_mut();
            let index_start_pointer = index_in.get_file_pointer();
            let min_leaf_block_fp = index_in.read_vlong()?;
            index_in.seek(index_start_pointer)?;
            (min_leaf_block_fp, index_start_pointer)
        };
        let mut reader = Self {
            config,
            num_leaves,
            index_in,
            data_in,
            min_packed_value,
            max_packed_value,
            point_count,
            doc_count,
            version,
            min_leaf_block_fp,
            index_start_pointer,
            num_index_bytes,
            is_tree_balanced: false,
        };
        reader.is_tree_balanced = num_leaves != 1 && reader.is_tree_balanced()?;
        Ok(reader)
    }
    /// Checks if the tree is balanced.
    fn is_tree_balanced(&self) -> Result<bool, LuceneError> {
        if self.version >= BKDWriter::VERSION_META_FILE {
            // Since Lucene 8.6 all trees are unbalanced.
            return Ok(false);
        }
        if self.config.num_dims > 1 {
            // High dimensional tree in pre-8.6 indices are balanced.
            debug_assert!((1 << MathUtil::log(self.num_leaves as i64, 2)?) == self.num_leaves);
            return Ok(true);
        }
        if (1 << MathUtil::log(self.num_leaves as i64, 2)?) != self.num_leaves {
            // If we don't have enough leaves to fill the last level then it is unbalanced.
            return Ok(false);
        }

        // Count of the last node for unbalanced trees.
        let last_leaf_node_point_count =
            (self.point_count % self.config.max_points_in_leaf_node as i64) as i32;

        // Navigate to last node.
        let mut point_tree = self.get_point_tree()?;
        while point_tree.move_to_sibling()? {}
        while point_tree.move_to_child()? {}

        // Count number of docs in the node.
        let mut count = vec![0; 1];
        let mut visitor = IntersectVisitorImpl { count: &mut count };
        point_tree.visit_doc_ids(&mut visitor)?;

        Ok(count[0] != last_leaf_node_point_count)
    }
}
impl<I> PointValuesBase for BKDReader<I>
where
    I: IndexInput,
{
    fn get_min_packed_value(&self) -> Result<Option<Vec<u8>>, LuceneError> {
        Ok(Option::from(self.min_packed_value.clone()))
    }

    fn get_max_packed_value(&self) -> Result<Option<Vec<u8>>, LuceneError> {
        Ok(Option::from(self.max_packed_value.clone()))
    }

    fn get_num_dimensions(&self) -> Result<i32, LuceneError> {
        Ok(self.config.num_dims)
    }

    fn get_num_index_dimensions(&self) -> Result<i32, LuceneError> {
        Ok(self.config.num_index_dims)
    }

    fn get_bytes_per_dimension(&self) -> Result<i32, LuceneError> {
        Ok(self.config.bytes_per_dim)
    }

    fn size(&self) -> Result<i64, LuceneError> {
        Ok(self.point_count)
    }

    fn get_doc_count(&self) -> Result<i32, LuceneError> {
        Ok(self.doc_count)
    }

    type PointTreeType = BKDPointTree<I>;

    fn get_point_tree(&self) -> Result<Self::PointTreeType, LuceneError> {
        let slice = self.index_in.borrow_mut().slice(
            "packedIndex",
            self.index_start_pointer,
            self.num_index_bytes as i64,
        )?;
        BKDPointTree::new(
            slice,
            self.data_in.clone(),
            self.config.clone(),
            self.num_leaves,
            self.version,
            self.point_count,
            &self.min_packed_value,
            &self.max_packed_value,
            self.is_tree_balanced,
        )
    }
}

pub struct BKDPointTree<I: IndexInput> {
    /// Current node ID in the tree.
    node_id: i32,
    /// During clone, the node root can be different from 1.
    node_root: i32,
    /// Level is 1-based so that we can do `level - 1` without checking each time.
    level: i32,
    /// Used to read the packed tree off-heap.
    inner_nodes: I::Slice,
    /// Used to read the packed leaves off-heap.
    leaf_nodes: Rc<RefCell<I>>,
    /// Holds the minimum (left-most) leaf block file pointer for each level we've recursed to.
    leaf_block_fp_stack: Vec<i64>,
    /// Holds the address, in the off-heap index, after reading the node data of each level.
    read_node_data_positions: Vec<i32>,
    /// Holds the address, in the off-heap index, of the right-node of each level.
    right_node_positions: Vec<i32>,
    /// Holds the splitDim position for each level.
    split_dims_pos: Vec<i32>,
    /// True if the per-dimension delta we read for the node at this level is a
    /// negative offset versus the last split on this dimension.
    /// This is a packed 2D array, i.e., to access `array[level][dim]`,
    /// you read from `negative_deltas[level * num_dims + dim]`.
    /// This will be true if the last time we split on this dimension,
    /// we next pushed to the left sub-tree.
    negative_deltas: Vec<bool>,
    /// Holds the packed per-level split values.
    split_values_stack: Vec<Vec<u8>>,
    /// Holds the min / max value of the current node.
    // TODO: 复制操作使用copy_from
    min_packed_value: Vec<u8>,
    max_packed_value: Vec<u8>,
    /// Holds the previous value of the split dimension.
    split_dim_value_stack: Vec<Vec<u8>>,
    /// Tree parameters.
    config: Rc<BKDConfig>,
    /// Number of leaves.
    leaf_node_offset: i32,
    /// Version of the index.
    version: i32,
    /// Total number of points.
    point_count: i64,
    /// Last node might not be fully populated.
    last_leaf_node_point_count: i32,
    /// Right-most leaf node ID.
    right_most_leaf_node: i32,
    /// Helper objects for reading doc values.
    scratch_data_packed_value: Vec<u8>,
    scratch_min_index_packed_value: Vec<u8>,
    scratch_max_index_packed_value: Vec<u8>,
    common_prefix_lengths: Vec<i32>,
    scratch_iterator: BKDReaderDocIDSetIterator,
    /// If true, the tree is balanced; otherwise, it is unbalanced.
    is_tree_balanced: bool,
}

impl<I> BKDPointTree<I>
where
    I: IndexInput,
{
    #[allow(clippy::too_many_arguments)]
    fn new(
        inner_nodes: I::Slice,
        leaf_nodes: Rc<RefCell<I>>,
        config: Rc<BKDConfig>,
        num_leaves: i32,
        version: i32,
        point_count: i64,
        min_packed_value: &[u8],
        max_packed_value: &[u8],
        is_tree_balanced: bool,
    ) -> Result<Self, LuceneError> {
        let packed_bytes_len = config.packed_bytes_length() as usize;
        let packed_index_bytes_len = config.packed_index_bytes_length() as usize;
        let num_dims = config.num_dims as usize;
        let disi_len = config.max_points_in_leaf_node;

        let mut tree = Self::with_scratch_iterator(
            inner_nodes,
            leaf_nodes,
            config,
            num_leaves,
            version,
            point_count,
            1,
            1,
            min_packed_value,
            max_packed_value,
            BKDReaderDocIDSetIterator::new(disi_len),
            vec![0; packed_bytes_len],
            vec![0; packed_index_bytes_len],
            vec![0; packed_index_bytes_len],
            vec![0; num_dims],
            is_tree_balanced,
        )?;
        tree.read_node_data(false)?;
        Ok(tree)
    }
    #[allow(clippy::too_many_arguments)]
    fn with_scratch_iterator(
        inner_nodes: I::Slice,
        leaf_nodes: Rc<RefCell<I>>,
        config: Rc<BKDConfig>,
        num_leaves: i32,
        version: i32,
        point_count: i64,
        node_id: i32,
        level: i32,
        min_packed_value: &[u8],
        max_packed_value: &[u8],
        scratch_iterator: BKDReaderDocIDSetIterator,
        scratch_data_packed_value: Vec<u8>,
        scratch_min_index_packed_value: Vec<u8>,
        scratch_max_index_packed_value: Vec<u8>,
        common_prefix_lengths: Vec<i32>,
        is_tree_balanced: bool,
    ) -> Result<Self, LuceneError> {
        // stack arrays that keep information at different levels
        let tree_depth = Self::get_tree_depth(num_leaves)? as usize;
        let split_values_stack =
            vec![vec![0; config.packed_index_bytes_length() as usize]; tree_depth];
        let right_most_leaf_node = (1 << (tree_depth - 1)) - 1;
        let last_leaf_node_point_count =
            i32::try_from(point_count % config.max_points_in_leaf_node as i64).map_err(|_| {
                LuceneError::integer_overflow(format!(
                    "too large: {}",
                    point_count % config.max_points_in_leaf_node as i64
                ))
            })?;
        let last_leaf_node_point_count = if last_leaf_node_point_count == 0 {
            config.max_points_in_leaf_node
        } else {
            last_leaf_node_point_count
        };
        let negative_deltas_len = config.num_index_dims as usize * tree_depth;

        Ok(BKDPointTree {
            config,
            version,
            node_id,
            node_root: node_id,
            level,
            is_tree_balanced,
            leaf_node_offset: num_leaves,
            inner_nodes,
            leaf_nodes,
            min_packed_value: min_packed_value.to_vec(),
            max_packed_value: max_packed_value.to_vec(),
            split_dim_value_stack: vec![vec![]; tree_depth],
            split_values_stack,
            leaf_block_fp_stack: vec![0; tree_depth + 1],
            read_node_data_positions: vec![0; tree_depth + 1],
            right_node_positions: vec![0; tree_depth],
            split_dims_pos: vec![0; tree_depth],
            negative_deltas: vec![false; negative_deltas_len],
            point_count,
            right_most_leaf_node,
            last_leaf_node_point_count,
            // scratch objects, reused between clones so NN search are not creating those objects
            // in every clone.
            scratch_iterator,
            common_prefix_lengths,
            scratch_data_packed_value,
            scratch_min_index_packed_value,
            scratch_max_index_packed_value,
        })
    }
    fn reset_node_data_position(&mut self) -> Result<(), LuceneError> {
        // move position of the inner nodes index to visit the first child
        let position = self.read_node_data_positions[self.level as usize] as i64;
        debug_assert!(position <= self.inner_nodes.get_file_pointer());
        self.inner_nodes.seek(position)?;
        Ok(())
    }
    fn push_bounds_left(&mut self) {
        let level = self.level as usize;
        let bytes_per_dim = self.config.bytes_per_dim as usize;
        let split_dim_pos = self.split_dims_pos[level] as usize;

        if self.split_dim_value_stack[level].is_empty() {
            self.split_dim_value_stack[level] = vec![0; bytes_per_dim];
        }
        // save the dimension we are going to change
        self.split_dim_value_stack[level][..bytes_per_dim]
            .copy_from_slice(&self.max_packed_value[split_dim_pos..split_dim_pos + bytes_per_dim]);

        debug_assert!(
            ArrayUtil::get_unsigned_comparator(bytes_per_dim).compare(
                &self.max_packed_value,
                split_dim_pos,
                &self.split_values_stack[level],
                split_dim_pos
            ) >= 0,
            "config.bytesPerDim={} splitDimPos={} config.numIndexDims={} config.numDims={}",
            self.config.bytes_per_dim,
            self.split_dims_pos[level],
            self.config.num_index_dims,
            self.config.num_dims
        );

        // add the split dim value:
        self.max_packed_value[split_dim_pos..split_dim_pos + bytes_per_dim].copy_from_slice(
            &self.split_values_stack[level][split_dim_pos..split_dim_pos + bytes_per_dim],
        );
    }
    fn push_left(&mut self) -> Result<(), LuceneError> {
        self.node_id *= 2;
        self.level += 1;
        self.read_node_data(true)
    }
    fn push_bounds_right(&mut self) {
        let level = self.level as usize;
        let bytes_per_dim = self.config.bytes_per_dim as usize;
        let split_dim_pos = self.split_dims_pos[level] as usize;
        // we should have already visited the left node
        debug_assert!(!self.split_dim_value_stack[level].is_empty());
        // save the dimension we are going to change
        self.split_dim_value_stack[level][..bytes_per_dim]
            .copy_from_slice(&self.min_packed_value[split_dim_pos..split_dim_pos + bytes_per_dim]);

        debug_assert!(
            ArrayUtil::get_unsigned_comparator(bytes_per_dim).compare(
                &self.min_packed_value,
                split_dim_pos,
                &self.split_values_stack[level],
                split_dim_pos
            ) <= 0,
            "config.bytesPerDim={} splitDimPos={} config.numIndexDims={} config.numDims={}",
            self.config.bytes_per_dim,
            self.split_dims_pos[level],
            self.config.num_index_dims,
            self.config.num_dims
        );
        // add the split dim value:
        self.min_packed_value[split_dim_pos..split_dim_pos + bytes_per_dim].copy_from_slice(
            &self.split_values_stack[level][split_dim_pos..split_dim_pos + bytes_per_dim],
        );
    }
    fn push_right(&mut self) -> Result<(), LuceneError> {
        let node_position = self.right_node_positions[self.level as usize] as i64;

        debug_assert!(
            node_position >= self.inner_nodes.get_file_pointer(),
            "nodePosition = {} < currentPosition={}",
            node_position,
            self.inner_nodes.get_file_pointer()
        );

        self.inner_nodes.seek(node_position)?;
        self.node_id = 2 * self.node_id + 1;
        self.level += 1;
        self.read_node_data(false)
    }
    fn pop(&mut self) {
        self.node_id /= 2;
        self.level -= 1;
    }

    fn pop_bounds(&mut self, is_left: bool) {
        let level = self.level as usize;
        let split_dim_pos = self.split_dims_pos[level] as usize;
        let bytes_per_dim = self.config.bytes_per_dim as usize;

        if is_left {
            self.max_packed_value[split_dim_pos..split_dim_pos + bytes_per_dim]
                .copy_from_slice(&self.split_dim_value_stack[level][..bytes_per_dim]);
        } else {
            self.min_packed_value[split_dim_pos..split_dim_pos + bytes_per_dim]
                .copy_from_slice(&self.split_dim_value_stack[level][..bytes_per_dim]);
        }
    }
    fn is_root_node(&self) -> bool {
        self.node_id == self.node_root
    }

    fn is_left_node(&self) -> bool {
        (self.node_id & 1) == 0
    }

    fn is_leaf_node(&self) -> bool {
        self.node_id >= self.leaf_node_offset
    }

    fn node_exists(&self) -> bool {
        self.node_id - self.leaf_node_offset < self.leaf_node_offset
    }
    /// Only valid after pushLeft or pushRight, not pop!.
    fn get_leaf_block_fp(&self) -> Result<i64, LuceneError> {
        debug_assert!(self.is_leaf_node(), "nodeID={} is not a leaf", self.node_id);
        Ok(self.leaf_block_fp_stack[self.level as usize])
    }
    fn size_from_balanced_tree(
        &self,
        left_most_leaf_node: i32,
        right_most_leaf_node: i32,
    ) -> Result<i64, LuceneError> {
        // number of points that need to be distributed between leaves, one per leaf
        let extra_points = i32::try_from(
            self.config.max_points_in_leaf_node as i64 * self.leaf_node_offset as i64
                - self.point_count,
        )
        .map_err(|_| {
            LuceneError::integer_overflow(format!(
                "value too large: {}",
                self.inner_nodes.get_file_pointer()
            ))
        })?;

        debug_assert!(
            extra_points < self.leaf_node_offset,
            "point excess should be lower than leafNodeOffset"
        );

        // offset where we stop adding one point to the leaves
        let node_offset = self.leaf_node_offset - extra_points;
        let mut count: i64 = 0;

        for node in left_most_leaf_node..=right_most_leaf_node {
            // offsetPosition provides which extra point will be added to this node
            if self.balance_tree_node_position(
                0,
                self.leaf_node_offset,
                node - self.leaf_node_offset,
                0,
                0,
            ) < node_offset
            {
                count += self.config.max_points_in_leaf_node as i64;
            } else {
                count += (self.config.max_points_in_leaf_node - 1) as i64;
            }
        }
        Ok(count)
    }
    fn balance_tree_node_position(
        &self,
        min_node: i32,
        max_node: i32,
        node: i32,
        position: i32,
        level: i32,
    ) -> i32 {
        if max_node - min_node == 1 {
            return position;
        }
        let mid = (min_node + max_node + 1) / 2;
        if mid > node {
            self.balance_tree_node_position(min_node, mid, node, position, level + 1)
        } else {
            self.balance_tree_node_position(mid, max_node, node, position + (1 << level), level + 1)
        }
    }
    fn add_all(
        &mut self,
        visitor: &mut impl IntersectVisitor,
        mut grown: bool,
    ) -> Result<(), LuceneError> {
        if !grown {
            let size = self.size()?;
            if size <= i32::MAX as i64 {
                visitor.grow(size as i32);
                grown = true;
            }
        }

        if self.is_leaf_node() {
            let mut leaf_nodes = self.leaf_nodes.borrow_mut();
            // Leaf node
            let leaf_fp = self.get_leaf_block_fp()?;
            leaf_nodes.seek(leaf_fp)?;
            // How many points are stored in this leaf cell:
            let count = leaf_nodes.read_vint()?;
            // No need to call grow(), it has been called up-front
            self.scratch_iterator
                .doc_ids_writer
                .read_ints_with_visitor(&mut *leaf_nodes, count, visitor)?;
        } else {
            self.push_left()?;
            self.add_all(visitor, grown)?;
            self.pop();
            self.push_right()?;
            self.add_all(visitor, grown)?;
            self.pop();
        }

        Ok(())
    }
    fn visit_leaves_one_by_one(
        &mut self,
        visitor: &mut impl IntersectVisitor,
    ) -> Result<(), LuceneError> {
        if self.is_leaf_node() {
            let leaf_fp = self.get_leaf_block_fp()?;
            self.visit_doc_values(visitor, leaf_fp)?;
        } else {
            self.push_left()?;
            self.visit_leaves_one_by_one(visitor)?;
            self.pop();

            self.push_right()?;
            self.visit_leaves_one_by_one(visitor)?;
            self.pop();
        }
        Ok(())
    }

    fn visit_doc_values(
        &mut self,
        visitor: &mut impl IntersectVisitor,
        fp: i64,
    ) -> Result<(), LuceneError> {
        let count = self.read_doc_ids(fp)?;

        if self.version >= BKDWriter::VERSION_LOW_CARDINALITY_LEAVES {
            self.visit_doc_values_with_cardinality(count, visitor)?;
        } else {
            self.visit_doc_values_no_cardinality(count, visitor)?;
        }

        Ok(())
    }

    fn read_doc_ids(&mut self, block_fp: i64) -> Result<i32, LuceneError> {
        let mut index_input = self.leaf_nodes.borrow_mut();
        index_input.seek(block_fp)?;
        let count = index_input.read_vint()?;
        self.scratch_iterator.doc_ids_writer.read_ints(
            &mut *index_input,
            count,
            &mut self.scratch_iterator.doc_ids,
        )?;
        Ok(count)
    }

    fn get_num_leaves_slow(&self, node: i32) -> i32 {
        if node >= 2 * self.leaf_node_offset {
            0
        } else if node >= self.leaf_node_offset {
            1
        } else {
            let left_count = self.get_num_leaves_slow(node * 2);
            let right_count = self.get_num_leaves_slow(node * 2 + 1);
            left_count + right_count
        }
    }

    fn read_node_data(&mut self, is_left: bool) -> Result<(), LuceneError> {
        self.leaf_block_fp_stack[self.level as usize] =
            self.leaf_block_fp_stack[(self.level - 1) as usize];
        if !is_left {
            // Read leaf block FP delta
            self.leaf_block_fp_stack[self.level as usize] += self.inner_nodes.read_vlong()?;
        }

        if !self.is_leaf_node() {
            let num_index_dims = self.config.num_index_dims as usize;
            let level = self.level as usize;

            // Copy the negative deltas from the previous level
            let prev_offset = (level - 1) * num_index_dims;
            let curr_offset = level * num_index_dims;
            self.negative_deltas
                .copy_within(prev_offset..prev_offset + num_index_dims, curr_offset);
            self.negative_deltas[curr_offset
                + (self.split_dims_pos[level - 1] / self.config.bytes_per_dim) as usize] = is_left;

            // Clone or copy the previous level's split values
            if self.split_values_stack[level].is_empty() {
                self.split_values_stack[level] = self.split_values_stack[level - 1].clone();
            } else {
                let (before, after) = self.split_values_stack.split_at_mut(level);
                let source = &before[level - 1][..self.config.packed_index_bytes_length() as usize];
                after[0].copy_from(source, 0);
            }

            // Read split dim, prefix, and firstDiffByteDelta encoded as an int
            let mut code = self.inner_nodes.read_vint()?;
            let split_dim = code % self.config.num_index_dims;
            self.split_dims_pos[level] = split_dim * self.config.bytes_per_dim;
            code /= self.config.num_index_dims;
            let prefix = code % (1 + self.config.bytes_per_dim);
            let suffix = self.config.bytes_per_dim - prefix;

            if suffix > 0 {
                let mut first_diff_byte_delta = code / (1 + self.config.bytes_per_dim);
                if self.negative_deltas[curr_offset + split_dim as usize] {
                    first_diff_byte_delta = -first_diff_byte_delta;
                }
                let start_pos = self.split_dims_pos[level] + prefix;
                let old_byte = self.split_values_stack[level][start_pos as usize] as i32;
                self.split_values_stack[level][start_pos as usize] =
                    (old_byte + first_diff_byte_delta) as u8;
                DataInput::read_bytes(
                    &mut self.inner_nodes,
                    &mut self.split_values_stack[level],
                    start_pos + 1,
                    suffix - 1,
                )?;
            } else {
                // Our split value is == last split value in this dim, which can happen when there are
                // many duplicate values.
            }

            let left_num_bytes = if self.node_id * 2 < self.leaf_node_offset {
                self.inner_nodes.read_vint()?
            } else {
                0
            };
            let file_pointer =
                i32::try_from(self.inner_nodes.get_file_pointer()).map_err(|_| {
                    LuceneError::integer_overflow(format!(
                        "value too large: {}",
                        self.inner_nodes.get_file_pointer()
                    ))
                })?;
            self.right_node_positions[level] = file_pointer + left_num_bytes;
            self.read_node_data_positions[level] = file_pointer;
        }
        Ok(())
    }
    /// Computes the depth of the tree based on the number of leaves.
    ///
    /// - The first `+1` accounts for the fact that all non-leaf nodes form another power of 2.
    ///   For example, to have a fully balanced tree with 4 leaves, you need a tree of depth 3.
    /// - The second `+1` ensures that the depth is correctly calculated, as `log2(num_leaves)`
    ///   computes the floor of the logarithm. For example, with 5 leaves, you need a tree of depth 4.
    fn get_tree_depth(num_leaves: i32) -> Result<i32, LuceneError> {
        Ok(MathUtil::log(num_leaves as i64, 2)? + 2)
    }
    fn visit_doc_values_no_cardinality(
        &mut self,
        count: i32,
        visitor: &mut impl IntersectVisitor,
    ) -> Result<(), LuceneError> {
        let packed_index_bytes_length = self.config.packed_index_bytes_length() as usize;

        self.read_common_prefixes()?;

        if self.config.num_index_dims > 1 && self.version >= BKDWriter::VERSION_LEAF_STORES_BOUNDS {
            self.scratch_max_index_packed_value[..packed_index_bytes_length]
                .copy_from_slice(&self.scratch_data_packed_value[..packed_index_bytes_length]);
            self.scratch_max_index_packed_value[..packed_index_bytes_length]
                .copy_from_slice(&self.scratch_min_index_packed_value[..packed_index_bytes_length]);

            self.read_min_max()?;

            // The index gives us range of values for each dimension, but the actual range of values
            // might be much more narrow than what the index told us, so we double check the relation
            // here, which is cheap yet might help figure out that the block either entirely matches
            // or does not match at all. This is especially more likely in the case that there are
            // multiple dimensions that have correlation, ie. splitting on one dimension also
            // significantly changes the range of values in another dimension.
            let relation = visitor.compare(
                &self.scratch_min_index_packed_value,
                &self.scratch_max_index_packed_value,
            )?;
            if relation == Relation::CellOutsideQuery {
                return Ok(());
            }
            visitor.grow(count);

            if relation == Relation::CellInsideQuery {
                for i in 0..count as usize {
                    visitor.visit(self.scratch_iterator.doc_ids[i])?;
                }
                return Ok(());
            }
        } else {
            visitor.grow(count);
        }

        let compressed_dim = self.read_compressed_dim()?;

        if compressed_dim == -1 {
            self.visit_unique_raw_doc_values(count, visitor)?;
        } else {
            self.visit_compressed_doc_values(count, visitor, compressed_dim)?;
        }

        Ok(())
    }
    fn visit_doc_values_with_cardinality(
        &mut self,
        count: i32,
        visitor: &mut impl IntersectVisitor,
    ) -> Result<(), LuceneError> {
        let packed_index_bytes_length = self.config.packed_index_bytes_length() as usize;
        self.read_common_prefixes()?;
        let compressed_dim = self.read_compressed_dim()?;
        if compressed_dim == -1 {
            // all values are the same
            visitor.grow(count)?;
            self.visit_unique_raw_doc_values(count, visitor)?;
        } else {
            if self.config.num_index_dims != 1 {
                self.scratch_min_index_packed_value[..packed_index_bytes_length]
                    .copy_from_slice(&self.scratch_data_packed_value[..packed_index_bytes_length]);

                self.scratch_max_index_packed_value[..packed_index_bytes_length].copy_from_slice(
                    &self.scratch_min_index_packed_value[..packed_index_bytes_length],
                );

                self.read_min_max()?;

                // The index gives us range of values for each dimension, but the actual range of values
                // might be much more narrow than what the index told us, so we double check the relation
                // here, which is cheap yet might help figure out that the block either entirely matches
                // or does not match at all. This is especially more likely in the case that there are
                // multiple dimensions that have correlation, ie. splitting on one dimension also
                // significantly changes the range of values in another dimension.
                let relation = visitor.compare(
                    &self.scratch_min_index_packed_value,
                    &self.scratch_max_index_packed_value,
                )?;
                if relation == Relation::CellOutsideQuery {
                    return Ok(());
                }
                visitor.grow(count)?;

                if relation == Relation::CellInsideQuery {
                    for i in 0..count as usize {
                        visitor.visit(self.scratch_iterator.doc_ids[i])?;
                    }
                    return Ok(());
                }
            } else {
                visitor.grow(count)?;
            }

            if compressed_dim == -2 {
                // low cardinality values
                self.visit_sparse_raw_doc_values(count, visitor)?;
            } else {
                // high cardinality
                self.visit_compressed_doc_values(count, visitor, compressed_dim)?;
            }
        }

        Ok(())
    }
    fn read_min_max(&mut self) -> Result<(), LuceneError> {
        let index_input = &mut *self.leaf_nodes.borrow_mut();
        for dim in 0..self.config.num_index_dims {
            let prefix = self.common_prefix_lengths[dim as usize];
            DataInput::read_bytes(
                index_input,
                &mut self.scratch_min_index_packed_value,
                dim * self.config.bytes_per_dim + prefix,
                self.config.bytes_per_dim - prefix,
            )?;
            DataInput::read_bytes(
                index_input,
                &mut self.scratch_max_index_packed_value,
                dim * self.config.bytes_per_dim + prefix,
                self.config.bytes_per_dim - prefix,
            )?;
        }

        Ok(())
    }

    // read cardinality and point
    fn visit_sparse_raw_doc_values(
        &mut self,
        count: i32,
        visitor: &mut impl IntersectVisitor,
    ) -> Result<(), LuceneError> {
        let mut i = 0;
        {
            let index_input = &mut *self.leaf_nodes.borrow_mut();
            while i < count {
                let length = DataInput::read_vint(index_input)?;
                for dim in 0..self.config.num_dims {
                    let prefix = self.common_prefix_lengths[dim as usize];
                    DataInput::read_bytes(
                        index_input,
                        &mut self.scratch_data_packed_value,
                        dim * self.config.bytes_per_dim + prefix,
                        self.config.bytes_per_dim - prefix,
                    )?;
                }
                self.scratch_iterator.reset(i, length);
                visitor.visit_iterator_with_packed_value(
                    &mut self.scratch_iterator,
                    &self.scratch_data_packed_value,
                )?;
                i += length;
            }
        }

        if i != count {
            return Err(LuceneError::corrupt_index(format!(
                "Sub blocks do not add up to the expected count: {} != {}, (resource={})",
                count,
                i,
                self.leaf_nodes.borrow()
            )));
        }

        Ok(())
    }

    // point is under commonPrefix
    pub fn visit_unique_raw_doc_values(
        &mut self,
        count: i32,
        visitor: &mut impl IntersectVisitor,
    ) -> Result<(), LuceneError> {
        self.scratch_iterator.reset(0, count);
        visitor.visit_iterator_with_packed_value(
            &mut self.scratch_iterator,
            &self.scratch_data_packed_value,
        )?;
        Ok(())
    }
    fn visit_compressed_doc_values(
        &mut self,
        count: i32,
        visitor: &mut impl IntersectVisitor,
        compressed_dim: i32,
    ) -> Result<(), LuceneError> {
        let bytes_per_dim = self.config.bytes_per_dim as usize;
        let compressed_dim = compressed_dim as usize;

        // the byte at `compressedByteOffset` is compressed using run-length compression,
        // other suffix bytes are stored verbatim
        let compressed_byte_offset =
            compressed_dim * bytes_per_dim + self.common_prefix_lengths[compressed_dim] as usize;
        self.common_prefix_lengths[compressed_dim] += 1;

        let mut i = 0;
        {
            let index_input = &mut *self.leaf_nodes.borrow_mut();
            while i < count {
                self.scratch_data_packed_value[compressed_byte_offset] =
                    DataInput::read_byte(index_input)?;
                let run_len = DataInput::read_byte(index_input)? as usize;
                for j in 0..run_len {
                    for dim in 0..self.config.num_dims {
                        let prefix = self.common_prefix_lengths[dim as usize];
                        DataInput::read_bytes(
                            index_input,
                            &mut self.scratch_data_packed_value,
                            dim * self.config.bytes_per_dim + prefix,
                            self.config.bytes_per_dim - prefix,
                        )?;
                    }
                    visitor.visit_with_packed_value(
                        self.scratch_iterator.doc_ids[i as usize + j],
                        &self.scratch_data_packed_value,
                    )?;
                }
                i += run_len as i32;
            }
        }

        if i != count {
            return Err(LuceneError::corrupt_index(format!(
                "Sub blocks do not add up to the expected count: {} != {}, (resource={})",
                count,
                i,
                self.leaf_nodes.borrow()
            )));
        }

        Ok(())
    }
    fn read_compressed_dim(&mut self) -> Result<i32, LuceneError> {
        let compressed_dim = DataInput::read_byte(&mut *self.leaf_nodes.borrow_mut())? as i8 as i32;

        if compressed_dim < -2
            || compressed_dim >= self.config.num_dims
            || (self.version < BKDWriter::VERSION_LOW_CARDINALITY_LEAVES && compressed_dim == -2)
        {
            return Err(LuceneError::corrupt_index(format!(
                "Got compressedDim={} from input, (resource={})",
                compressed_dim,
                self.leaf_nodes.borrow()
            )));
        }

        Ok(compressed_dim)
    }

    pub fn read_common_prefixes(&mut self) -> Result<(), LuceneError> {
        let num_dims = self.config.num_dims;
        let index_input = &mut *self.leaf_nodes.borrow_mut();
        for dim in 0..num_dims {
            let prefix = index_input.read_vint()?;
            self.common_prefix_lengths[dim as usize] = prefix;
            if prefix > 0 {
                DataInput::read_bytes(
                    index_input,
                    &mut self.scratch_data_packed_value,
                    dim * self.config.bytes_per_dim,
                    prefix,
                )?;
            }
        }

        Ok(())
    }
}

impl<I> Clone for BKDPointTree<I>
where
    I: IndexInput,
{
    fn clone(&self) -> Self {
        // TODO: do we need this?
        unimplemented!()
    }
}

impl<I> PointTree for BKDPointTree<I>
where
    I: IndexInput,
{
    fn move_to_child(&mut self) -> Result<bool, LuceneError> {
        if self.is_leaf_node() {
            return Ok(false);
        }
        self.reset_node_data_position()?;
        self.push_bounds_left();
        self.push_left()?;
        Ok(true)
    }

    fn move_to_sibling(&mut self) -> Result<bool, LuceneError> {
        if !self.is_left_node() || self.is_root_node() {
            return Ok(false);
        }

        self.pop();
        self.pop_bounds(true);
        self.push_bounds_right();
        self.push_right()?;

        debug_assert!(self.node_exists(), "Sibling node must exist");
        Ok(true)
    }

    fn move_to_parent(&mut self) -> Result<bool, LuceneError> {
        if self.is_root_node() {
            return Ok(false);
        }
        let is_left = self.is_left_node();
        self.pop();
        self.pop_bounds(is_left);
        Ok(true)
    }

    fn get_min_packed_value(&self) -> Result<&[u8], LuceneError> {
        Ok(&self.min_packed_value)
    }

    fn get_max_packed_value(&self) -> Result<&[u8], LuceneError> {
        Ok(&self.max_packed_value)
    }

    fn size(&self) -> Result<i64, LuceneError> {
        let mut left_most_leaf_node = self.node_id;
        while left_most_leaf_node < self.leaf_node_offset {
            left_most_leaf_node *= 2;
        }

        let mut right_most_leaf_node = self.node_id;
        while right_most_leaf_node < self.leaf_node_offset {
            right_most_leaf_node = right_most_leaf_node * 2 + 1;
        }

        let num_leaves = if right_most_leaf_node >= left_most_leaf_node {
            // both are on the same level
            right_most_leaf_node - left_most_leaf_node + 1
        } else {
            // left is one level deeper than right
            right_most_leaf_node - left_most_leaf_node + 1 + self.leaf_node_offset
        };

        debug_assert!(
            num_leaves == self.get_num_leaves_slow(self.node_id),
            "numLeaves mismatch: {} vs {}",
            num_leaves,
            self.get_num_leaves_slow(self.node_id)
        );

        if self.is_tree_balanced {
            // before lucene 8.6, trees might have been constructed as fully balanced trees.
            return self.size_from_balanced_tree(left_most_leaf_node, right_most_leaf_node);
        }

        // size for an unbalanced tree.
        let size = if right_most_leaf_node == self.right_most_leaf_node {
            (num_leaves as i64 - 1) * self.config.max_points_in_leaf_node as i64
                + self.last_leaf_node_point_count as i64
        } else {
            num_leaves as i64 * self.config.max_points_in_leaf_node as i64
        };

        Ok(size)
    }

    fn visit_doc_ids(&mut self, visitor: &mut impl IntersectVisitor) -> Result<(), LuceneError> {
        self.reset_node_data_position()?;
        self.add_all(visitor, false)
    }

    fn visit_doc_values(&mut self, visitor: &mut impl IntersectVisitor) -> Result<(), LuceneError> {
        self.reset_node_data_position()?;
        self.visit_leaves_one_by_one(visitor)
    }
}
/// Reusable [`DocIdSetIterator`] to handle low cardinality leaves.
struct BKDReaderDocIDSetIterator {
    idx: i32,
    length: i32,
    offset: i32,
    doc_id: i32,
    doc_ids: Vec<i32>,
    doc_ids_writer: DocIdsWriter,
}

impl BKDReaderDocIDSetIterator {
    pub fn new(max_points_in_leaf_node: i32) -> Self {
        Self {
            idx: 0,
            length: 0,
            offset: 0,
            doc_id: -1,
            doc_ids: vec![0; max_points_in_leaf_node as usize],
            doc_ids_writer: DocIdsWriter::new(max_points_in_leaf_node),
        }
    }
    fn reset(&mut self, offset: i32, length: i32) {
        self.offset = offset;
        self.length = length;
        debug_assert!((offset + length) as usize <= self.doc_ids.len());
        self.doc_id = -1;
        self.idx = 0;
    }
}
impl DocIdSetIterator for BKDReaderDocIDSetIterator {
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32, LuceneError> {
        if self.idx == self.length {
            self.doc_id = NO_MORE_DOCS;
        } else {
            self.doc_id = self.doc_ids[(self.offset + self.idx) as usize];
            self.idx += 1;
        }
        Ok(self.doc_id)
    }

    fn advance(&mut self, target: i32) -> Result<i32, LuceneError> {
        DocIdSetIterator::slow_advance(self, target)
    }

    fn cost(&self) -> Result<i64, LuceneError> {
        Ok(self.length as i64)
    }
}

struct IntersectVisitorImpl<'a> {
    count: &'a mut [i32],
}
impl IntersectVisitor for IntersectVisitorImpl<'_> {
    fn visit(&mut self, _doc_id: i32) -> Result<(), LuceneError> {
        self.count[0] += 1;
        Ok(())
    }

    fn visit_with_packed_value(
        &mut self,
        _doc_id: i32,
        _packed_value: &[u8],
    ) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented(""))
    }

    fn compare(
        &self,
        _min_packed_value: &[u8],
        _max_packed_value: &[u8],
    ) -> Result<Relation, LuceneError> {
        Err(LuceneError::not_implemented(""))
    }
}
#[cfg(test)]
pub mod tests {
    use crate::index::merge_state::{DocMap, DocMapEnum};
    use crate::index::point_values::{
        IntersectVisitor, PointTree, PointValues, PointValuesBase, Relation,
    };
    use crate::index::BytesRef;
    use crate::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
    use crate::store::directory::Directory;
    use crate::store::{IOContext, IndexInput, IndexOutput};
    use crate::test::util::lucene_test_case::{at_least, new_directory, random, rarely};
    use crate::test::util::test_util::TestUtil;
    use crate::util::bit_util::BitUtil;
    use crate::util::bkd::bkd_config::BKDConfig;
    use crate::util::bkd::bkd_reader::BKDReader;
    use crate::util::bkd::bkd_writer::BKDWriter;
    use crate::util::error::lucene_error::LuceneError;
    use crate::util::numeric_utils::NumericUtils;
    use crate::util::ToInt;
    use bit_set::BitSet;
    use num_bigint::{BigInt, Sign};
    use num_traits::Zero;
    use rand::rngs::StdRng;
    use rand::{Rng, RngCore};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[allow(dead_code)] // for quick search
    struct TestBKD;

    fn get_point_values<I: IndexInput>(
        index_input: Rc<RefCell<I>>,
    ) -> Result<BKDReader<I>, LuceneError> {
        BKDReader::new(index_input.clone(), index_input.clone(), index_input)
    }
    #[test]
    fn test_basic_ints_1d() -> Result<(), LuceneError> {
        let mut random = random();
        let config = Rc::new(BKDConfig::new(1, 1, 4, 2)?);
        let dir = Rc::new(RefCell::new(new_directory(&mut random)?));

        {
            let mut writer = BKDWriter::new(100, dir.clone(), "tmp", config.clone(), 1.0, 100)?;
            let mut scratch = [0u8; 4];

            for doc_id in 0..100 {
                NumericUtils::int_to_sortable_bytes(doc_id, &mut scratch, 0);
                writer.add(&scratch, doc_id)?;
            }

            let index_fp;
            {
                let out = Rc::new(RefCell::new(
                    dir.borrow_mut()
                        .create_output("bkd", &IOContext::default_io_context()?)?,
                ));
                let finalizer = writer.finish(out.clone())?.unwrap();
                {
                    index_fp = out.borrow().get_file_pointer();
                }
                writer.write_index(out.clone(), out.clone(), &finalizer)?;
            }

            {
                let mut input = dir
                    .borrow_mut()
                    .open_input("bkd", &IOContext::default_io_context()?)?;
                input.seek(index_fp)?;
                let sub_point_values = get_point_values(Rc::new(RefCell::new(input)))?;

                // Simple 1D range query:
                let mut query_min = vec![vec![0u8; 4]];
                NumericUtils::int_to_sortable_bytes(42, &mut query_min[0], 0);
                let mut query_max = vec![vec![0u8; 4]];
                NumericUtils::int_to_sortable_bytes(87, &mut query_max[0], 0);

                let mut hits = BitSet::new();
                let mut visitor = IntersectVisitorImpl {
                    hits: &mut hits,
                    query_min: &query_min,
                    query_max: &query_max,
                    config: config.clone(),
                    random: &mut random,
                };
                let r = PointValues::new(sub_point_values);
                r.intersect(&mut visitor)?;

                for doc_id in 0..100 {
                    let expected = (42..=87).contains(&doc_id);
                    let actual = hits.contains(doc_id);
                    assert_eq!(expected, actual, "docID={}", doc_id);
                }
            }
        }

        Ok(())
    }

    #[test]
    fn test_random_ints_n_dims() -> Result<(), LuceneError> {
        let mut random = random();
        let num_docs = at_least(&mut random, 1000);
        let dir = Rc::new(RefCell::new(new_directory(&mut random)?));

        let num_dims = TestUtil::next_int(&mut random, 1, 5);
        let num_index_dims = TestUtil::next_int(&mut random, 1, num_dims);
        let max_points_in_leaf_node = TestUtil::next_int(&mut random, 50, 100);
        let max_mb: f32 = 3.0 + (3.0 * random.random::<f32>());
        let config = Rc::new(BKDConfig::new(
            num_dims,
            num_index_dims,
            4,
            max_points_in_leaf_node,
        )?);
        let mut writer = BKDWriter::new(
            num_docs,
            dir.clone(),
            "tmp",
            config.clone(),
            max_mb as f64,
            num_docs as i64,
        )?;
        let num_dims = num_dims as usize;
        let mut docs = vec![vec![]; num_docs as usize];
        let mut scratch = vec![0u8; 4 * num_dims];
        let mut min_value = vec![i32::MAX; num_dims];
        let mut max_value = vec![i32::MIN; num_dims];

        for doc_id in 0..num_docs {
            let mut values = vec![0; num_dims];
            if cfg!(feature = "test_log_verbose") {
                println!("doc_id={}", doc_id);
            }
            for dim in 0..num_dims {
                values[dim] = random.random();
                min_value[dim] = min_value[dim].min(values[dim]);
                max_value[dim] = max_value[dim].max(values[dim]);
                NumericUtils::int_to_sortable_bytes(
                    values[dim],
                    &mut scratch,
                    dim * BitUtil::INT_BYTES,
                );
                if cfg!(feature = "test_log_verbose") {
                    println!("    {} -> {}", doc_id, values[dim]);
                }
            }
            docs[doc_id as usize] = values;
            writer.add(&scratch, doc_id)?;
        }

        let index_fp;
        {
            let out = Rc::new(RefCell::new(
                dir.borrow_mut()
                    .create_output("bkd", &IOContext::default_io_context()?)?,
            ));
            let finalizer = writer.finish(out.clone())?.unwrap();
            {
                index_fp = out.borrow().get_file_pointer();
            }
            writer.write_index(out.clone(), out.clone(), &finalizer)?;
        }

        {
            let mut input = dir
                .borrow_mut()
                .open_input("bkd", &IOContext::default_io_context()?)?;
            input.seek(index_fp)?;
            let sub_point_values = get_point_values(Rc::new(RefCell::new(input)))?;
            let r = PointValues::new(sub_point_values);

            let min_packed_value = r.get_min_packed_value()?.unwrap();
            let max_packed_value = r.get_max_packed_value()?.unwrap();
            for dim in 0..num_index_dims as usize {
                assert_eq!(
                    min_value[dim],
                    NumericUtils::sortable_bytes_to_int(
                        &min_packed_value,
                        dim * BitUtil::INT_BYTES
                    ),
                    "Mismatch in min value for dim {}",
                    dim
                );
                assert_eq!(
                    max_value[dim],
                    NumericUtils::sortable_bytes_to_int(
                        &max_packed_value,
                        dim * BitUtil::INT_BYTES
                    ),
                    "Mismatch in max value for dim {}",
                    dim
                );
            }

            let iters = at_least(&mut random, 100);
            for iter in 0..iters {
                if cfg!(feature = "test_log_verbose") {
                    println!("TEST: iter={}", iter);
                }
                let mut query_min = vec![0; num_dims];
                let mut query_min_bytes = vec![vec![0u8; 4]; num_dims];
                let mut query_max = vec![0; num_dims];
                let mut query_max_bytes = vec![vec![0u8; 4]; num_dims];

                for dim in 0..num_index_dims as usize {
                    query_min[dim] = random.random();
                    query_max[dim] = random.random();
                    if query_min[dim] > query_max[dim] {
                        std::mem::swap(&mut query_min[dim], &mut query_max[dim]);
                    }
                    NumericUtils::int_to_sortable_bytes(
                        query_min[dim],
                        &mut query_min_bytes[dim],
                        0,
                    );
                    NumericUtils::int_to_sortable_bytes(
                        query_max[dim],
                        &mut query_max_bytes[dim],
                        0,
                    );
                }

                let mut hits = BitSet::new();
                let mut visitor = IntersectVisitorImpl {
                    hits: &mut hits,
                    query_min: &query_min_bytes,
                    query_max: &query_max_bytes,
                    config: config.clone(),
                    random: &mut random,
                };

                r.intersect(&mut visitor)?;

                for (doc_id, doc_values) in docs.iter().enumerate() {
                    let mut expected = true;
                    for dim in 0..num_index_dims as usize {
                        let x = doc_values[dim];
                        if x < query_min[dim] || x > query_max[dim] {
                            expected = false;
                            break;
                        }
                    }
                    let actual = hits.contains(doc_id);
                    assert_eq!(expected, actual, "docID={}", doc_id);
                }
            }
        }

        Ok(())
    }
    // Tests on N-dimensional points where each dimension is a BigInteger
    #[test]
    fn test_big_int_n_dims() -> Result<(), LuceneError> {
        let mut random = random();
        let num_docs = at_least(&mut random, 1000);
        let dir = Rc::new(RefCell::new(new_directory(&mut random)?));

        let num_bytes_per_dim = TestUtil::next_int(&mut random, 2, 30);
        let num_dims = TestUtil::next_int(&mut random, 1, 5);
        let max_points_in_leaf_node = TestUtil::next_int(&mut random, 50, 100);
        let max_mb: f32 = 3.0 + (3.0 * random.random::<f32>());
        let config = Rc::new(BKDConfig::new(
            num_dims,
            num_dims,
            num_bytes_per_dim,
            max_points_in_leaf_node,
        )?);
        let mut writer = BKDWriter::new(
            num_docs,
            dir.clone(),
            "tmp",
            config.clone(),
            max_mb as f64,
            num_docs as i64,
        )?;

        let num_bytes_per_dim = num_bytes_per_dim as usize;
        let num_dims = num_dims as usize;
        let mut docs = vec![vec![]; num_docs as usize];
        let mut scratch = vec![0u8; num_bytes_per_dim * num_dims];

        for doc_id in 0..num_docs {
            let mut values = vec![BigInt::zero(); num_dims];
            if cfg!(feature = "test_log_verbose") {
                println!("  doc_id={}", doc_id);
            }
            for dim in 0..num_dims {
                values[dim] = random_big_int(num_bytes_per_dim, &mut random);
                NumericUtils::big_int_to_sortable_bytes(
                    &values[dim],
                    num_bytes_per_dim,
                    &mut scratch,
                    dim * num_bytes_per_dim,
                )?;
                if cfg!(feature = "test_log_verbose") {
                    println!("    {} -> {}", dim, values[dim]);
                }
            }
            docs[doc_id as usize] = values;
            writer.add(&scratch, doc_id)?;
        }

        let index_fp;
        {
            let out = Rc::new(RefCell::new(
                dir.borrow_mut()
                    .create_output("bkd", &IOContext::default_io_context()?)?,
            ));
            let finalizer = writer.finish(out.clone())?.unwrap();
            {
                index_fp = out.borrow().get_file_pointer();
            }
            writer.write_index(out.clone(), out.clone(), &finalizer)?;
        }

        {
            let mut input = dir
                .borrow_mut()
                .open_input("bkd", &IOContext::default_io_context()?)?;
            input.seek(index_fp)?;
            let sub_point_values = get_point_values(Rc::new(RefCell::new(input)))?;
            let point_values = PointValues::new(sub_point_values);

            let iters = at_least(&mut random, 100);
            for iter in 0..iters {
                if cfg!(feature = "test_log_verbose") {
                    println!("TEST: iter={}", iter);
                }
                let mut query_min = vec![BigInt::zero(); num_dims];
                let mut query_min_bytes = vec![vec![0u8; num_bytes_per_dim]; num_dims];
                let mut query_max = vec![BigInt::zero(); num_dims];
                let mut query_max_bytes = vec![vec![0u8; num_bytes_per_dim]; num_dims];

                for dim in 0..num_dims {
                    query_min[dim] = random_big_int(num_bytes_per_dim, &mut random);
                    query_max[dim] = random_big_int(num_bytes_per_dim, &mut random);
                    if query_min[dim] > query_max[dim] {
                        std::mem::swap(&mut query_min[dim], &mut query_max[dim]);
                    }
                    NumericUtils::big_int_to_sortable_bytes(
                        &query_min[dim],
                        num_bytes_per_dim,
                        &mut query_min_bytes[dim],
                        0,
                    )?;
                    NumericUtils::big_int_to_sortable_bytes(
                        &query_max[dim],
                        num_bytes_per_dim,
                        &mut query_max_bytes[dim],
                        0,
                    )?;
                }

                let mut hits = BitSet::new();
                let mut visitor = IntersectVisitorImpl {
                    hits: &mut hits,
                    query_min: &query_min_bytes,
                    query_max: &query_max_bytes,
                    config: config.clone(),
                    random: &mut random,
                };

                point_values.intersect(&mut visitor)?;

                for (doc_id, doc_values) in docs.iter().enumerate() {
                    let mut expected = true;
                    for dim in 0..num_dims {
                        let x = &doc_values[dim];
                        if x < &query_min[dim] || x > &query_max[dim] {
                            expected = false;
                            break;
                        }
                    }
                    let actual = hits.contains(doc_id);
                    assert_eq!(expected, actual, "docID={}", doc_id);
                }
            }
        }
        Ok(())
    }
    #[test]
    fn test_with_exceptions() {
        // TODO: MockDirectoryWrapper not Implemented
    }

    #[test]
    fn test_random_binary_tiny() -> Result<(), LuceneError> {
        let mut random = random();
        do_test_random_binary(&mut random, 10)
    }

    #[test]
    fn test_random_binary_medium() -> Result<(), LuceneError> {
        let mut random = random();
        do_test_random_binary(&mut random, 10_000)
    }

    #[cfg(feature = "nightly")]
    #[test]
    fn test_random_binary_big() -> Result<(), LuceneError> {
        let mut random = random();
        do_test_random_binary(&mut random, 200_000)
    }
    #[test]
    fn test_too_little_heap() -> Result<(), LuceneError> {
        let dir = Rc::new(RefCell::new(new_directory(&mut random())?));

        let err = BKDWriter::new(
            1,
            dir.clone(),
            "bkd",
            Rc::new(BKDConfig::new(1, 1, 16, 1_000_000)?),
            0.001,
            0,
        );
        assert!(err.is_err());
        if let Err(err) = err {
            let err_msg = format!("{:?}", err);
            assert!(
                err_msg.contains("either increase maxMBSortInHeap or decrease maxPointsInLeafNode")
            );
        }
        Ok(())
    }
    fn do_test_random_binary(random: &mut StdRng, count: i32) -> Result<(), LuceneError> {
        let num_docs = TestUtil::next_int(random, count, count * 2);
        let num_bytes_per_dim = TestUtil::next_int(random, 2, 30);

        let num_data_dims = TestUtil::next_int(random, 1, PointValues::MAX_DIMENSIONS);
        let num_index_dims = std::cmp::min(
            TestUtil::next_int(random, 1, num_data_dims),
            PointValues::MAX_INDEX_DIMENSIONS,
        );

        let mut doc_values =
            vec![
                vec![vec![0u8; num_bytes_per_dim as usize]; num_data_dims as usize];
                num_docs as usize
            ];

        for doc_id in 0..num_docs as usize {
            for dim in 0..num_data_dims as usize {
                random.fill_bytes(&mut doc_values[doc_id][dim]);
            }
        }

        verify(
            random,
            &doc_values,
            None,
            num_data_dims,
            num_index_dims,
            num_bytes_per_dim,
        )
    }

    #[test]
    fn test_all_equal() -> Result<(), LuceneError> {
        let mut random = random();

        let num_bytes_per_dim = TestUtil::next_int(&mut random, 2, 30);
        let num_data_dims = TestUtil::next_int(&mut random, 1, PointValues::MAX_DIMENSIONS);
        let num_index_dims = std::cmp::min(
            TestUtil::next_int(&mut random, 1, num_data_dims),
            PointValues::MAX_INDEX_DIMENSIONS,
        );

        let num_docs = at_least(&mut random, 1000);
        let mut doc_values =
            vec![
                vec![vec![0u8; num_bytes_per_dim as usize]; num_data_dims as usize];
                num_docs as usize
            ];

        for doc_id in 0..num_docs as usize {
            if doc_id == 0 {
                for dim in 0..num_data_dims as usize {
                    random.fill_bytes(&mut doc_values[doc_id][dim]);
                }
            } else {
                doc_values[doc_id] = doc_values[0].clone();
            }
        }

        verify(
            &mut random,
            &doc_values,
            None,
            num_data_dims,
            num_index_dims,
            num_bytes_per_dim,
        )
    }

    #[test]
    fn test_index_dim_equal_data_dim_different() -> Result<(), LuceneError> {
        let mut random = random();

        let num_bytes_per_dim = TestUtil::next_int(&mut random, 2, 30);
        let num_data_dims = TestUtil::next_int(&mut random, 2, PointValues::MAX_DIMENSIONS);
        let num_index_dims = std::cmp::min(
            TestUtil::next_int(&mut random, 1, num_data_dims - 1),
            PointValues::MAX_INDEX_DIMENSIONS,
        );

        let num_docs = at_least(&mut random, 1000);
        let mut doc_values =
            vec![
                vec![vec![0u8; num_bytes_per_dim as usize]; num_data_dims as usize];
                num_docs as usize
            ];

        let mut index_dimensions =
            vec![vec![0u8; num_bytes_per_dim as usize]; num_data_dims as usize];
        for dim in 0..num_index_dims as usize {
            random.fill_bytes(&mut index_dimensions[dim]);
        }

        for doc_id in 0..num_docs as usize {
            for dim in 0..num_index_dims as usize {
                doc_values[doc_id][dim] = index_dimensions[dim].clone();
            }
            for dim in num_index_dims as usize..num_data_dims as usize {
                random.fill_bytes(&mut doc_values[doc_id][dim]);
            }
        }

        verify(
            &mut random,
            &doc_values,
            None,
            num_data_dims,
            num_index_dims,
            num_bytes_per_dim,
        )
    }

    #[test]
    fn test_one_dim_equal() -> Result<(), LuceneError> {
        let mut random = random();

        let num_bytes_per_dim = TestUtil::next_int(&mut random, 2, 30);
        let num_data_dims = TestUtil::next_int(&mut random, 1, PointValues::MAX_DIMENSIONS);
        let num_index_dims = std::cmp::min(
            TestUtil::next_int(&mut random, 1, num_data_dims),
            PointValues::MAX_INDEX_DIMENSIONS,
        );

        let num_docs = at_least(&mut random, 1000);
        let the_equal_dim = random.random_range(0..num_data_dims);
        let mut doc_values =
            vec![
                vec![vec![0u8; num_bytes_per_dim as usize]; num_data_dims as usize];
                num_docs as usize
            ];

        for doc_id in 0..num_docs as usize {
            for dim in 0..num_data_dims as usize {
                random.fill_bytes(&mut doc_values[doc_id][dim]);
            }
            if doc_id > 0 {
                doc_values[doc_id][the_equal_dim as usize] =
                    doc_values[0][the_equal_dim as usize].clone();
            }
        }

        let max_points_in_leaf_node = TestUtil::next_int(&mut random, 20, 50);

        verify_full(
            &mut random,
            &doc_values,
            None,
            num_data_dims,
            num_index_dims,
            num_bytes_per_dim,
            max_points_in_leaf_node,
        )
    }

    #[test]
    fn test_one_dim_low_card() -> Result<(), LuceneError> {
        let mut random = random();

        let num_bytes_per_dim = TestUtil::next_int(&mut random, 2, 30);
        let num_data_dims = TestUtil::next_int(&mut random, 2, PointValues::MAX_DIMENSIONS);
        let num_index_dims = std::cmp::min(
            TestUtil::next_int(&mut random, 2, num_data_dims),
            PointValues::MAX_INDEX_DIMENSIONS,
        );

        let num_docs = at_least(&mut random, 10_000);
        let the_low_card_dim = random.random_range(0..num_data_dims);

        let mut value1 = vec![0u8; num_bytes_per_dim as usize];
        random.fill_bytes(&mut value1);
        let mut value2 = value1.clone();

        let last = &mut value2[num_bytes_per_dim as usize - 1];
        if *last == 0 || random.random_bool(0.5) {
            *last = last.wrapping_add(1);
        } else {
            *last = last.wrapping_sub(1);
        }

        let mut doc_values =
            vec![
                vec![vec![0u8; num_bytes_per_dim as usize]; num_data_dims as usize];
                num_docs as usize
            ];

        for doc_id in 0..num_docs as usize {
            for dim in 0..num_data_dims as usize {
                if dim == the_low_card_dim as usize {
                    doc_values[doc_id][dim] = if random.random_bool(0.5) {
                        value1.clone()
                    } else {
                        value2.clone()
                    };
                } else {
                    random.fill_bytes(&mut doc_values[doc_id][dim]);
                }
            }
        }
        let max_points_in_leaf_node = TestUtil::next_int(&mut random, 20, 50);
        verify_full(
            &mut random,
            &doc_values,
            None,
            num_data_dims,
            num_index_dims,
            num_bytes_per_dim,
            max_points_in_leaf_node,
        )
    }

    #[test]
    fn test_one_dim_two_values() -> Result<(), LuceneError> {
        let mut random = random();

        let num_bytes_per_dim = TestUtil::next_int(&mut random, 2, 30);
        let num_data_dims = TestUtil::next_int(&mut random, 1, PointValues::MAX_DIMENSIONS);
        let num_index_dims = std::cmp::min(
            TestUtil::next_int(&mut random, 1, num_data_dims),
            PointValues::MAX_INDEX_DIMENSIONS,
        );

        let num_docs = at_least(&mut random, 1000);
        let the_dim = random.random_range(0..num_data_dims);

        let mut value1 = vec![0u8; num_bytes_per_dim as usize];
        random.fill_bytes(&mut value1);
        let mut value2 = vec![0u8; num_bytes_per_dim as usize];
        random.fill_bytes(&mut value2);

        let mut doc_values =
            vec![
                vec![vec![0u8; num_bytes_per_dim as usize]; num_data_dims as usize];
                num_docs as usize
            ];

        for doc_id in 0..num_docs as usize {
            for dim in 0..num_data_dims as usize {
                if dim == the_dim as usize {
                    doc_values[doc_id][dim] = if random.random_bool(0.5) {
                        value1.clone()
                    } else {
                        value2.clone()
                    };
                } else {
                    random.fill_bytes(&mut doc_values[doc_id][dim]);
                }
            }
        }

        verify(
            &mut random,
            &doc_values,
            None,
            num_data_dims,
            num_index_dims,
            num_bytes_per_dim,
        )
    }

    #[test]
    fn test_random_few_different_values() -> Result<(), LuceneError> {
        let mut random = random();

        let num_bytes_per_dim = TestUtil::next_int(&mut random, 2, 30);
        let num_data_dims = TestUtil::next_int(&mut random, 1, PointValues::MAX_DIMENSIONS);
        let num_index_dims = std::cmp::min(
            TestUtil::next_int(&mut random, 1, num_data_dims),
            PointValues::MAX_INDEX_DIMENSIONS,
        );

        let num_docs = at_least(&mut random, 10000);
        let cardinality = TestUtil::next_int(&mut random, 2, 100);

        let mut values = vec![
            vec![vec![0u8; num_bytes_per_dim as usize]; num_data_dims as usize];
            cardinality as usize
        ];
        for i in 0..cardinality as usize {
            for j in 0..num_data_dims as usize {
                random.fill_bytes(&mut values[i][j]);
            }
        }

        let mut doc_values =
            vec![
                vec![vec![0u8; num_bytes_per_dim as usize]; num_data_dims as usize];
                num_docs as usize
            ];
        for doc_id in 0..num_docs as usize {
            let v = random.random_range(0..cardinality);
            doc_values[doc_id] = values[v as usize].clone();
        }

        verify(
            &mut random,
            &doc_values,
            None,
            num_data_dims,
            num_index_dims,
            num_bytes_per_dim,
        )
    }

    pub struct DocMapImpl {
        cur_doc_id_base: i32,
    }
    impl DocMap for DocMapImpl {
        fn get(&self, doc_id: i32) -> i32 {
            self.cur_doc_id_base + doc_id
        }
    }
    #[test]
    fn test111() -> Result<(), LuceneError> {
        for i in 0..100 {
            test_multi_valued()?;
        }
        Ok(())
    }
    #[test]
    fn test_multi_valued() -> Result<(), LuceneError> {
        let mut random = random();

        let num_bytes_per_dim = TestUtil::next_int(&mut random, 2, 30);
        let num_data_dims = TestUtil::next_int(&mut random, 1, PointValues::MAX_DIMENSIONS);
        let num_index_dims = std::cmp::min(
            TestUtil::next_int(&mut random, 1, num_data_dims),
            PointValues::MAX_INDEX_DIMENSIONS,
        );

        let num_docs = at_least(&mut random, 1000);
        let mut doc_values: Vec<Vec<Vec<u8>>> = Vec::new();
        let mut doc_ids: Vec<i32> = Vec::new();

        for doc_id in 0..num_docs {
            let num_values_in_doc = TestUtil::next_int(&mut random, 1, 5);
            for _ in 0..num_values_in_doc {
                doc_ids.push(doc_id);
                let mut values =
                    vec![vec![0u8; num_bytes_per_dim as usize]; num_data_dims as usize];
                for dim in 0..num_data_dims as usize {
                    random.fill_bytes(&mut values[dim]);
                }
                doc_values.push(values);
            }
        }

        let doc_values_array: Vec<Vec<Vec<u8>>> = doc_values.clone();
        let mut doc_ids_array = vec![0i32; doc_ids.len()];
        for i in 0..doc_ids_array.len() {
            doc_ids_array[i] = doc_ids[i];
        }

        verify(
            &mut random,
            &doc_values_array,
            Some(doc_ids_array),
            num_data_dims,
            num_index_dims,
            num_bytes_per_dim,
        )
    }

    /// `doc_ids` can be `None` for the single-valued case; otherwise, it maps value to `doc_id`.
    fn verify(
        random: &mut StdRng,
        doc_values: &Vec<Vec<Vec<u8>>>,
        doc_ids: Option<Vec<i32>>,
        num_data_dims: i32,
        num_index_dims: i32,
        num_bytes_per_dim: i32,
    ) -> Result<(), LuceneError> {
        let max_points_in_leaf_node = TestUtil::next_int(random, 50, 1000);
        verify_full(
            random,
            doc_values,
            doc_ids,
            num_data_dims,
            num_index_dims,
            num_bytes_per_dim,
            max_points_in_leaf_node,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn verify_full(
        random: &mut StdRng,
        doc_values: &Vec<Vec<Vec<u8>>>,
        doc_ids: Option<Vec<i32>>,
        num_data_dims: i32,
        num_index_dims: i32,
        num_bytes_per_dim: i32,
        max_points_in_leaf_node: i32,
    ) -> Result<(), LuceneError> {
        let dir = Rc::new(RefCell::new(new_directory(random)?));
        let max_mb: f64 = 3.0 + (3.0 * random.random::<f64>());
        verify_with_max_mb(
            random,
            dir,
            doc_values,
            doc_ids,
            num_data_dims,
            num_index_dims,
            num_bytes_per_dim,
            max_points_in_leaf_node,
            max_mb,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn verify_with_max_mb<D: Directory>(
        random: &mut StdRng,
        dir: Rc<RefCell<D>>,
        doc_values: &Vec<Vec<Vec<u8>>>,
        doc_ids: Option<Vec<i32>>,
        num_data_dims: i32,
        num_index_dims: i32,
        num_bytes_per_dim: i32,
        mut max_points_in_leaf_node: i32,
        mut max_mb: f64,
    ) -> Result<(), LuceneError> {
        let num_values = doc_values.len();

        if cfg!(feature = "test_log_verbose") {
            println!(
                "TEST: numValues={} numDataDims={} numIndexDims={} numBytesPerDim={} maxPointsInLeafNode={} maxMB={}",
                num_values, num_data_dims, num_index_dims, num_bytes_per_dim, max_points_in_leaf_node, max_mb
            );
        }

        let mut to_merge: Option<Vec<i64>> = None;
        let mut doc_maps: Option<Vec<Rc<DocMapEnum>>> = None;
        let mut seg = 0;

        let max_docs = if random.random_bool(0.5) {
            num_values as i64
        } else {
            let mut v = i64::MIN;
            while v < num_values as i64 {
                v = random.random::<i64>();
            }
            v
        };

        let mut writer = BKDWriter::new(
            num_values as i32,
            dir.clone(),
            &format!("_{}", seg),
            Rc::new(BKDConfig::new(
                num_data_dims,
                num_index_dims,
                num_bytes_per_dim,
                max_points_in_leaf_node,
            )?),
            max_mb,
            max_docs,
        )?;

        let out = Rc::new(RefCell::new(
            dir.borrow_mut()
                .create_output("bkd", &IOContext::default_io_context()?)?,
        ));

        let mut scratch = vec![0u8; (num_bytes_per_dim * num_data_dims) as usize];
        let mut last_doc_id_base = 0;
        let use_merge = num_data_dims == 1 && num_values >= 10 && random.random_bool(0.5);
        let mut values_in_this_seg = if use_merge {
            TestUtil::next_int(random, num_values as i32 / 10, num_values as i32) as usize
        } else {
            0
        };

        let mut seg_count = 0;

        for ord in 0..num_values {
            let doc_id = doc_ids.as_ref().map_or(ord as i32, |ids| ids[ord]);

            if cfg!(feature = "test_log_verbose") {
                println!(
                    "  ord={} docID={} lastDocIDBase={}",
                    ord, doc_id, last_doc_id_base
                );
            }

            for dim in 0..num_data_dims {
                if cfg!(feature = "test_log_verbose") {
                    println!(
                        "  {} -> {}",
                        dim,
                        BytesRef::from_bytes(doc_values[ord][dim as usize].to_vec())
                    );
                }
                scratch[(dim * num_bytes_per_dim) as usize
                    ..(dim * num_bytes_per_dim + num_bytes_per_dim) as usize]
                    .copy_from_slice(&doc_values[ord][dim as usize][0..num_bytes_per_dim as usize]);
            }

            writer.add(&scratch, doc_id - last_doc_id_base)?;

            seg_count += 1;

            if use_merge && seg_count == values_in_this_seg {
                if to_merge.is_none() {
                    to_merge = Some(Vec::new());
                    doc_maps = Some(Vec::new());
                }

                let cur_doc_id_base = last_doc_id_base;
                doc_maps
                    .as_mut()
                    .unwrap()
                    .push(Rc::new(DocMapEnum::DocMapMock(DocMapImpl {
                        cur_doc_id_base,
                    })));

                let finalizer = writer.finish(out.clone())?.unwrap();
                to_merge
                    .as_mut()
                    .unwrap()
                    .push(out.borrow().get_file_pointer());
                writer.write_index(out.clone(), out.clone(), &finalizer)?;
                values_in_this_seg =
                    TestUtil::next_int(random, num_values as i32 / 10, num_values as i32 / 2)
                        as usize;
                seg_count = 0;

                seg += 1;
                max_points_in_leaf_node = TestUtil::next_int(random, 50, 1000);
                max_mb = 3.0 + (3.0 * random.random::<f64>());

                writer = BKDWriter::new(
                    num_values as i32,
                    dir.clone(),
                    &format!("_{}", seg),
                    Rc::new(BKDConfig::new(
                        num_data_dims,
                        num_index_dims,
                        num_bytes_per_dim,
                        max_points_in_leaf_node,
                    )?),
                    max_mb,
                    doc_values.len() as i64,
                )?;
                last_doc_id_base = doc_id;
            }
        }

        let index_fp;

        let mut input;
        if let Some(to_merge) = &mut to_merge {
            if seg_count > 0 {
                let finalizer = writer.finish(out.clone())?.unwrap();
                to_merge.push(out.borrow().get_file_pointer());
                writer.write_index(out.clone(), out.clone(), &finalizer)?;
                let cur_doc_id_base = last_doc_id_base;
                doc_maps
                    .as_mut()
                    .unwrap()
                    .push(Rc::new(DocMapEnum::DocMapMock(DocMapImpl {
                        cur_doc_id_base,
                    })));
            }
            drop(out);
            input = Rc::new(RefCell::new(
                dir.borrow_mut()
                    .open_input("bkd", &IOContext::default_io_context()?)?,
            ));
            seg += 1;
            writer = BKDWriter::new(
                num_values as i32,
                dir.clone(),
                &format!("_{}", seg),
                Rc::new(BKDConfig::new(
                    num_data_dims,
                    num_index_dims,
                    num_bytes_per_dim,
                    max_points_in_leaf_node,
                )?),
                max_mb,
                doc_values.len() as i64,
            )?;

            let mut readers = Vec::new();
            for fp in to_merge {
                input.borrow_mut().seek(*fp)?;
                readers.push(PointValues::new(get_point_values(input.clone())?));
            }

            {
                let out = Rc::new(RefCell::new(
                    dir.borrow_mut()
                        .create_output("bkd2", &IOContext::default_io_context()?)?,
                ));
                let finalizer = writer
                    .merge(out.clone(), out.clone(), out.clone(), doc_maps, readers)?
                    .unwrap();
                index_fp = out.borrow().get_file_pointer();
                writer.write_index(out.clone(), out.clone(), &finalizer)?;
            }
            input = Rc::new(RefCell::new(
                dir.borrow_mut()
                    .open_input("bkd2", &IOContext::default_io_context()?)?,
            ));
        } else {
            let finalizer = writer.finish(out.clone())?.unwrap();
            index_fp = out.borrow().get_file_pointer();
            writer.write_index(out.clone(), out.clone(), &finalizer)?;
            drop(out);
            input = Rc::new(RefCell::new(
                dir.borrow_mut()
                    .open_input("bkd", &IOContext::default_io_context()?)?,
            ));
        }

        input.borrow_mut().seek(index_fp)?;
        let sub_point_values = get_point_values(input.clone())?;
        assert_size(&mut sub_point_values.get_point_tree()?, random)?;
        let point_values = PointValues::new(sub_point_values);

        let iters = at_least(random, 100);
        for _ in 0..iters {
            let mut query_min = vec![vec![0u8; num_bytes_per_dim as usize]; num_data_dims as usize];
            let mut query_max = vec![vec![0u8; num_bytes_per_dim as usize]; num_data_dims as usize];

            for dim in 0..num_data_dims as usize {
                random.fill_bytes(&mut query_min[dim]);
                random.fill_bytes(&mut query_max[dim]);

                if query_min[dim] > query_max[dim] {
                    std::mem::swap(&mut query_min[dim], &mut query_max[dim]);
                }
            }

            let num_bytes_per_dim = num_bytes_per_dim as usize;
            let mut expected = BitSet::new();
            for ord in 0..num_values {
                let mut matches = true;
                for dim in 0..num_index_dims as usize {
                    if doc_values[ord][dim][0..num_bytes_per_dim]
                        .cmp(&query_min[dim][0..num_bytes_per_dim])
                        .to_int()
                        < 0
                        || doc_values[ord][dim][0..num_bytes_per_dim]
                            .cmp(&query_max[dim][0..num_bytes_per_dim])
                            .to_int()
                            > 0
                    {
                        matches = false;
                        break;
                    }
                }
                if matches {
                    let doc_id = if doc_ids.is_none() {
                        ord as i32
                    } else {
                        doc_ids.as_ref().unwrap()[ord]
                    };
                    expected.insert(doc_id as usize);
                }
            }

            let config = Rc::new(BKDConfig::new(
                num_data_dims,
                num_index_dims,
                num_bytes_per_dim as i32,
                max_points_in_leaf_node,
            )?);
            let mut hits = BitSet::new();
            point_values.intersect(&mut IntersectVisitorImpl {
                hits: &mut hits,
                query_min: &query_min,
                query_max: &query_max,
                config: config.clone(),
                random,
            })?;
            assert_hits(&hits, &expected);
            hits.clear();
            PointTree::visit_doc_values(
                &mut point_values.get_point_tree()?,
                &mut IntersectVisitorImpl {
                    hits: &mut hits,
                    query_min: &query_min,
                    query_max: &query_max,
                    config: config.clone(),
                    random,
                },
            )?;
            assert_hits(&hits, &expected);
        }
        dir.borrow_mut().delete_file("bkd")?;
        if to_merge.is_some() {
            dir.borrow_mut().delete_file("bkd2")?;
        }

        Ok(())
    }
    fn assert_size(tree: &mut impl PointTree, random: &mut StdRng) -> Result<(), LuceneError> {
        // TODO:do we need clone?
        // let mut clone = tree.clone();
        // assert_eq!(clone.size()?, tree.size()?);

        // Rarely continue with the clone tree
        // let tree = if rarely(random) { &mut clone } else { tree };

        let mut visit_doc_id_size = vec![0; 1];
        let mut visit_doc_values_size = vec![0; 1];

        let mut visitor = IntersectVisitorImpl1 {
            visit_doc_id_size: &mut visit_doc_id_size,
            visit_doc_values_size: &mut visit_doc_values_size,
        };

        if random.random_bool(0.5) {
            tree.visit_doc_ids(&mut visitor)?;
            tree.visit_doc_values(&mut visitor)?;
        } else {
            tree.visit_doc_values(&mut visitor)?;
            tree.visit_doc_ids(&mut visitor)?;
        }

        assert_eq!(visit_doc_id_size[0], visit_doc_values_size[0]);
        assert_eq!(visit_doc_id_size[0], tree.size()?);

        if tree.move_to_child()? {
            loop {
                random_point_tree_navigation(tree, random)?;
                assert_size(tree, random)?;
                if !tree.move_to_sibling()? {
                    break;
                }
            }
            tree.move_to_parent()?;
        }
        Ok(())
    }

    struct IntersectVisitorImpl1<'a> {
        visit_doc_id_size: &'a mut [i64],
        visit_doc_values_size: &'a mut [i64],
    }
    impl IntersectVisitor for IntersectVisitorImpl1<'_> {
        fn visit(&mut self, doc_id: i32) -> Result<(), LuceneError> {
            self.visit_doc_id_size[0] += 1;
            Ok(())
        }

        fn visit_with_packed_value(
            &mut self,
            doc_id: i32,
            packed_value: &[u8],
        ) -> Result<(), LuceneError> {
            self.visit_doc_values_size[0] += 1;
            Ok(())
        }

        fn compare(
            &self,
            min_packed_value: &[u8],
            max_packed_value: &[u8],
        ) -> Result<Relation, LuceneError> {
            Ok(Relation::CellCrossesQuery)
        }
    }
    fn random_point_tree_navigation(
        tree: &mut impl PointTree,
        random: &mut StdRng,
    ) -> Result<(), LuceneError> {
        let min_packed_value = tree.get_min_packed_value()?.to_vec();
        let max_packed_value = tree.get_max_packed_value()?.to_vec();
        let size = tree.size()?;

        if random.random_bool(0.5) && tree.move_to_child()? {
            random_point_tree_navigation(tree, random)?;
            if random.random_bool(0.5) && tree.move_to_sibling()? {
                random_point_tree_navigation(tree, random)?;
            }
            tree.move_to_parent()?;
        }

        // Ensure we always finish on the same node we started
        assert_eq!(min_packed_value, tree.get_min_packed_value()?);
        assert_eq!(max_packed_value, tree.get_max_packed_value()?);
        assert_eq!(size, tree.size()?);

        Ok(())
    }

    fn assert_hits(hits: &BitSet, expected: &BitSet) {
        let limit = expected.len().max(hits.len());
        for doc_id in 0..limit {
            assert_eq!(
                expected.contains(doc_id),
                hits.contains(doc_id),
                "docID={}",
                doc_id
            );
        }
    }

    fn random_big_int(num_bytes: usize, random: &mut StdRng) -> BigInt {
        let num_bits = num_bytes * 8 - 1;
        let mut bytes = vec![0u8; (num_bits + 7) / 8];

        random.fill_bytes(&mut bytes);

        if let Some(first_byte) = bytes.first_mut() {
            *first_byte &= !(1 << (num_bits % 8));
        }

        let x = BigInt::from_bytes_be(Sign::Plus, &bytes);

        if random.random_bool(0.5) {
            -x
        } else {
            x
        }
    }

    // TODO:
    // fn get_directory(num_points: i32) {
    // }
    struct IntersectVisitorImpl<'a> {
        hits: &'a mut BitSet,
        query_min: &'a [Vec<u8>],
        query_max: &'a [Vec<u8>],
        config: Rc<BKDConfig>,
        random: &'a mut StdRng,
    }

    impl IntersectVisitor for IntersectVisitorImpl<'_> {
        fn visit(&mut self, doc_id: i32) -> Result<(), LuceneError> {
            self.hits.insert(doc_id as usize);
            Ok(())
        }
        fn visit_with_packed_value(
            &mut self,
            doc_id: i32,
            packed_value: &[u8],
        ) -> Result<(), LuceneError> {
            let num_index_dims = self.config.num_index_dims as usize;
            let bytes_per_dim = self.config.bytes_per_dim as usize;

            for dim in 0..num_index_dims {
                let offset = dim * bytes_per_dim;
                if packed_value[offset..offset + bytes_per_dim]
                    .cmp(&self.query_min[dim][0..bytes_per_dim])
                    .to_int()
                    < 0
                    || packed_value[offset..offset + bytes_per_dim]
                        .cmp(&self.query_max[dim][0..bytes_per_dim])
                        .to_int()
                        > 0
                {
                    return Ok(());
                }
            }
            // If all dimensions pass the range check, mark the document as a hit
            self.hits.insert(doc_id as usize);
            Ok(())
        }

        fn visit_iterator_with_packed_value(
            &mut self,
            iterator: &mut impl DocIdSetIterator,
            packed_value: &[u8],
        ) -> Result<(), LuceneError> {
            if self.random.random_bool(0.5) {
                // Check the default method is correct
                IntersectVisitor::default_visit_iterator_with_packed_value_(
                    self,
                    iterator,
                    packed_value,
                )?;
            } else {
                assert_eq!(iterator.doc_id(), -1);

                let cost = iterator.cost()? as i32;
                let mut number_of_points = 0;

                while let Ok(doc_id) = iterator.next_doc() {
                    if doc_id == NO_MORE_DOCS {
                        break;
                    }

                    assert_eq!(iterator.doc_id(), doc_id);
                    self.visit_with_packed_value(doc_id, packed_value)?;
                    number_of_points += 1;
                }

                assert_eq!(cost, number_of_points);
                assert_eq!(iterator.doc_id(), NO_MORE_DOCS);
                assert_eq!(iterator.next_doc()?, NO_MORE_DOCS);
                assert_eq!(iterator.doc_id(), NO_MORE_DOCS);
            }
            Ok(())
        }

        fn compare(&self, min_packed: &[u8], max_packed: &[u8]) -> Result<Relation, LuceneError> {
            let num_index_dims = self.config.num_index_dims as usize;
            let bytes_per_dim = self.config.bytes_per_dim as usize;
            let mut crosses = false;

            for dim in 0..num_index_dims {
                let offset = dim * bytes_per_dim;

                if max_packed[offset..offset + bytes_per_dim]
                    .cmp(&self.query_min[dim][..bytes_per_dim])
                    .to_int()
                    < 0
                    || min_packed[offset..offset + bytes_per_dim]
                        .cmp(&self.query_max[dim][..bytes_per_dim])
                        .to_int()
                        > 0
                {
                    return Ok(Relation::CellOutsideQuery);
                } else if min_packed[offset..offset + bytes_per_dim]
                    .cmp(&self.query_min[dim][..bytes_per_dim])
                    .to_int()
                    < 0
                    || max_packed[offset..offset + bytes_per_dim]
                        .cmp(&self.query_max[dim][..bytes_per_dim])
                        .to_int()
                        > 0
                {
                    crosses = true;
                }
            }

            if crosses {
                Ok(Relation::CellCrossesQuery)
            } else {
                Ok(Relation::CellInsideQuery)
            }
        }
    }
}
