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
use crate::index::knn_vector_values::DocIndexIteratorBase;
use crate::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::store::dummy::dummy_index_input::DummyIndexInput;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{DataInput, DataOutput, IndexInput, IndexOutput};
use crate::util::array_util::ArrayUtil;
use crate::util::bit_set::BitSet;
use crate::util::bit_set_iterator::BitSetIterator;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fixed_bit_set::FixedBitSet;
use byteorder::ReadBytesExt;
use std::cell::RefCell;
use std::rc::Rc;

pub struct IndexedDISI<I>
where
    I: IndexInput,
{
    slice: I::Slice,
    jump_table: Rc<RefCell<Option<I::RandomAccessSlice>>>,
    jump_table_entry_count: i32,
    dense_rank_power: i8,
    dense_rank_table: Option<Vec<u8>>,
    cost: i64,
    block: i32,
    block_end: i64,
    // Only used for DENSE blocks
    dense_bitmap_offset: i64,
    next_block_index: i32,
    method: Method,
    doc: i32,
    index: i32,
    // SPARSE variables
    exists: bool,
    next_exist_doc_in_block: i32,
    // DENSE variables
    word: i64,
    word_index: i32,
    // number of one bits encountered so far, including those of `word`
    number_of_ones: i32,
    // Used with rank for jumps inside of DENSE as they are absolute instead of relative
    dense_origo_index: i32,
    // ALL variables
    gap: i32,
}
impl IndexedDISI<DummyIndexInput> {
    // The number of docIDs that a single block represents
    const BLOCK_SIZE: i32 = 65536;
    // Long.SIZE = 64 bits
    const DENSE_BLOCK_LONGS: i32 = Self::BLOCK_SIZE / i64::BITS as i32;
    // Every 512 docIDs / 8 longs
    pub const DEFAULT_DENSE_RANK_POWER: i8 = 9;
    const MAX_ARRAY_LENGTH: i32 = (1 << 12) - 1;
    pub fn write_bitset<O>(it: &mut impl DocIdSetIterator, out: &mut O) -> Result<i16>
    where
        O: IndexOutput,
    {
        Self::write_bitset_with_dense_rank_power(it, out, IndexedDISI::DEFAULT_DENSE_RANK_POWER)
    }
    pub fn write_bitset_with_dense_rank_power<O>(
        it: &mut impl DocIdSetIterator,
        out: &mut O,
        dense_rank_power: i8,
    ) -> Result<i16>
    where
        O: IndexOutput,
    {
        let origo = out.get_file_pointer();
        if !(7..=15).contains(&dense_rank_power) && dense_rank_power != -1 {
            return Err(LuceneError::illegal_argument(format!(
                "Acceptable values for denseRankPower are 7-15 (every 128-32768 docIDs). \
             The provided power was {} (every {} docIDs)",
                dense_rank_power,
                1 << dense_rank_power
            )));
        }

        let mut total_cardinality = 0;
        let mut block_cardinality = 0;
        let mut buffer = FixedBitSet::new(1 << 16);
        let jumps_len = ArrayUtil::oversize(1, (BitUtil::INT_BYTES * 2) as i32);
        let mut jumps: Vec<i32> = vec![0; jumps_len as usize];
        let mut prev_block = -1;
        let mut jump_block_index = 0;

        let mut doc = it.next_doc()?;
        while doc != NO_MORE_DOCS {
            let block = doc >> 16;

            if prev_block != -1 && block != prev_block {
                Self::add_jumps(
                    &mut jumps,
                    out.get_file_pointer() - origo,
                    total_cardinality,
                    jump_block_index,
                    prev_block + 1,
                )?;
                jump_block_index = prev_block + 1;
                Self::flush(
                    prev_block,
                    &buffer,
                    block_cardinality,
                    dense_rank_power,
                    out,
                )?;
                buffer.clear();
                total_cardinality += block_cardinality;
                block_cardinality = 0;
            }

            buffer.set(doc & 0xFFFF);
            block_cardinality += 1;
            prev_block = block;

            doc = it.next_doc()?;
        }

        if block_cardinality > 0 {
            Self::add_jumps(
                &mut jumps,
                out.get_file_pointer() - origo,
                total_cardinality,
                jump_block_index,
                prev_block + 1,
            )?;
            total_cardinality += block_cardinality;
            Self::flush(
                prev_block,
                &buffer,
                block_cardinality,
                dense_rank_power,
                out,
            )?;
            buffer.clear();
            prev_block += 1;
        }

        let last_block = if prev_block == -1 { 0 } else { prev_block };
        // There will always be at least 1 block (NO_MORE_DOCS)
        // Last entry is a SPARSE with blockIndex == 32767 and the single entry 65535, which becomes the
        // docID NO_MORE_DOCS
        // To avoid creating 65K jump-table entries, only a single entry is created pointing to the
        // offset of the
        // NO_MORE_DOCS block, with the jumpBlockIndex set to the logical EMPTY block after all real
        // blocks.
        Self::add_jumps(
            &mut jumps,
            out.get_file_pointer() - origo,
            total_cardinality,
            last_block,
            last_block + 1,
        )?;

        buffer.set(NO_MORE_DOCS & 0xFFFF);
        Self::flush(NO_MORE_DOCS >> 16, &buffer, 1, dense_rank_power, out)?;

        Self::flush_block_jumps(&jumps, last_block + 1, out)
    }
}
impl<I> IndexedDISI<I>
where
    I: IndexInput,
{
    pub fn new(
        index_input: &mut I,
        offset: i64,
        length: i64,
        jump_table_entry_count: i32,
        dense_rank_power: i8,
        cost: i64,
    ) -> Result<Self> {
        let block_slice =
            Self::create_block_slice(index_input, "docs", offset, length, jump_table_entry_count)?;
        let jump_table = Rc::new(RefCell::new(Self::create_jump_table(
            index_input,
            offset,
            length,
            jump_table_entry_count,
        )?));

        Self::from_components(
            block_slice,
            jump_table,
            jump_table_entry_count,
            dense_rank_power,
            cost,
        )
    }

    pub fn from_components(
        mut index_input: I::Slice,
        jump_table: Rc<RefCell<Option<I::RandomAccessSlice>>>,
        jump_table_entry_count: i32,
        dense_rank_power: i8,
        cost: i64,
    ) -> Result<Self> {
        if !(7..=15).contains(&dense_rank_power) && dense_rank_power != -1 {
            return Err(LuceneError::illegal_argument(format!(
                "Acceptable values for denseRankPower are 7-15 (every 128-32768 docIDs). \
                     The provided power was {} (every {} docIDs).",
                dense_rank_power,
                1 << dense_rank_power
            )));
        }

        if index_input.length() > 0 {
            index_input.prefetch(0, 1)?;
        }

        if let Some(jump) = &mut *jump_table.borrow_mut() {
            if jump.length() > 0 {
                jump.pre_fetch(0, 1)?;
            }
        }

        let dense_rank_table = if dense_rank_power == -1 {
            None
        } else {
            let rank_index_shift = dense_rank_power - 7;
            Some(vec![
                0u8;
                (IndexedDISI::DENSE_BLOCK_LONGS >> rank_index_shift)
                    as usize
            ])
        };

        Ok(Self {
            slice: index_input,
            jump_table,
            jump_table_entry_count,
            dense_rank_power,
            dense_rank_table,
            cost,
            block: -1,
            block_end: 0,
            dense_bitmap_offset: -1,
            next_block_index: -1,
            method: Method::Sparse,
            doc: -1,
            index: -1,
            exists: false,
            next_exist_doc_in_block: 0,
            word: 0,
            word_index: -1,
            number_of_ones: 0,
            dense_origo_index: 0,
            gap: 0,
        })
    }
    fn flush<O>(
        block: i32,
        buffer: &FixedBitSet,
        cardinality: i32,
        dense_rank_power: i8,
        out: &mut O,
    ) -> Result<()>
    where
        O: IndexOutput,
    {
        debug_assert!((0..IndexedDISI::BLOCK_SIZE).contains(&block));
        out.write_short(block as i16)?;
        debug_assert!(cardinality > 0 && cardinality <= IndexedDISI::BLOCK_SIZE);
        out.write_short((cardinality - 1) as i16)?;

        if cardinality > IndexedDISI::MAX_ARRAY_LENGTH {
            if cardinality != IndexedDISI::BLOCK_SIZE {
                if dense_rank_power != -1 {
                    let rank = Self::create_rank(buffer, dense_rank_power as u8);
                    let rank_len = rank.len();
                    debug_assert!(rank_len <= i32::MAX as usize);
                    out.write_bytes_with_len(&rank, rank_len as i32)?;
                }
                for word in buffer.get_bits() {
                    out.write_long(*word)?;
                }
            }
        } else {
            let mut iter = BitSetIterator::new(buffer, cardinality as i64)?;
            let mut doc;
            while {
                doc = iter.next_doc()?;
                doc != NO_MORE_DOCS
            } {
                out.write_short(doc as i16)?;
            }
        }

        Ok(())
    }

    // Creates a DENSE rank-entry (the number of set bits up to a given point) for the buffer.
    // One rank-entry for every {@code 2^denseRankPower} bits, with each rank-entry using 2 bytes.
    // Represented as a byte[] for fast flushing and mirroring of the retrieval representation.
    fn create_rank(buffer: &FixedBitSet, dense_rank_power: u8) -> Vec<u8> {
        let longs_per_rank = 1 << (dense_rank_power - 6);
        let rank_mark = longs_per_rank - 1;
        // 6 for the long (2^6) + 1 for 2 bytes/entry
        let rank_index_shift = dense_rank_power - 7;
        let rank = (IndexedDISI::DENSE_BLOCK_LONGS >> rank_index_shift) as usize;
        let mut rank = vec![0u8; rank];
        let bits = buffer.get_bits();
        let mut bit_count = 0;
        for word in 0..IndexedDISI::DENSE_BLOCK_LONGS {
            // Every longsPerRank longs
            if (word & rank_mark) == 0 {
                let rank_index = (word >> rank_index_shift) as usize;
                rank[rank_index] = (bit_count >> 8) as u8;
                rank[rank_index + 1] = (bit_count & 0xFF) as u8;
            }
            bit_count += bits[word as usize].count_ones() as i32;
        }
        rank
    }

    // Adds entries to the offset & index jump-table for blocks
    fn add_jumps(
        jumps: &mut Vec<i32>,
        offset: i64,
        index: i32,
        start_block: i32,
        end_block: i32,
    ) -> Result<()> {
        debug_assert!(
            offset < i32::MAX as i64,
            "Logically the offset should not exceed 2^30 but was >= i32::MAX"
        );
        ArrayUtil::grow_i32(jumps, (end_block + 1) * 2)?;
        for b in start_block..end_block {
            let i = (b * 2) as usize;
            jumps[i] = index;
            jumps[i + 1] = offset as i32;
        }
        Ok(())
    }
    // Flushes the offset & index jump-table for blocks. This should be the last data written to out
    // This method returns the blockCount for the blocks reachable for the jump_table or -1 for no
    // jump-table
    fn flush_block_jumps<O: IndexOutput>(
        jumps: &[i32],
        mut block_count: i32,
        out: &mut O,
    ) -> Result<i16> {
        // Jumps with a single real entry + NO_MORE_DOCS is just wasted space so we ignore
        // that
        if block_count == 2 {
            block_count = 0;
        }

        for i in 0..block_count as usize {
            out.write_int(jumps[i * 2])?;
            out.write_int(jumps[i * 2 + 1])?;
        }
        // As there are at most 32k blocks, the count is a short
        // The jumpTableOffset will be at lastPos - (blockCount * Long.BYTES)
        Ok(block_count as i16)
    }
    fn create_block_slice(
        slice: &mut I,
        slice_description: &str,
        offset: i64,
        length: i64,
        jump_table_entry_count: i32,
    ) -> Result<I::Slice> {
        let jump_table_bytes = if jump_table_entry_count < 0 {
            0
        } else {
            jump_table_entry_count as i64 * BitUtil::INT_BYTES as i64 * 2
        };
        slice.slice(slice_description, offset, length - jump_table_bytes)
    }
    fn create_jump_table(
        slice: &mut I,
        offset: i64,
        length: i64,
        jump_table_entry_count: i32,
    ) -> Result<Option<I::RandomAccessSlice>> {
        if jump_table_entry_count <= 0 {
            Ok(None)
        } else {
            let jump_table_bytes = (jump_table_entry_count as i64) * BitUtil::INT_BYTES as i64 * 2;
            slice
                .random_access_slice(offset + length - jump_table_bytes, jump_table_bytes)
                .map(Some)
        }
    }
    pub fn advance_exact(&mut self, target: i32) -> Result<bool> {
        let target_block = ((target as u32) & 0xFFFF_0000) as i32;

        if self.block < target_block {
            self.advance_block(target_block)?;
        }

        let found = self.block == target_block && {
            match self.method {
                Method::Sparse => SparseMethod.advance_exact_within_block(self, target)?,
                Method::Dense => DenseMethod.advance_exact_within_block(self, target)?,
                Method::ALL => All.advance_exact_within_block(self, target)?,
            }
        };
        self.doc = target;
        Ok(found)
    }
    fn advance_block(&mut self, target_block: i32) -> Result<()> {
        let block_index = target_block >> 16;
        // If the destination block is 2 blocks or more ahead, we use the jump-table.
        let is_some = {
            let writer = self.jump_table.borrow();
            writer.is_some()
        };
        if is_some && block_index >= (self.block >> 16) + 2 {
            // If the jumpTableEntryCount is exceeded, there are no further bits. Last entry is always
            // NO_MORE_DOCS
            let in_range_block_index = if block_index < self.jump_table_entry_count {
                block_index
            } else {
                self.jump_table_entry_count - 1
            };

            let jump_pos = in_range_block_index as i64 * BitUtil::INT_BYTES as i64 * 2;
            let index;
            let offset;
            {
                let mut jump_table_borrow = self.jump_table.borrow_mut();
                let jump_table = jump_table_borrow.as_mut().unwrap();
                index = jump_table.read_int(jump_pos)?;
                offset = jump_table.read_int(jump_pos + BitUtil::INT_BYTES as i64)?;
            }
            // -1 to compensate for the always-added 1 in readBlockHeader
            self.next_block_index = index - 1;
            self.slice.seek(offset as i64)?;
            self.read_block_header()?;
            return Ok(());
        }
        // Fallback to iteration of blocks
        while self.block < target_block {
            self.slice.seek(self.block_end)?;
            self.read_block_header()?;
        }

        Ok(())
    }

    fn read_block_header(&mut self) -> Result<()> {
        self.block = (self.slice.read_short()? as u16 as i32) << 16;
        debug_assert!(self.block >= 0);
        let num_values = 1 + self.slice.read_short()? as u16 as i32;

        self.index = self.next_block_index;
        self.next_block_index = self.index + num_values;

        if num_values <= IndexedDISI::MAX_ARRAY_LENGTH {
            self.method = Method::Sparse;
            self.block_end = self.slice.get_file_pointer() + (num_values << 1) as i64;
            self.next_exist_doc_in_block = -1;
        } else if num_values == IndexedDISI::BLOCK_SIZE {
            self.method = Method::ALL;
            self.block_end = self.slice.get_file_pointer();
            self.gap = self.block - self.index - 1;
        } else {
            self.method = Method::Dense;
            self.dense_bitmap_offset = self.slice.get_file_pointer()
                + self.dense_rank_table.as_ref().map(|v| v.len()).unwrap_or(0) as i64;
            self.block_end = self.dense_bitmap_offset + (1 << 13);
            // Performance consideration: All rank (default 128 * 16 bits) are loaded up front. This
            // should be fast with the
            // reusable byte[] buffer, but it is still wasted if the DENSE block is iterated in small
            // steps.
            // If this results in too great a performance regression, a heuristic strategy might work
            // where the rank data
            // are loaded on first in-block advance, if said advance is > X docIDs. The hope being that a
            // small first
            // advance means that subsequent advances will be small too.
            // Another alternative is to maintain an extra slice for DENSE rank, but IndexedDISI is
            // already slice-heavy.
            if self.dense_rank_power != -1 {
                debug_assert!(self.dense_rank_table.is_some());
                let rank_table_len = self.dense_rank_table.as_ref().unwrap().len();
                debug_assert!(rank_table_len <= i32::MAX as usize);
                if let Some(rank_table) = self.dense_rank_table.as_mut() {
                    self.slice
                        .read_bytes(rank_table, 0, rank_table_len as i32)?;
                }
            }

            self.word_index = -1;
            self.number_of_ones = self.index + 1;
            self.dense_origo_index = self.number_of_ones;
        }

        Ok(())
    }
    fn index(&self) -> i32 {
        self.index
    }

    fn rank_skip(disi: &mut IndexedDISI<I>, target_in_block: i32) -> Result<()> {
        debug_assert!(
            disi.dense_rank_power >= 0,
            "dense_rank_power = {}",
            disi.dense_rank_power
        );
        // Resolve the rank as close to targetInBlock as possible (maximum distance is 8 longs)
        // Note: rankOrigoOffset is tracked on block open, so it is absolute (e.g. don't add origo)
        let rank_index = target_in_block >> disi.dense_rank_power; // Default is 9 (8 longs: 2^3 * 2^6 = 512 docIDs)
        let byte_index = (rank_index << 1) as usize;
        let mut rank = 0;
        match &disi.dense_rank_table {
            None => {
                Err::<(), LuceneError>(LuceneError::unreachable("should not be here"))?;
            }
            Some(rank_table) => {
                let high = rank_table[byte_index] as u16;
                let low = rank_table[byte_index + 1] as u16;
                rank = ((high << 8) | low) as i32;
            }
        }
        // Position the counting logic just after the rank point
        let rank_aligned_word_index = (rank_index << disi.dense_rank_power) >> 6;
        let offset = disi.dense_bitmap_offset
            + (rank_aligned_word_index as i64) * BitUtil::LONG_BYTES as i64;
        disi.slice.seek(offset)?;
        let rank_word = disi.slice.read_long()?;
        let dense_noo = rank + rank_word.count_ones() as i32;

        disi.word_index = rank_aligned_word_index;
        disi.word = rank_word;
        disi.number_of_ones = disi.dense_origo_index + dense_noo;

        Ok(())
    }

    pub fn get_doc_index_iterator<D>(disi: &mut IndexedDISI<I>) -> DocIndexIteratorImpl<I>
    where
        D: DocIdSetIterator + DocIndexIteratorBase,
    {
        DocIndexIteratorImpl::new(disi)
    }
}
pub struct DocIndexIteratorImpl<'a, I>
where
    I: IndexInput,
{
    disi: &'a mut IndexedDISI<I>,
}
impl<'a, I> DocIndexIteratorImpl<'a, I>
where
    I: IndexInput,
{
    pub fn new(disi: &'a mut IndexedDISI<I>) -> Self {
        DocIndexIteratorImpl { disi }
    }
}
impl<'a, I> DocIdSetIterator for DocIndexIteratorImpl<'a, I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.disi.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.disi.next_doc()
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        self.disi.advance(_target)
    }

    fn cost(&self) -> Result<i64> {
        self.disi.cost()
    }
}
impl<'a, I> DocIndexIteratorBase for DocIndexIteratorImpl<'a, I>
where
    I: IndexInput,
{
    fn index(&self) -> Result<i32> {
        Ok(self.disi.index())
    }
}
impl<I> DocIdSetIterator for IndexedDISI<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc + 1)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        let target_block = ((target as u32) & 0xFFFF_0000) as i32;

        if self.block < target_block {
            self.advance_block(target_block)?;
        }

        if self.block == target_block {
            let advanced = match self.method {
                Method::Sparse => SparseMethod.advance_within_block(self, target)?,
                Method::Dense => DenseMethod.advance_within_block(self, target)?,
                Method::ALL => All.advance_within_block(self, target)?,
            };
            if advanced {
                return Ok(self.doc);
            }
            self.read_block_header()?;
        }

        let found = match self.method {
            Method::Sparse => SparseMethod.advance_within_block(self, self.block)?,
            Method::Dense => DenseMethod.advance_within_block(self, self.block)?,
            Method::ALL => All.advance_within_block(self, self.block)?,
        };
        debug_assert!(found);
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.cost)
    }
}
impl<I> DocIndexIteratorBase for IndexedDISI<I>
where
    I: IndexInput,
{
    fn index(&self) -> Result<i32> {
        Ok(self.index)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Sparse,
    Dense,
    ALL,
}
impl MethodBehavior for Method {
    fn advance_within_block<I: IndexInput>(
        &self,
        disi: &mut IndexedDISI<I>,
        target: i32,
    ) -> Result<bool> {
        match self {
            Method::Sparse => SparseMethod.advance_within_block(disi, target),
            Method::Dense => DenseMethod.advance_within_block(disi, target),
            Method::ALL => All.advance_within_block(disi, target),
        }
    }

    fn advance_exact_within_block<I: IndexInput>(
        &self,
        disi: &mut IndexedDISI<I>,
        target: i32,
    ) -> Result<bool> {
        match self {
            Method::Sparse => SparseMethod.advance_exact_within_block(disi, target),
            Method::Dense => DenseMethod.advance_exact_within_block(disi, target),
            Method::ALL => All.advance_exact_within_block(disi, target),
        }
    }
}
trait MethodBehavior {
    fn advance_within_block<I: IndexInput>(
        &self,
        disi: &mut IndexedDISI<I>,
        target: i32,
    ) -> Result<bool>;
    fn advance_exact_within_block<I: IndexInput>(
        &self,
        disi: &mut IndexedDISI<I>,
        target: i32,
    ) -> Result<bool>;
}

struct SparseMethod;
impl MethodBehavior for SparseMethod {
    fn advance_within_block<I: IndexInput>(
        &self,
        disi: &mut IndexedDISI<I>,
        target: i32,
    ) -> Result<bool> {
        let target_in_block = target & 0xFFFF;
        // TODO: binary search
        while disi.index < disi.next_block_index {
            let doc = disi.slice.read_short()? as u16 as i32;
            disi.index += 1;
            if doc >= target_in_block {
                disi.doc = disi.block | doc;
                disi.exists = true;
                disi.next_exist_doc_in_block = doc;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn advance_exact_within_block<I: IndexInput>(
        &self,
        disi: &mut IndexedDISI<I>,
        target: i32,
    ) -> Result<bool> {
        let target_in_block = target & 0xFFFF;
        // TODO: binary search
        if disi.next_exist_doc_in_block > target_in_block {
            debug_assert!(!disi.exists);
            return Ok(false);
        }

        if disi.doc == target {
            return Ok(disi.exists);
        }
        while disi.index < disi.next_block_index {
            let doc = disi.slice.read_short()? as u16 as i32;
            disi.index += 1;
            if doc >= target_in_block {
                disi.next_exist_doc_in_block = doc;
                if doc != target_in_block {
                    disi.index -= 1;
                    disi.slice.seek(disi.slice.get_file_pointer() - 2)?;
                    break;
                }
                disi.exists = true;
                return Ok(true);
            }
        }
        disi.exists = false;
        Ok(false)
    }
}
struct DenseMethod;
impl MethodBehavior for DenseMethod {
    fn advance_within_block<I: IndexInput>(
        &self,
        disi: &mut IndexedDISI<I>,
        target: i32,
    ) -> Result<bool> {
        let target_in_block = target & 0xFFFF;
        let target_word_index = (target_in_block as u32 >> 6) as i32;
        // If possible, skip ahead using the rank cache
        // If the distance between the current position and the target is < rank-longs
        // there is no sense in using rank
        if disi.dense_rank_power != -1
            && target_word_index - disi.word_index >= (1 << (disi.dense_rank_power - 6))
        {
            IndexedDISI::rank_skip(disi, target_in_block)?;
        }

        for i in disi.word_index + 1..=target_word_index {
            disi.word = disi.slice.read_long()?;
            disi.number_of_ones += disi.word.count_ones() as i32;
        }
        disi.word_index = target_word_index;

        let left_bits = (disi.word as u64) >> (target_in_block & 63);
        if left_bits != 0 {
            disi.doc = target + left_bits.trailing_zeros() as i32;
            disi.index = disi.number_of_ones - left_bits.count_ones() as i32;
            return Ok(true);
        }

        while {
            disi.word_index += 1;
            disi.word_index < 1024
        } {
            disi.word = disi.slice.read_long()?;
            if disi.word != 0 {
                disi.index = disi.number_of_ones;
                disi.number_of_ones += disi.word.count_ones() as i32;
                disi.doc =
                    disi.block | ((disi.word_index << 6) | disi.word.trailing_zeros() as i32);
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn advance_exact_within_block<I: IndexInput>(
        &self,
        disi: &mut IndexedDISI<I>,
        target: i32,
    ) -> Result<bool> {
        let target_in_block = target & 0xFFFF;
        let target_word_index = ((target_in_block as u32) >> 6) as i32;
        // If possible, skip ahead using the rank cache
        // If the distance between the current position and the target is < rank-longs
        // there is no sense in using rank
        if disi.dense_rank_power != -1
            && target_word_index - disi.word_index >= (1 << (disi.dense_rank_power - 6))
        {
            IndexedDISI::rank_skip(disi, target_in_block)?;
        }

        for i in (disi.word_index + 1)..=target_word_index {
            disi.word = disi.slice.read_long()?;
            disi.number_of_ones += disi.word.count_ones() as i32;
        }
        disi.word_index = target_word_index;

        let left_bits = (disi.word as u64 >> (target_in_block & 63)) as i64;
        disi.index = disi.number_of_ones - left_bits.count_ones() as i32;

        Ok((left_bits & 1) != 0)
    }
}
struct All;
impl MethodBehavior for All {
    fn advance_within_block<I: IndexInput>(
        &self,
        disi: &mut IndexedDISI<I>,
        target: i32,
    ) -> Result<bool> {
        disi.doc = target;
        disi.index = target - disi.gap;
        Ok(true)
    }

    fn advance_exact_within_block<I: IndexInput>(
        &self,
        disi: &mut IndexedDISI<I>,
        target: i32,
    ) -> Result<bool> {
        disi.index = target - disi.gap;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use crate::codecs::lucene90::indexed_disi::{IndexedDISI, Method};
    use crate::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
    use crate::store::directory::Directory;
    use crate::store::{IOContext, IndexInput, IndexOutput};
    use crate::test::util::lucene_test_case::{at_least, new_directory, random, rarely};
    use crate::test::util::test_util::TestUtil;
    use crate::util::bit_set::BitSet;
    use crate::util::bit_set_iterator::BitSetIterator;
    use crate::util::bit_set_type::BitSetType;
    use crate::util::error::lucene_error::{LuceneError, Result};
    use crate::util::fixed_bit_set::FixedBitSet;
    use crate::util::sparse_fixed_bit_set::SparseFixedBitSet;
    use rand::rngs::StdRng;
    use rand::Rng;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[allow(dead_code)] // for quick search
    struct TestIndexedDISI;

    #[test]
    fn test_empty() -> Result<()> {
        let mut random = random();
        let max_doc = TestUtil::next_int(&mut random, 1, 100_000);
        let set = SparseFixedBitSet::new(max_doc)?;
        let mut dir = new_directory(&mut random)?;
        do_test(&set, &mut dir, &mut random)
    }

    #[test]
    #[cfg(feature = "nightly")]
    fn test_empty_blocks() -> Result<()> {
        const B: i32 = 65536;
        let mut random = random();
        let max_doc = B * 11;
        let mut set = SparseFixedBitSet::new(max_doc)?;
        set.set(B + 5);
        set.set(B * 4 + 5);
        for i in 0..B {
            set.set(B * 6 + i);
        }
        for i in (0..B).step_by(3) {
            set.set(B * 7 + i);
        }
        for i in 0..B {
            if i != 32768 {
                set.set(B * 8 + i);
            }
        }
        {
            let mut dir = new_directory(&mut random)?;
            do_test_all_single_jump(&mut random, &set, &mut dir)?;
        }
        set.set(0);
        {
            let mut dir = new_directory(&mut random)?;
            do_test_all_single_jump(&mut random, &set, &mut dir)?
        }
        Ok(())
    }

    #[test]
    fn test_last_empty_blocks() -> Result<()> {
        let mut random = random();
        let mut dir = new_directory(&mut random)?;
        const B: i32 = 65536;
        let max_doc = B * 3;
        let mut set = SparseFixedBitSet::new(max_doc)?;
        for i in 0..(B * 2) {
            set.set(i);
        }
        do_test_all_single_jump(&mut random, &set, &mut dir)?;
        assert_advance_beyond_end(&set, &mut dir)
    }

    fn assert_advance_beyond_end(set: &impl BitSet, dir: &mut impl Directory) -> Result<()> {
        let cardinality = set.cardinality();
        let dense_rank_power = 9;
        let mut out = dir.create_output("bar", &IOContext::default_io_context()?)?;
        let jump_count = IndexedDISI::write_bitset_with_dense_rank_power(
            &mut BitSetIterator::new(set, cardinality as i64)?,
            &mut out,
            dense_rank_power,
        )?;
        let length = out.get_file_pointer();
        drop(out);

        let mut disi2 = BitSetIterator::new(set, cardinality as i64)?;
        let mut doc = disi2.doc_id();
        let mut index = 0;
        while doc < cardinality {
            doc = disi2.next_doc()?;
            index += 1;
        }

        let mut input = dir.open_input("bar", &IOContext::default_io_context()?)?;
        let mut disi = IndexedDISI::new(
            &mut input,
            0,
            length,
            jump_count as i32,
            dense_rank_power,
            cardinality as i64,
        )?;
        assert!(
            !disi.advance_exact(set.length())?,
            "There should be no set bit beyond the valid docID range"
        );
        disi.advance(doc)?;
        assert_eq!(
            index,
            disi.index() + 1,
            "The index when advancing beyond the last defined docID should be correct"
        );
        Ok(())
    }

    #[cfg(feature = "nightly")]
    #[test]
    fn test_random_blocks() -> Result<()> {
        let mut random = random();
        let mut dir = new_directory(&mut random)?;
        let set = create_set_with_random_blocks(&mut random, 5)?;
        do_test_all_single_jump(&mut random, &set, &mut dir)
    }

    #[test]
    fn test_position_not_zero() -> Result<()> {
        let mut random = random();
        let mut dir = new_directory(&mut random)?;
        const BLOCKS: i32 = 10;
        let dense_rank_power = if rarely(&mut random) {
            -1
        } else {
            (random.random_range(0..7) + 7) as i8
        };
        let set = create_set_with_random_blocks(&mut random, BLOCKS)?;
        let cardinality = set.cardinality();
        let mut out = dir.create_output("foo", &IOContext::default_io_context()?)?;
        let jump_table_entry_count = IndexedDISI::write_bitset_with_dense_rank_power(
            &mut BitSetIterator::new(&set, cardinality as i64)?,
            &mut out,
            dense_rank_power,
        )? as i32;
        let length = out.get_file_pointer();
        drop(out);

        let mut full_input = dir.open_input("foo", &IOContext::default_io_context()?)?;
        test_position_not_zero_extra(
            &mut random,
            &mut full_input,
            dense_rank_power,
            length,
            jump_table_entry_count,
            cardinality as i64,
            BLOCKS,
        )
    }
    fn test_position_not_zero_extra<I: IndexInput>(
        random: &mut StdRng,
        full_input: &mut I,
        dense_rank_power: i8,
        length: i64,
        jump_table_entry_count: i32,
        cardinality: i64,
        blocks: i32,
    ) -> Result<()> {
        let mut block_data = IndexedDISI::create_block_slice(
            full_input,
            "blocks",
            0,
            length,
            jump_table_entry_count,
        )?;
        block_data.seek(random.random_range(0..block_data.length()))?;
        let jump_table =
            IndexedDISI::create_jump_table(full_input, 0, length, jump_table_entry_count)?;
        assert!(jump_table.is_some());
        let mut disi: IndexedDISI<I> = IndexedDISI::from_components(
            block_data,
            Rc::new(RefCell::new(jump_table)),
            jump_table_entry_count,
            dense_rank_power,
            cardinality,
        )?;
        disi.advance_exact(blocks * 65536 - 1)?;
        Ok(())
    }

    fn create_set_with_random_blocks(
        random: &mut StdRng,
        block_count: i32,
    ) -> Result<SparseFixedBitSet> {
        const B: i32 = 65536;
        let mut set = SparseFixedBitSet::new(block_count * B)?;
        for block in 0..block_count {
            match random.random_range(0..4) {
                0 => {}
                1 => {
                    for doc_id in (block * B)..((block + 1) * B) {
                        set.set(doc_id);
                    }
                }
                2 => {
                    for doc_id in (block * B..(block + 1) * B).step_by(101) {
                        set.set(doc_id);
                    }
                }
                3 => {
                    for doc_id in (block * B..(block + 1) * B).step_by(3) {
                        set.set(doc_id);
                    }
                }
                _ => unreachable!(),
            }
        }
        Ok(set)
    }

    fn do_test_all_single_jump<D: Directory>(
        random: &mut StdRng,
        set: &impl BitSet,
        dir: &mut D,
    ) -> Result<()> {
        let cardinality = set.cardinality();
        let dense_rank_power = if rarely(random) {
            -1
        } else {
            (random.random_range(0..7) + 7) as i8
        };
        let mut out = dir.create_output("foo", &IOContext::default_io_context()?)?;
        let jump_table_entry_count = IndexedDISI::write_bitset_with_dense_rank_power(
            &mut BitSetIterator::new(set, cardinality as i64)?,
            &mut out,
            dense_rank_power,
        )? as i32;
        let length = out.get_file_pointer();
        drop(out);

        let mut input = dir.open_input("foo", &IOContext::default_io_context()?)?;
        for i in 0..set.length() {
            let mut disi = IndexedDISI::new(
                &mut input,
                0,
                length,
                jump_table_entry_count,
                dense_rank_power,
                cardinality as i64,
            )?;
            assert_eq!(set.get(i), disi.advance_exact(i)?);

            let mut disi2 = IndexedDISI::new(
                &mut input,
                0,
                length,
                jump_table_entry_count,
                dense_rank_power,
                cardinality as i64,
            )?;
            let doc = disi2.advance(i)?;
            assert!(i <= doc);
            if set.get(i) {
                assert_eq!(i, doc);
            } else {
                assert_ne!(i, doc);
            }
        }
        Ok(())
    }
    #[test]
    fn test_one_doc() -> Result<()> {
        let mut random = random();
        let max_doc = TestUtil::next_int(&mut random, 1, 100_000);
        let mut set = SparseFixedBitSet::new(max_doc)?;
        set.set(random.random_range(0..max_doc));
        let mut dir = new_directory(&mut random)?;
        do_test(&set, &mut dir, &mut random)
    }

    #[test]
    fn test_two_docs() -> Result<()> {
        let mut random = random();
        let max_doc = TestUtil::next_int(&mut random, 1, 100_000);
        let mut set = SparseFixedBitSet::new(max_doc)?;
        set.set(random.random_range(0..max_doc));
        set.set(random.random_range(0..max_doc));
        let mut dir = new_directory(&mut random)?;
        do_test(&set, &mut dir, &mut random)
    }

    #[test]
    fn test_all_docs() -> Result<()> {
        let mut random = random();
        let max_doc = TestUtil::next_int(&mut random, 1, 100_000);
        let mut set = FixedBitSet::new(max_doc);
        set.set_with_range(1, max_doc);
        let mut dir = new_directory(&mut random)?;
        do_test(&set, &mut dir, &mut random)
    }

    #[test]
    fn test_half_full() -> Result<()> {
        let mut random = random();
        let max_doc = TestUtil::next_int(&mut random, 1, 100_000);
        let mut set = SparseFixedBitSet::new(max_doc)?;
        let mut i = random.random_range(0..2);
        while i < max_doc {
            set.set(i);
            i += TestUtil::next_int(&mut random, 1, 3);
        }
        let mut dir = new_directory(&mut random)?;
        do_test(&set, &mut dir, &mut random)
    }

    #[test]
    fn test_doc_range() -> Result<()> {
        let mut random = random();
        let mut dir = new_directory(&mut random)?;

        for _ in 0..10 {
            let max_doc = TestUtil::next_int(&mut random, 1, 1_000_000);
            let mut set = FixedBitSet::new(max_doc);
            let start = random.random_range(0..max_doc);
            let end = TestUtil::next_int(&mut random, start + 1, max_doc);
            set.set_with_range(start, end);
            do_test(&set.clone(), &mut dir, &mut random)?;
        }

        Ok(())
    }

    #[test]
    fn test_sparse_dense_boundary() -> Result<()> {
        let mut random = random();
        let mut dir = new_directory(&mut random)?;
        let mut set = FixedBitSet::new(200_000);
        let start = 65536 + random.random_range(0..100);
        let dense_rank_power = if rarely(&mut random) {
            -1
        } else {
            (random.random_range(0..7) + 7) as i8
        };

        set.set_with_range(start, start + IndexedDISI::MAX_ARRAY_LENGTH);
        let mut out = dir.create_output("sparse", &IOContext::default_io_context()?)?;
        let jump_table_entry_count = IndexedDISI::write_bitset_with_dense_rank_power(
            &mut BitSetIterator::new(&set, IndexedDISI::MAX_ARRAY_LENGTH as i64)?,
            &mut out,
            dense_rank_power,
        )? as i32;
        let length = out.get_file_pointer();
        drop(out);

        {
            let mut input = dir.open_input("sparse", &IOContext::default_io_context()?)?;
            let mut disi = IndexedDISI::new(
                &mut input,
                0,
                length,
                jump_table_entry_count,
                dense_rank_power,
                IndexedDISI::MAX_ARRAY_LENGTH as i64,
            )?;
            assert_eq!(start, disi.next_doc()?);
            assert_eq!(Method::Sparse, disi.method);
        }

        do_test(&set, &mut dir, &mut random)?;

        set.set(start + IndexedDISI::MAX_ARRAY_LENGTH + random.random_range(0..100));
        let mut out = dir.create_output("bar", &IOContext::default_io_context()?)?;
        IndexedDISI::write_bitset_with_dense_rank_power(
            &mut BitSetIterator::new(&set, (IndexedDISI::MAX_ARRAY_LENGTH + 1) as i64)?,
            &mut out,
            dense_rank_power,
        )?;
        let length = out.get_file_pointer();
        drop(out);

        {
            let mut input = dir.open_input("bar", &IOContext::default_io_context()?)?;
            let mut disi = IndexedDISI::new(
                &mut input,
                0,
                length,
                jump_table_entry_count,
                dense_rank_power,
                (IndexedDISI::MAX_ARRAY_LENGTH + 1) as i64,
            )?;
            assert_eq!(start, disi.next_doc()?);
            assert_eq!(Method::Dense, disi.method);
        }

        do_test(&set, &mut dir, &mut random)
    }

    #[test]
    fn test_one_doc_missing() -> Result<()> {
        let mut random = random();
        let max_doc = TestUtil::next_int(&mut random, 1, 1_000_000);
        let mut set = FixedBitSet::new(max_doc);
        set.set_with_range(0, max_doc);
        set.clear_with_index(random.random_range(0..max_doc));
        let mut dir = new_directory(&mut random)?;
        do_test(&set, &mut dir, &mut random)
    }

    #[test]
    fn test_few_missing_docs() -> Result<()> {
        let mut random = random();
        let mut dir = new_directory(&mut random)?;
        let num_iters = at_least(&mut random, 10);

        for _ in 0..num_iters {
            let max_doc = TestUtil::next_int(&mut random, 1, 100_000);
            let mut set = FixedBitSet::new(max_doc);
            set.set_with_range(0, max_doc);
            let num_missing = TestUtil::next_int(&mut random, 2, 1000);
            for _ in 0..num_missing {
                set.clear_with_index(random.random_range(0..max_doc));
            }
            do_test(&set, &mut dir, &mut random)?;
        }

        Ok(())
    }

    #[test]
    fn test_dense_multi_block() -> Result<()> {
        let mut random = random();
        let mut dir = new_directory(&mut random)?;
        let max_doc = 10 * 65536;
        let mut set = FixedBitSet::new(max_doc);
        for i in (0..max_doc).step_by(2) {
            set.set(i);
        }
        do_test(&set, &mut dir, &mut random)
    }

    #[test]
    fn test_illegal_dense_rank_power() -> Result<()> {
        for &power in &[-1, 7, 8, 9, 10, 11, 12, 13, 14, 15] {
            create_and_open_disi(power, power)?;
        }

        for &power in &[-2, 0, 1, 6, 16] {
            assert!(matches!(
                create_and_open_disi(power, 8),
                Err(LuceneError::IllegalArgument(_))
            ));

            assert!(matches!(
                create_and_open_disi(8, power),
                Err(LuceneError::IllegalArgument(_))
            ));
        }

        Ok(())
    }

    fn create_and_open_disi(write_power: i8, read_power: i8) -> Result<()> {
        let mut set = FixedBitSet::new(10);
        set.set(9);
        let mut random = random();
        let mut dir = new_directory(&mut random)?;
        let mut out = dir.create_output("foo", &IOContext::default_io_context()?)?;
        let jump_count = IndexedDISI::write_bitset_with_dense_rank_power(
            &mut BitSetIterator::new(&set, set.cardinality() as i64)?,
            &mut out,
            write_power,
        )? as i32;
        let length = out.get_file_pointer();
        drop(out);

        let mut input = dir.open_input("foo", &IOContext::default_io_context()?)?;
        let _ = IndexedDISI::new(
            &mut input,
            0,
            length,
            jump_count,
            read_power,
            set.cardinality() as i64,
        )?;
        Ok(())
    }

    #[test]
    fn test_one_doc_missing_fixed() -> Result<()> {
        let mut random = random();
        let max_doc = 9699;
        let dense_rank_power = if rarely(&mut random) {
            -1
        } else {
            (random.random_range(0..7) + 7) as i8
        };

        let mut set = FixedBitSet::new(max_doc);
        set.set_with_range(0, max_doc);
        set.clear_with_index(1345);
        let cardinality = set.cardinality() as i64;

        let mut dir = new_directory(&mut random)?;
        let mut out = dir.create_output("foo", &IOContext::default_io_context()?)?;
        let jump_table_entry_count = IndexedDISI::write_bitset_with_dense_rank_power(
            &mut BitSetIterator::new(&set, cardinality)?,
            &mut out,
            dense_rank_power,
        )? as i32;
        let length = out.get_file_pointer();
        drop(out);

        let mut disi2 = BitSetIterator::new(&set, cardinality)?;
        let mut input = dir.open_input("foo", &IOContext::default_io_context()?)?;
        let mut disi = IndexedDISI::new(
            &mut input,
            0,
            length,
            jump_table_entry_count,
            dense_rank_power,
            cardinality,
        )?;
        assert_advance_equality(&mut disi, &mut disi2, 16000)
    }

    #[test]
    fn test_random() -> Result<()> {
        let mut random = random();
        let mut dir = new_directory(&mut random)?;
        let num_iters = at_least(&mut random, 3);

        for _ in 0..num_iters {
            do_test_random(&mut dir, &mut random)?;
        }

        Ok(())
    }

    fn do_test_random(dir: &mut impl Directory, random: &mut StdRng) -> Result<()> {
        let end = TestUtil::next_int(random, 2, 20);
        let max_step = TestUtil::next_int(random, 1, 1 << end);
        let num_docs =
            TestUtil::next_int(random, 1, std::cmp::min(100_000, (i32::MAX - 1) / max_step));

        let mut docs = SparseFixedBitSet::new(num_docs * max_step + 1)?;
        let mut last_doc = -1;

        let mut doc = -1;
        for _ in 0..num_docs {
            doc += TestUtil::next_int(random, 1, max_step);
            docs.set(doc);
            last_doc = doc;
        }

        let max_doc = last_doc + TestUtil::next_int(random, 1, 100);
        let cardinality = docs.approximate_cardinality();
        let bit_set_iterator = BitSetIterator::new(&docs, cardinality as i64)?;
        let set = <BitSetType as BitSet>::of(bit_set_iterator, max_doc)?;

        do_test(&set, dir, random)
    }

    fn do_test(set: &impl BitSet, dir: &mut impl Directory, random: &mut StdRng) -> Result<()> {
        let cardinality = set.cardinality() as i64;
        let dense_rank_power = if rarely(random) {
            -1
        } else {
            (random.random_range(0..7) + 7) as i8
        };

        let length;
        let jump_table_entry_count;

        {
            let mut out = dir.create_output("foo", &IOContext::default_io_context()?)?;
            jump_table_entry_count = IndexedDISI::write_bitset_with_dense_rank_power(
                &mut BitSetIterator::new(set, cardinality)?,
                &mut out,
                dense_rank_power,
            )? as i32;
            length = out.get_file_pointer();
        }

        {
            let mut input = dir.open_input("foo", &IOContext::default_io_context()?)?;
            let mut disi = IndexedDISI::new(
                &mut input,
                0,
                length,
                jump_table_entry_count,
                dense_rank_power,
                cardinality,
            )?;
            let mut disi2 = BitSetIterator::new(set, cardinality)?;
            assert_single_step_equality(&mut disi, &mut disi2)?;
        }

        for &step in &[1, 10, 100, 1000, 10000, 100000] {
            let mut input = dir.open_input("foo", &IOContext::default_io_context()?)?;
            let mut disi = IndexedDISI::new(
                &mut input,
                0,
                length,
                jump_table_entry_count,
                dense_rank_power,
                cardinality,
            )?;
            let mut disi2 = BitSetIterator::new(set, cardinality)?;
            assert_advance_equality(&mut disi, &mut disi2, step)?;
        }

        for &step in &[10, 100, 1000, 10000, 100000] {
            let mut input = dir.open_input("foo", &IOContext::default_io_context()?)?;
            let mut disi = IndexedDISI::new(
                &mut input,
                0,
                length,
                jump_table_entry_count,
                dense_rank_power,
                cardinality,
            )?;
            let disi2_length = set.length();
            let mut disi2 = BitSetIterator::new(set, cardinality)?;
            assert_advance_exact_randomized(random, &mut disi, &mut disi2, disi2_length, step)?;
        }

        dir.delete_file("foo")?;
        Ok(())
    }

    fn assert_advance_exact_randomized<I: IndexInput, T: BitSet>(
        random: &mut StdRng,
        disi: &mut IndexedDISI<I>,
        disi2: &mut BitSetIterator<T>,
        disi2_length: i32,
        step: i32,
    ) -> Result<()> {
        let mut index = -1;
        let mut target = 0;

        while target < disi2_length {
            target += TestUtil::next_int(random, 0, step);
            let mut doc = disi2.doc_id();
            while doc < target {
                doc = disi2.next_doc()?;
                index += 1;
            }

            let exists = disi.advance_exact(target)?;
            assert_eq!(doc == target, exists);
            if exists {
                assert_eq!(index, disi.index());
            } else if random.random_bool(0.5) {
                let advanced_doc = disi.next_doc()?;
                assert_eq!(doc, advanced_doc);
                // This is a bit strange when doc == NO_MORE_DOCS as the index overcounts in the disi2
                // while-loop
                assert_eq!(index, disi.index());
                target = doc;
            }
        }

        Ok(())
    }
    fn assert_single_step_equality<I: IndexInput, T: BitSet>(
        disi: &mut IndexedDISI<I>,
        disi2: &mut BitSetIterator<T>,
    ) -> Result<()> {
        let mut i = 0;
        let mut doc = disi2.next_doc()?;

        while doc != NO_MORE_DOCS {
            assert_eq!(doc, disi.next_doc()?);
            assert_eq!(i, disi.index());
            i += 1;
            doc = disi2.next_doc()?;
        }

        assert_eq!(NO_MORE_DOCS, disi.next_doc()?);
        Ok(())
    }
    fn assert_advance_equality<I: IndexInput, T: BitSet>(
        disi: &mut IndexedDISI<I>,
        disi2: &mut BitSetIterator<T>,
        step: i32,
    ) -> Result<()> {
        let mut index = -1;

        loop {
            let target = disi2.doc_id() + step;
            let mut doc;

            loop {
                doc = disi2.next_doc()?;
                index += 1;
                if doc >= target {
                    break;
                }
            }

            let advanced = disi.advance(target)?;
            assert_eq!(doc, advanced);

            if doc == NO_MORE_DOCS {
                break;
            }

            assert_eq!(
                index,
                disi.index(),
                "Expected equality using step {} at docID {}",
                step,
                doc
            );
        }

        Ok(())
    }
}
