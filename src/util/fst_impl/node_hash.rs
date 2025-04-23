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
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::store::directory::Directory;
use crate::util::access::Access;
use crate::util::allocator_byte::{AllocatorByteEnum, DirectAllocatorByte};
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fst_impl::byte_block_pool_reverse_bytes_reader::ByteBlockPoolReverseBytesReader;
use crate::util::fst_impl::fst::{fst_util, Arc, BitTable, BytesReader};
use crate::util::fst_impl::fst_compiler::{FSTCompilerInner, NodeEnum, UnCompiledNode};
use crate::util::fst_impl::outputs::{Outputs, OutputsBound};
use crate::util::long_values::LongValues;
use crate::util::packed::abstract_paged_mutable::AbstractPagedMutable;
use crate::util::packed::paged_growable_writer::PagedGrowableWriter;
use crate::util::packed::PackedInts;
use crate::util::{ByteBlockPool, ByteBlockPoolBorrow, CoreHelper};
// TODO: any way to make a reverse suffix lookup (msokolov's idea) instead of
// more costly hash? hmmm, though, hash is not so wasteful
// since it does not have to store value of each entry: the value is the node
// pointer in the FST. actually, there is much to save
// there -- we would not need any long per entry -- we'd be able to start at the
// FST end node and work backwards from the transitions

// TODO: couldn't we prune naturally back until we see a transition with an
// output?  it's highly unlikely (mostly impossible) such suffixes can be
// shared?

// Used to dedup states (lookup already-frozen states)
pub struct NodeHash<T, O, D>
where
    T: OutputsBound,
    O: Outputs<T>,
    D: Directory,
{
    // primary table -- we add nodes into this until it reaches the requested
    // tableSizeLimit/2, then we move it to fallback
    primary_table: PagedGrowableHash<T, O, D>,
    // how many nodes are allowed to store in both primary and fallback tables;
    // when primary gets full (tableSizeLimit/2), we move it to the
    // fallback table
    ram_limit_bytes: i64,
    // fallback table.  if we fallback and find the frozen node here, we
    // promote it to primary table, for a simplistic and
    // lowish-RAM-overhead (compared to e.g. LinkedHashMap) LRU behaviour.
    // fallbackTable is read-only.
    fallback_table: Option<PagedGrowableHash<T, O, D>>,

    fst_compiler: Rc<RefCell<FSTCompilerInner<T, O, D>>>,
    scratch_arc: Arc<T>,
    // store the last fallback table node length in getFallback()
    last_fallback_node_length: i32,
    // store the last fallback table hashtable slot in getFallback()
    last_fallback_hash_slot: i64,
    phantom: PhantomData<T>,
}
impl<T, O, D> NodeHash<T, O, D>
where
    T: OutputsBound,
    O: Outputs<T>,
    D: Directory,
{
    /// Creates a new `NodeHash` instance.
    ///
    /// `ram_limit_mb` is the max RAM (in MB) allowed for recording suffixes.
    /// If this limit is hit, least recently used suffixes are discarded and the
    /// FST is no longer minimal. A larger `ram_limit_mb` makes the FST
    /// smaller (closer to minimal).
    pub fn new(
        fst_compiler: Rc<RefCell<FSTCompilerInner<T, O, D>>>,
        ram_limit_mb: f64,
    ) -> Result<Self> {
        if ram_limit_mb <= 0.0 {
            return Err(LuceneError::illegal_argument(format!(
                "ram_limit_mb must be > 0; got: {ram_limit_mb}"
            )));
        }

        let ram_limit_bytes = if ram_limit_mb >= (i64::MAX as f64) / 1024.0 / 1024.0 {
            // quietly truncate to Long.MAX_VALUE in bytes too
            i64::MAX
        } else {
            (ram_limit_mb * 1024.0 * 1024.0) as i64
        };

        Ok(Self {
            primary_table: PagedGrowableHash::new(fst_compiler.clone())?,
            fallback_table: None, // Empty initially
            ram_limit_bytes,
            fst_compiler,
            scratch_arc: Arc::default(),
            last_fallback_node_length: 0,
            last_fallback_hash_slot: 0,
            phantom: PhantomData,
        })
    }
    fn get_fallback(&mut self, node_in: &UnCompiledNode<T, O, D>, hash: i64) -> Result<i64> {
        self.last_fallback_node_length = -1;
        self.last_fallback_hash_slot = -1;

        match self.fallback_table {
            Some(ref mut fallback_table) => {
                let mut hash_slot = hash & fallback_table.mask;
                let mut c = 0;

                loop {
                    let node_address = fallback_table.get_node_address(hash_slot)?;
                    if node_address == 0 {
                        // not found
                        return Ok(0);
                    } else {
                        let length =
                            fallback_table.nodes_equal(node_in, node_address, hash_slot)?;
                        if length != -1 {
                            // store the node length for further use
                            self.last_fallback_node_length = length;
                            self.last_fallback_hash_slot = hash_slot;
                            // frozen version of this node is already here
                            return Ok(node_address);
                        }
                    }

                    // quadratic probe (but is it, really?)
                    c += 1;
                    hash_slot = (hash_slot + c) & fallback_table.mask;
                }
            },
            None => {
                // no fallback yet (primary table is not yet large enough to
                // swap)
                Ok(0)
            },
        }
    }
    pub fn add(&mut self, node_in: &UnCompiledNode<T, O, D>) -> Result<i64> {
        let hash = self.hash(node_in)?;
        let mut hash_slot = hash & self.primary_table.mask;
        let mut c = 0;

        loop {
            let mut node_address = self.primary_table.get_node_address(hash_slot)?;
            if node_address == 0 {
                // not in primary, check fallback
                node_address = self.get_fallback(node_in, hash)?;
                if node_address != 0 {
                    debug_assert!(
                        self.last_fallback_hash_slot != -1 && self.last_fallback_node_length != -1
                    );
                    // it was already in fallback -- promote to primary
                    self.primary_table
                        .set_node_address(hash_slot, node_address)?;
                    self.primary_table.copy_fallback_node_bytes(
                        hash_slot,
                        self.fallback_table.as_mut().unwrap(),
                        self.last_fallback_hash_slot,
                        self.last_fallback_node_length,
                    )?;
                } else {
                    // not in fallback either -- freeze & add the incoming node

                    // freeze & add
                    node_address = self.fst_compiler.borrow_mut().add_node(node_in)?;
                    // we use 0 as empty marker in hash table, so it better be
                    // impossible to get a frozen node at 0:
                    debug_assert!(
                        node_address != fst_util::FINAL_END_NODE
                            && node_address != fst_util::NON_FINAL_END_NODE
                    );

                    self.primary_table
                        .set_node_address(hash_slot, node_address)?;
                    let compiler = &mut *self.fst_compiler.borrow_mut();
                    let pos = compiler.scratch_bytes.get_position();
                    self.primary_table.copy_node_bytes(
                        hash_slot,
                        compiler.scratch_bytes.get_bytes(),
                        pos,
                    )?;

                    // confirm frozen hash and unfrozen hash are the same
                    debug_assert_eq!(
                        self.primary_table.hash(node_address, hash_slot)?,
                        hash,
                        "Frozen hash mismatch"
                    );
                }

                // how many bytes would be used if we had "perfect" hashing:
                //  - x2 for fstNodeAddress for FST node address
                //  - x2 for copiedNodeAddress for copied node address
                //  - the bytes copied out FST to the hashtable copiedNodes
                // each account for approximate hash table overhead halfway
                // between 33.3% and 66.6% note that some of the
                // copiedNodes are shared between fallback and primary tables so
                // this computation is pessimistic
                let copied_bytes = self.primary_table.copied_nodes.borrow_mut().get_position();
                let ram_bytes_used =
                    self.primary_table.count * 2 * PackedInts::bits_required(node_address)? as i64
                        / 8
                        + self.primary_table.count
                            * 2
                            * PackedInts::bits_required(copied_bytes)? as i64
                            / 8
                        + copied_bytes;
                // NOTE: we could instead use the more precise RAM used, but
                // this leads to unpredictable
                // quantized behavior due to 2X rehashing where for large ranges
                // of the RAM limit, the size of the FST does
                // not change, and then suddenly when you cross a secret
                // threshold, it drops.  With this approach
                // (measuring "perfect" hash storage and approximating the
                // overhead), the behaviour is more strictly monotonic: larger
                // RAM limits smoothly result in smaller FSTs,
                // even if the precise RAM used is not always under the limit.

                // divide limit by 2 because fallback gets half the RAM and
                // primary gets the other half
                if ram_bytes_used >= self.ram_limit_bytes / 2 {
                    // time to fallback -- fallback is now used read-only to
                    // promote a node (suffix) to primary if
                    // we encounter it again
                    let size = self.primary_table.inner.fst_node_address.size();
                    self.fallback_table = Some(std::mem::replace(&mut self.primary_table, {
                        PagedGrowableHash::with_size(
                            self.fst_compiler.clone(),
                            node_address,
                            size.max(16),
                        )?
                    }));
                } else if self.primary_table.count
                    > self.primary_table.inner.fst_node_address.size() * (2f32 / 3f32) as i64
                {
                    // rehash at 2/3 occupancy
                    self.primary_table.rehash(node_address)?;
                }

                return Ok(node_address);
            } else if self
                .primary_table
                .nodes_equal(node_in, node_address, hash_slot)?
                != -1
            {
                // same node (in frozen form) is already in primary table
                return Ok(node_address);
            }

            c += 1;
            // quadratic probe (but is it, really?)
            hash_slot = (hash_slot + c) & self.primary_table.mask;
        }
    }
    fn hash(&self, node: &UnCompiledNode<T, O, D>) -> Result<i64> {
        const PRIME: i64 = 31;
        let mut h: i64 = 0;

        for arc in &node.arcs[..node.num_arcs as usize] {
            h = PRIME.wrapping_mul(h).wrapping_add(arc.label as i64);

            let n = match &arc.target {
                NodeEnum::CompiledNode(compiled_node) => compiled_node.node,
                _ => return Err(LuceneError::illegal_state("Node should be compiled")),
            };
            h = PRIME.wrapping_mul(h).wrapping_add(n ^ (n >> 32));

            h = PRIME
                .wrapping_mul(h)
                .wrapping_add(CoreHelper::compute_hash(&arc.output));
            h = PRIME
                .wrapping_mul(h)
                .wrapping_add(CoreHelper::compute_hash(&arc.next_final_output));

            if arc.is_final {
                h = h.wrapping_add(17);
            }
        }
        Ok(h)
    }
}

/// Inner class because it needs access to hash function and FST bytes.
pub struct PagedGrowableHash<T, O, D>
where
    T: OutputsBound,
    O: Outputs<T>,
    D: Directory,
{
    count: i64,
    mask: i64,
    /// Storing the byte slice from the FST for nodes added to the hash,
    /// allowing append-only writes without needing to read from the FST.
    copied_nodes: ByteBlockPoolBorrow,
    fst_compiler: Rc<RefCell<FSTCompilerInner<T, O, D>>>,
    inner: Inner,
}
pub struct Inner {
    /// the [`FST.BytesReader`](crate::util::fst_impl::fst_reader::FstReader)
    /// to read from copiedNodes. we use this when computing a frozen
    /// node hash or comparing if a frozen and unfrozen nodes are equal
    bytes_reader: ByteBlockPoolReverseBytesReader,
    /// Storing the FST node address where the position is the masked hash of
    /// the node arcs.
    fst_node_address: AbstractPagedMutable<PagedGrowableWriter>,
    /// Storing the local copiedNodes address in the same position as
    /// fst_node_address. Effectively a map from FST node address to
    /// copiedNodes address.
    copied_node_address: AbstractPagedMutable<PagedGrowableWriter>,
}
impl Inner {
    /// Returns the [`ByteBlockPoolReverseBytesReader`] positioned for the given
    /// node.
    pub(crate) fn get_bytes_reader(
        &mut self,
        node_address: i64,
        hash_slot: i64,
    ) -> Result<&mut ByteBlockPoolReverseBytesReader> {
        debug_assert_eq!(self.fst_node_address.get(hash_slot)?, node_address);
        let local_address = self.copied_node_address.get(hash_slot)?;
        self.bytes_reader
            .set_pos_delta(node_address - local_address);
        Ok(&mut self.bytes_reader)
    }
}

impl<T, O, D> PagedGrowableHash<T, O, D>
where
    T: OutputsBound,
    O: Outputs<T>,
    D: Directory,
{
    // 256K blocks, but note that the final block is sized only as needed so it
    // won't use the full block size when just a few elements were written
    // to it
    const BLOCK_SIZE_BYTES: i32 = 1 << 18;
    pub(crate) fn new(fst_compiler: Rc<RefCell<FSTCompilerInner<T, O, D>>>) -> Result<Self> {
        Self::build(fst_compiler, 8, 8, 16, 15)
    }

    pub(crate) fn with_size(
        fst_compiler: Rc<RefCell<FSTCompilerInner<T, O, D>>>,
        last_node_address: i64,
        size: i64,
    ) -> Result<Self> {
        let fst_node_address_bits_per_value = PackedInts::bits_required(last_node_address)?;
        let mask = size - 1;
        debug_assert!(
            mask & size == 0,
            "size must be a power-of-2; got size={} mask={}",
            size,
            mask
        );
        Self::build(fst_compiler, fst_node_address_bits_per_value, 8, size, mask)
    }
    fn build(
        fst_compiler: Rc<RefCell<FSTCompilerInner<T, O, D>>>,
        fst_node_address_bits_per_value: i32,
        copied_node_address_bits_per_value: i32,
        size: i64,
        mask: i64,
    ) -> Result<Self> {
        let sub_reader = PagedGrowableWriter::with_fill_page(
            fst_node_address_bits_per_value,
            PackedInts::COMPACT,
        );
        let fst_node_address = AbstractPagedMutable::new(size, Self::BLOCK_SIZE_BYTES, sub_reader)?;
        let sub_reader = PagedGrowableWriter::with_fill_page(
            copied_node_address_bits_per_value,
            PackedInts::COMPACT,
        );
        let copied_node_address =
            AbstractPagedMutable::new(size, Self::BLOCK_SIZE_BYTES, sub_reader)?;

        let allocator = AllocatorByteEnum::DA(DirectAllocatorByte::new());
        let copied_nodes = Rc::new(RefCell::new(ByteBlockPool::new(allocator)));
        let bytes_reader = ByteBlockPoolReverseBytesReader::new(copied_nodes.clone());
        let inner = Inner {
            bytes_reader,
            fst_node_address,
            copied_node_address,
        };
        Ok(Self {
            count: 0,
            mask,
            copied_nodes,
            inner,
            fst_compiler,
        })
    }
    /// Get the copied bytes at the provided hash slot.
    ///
    /// # Arguments
    ///
    /// * `hash_slot` - The hash slot to read from
    /// * `length` - The number of bytes to read
    ///
    /// # Returns
    ///
    /// The copied byte array
    pub fn get_bytes(&mut self, hash_slot: i64, length: i32) -> Result<Vec<u8>> {
        let address = self.inner.copied_node_address.get(hash_slot)?;
        debug_assert!(address - length as i64 + 1 >= 0);

        let mut buf = vec![0u8; length as usize];
        self.copied_nodes.access_mut(|copied_nodes| {
            copied_nodes.read_bytes(address - length as i64 + 1, &mut buf, 0, length)?;
            // Help the compiler infer types
            Ok::<(), LuceneError>(())
        })?;
        Ok(buf)
    }
    /// Get the node address from the provided hash slot.
    pub fn get_node_address(&mut self, hash_slot: i64) -> Result<i64> {
        self.inner.fst_node_address.get(hash_slot)
    }
    /// Set the node address for the given hash slot.
    pub fn set_node_address(&mut self, hash_slot: i64, node_address: i64) -> Result<()> {
        debug_assert_eq!(self.inner.fst_node_address.get(hash_slot)?, 0);
        self.inner.fst_node_address.set(hash_slot, node_address)?;
        self.count += 1;
        Ok(())
    }
    /// Copy the node bytes from the FST.
    pub(crate) fn copy_node_bytes(
        &mut self,
        hash_slot: i64,
        bytes: &[u8],
        length: i32,
    ) -> Result<()> {
        debug_assert_eq!(self.inner.copied_node_address.get(hash_slot)?, 0);

        self.copied_nodes.access_mut(|copied_nodes| {
            copied_nodes.append_range(bytes, 0, length)?;
            let position = copied_nodes.get_position();
            // write the offset, which points to the last byte of the node we
            // copied since we later read this node in reverse
            self.inner
                .copied_node_address
                .set(hash_slot, position - 1)?;
            Ok(())
        })
    }
    /// Promote the node bytes from the fallback table.
    pub(crate) fn copy_fallback_node_bytes(
        &mut self,
        hash_slot: i64,
        fallback_table: &mut PagedGrowableHash<T, O, D>,
        fallback_hash_slot: i64,
        node_length: i32,
    ) -> Result<()> {
        debug_assert_eq!(self.inner.copied_node_address.get(hash_slot)?, 0);

        let fallback_address = fallback_table
            .inner
            .copied_node_address
            .get(fallback_hash_slot)?;
        // fallbackAddress is the last offset of the node, but we need to copy
        // the bytes from the start address
        let fallback_start_address = fallback_address - node_length as i64 + 1;
        debug_assert!(fallback_start_address >= 0);

        let position = self.copied_nodes.access_mut(|copied_nodes| {
            copied_nodes.append_from_byte_block_pool(
                &*fallback_table.copied_nodes.borrow(),
                fallback_start_address,
                node_length,
            )?;
            // Help the compiler infer types
            Ok::<i64, LuceneError>(copied_nodes.get_position())
        })?;
        self.inner
            .copied_node_address
            .set(hash_slot, position - 1)?;
        Ok(())
    }
    fn rehash(&mut self, last_node_address: i64) -> Result<()> {
        // TODO: https://github.com/apache/lucene/issues/12744
        // should we always use a small startBitsPerValue here (e.g 8) instead
        // base off of lastNodeAddress?

        // double hash table size on each rehash
        // Double the hash table size
        let new_size = self.inner.fst_node_address.size() * 2;

        let sub_reader = PagedGrowableWriter::with_fill_page(
            PackedInts::bits_required(self.copied_nodes.borrow_mut().get_position())?,
            PackedInts::COMPACT,
        );
        let mut new_copied_node_address =
            AbstractPagedMutable::new(new_size, Self::BLOCK_SIZE_BYTES, sub_reader)?;

        let sub_reader = PagedGrowableWriter::with_fill_page(
            PackedInts::bits_required(last_node_address)?,
            PackedInts::COMPACT,
        );
        let mut new_fst_node_address =
            AbstractPagedMutable::new(new_size, Self::BLOCK_SIZE_BYTES, sub_reader)?;

        let new_mask = new_fst_node_address.size() - 1;

        for idx in 0..self.inner.fst_node_address.size() {
            let address = self.inner.fst_node_address.get(idx)?;
            if address != 0 {
                let mut hash_slot = self.hash(address, idx)? & new_mask;
                let mut c = 0;
                loop {
                    if new_fst_node_address.get(hash_slot)? == 0 {
                        new_fst_node_address.set(hash_slot, address)?;
                        new_copied_node_address
                            .set(hash_slot, self.inner.copied_node_address.get(idx)?)?;
                        break;
                    }
                    // quadratic probe
                    c += 1;
                    hash_slot = (hash_slot + c) & new_mask;
                }
            }
        }

        self.mask = new_mask;
        self.inner.fst_node_address = new_fst_node_address;
        self.inner.copied_node_address = new_copied_node_address;

        Ok(())
    }
    fn hash(&mut self, node_address: i64, hash_slot: i64) -> Result<i64> {
        let mut scratch_arc: Arc<T> = Arc::default();
        let reader = self.inner.get_bytes_reader(node_address, hash_slot)?;
        let compiler = self.fst_compiler.borrow();

        let prime: i64 = 31;
        let mut h: i64 = 0;

        compiler
            .fst
            .read_first_real_target_arc(node_address, &mut scratch_arc, reader)?;
        // TODO: 这里要改成wrapping_mul跟wrapping_add
        loop {
            h = prime * h + scratch_arc.label() as i64;
            let target = scratch_arc.target();
            h = prime * h + (target ^ (target >> 32));
            h = prime * h + CoreHelper::compute_hash(&scratch_arc.output());
            h = prime * h + CoreHelper::compute_hash(&scratch_arc.next_final_output());
            if scratch_arc.is_final() {
                h += 17;
            }
            if scratch_arc.is_last() {
                break;
            }
            compiler.fst.read_next_real_arc(&mut scratch_arc, reader)?;
        }

        Ok(h)
    }
    /// Compares an unfrozen node (`UnCompiledNode`) with a frozen node at byte
    /// location address (`i64`), returning the node length if the two nodes
    /// are equals, or `-1` otherwise.
    ///
    ///
    /// The node length will be used to promote the node from the fallback table
    /// to the primary table.
    fn nodes_equal(
        &mut self,
        node: &UnCompiledNode<T, O, D>,
        address: i64,
        hash_slot: i64,
    ) -> Result<i32> {
        let in_reader = self.inner.get_bytes_reader(address, hash_slot)?;
        let mut scratch_arc: Arc<T> = Arc::default();
        self.fst_compiler.borrow().fst.read_first_real_target_arc(
            address,
            &mut scratch_arc,
            in_reader,
        )?;
        // fail fast for a node with fixed length arcs
        if scratch_arc.bytes_per_arc() != 0 {
            debug_assert!(node.num_arcs > 0);
            // the frozen node uses fixed-with arc encoding (same number of
            // bytes per arc), but may be sparse or dense
            match scratch_arc.node_flags() {
                fst_util::ARCS_FOR_BINARY_SEARCH => {
                    if node.num_arcs != scratch_arc.num_arcs() {
                        // sparse
                        return Ok(-1);
                    }
                },
                fst_util::ARCS_FOR_DIRECT_ADDRESSING => {
                    // dense -- compare both the number of labels allocated in
                    // the array (some of which may
                    // not actually be arcs), and the number of arcs
                    let first_label = node.arcs[0].label;
                    let last_label = node.arcs[node.num_arcs as usize - 1].label;
                    if (last_label - first_label + 1) != scratch_arc.num_arcs()
                        || node.num_arcs != BitTable::count_bits(&scratch_arc, in_reader)?
                    {
                        return Ok(-1);
                    }
                },
                fst_util::ARCS_FOR_CONTINUOUS => {
                    let first_label = node.arcs[0].label;
                    let last_label = node.arcs[node.num_arcs as usize - 1].label;
                    if (last_label - first_label + 1) != scratch_arc.num_arcs() {
                        return Ok(-1);
                    }
                },
                _ => {
                    return Err(LuceneError::illegal_state(format!(
                        "unhandled scratchArc.nodeFlag() {}",
                        scratch_arc.node_flags()
                    )));
                },
            }
        }
        // compare arc by arc to see if there is a difference
        for arc_idx in 0..node.num_arcs as usize {
            let arc = &node.arcs[arc_idx];

            if arc.label != scratch_arc.label()
                || arc.output != scratch_arc.output()
                || {
                    match &arc.target {
                        NodeEnum::CompiledNode(compiled) => compiled.node != scratch_arc.target(),
                        _ => return Err(LuceneError::illegal_state("Node should be compiled")),
                    }
                }
                || arc.next_final_output != scratch_arc.next_final_output()
                || arc.is_final != scratch_arc.is_final()
            {
                return Ok(-1);
            }

            match &arc.target {
                NodeEnum::CompiledNode(compiled) => {
                    if compiled.node != scratch_arc.target() {
                        return Ok(-1);
                    }
                },
                _ => return Ok(-1),
            }

            if scratch_arc.is_last() {
                if arc_idx == (node.num_arcs as usize - 1) {
                    let len = address - in_reader.get_position();
                    return Ok(len as i32);
                } else {
                    return Ok(-1);
                }
            }

            self.fst_compiler
                .borrow()
                .fst
                .read_next_real_arc(&mut scratch_arc, in_reader)?;
        }

        // unfrozen node has fewer arcs than frozen node
        Ok(-1)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use rand::Rng;

    use crate::store::dummy::dummy_directory::DummyDirectory;
    use crate::test::util::lucene_test_case::{at_least, random};
    use crate::util::error::lucene_error::Result;
    use crate::util::fst_impl::byte_sequence_outputs::ByteSequenceOutputs;
    use crate::util::fst_impl::fst::InputType;
    use crate::util::fst_impl::fst_compiler::{DataOutputEnum, FSTCompilerInner};
    use crate::util::fst_impl::node_hash::PagedGrowableHash;
    use crate::util::fst_impl::read_write_data_output::ReadWriteDataOutput;

    #[allow(dead_code)]
    struct TestNodeHash;
    #[test]
    fn test_copy_fallback_node_bytes() -> Result<()> {
        let mut random = random();
        let data_output: DataOutputEnum<DummyDirectory> =
            DataOutputEnum::ReadWriter(ReadWriteDataOutput::new(10));
        let index_type = InputType::Byte1;
        let outputs = ByteSequenceOutputs;
        let fst_compiler_inner = Rc::new(RefCell::new(FSTCompilerInner::new(
            index_type,
            0f64,
            outputs,
            true,
            data_output,
            0f32,
            10,
        )?));
        // Create primary and fallback hash tables
        let mut primary_hash_table = PagedGrowableHash::new(fst_compiler_inner.clone())?;
        let mut fallback_hash_table = PagedGrowableHash::new(fst_compiler_inner)?;

        let node_length = at_least(&mut random, 500);
        let fallback_hash_slot = 1;
        let fallback_bytes: Vec<u8> = (0..node_length).map(|_| random.random()).collect();

        fallback_hash_table.copy_node_bytes(fallback_hash_slot, &fallback_bytes, node_length)?;

        // Check that fallback bytes stored correctly
        let stored_bytes = fallback_hash_table.get_bytes(fallback_hash_slot, node_length as i32)?;
        for i in 0..node_length as usize {
            assert_eq!(fallback_bytes[i], stored_bytes[i], "byte @ index={}", i);
        }

        let primary_hash_slot = 2;
        primary_hash_table.copy_fallback_node_bytes(
            primary_hash_slot,
            &mut fallback_hash_table,
            fallback_hash_slot,
            node_length,
        )?;

        // Check that primary copied bytes match original
        let copied_bytes = primary_hash_table.get_bytes(primary_hash_slot, node_length as i32)?;
        for i in 0..node_length as usize {
            assert_eq!(fallback_bytes[i], copied_bytes[i], "byte @ index={}", i);
        }
        Ok(())
    }
}
