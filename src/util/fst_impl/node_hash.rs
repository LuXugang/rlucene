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
use crate::util::access::Access;
use crate::util::allocator_byte::{AllocatorByteEnum, DirectAllocatorByte};
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::byte_block_pool_reverse_bytes_reader::ByteBlockPoolReverseBytesReader;
use crate::util::fst_impl::fst_compiler::Arc;
use crate::util::fst_impl::fst_reader::FstReader;
use crate::util::fst_impl::outputs::{Outputs, OutputsBound};
use crate::util::long_values::LongValues;
use crate::util::packed::abstract_paged_mutable::AbstractPagedMutable;
use crate::util::packed::paged_growable_writer::PagedGrowableWriter;
use crate::util::packed::PackedInts;
use crate::util::{ByteBlockPool, ByteBlockPoolBorrow};
use std::cell::RefCell;
use std::rc::Rc;

pub struct NodeHash<T, O, F>
where
    T: OutputsBound,
    O: Outputs<T>,
    F: FstReader,
{
    scratch_arc: Arc<T, O, F>,
}

pub struct PagedGrowableHash {
    /// Storing the FST node address where the position is the masked hash of the node arcs.
    fst_node_address: AbstractPagedMutable<PagedGrowableWriter>,
    /// Storing the local copiedNodes address in the same position as fst_node_address.
    /// Effectively a map from FST node address to copiedNodes address.
    copied_node_address: AbstractPagedMutable<PagedGrowableWriter>,
    count: i64,
    mask: i64,
    /// Storing the byte slice from the FST for nodes added to the hash,
    /// allowing append-only writes without needing to read from the FST.
    copied_nodes: ByteBlockPoolBorrow,
    /// the [`FST.BytesReader`](crate::util::fst_impl::fst_reader::FstReader) to read from copiedNodes. we use this when computing a frozen
    /// node hash or comparing if a frozen and unfrozen nodes are equal
    bytes_reader: ByteBlockPoolReverseBytesReader,
}

impl PagedGrowableHash {
    // 256K blocks, but note that the final block is sized only as needed so it won't use the full
    // block size when just a few elements were written to it
    const BLOCK_SIZE_BYTES: i32 = 1 << 18;
    pub(crate) fn new() -> Result<Self> {
        Self::build(8, 8, 16, 15)
    }

    pub(crate) fn new_with_size(last_node_address: i64, size: i64) -> Result<Self> {
        let fst_node_address_bits_per_value = PackedInts::bits_required(last_node_address)?;
        let mask = size - 1;
        debug_assert!(
            mask & size == 0,
            "size must be a power-of-2; got size={} mask={}",
            size,
            mask
        );
        Self::build(fst_node_address_bits_per_value, 8, size, mask)
    }
    fn build(
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

        Ok(Self {
            fst_node_address,
            copied_node_address,
            count: 0,
            mask,
            copied_nodes,
            bytes_reader,
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
        let address = self.copied_node_address.get(hash_slot)?;
        debug_assert!(address - length as i64 + 1 >= 0);

        let mut buf = vec![0u8; length as usize];
        self.copied_nodes.access_mut(|copied_nodes| {
            copied_nodes.read_bytes(address - length as i64 + 1, &mut buf, 0, length)?;
            Ok(())
        })?;
        Ok(buf)
    }
    /// Get the node address from the provided hash slot.
    pub fn get_node_address(&mut self, hash_slot: i64) -> Result<i64> {
        self.fst_node_address.get(hash_slot)
    }
    /// Set the node address for the given hash slot.
    pub fn set_node_address(&mut self, hash_slot: i64, node_address: i64) -> Result<()> {
        debug_assert_eq!(self.fst_node_address.get(hash_slot)?, 0);
        self.fst_node_address.set(hash_slot, node_address)?;
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
        debug_assert_eq!(self.copied_node_address.get(hash_slot)?, 0);

        self.copied_nodes.access_mut(|copied_nodes| {
            copied_nodes.append_range(bytes, 0, length)?;
            let position = copied_nodes.get_position();
            // write the offset, which points to the last byte of the node we copied since we later read
            // this node in reverse
            self.copied_node_address.set(hash_slot, position - 1)?;
            Ok(())
        })
    }
    /// Promote the node bytes from the fallback table.
    pub(crate) fn copy_fallback_node_bytes(
        &mut self,
        hash_slot: i64,
        fallback_table: &mut PagedGrowableHash,
        fallback_hash_slot: i64,
        node_length: i32,
    ) -> Result<()> {
        debug_assert_eq!(self.copied_node_address.get(hash_slot)?, 0);

        let fallback_address = fallback_table.copied_node_address.get(fallback_hash_slot)?;
        // fallbackAddress is the last offset of the node, but we need to copy the bytes from the
        // start address
        let fallback_start_address = fallback_address - node_length as i64 + 1;
        debug_assert!(fallback_start_address >= 0);

        let position = self.copied_nodes.access_mut(|copied_nodes| {
            copied_nodes.append_from_byte_block_pool(
                &*fallback_table.copied_nodes.borrow(),
                fallback_start_address,
                node_length,
            )?;
            Ok(copied_nodes.get_position())
        })?;
        self.copied_node_address.set(hash_slot, position - 1)?;
        Ok(())
    }
    /// Returns the [`ByteBlockPoolReverseBytesReader`] positioned for the given node.
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

#[cfg(test)]
mod tests {
    use crate::test::util::lucene_test_case::{at_least, random};
    use crate::util::error::lucene_error::Result;
    use crate::util::fst_impl::node_hash::PagedGrowableHash;
    use rand::Rng;

    #[allow(dead_code)]
    struct TestNodeHash;
    #[test]
    fn test_copy_fallback_node_bytes() -> Result<()> {
        let mut random = random();
        // Create primary and fallback hash tables
        let mut primary_hash_table = PagedGrowableHash::new()?;
        let mut fallback_hash_table = PagedGrowableHash::new()?;

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
            node_length as i32,
        )?;

        // Check that primary copied bytes match original
        let copied_bytes = primary_hash_table.get_bytes(primary_hash_slot, node_length as i32)?;
        for i in 0..node_length as usize {
            assert_eq!(fallback_bytes[i], copied_bytes[i], "byte @ index={}", i);
        }

        Ok(())
    }
}
