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
use crate::store::{DataInput, DataOutput};
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::LuceneError;
use crate::util::CommonUtil;
use std::sync::{Arc, Mutex};

/// LZ4 compression and decompression routines.
///
/// <https://github.com/lz4/lz4/tree/dev/lib> <http://fastcompression.blogspot.fr/p/lz4.html>
///
/// The high-compression option is a simpler version of the one of the original algorithm, and
/// only retains a better hash table that remembers about more occurrences of a previous 4-bytes
/// sequence, and removes all the logic about handling of the case when overlapping matches are
/// found.
pub struct LZ4;

impl LZ4 {
    /// Window size: this is the maximum supported distance between two strings so that LZ4 can replace
    /// the second one by a reference to the first one.
    pub const MAX_DISTANCE: i32 = 1 << 16; // maximum distance of a reference

    pub const MEMORY_USAGE: i32 = 14;
    pub const MIN_MATCH: i32 = 4; // minimum length of a match
    pub const LAST_LITERALS: i32 = 5; // the last 5 bytes must be encoded as literals
    pub const HASH_LOG_HC: i32 = 15; // log size of the dictionary for compressHC
    pub const HASH_TABLE_SIZE_HC: i32 = 1 << LZ4::HASH_LOG_HC;

    fn hash(i: i32, hash_bits: i32) -> i32 {
        ((i.wrapping_mul(-1640531535) as u32) >> (32 - hash_bits)) as i32
    }

    fn hash_hc(i: i32) -> i32 {
        Self::hash(i, LZ4::HASH_LOG_HC)
    }

    /// Note: This method expects the data to be read in little-endian byte order.
    /// Ensure that the input data is in little-endian format, or it may result in incorrect parsing.
    fn read_int(buf: &[u8], i: i32) -> i32 {
        // According to LZ4's algorithm the endianness does not matter at all:
        BitUtil::get_i32_le(buf, i as usize)
    }

    fn common_bytes(b: &[u8], o1: i32, o2: i32, limit: i32) -> i32 {
        debug_assert!(o1 < o2);
        // never -1 because lengths always differ
        CommonUtil::miss_match(
            &b[(o1 as usize)..(limit as usize)],
            &b[(o2 as usize)..(limit as usize)],
        )
    }

    /// Decompress at least `decompressed_len` bytes into `dest[d_off..]`.
    /// Please note that `dest` must be large enough to hold **all** decompressed data
    /// (meaning that you need to know the total decompressed length). If the given bytes were
    /// compressed using a preset dictionary, the same dictionary must be provided in
    /// `dest[d_off-dict_len..d_off]`.
    pub fn decompress<D>(
        compressed: &mut D,
        decompressed_len: i32,
        dest: &mut [u8],
        d_off: i32,
    ) -> Result<i32, LuceneError>
    where
        D: DataInput,
    {
        let dest_end = d_off + decompressed_len;
        let mut d_off = d_off;

        loop {
            let token = compressed.read_byte()? as i32;
            let mut literal_len = (token as u32 >> 4) as i32;

            if literal_len != 0 {
                if literal_len == 0x0F {
                    loop {
                        let len = compressed.read_byte()?;
                        if len != 0xFF {
                            literal_len += len as i32;
                            break;
                        }
                        literal_len += 0xFF;
                    }
                }
                compressed.read_bytes(dest, d_off, literal_len)?;
                d_off += literal_len;
            }

            if d_off >= dest_end {
                break;
            }

            // Read matches
            let match_dec = compressed.read_short()? as i32 & 0xFFFF;
            assert!(match_dec > 0);

            let mut match_len = token & 0x0F;
            if match_len == 0x0F {
                loop {
                    let len = compressed.read_byte()?;
                    if len != 0xFF {
                        match_len += len as i32;
                        break;
                    }
                    match_len += 0xFF;
                }
            }
            match_len += LZ4::MIN_MATCH;

            // copying a multiple of 8 bytes can make decompression from 5% to 10% faster
            let fast_len = (match_len + 7) & 0xFFF8;

            if match_dec < match_len || d_off + fast_len > dest_end {
                // overlap -> naive incremental copy
                let start = d_off - match_dec;
                let end = d_off + match_len;
                for (i, ref_idx) in (start..end).enumerate() {
                    dest[d_off as usize + i] = dest[ref_idx as usize];
                }
            } else {
                // Non-overlap block copy
                let src_start = d_off - match_dec;
                let dest_start = d_off;
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        dest.as_ptr().add(src_start as usize),
                        dest.as_mut_ptr().add(dest_start as usize),
                        fast_len as usize,
                    );
                }
                d_off += match_len;
            }

            if d_off >= dest_end {
                break;
            }
        }
        Ok(d_off)
    }
    fn encode_len<D>(mut l: i32, out: &mut D) -> Result<(), LuceneError>
    where
        D: DataOutput,
    {
        while l >= 0xFF {
            out.write_byte(0xFF)?;
            l -= 0xFF;
        }
        out.write_byte(l as u8)?;
        Ok(())
    }
    fn encode_literals<D>(
        bytes: &[u8],
        token: i32,
        anchor: i32,
        literal_len: i32,
        out: &mut D,
    ) -> Result<(), LuceneError>
    where
        D: DataOutput,
    {
        out.write_byte(token as u8)?;

        // encode literal length
        if literal_len >= 0x0F {
            Self::encode_len(literal_len - 0x0F, out)?;
        }

        // encode literals
        out.write_bytes_range(bytes, anchor, literal_len)?;

        Ok(())
    }
    fn encode_last_literals<D>(
        bytes: &[u8],
        anchor: i32,
        literal_len: i32,
        out: &mut D,
    ) -> Result<(), LuceneError>
    where
        D: DataOutput,
    {
        let token = std::cmp::min(literal_len, 0x0F) << 4;
        Self::encode_literals(bytes, token, anchor, literal_len, out)
    }

    fn encode_sequence<D>(
        bytes: &[u8],
        anchor: i32,
        match_ref: i32,
        match_off: i32,
        match_len: i32,
        out: &mut D,
    ) -> Result<(), LuceneError>
    where
        D: DataOutput,
    {
        let literal_len = match_off - anchor;
        debug_assert!(match_len >= 4);
        // Encode token
        let token = (i32::min(literal_len, 0x0F) << 4) | i32::min(match_len - 4, 0x0F);
        Self::encode_literals(bytes, token, anchor, literal_len, out)?;

        // Encode match dec
        let match_dec = match_off - match_ref;
        debug_assert!(match_dec > 0 && match_dec < (1 << 16));
        out.write_short(match_dec as i16)?;

        // Encode match len
        if match_len >= Self::MIN_MATCH + 0x0F {
            Self::encode_len(match_len - 0x0F - Self::MIN_MATCH, out)?;
        }

        Ok(())
    }
    /// Compress `bytes[off:off+len]` into `out` using at most 16kB of memory.
    /// `ht` shouldn't be shared across threads but can safely be reused.
    pub fn compress<D>(
        bytes: Arc<Mutex<Vec<u8>>>,
        off: i32,
        len: i32,
        out: &mut D,
        ht: &mut HashTableEnum,
    ) -> Result<(), LuceneError>
    where
        D: DataOutput,
    {
        Self::compress_with_dictionary(bytes, off, 0, len, out, ht)
    }
    /// Compress `[dictOff+dictLen:dictOff+dictLen+len]` into `out` using at most 16kB
    /// of memory. `[dictOff:dictOff+dictLen]` will be used as a dictionary. `dictLen`
    /// must not be greater than `MAX_DISTANCE 64kB`, the maximum window size.
    /// `ht` shouldn't be shared across threads but can safely be reused.
    pub fn compress_with_dictionary<D>(
        bytes: Arc<Mutex<Vec<u8>>>,
        dict_off: i32,
        dict_len: i32,
        len: i32,
        out: &mut D,
        ht: &mut HashTableEnum,
    ) -> Result<(), LuceneError>
    where
        D: DataOutput,
    {
        let mut bytes_guard = bytes
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;

        // Ensure the indices are valid
        CommonUtil::check_from_index_size(dict_off, dict_len, bytes_guard.len() as i32)?;
        CommonUtil::check_from_index_size(dict_off + dict_len, len, bytes_guard.len() as i32)?;

        if dict_len > LZ4::MAX_DISTANCE {
            return Err(LuceneError::illegal_argument(format!(
                "dictLen must not be greater than 64kB, but got {}",
                dict_len
            )));
        }

        let end = dict_off + dict_len + len;
        let mut off = dict_off + dict_len;
        let mut anchor = off;
        if len > LZ4::LAST_LITERALS + LZ4::MIN_MATCH {
            let limit = end - LZ4::LAST_LITERALS;
            let match_limit = limit - LZ4::MIN_MATCH;
            ht.reset(bytes.clone(), dict_off, dict_len + len)?;
            ht.init_dictionary(dict_len)?;

            'outer: while off <= limit {
                // find a match
                let mut ref_idx;
                while {
                    if off >= match_limit {
                        break 'outer;
                    }
                    ref_idx = ht.get(off)?;
                    ref_idx == -1
                } {
                    off += 1;
                }

                // Compute match length
                let mut match_len = LZ4::MIN_MATCH
                    + LZ4::common_bytes(
                        bytes_guard.as_slice(),
                        ref_idx + LZ4::MIN_MATCH,
                        off + LZ4::MIN_MATCH,
                        limit,
                    );

                // Try to find a better match
                let min = (off - LZ4::MAX_DISTANCE + 1).max(dict_off);
                let mut r = ht.previous(ref_idx)?;
                while r >= min {
                    assert_eq!(
                        LZ4::read_int(bytes_guard.as_mut_slice(), r),
                        LZ4::read_int(bytes_guard.as_mut_slice(), off)
                    );
                    let r_match_len = LZ4::MIN_MATCH
                        + LZ4::common_bytes(
                            bytes_guard.as_mut_slice(),
                            r + LZ4::MIN_MATCH,
                            off + LZ4::MIN_MATCH,
                            limit,
                        );
                    if r_match_len > match_len {
                        ref_idx = r;
                        match_len = r_match_len;
                    }

                    r = ht.previous(r)?;
                }

                // Encode match
                LZ4::encode_sequence(
                    bytes_guard.as_mut_slice(),
                    anchor,
                    ref_idx,
                    off,
                    match_len,
                    out,
                )?;
                off += match_len;
                anchor = off;
            }
        }

        // Handle last literals
        let literal_len = end - anchor;
        assert!(literal_len >= LZ4::LAST_LITERALS || literal_len == len);
        LZ4::encode_last_literals(bytes_guard.as_mut_slice(), anchor, literal_len, out)?;

        Ok(())
    }
}

/// A record of previous occurrences of sequences of 4 bytes.
pub(crate) trait HashTable {
    /// Reset this hash table in order to compress the given content.
    fn reset(&mut self, b: Arc<Mutex<Vec<u8>>>, off: i32, len: i32) -> Result<(), LuceneError>;

    /// Init `dict_len` bytes to be used as a dictionary.
    fn init_dictionary(&mut self, dict_len: i32) -> Result<(), LuceneError>;

    /// Advance the cursor to `off` and return an index that stored the same 4 bytes as `b[off:off+4]`.
    /// This may only be called on strictly increasing sequences of offsets.
    /// A return value of `-1` indicates that no other index could be found.
    fn get(&mut self, off: i32) -> Result<i32, LuceneError>;

    /// Return an index that is less than `off` and stores the same 4 bytes.
    /// Unlike `get`, it doesn't need to be called on increasing offsets.
    /// A return value of `-1` indicates that no other index could be found.
    fn previous(&mut self, off: i32) -> Result<i32, LuceneError>;

    /// For testing purposes.
    fn assert_reset(&self) -> bool;
}
trait Table {
    fn set(&mut self, offset: i32, value: i32);
    fn get_and_set(&mut self, offset: i32, value: i32) -> i32;
    fn get_bits_per_value(&self) -> i32;
    fn size(&self) -> i32;
}
/// 16 bits per offset. This is by far the most commonly used table since it gets used whenever
/// compressing inputs whose size is <= 64kB.
struct Table16 {
    table: Vec<u16>,
}

impl Table16 {
    pub fn new(size: i32) -> Self {
        Table16 {
            table: vec![0; size as usize],
        }
    }
}

impl Table for Table16 {
    fn set(&mut self, index: i32, value: i32) {
        debug_assert!((0..(1 << 16)).contains(&value));
        self.table[index as usize] = value as u16;
    }

    fn get_and_set(&mut self, index: i32, value: i32) -> i32 {
        let prev = self.table[index as usize] as i32;
        self.set(index, value);
        prev
    }

    fn get_bits_per_value(&self) -> i32 {
        16
    }

    fn size(&self) -> i32 {
        self.table.len() as i32
    }
}
/// 32 bits per value, only used when inputs exceed 64kB, e.g. very large stored fields.
pub struct Table32 {
    table: Vec<i32>,
}

impl Table32 {
    pub fn new(size: i32) -> Self {
        Table32 {
            table: vec![0; size as usize],
        }
    }
}

impl Table for Table32 {
    fn set(&mut self, index: i32, value: i32) {
        self.table[index as usize] = value;
    }

    fn get_and_set(&mut self, index: i32, value: i32) -> i32 {
        let prev = self.table[index as usize];
        self.set(index, value);
        prev
    }

    fn get_bits_per_value(&self) -> i32 {
        32
    }

    fn size(&self) -> i32 {
        self.table.len() as i32
    }
}
enum TableEnum {
    Table16(Table16),
    Table32(Table32),
}
impl Table for TableEnum {
    fn set(&mut self, offset: i32, value: i32) {
        match self {
            TableEnum::Table16(table) => table.set(offset, value),
            TableEnum::Table32(table) => table.set(offset, value),
        }
    }

    fn get_and_set(&mut self, offset: i32, value: i32) -> i32 {
        match self {
            TableEnum::Table16(table) => table.get_and_set(offset, value),
            TableEnum::Table32(table) => table.get_and_set(offset, value),
        }
    }

    fn get_bits_per_value(&self) -> i32 {
        match self {
            TableEnum::Table16(table) => table.get_bits_per_value(),
            TableEnum::Table32(table) => table.get_bits_per_value(),
        }
    }

    fn size(&self) -> i32 {
        match self {
            TableEnum::Table16(table) => table.size(),
            TableEnum::Table32(table) => table.size(),
        }
    }
}

/// Simple lossy `HashTable` that only stores the last occurrence for each hash on `2^14` bytes of memory.
pub(crate) struct FastCompressionHashTable {
    bytes: Arc<Mutex<Vec<u8>>>,
    base: i32,
    last_off: i32,
    end: i32,
    hash_log: i32,
    hash_table: Option<TableEnum>,
}

impl FastCompressionHashTable {
    /// Sole constructor
    pub fn new() -> Self {
        FastCompressionHashTable {
            bytes: Arc::new(Mutex::new(vec![])),
            base: 0,
            last_off: 0,
            end: 0,
            hash_log: 0,
            hash_table: None,
        }
    }
}
impl HashTable for FastCompressionHashTable {
    fn reset(&mut self, bytes: Arc<Mutex<Vec<u8>>>, off: i32, len: i32) -> Result<(), LuceneError> {
        {
            let bytes_guard = bytes
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
            CommonUtil::check_from_index_size(off, len, bytes_guard.len() as i32)?;
        }
        self.bytes = bytes;
        self.base = off;
        self.end = off + len;

        let bits_per_offset = if (len - LZ4::LAST_LITERALS) < (1 << 16) {
            16
        } else {
            32
        };

        let bits_per_offset_log = 32 - ((bits_per_offset - 1) as i64).leading_zeros() as i32;
        self.hash_log = LZ4::MEMORY_USAGE + 3 - bits_per_offset_log;

        let need_new_table = match &self.hash_table {
            None => true,
            Some(table) => {
                table.size() < (1 << self.hash_log) || table.get_bits_per_value() < bits_per_offset
            }
        };

        if need_new_table {
            self.hash_table = if bits_per_offset > 16 {
                assert_eq!(bits_per_offset, 32);
                Some(TableEnum::Table32(Table32::new(1 << self.hash_log)))
            } else {
                assert_eq!(bits_per_offset, 16);
                Some(TableEnum::Table16(Table16::new(1 << self.hash_log)))
            };
        } else {
            // Avoid calling hashTable.clear(), this makes it costly to compress many short sequences
            // otherwise.
            // Instead, get() checks that references are less than the current offset.
        }

        self.last_off = off - 1;
        Ok(())
    }

    fn init_dictionary(&mut self, dict_len: i32) -> Result<(), LuceneError> {
        let mut bytes_guard = self
            .bytes
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        for i in 0..dict_len {
            let v = LZ4::read_int(bytes_guard.as_mut_slice(), self.base + i);
            let h = LZ4::hash(v, self.hash_log);
            debug_assert!(self.hash_table.is_some());
            if let Some(table) = &mut self.hash_table {
                table.set(h, i);
            }
        }
        self.last_off += dict_len;
        Ok(())
    }

    fn get(&mut self, off: i32) -> Result<i32, LuceneError> {
        debug_assert!(off > self.last_off);
        debug_assert!(off < self.end);
        let mut bytes_guard = self
            .bytes
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        let v = LZ4::read_int(bytes_guard.as_mut_slice(), off);
        let h = LZ4::hash(v, self.hash_log);

        let ref_idx = self.base
            + self
                .hash_table
                .as_mut()
                .unwrap()
                .get_and_set(h, off - self.base);
        self.last_off = off;

        if ref_idx < off
            && off - ref_idx < LZ4::MAX_DISTANCE
            && LZ4::read_int(bytes_guard.as_mut_slice(), ref_idx) == v
        {
            Ok(ref_idx)
        } else {
            Ok(-1)
        }
    }

    fn previous(&mut self, _off: i32) -> Result<i32, LuceneError> {
        Ok(-1)
    }

    fn assert_reset(&self) -> bool {
        true
    }
}
/// A higher-precision `HashTable`. It stores up to 256 occurrences of 4-bytes sequences in
/// the last 2^16 bytes, which makes it much more likely to find matches than FastCompressionHashTable.
pub struct HighCompressionHashTable {
    bytes: Arc<Mutex<Vec<u8>>>,
    base: i32,
    next: i32,
    end: i32,
    hash_table: Vec<i32>,
    chain_table: Vec<u16>,
    attempts: i32,
}

impl HighCompressionHashTable {
    const MAX_ATTEMPTS: i32 = 256;
    pub const MASK: i32 = LZ4::MAX_DISTANCE - 1;
    /// Sole constructor
    pub fn new() -> Self {
        HighCompressionHashTable {
            bytes: Arc::new(Mutex::new(vec![])),
            base: 0,
            next: 0,
            end: 0,
            hash_table: vec![-1; LZ4::HASH_TABLE_SIZE_HC as usize],
            chain_table: vec![0xFFFF; LZ4::MAX_DISTANCE as usize],
            attempts: 0,
        }
    }
    fn add_hash(&mut self, off: i32) -> Result<(), LuceneError> {
        let mut bytes_guard = self
            .bytes
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        let v = LZ4::read_int(bytes_guard.as_mut_slice(), off);
        let h = LZ4::hash_hc(v);
        let mut delta = off - self.hash_table[h as usize];
        if delta <= 0 || delta >= LZ4::MAX_DISTANCE {
            delta = LZ4::MAX_DISTANCE - 1;
        }
        self.chain_table[(off & Self::MASK) as usize] = delta as u16;
        self.hash_table[h as usize] = off;
        Ok(())
    }
}
impl HashTable for HighCompressionHashTable {
    fn reset(&mut self, bytes: Arc<Mutex<Vec<u8>>>, off: i32, len: i32) -> Result<(), LuceneError> {
        {
            let bytes_guard = bytes
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
            CommonUtil::check_from_index_size(off, len, bytes_guard.len() as i32)?;
        }

        if self.end - self.base < self.chain_table.len() as i32 {
            // The last call to compress was done on less than 64kB, let's not reset
            // the hashTable and only reset the relevant parts of the chainTable.
            // This helps avoid slowing down calling compress() many times on short
            // inputs.
            let start_offset = self.base & Self::MASK;
            let end_offset = if self.end == 0 {
                0
            } else {
                ((self.end - 1) & Self::MASK) + 1
            };

            if start_offset < end_offset {
                self.chain_table[start_offset as usize..end_offset as usize]
                    .iter_mut()
                    .for_each(|x| *x = 0xFFFF);
            } else {
                self.chain_table[0..end_offset as usize]
                    .iter_mut()
                    .for_each(|x| *x = 0xFFFF);
                self.chain_table[start_offset as usize..]
                    .iter_mut()
                    .for_each(|x| *x = 0xFFFF);
            }
        } else {
            // The last call to compress was done on a large enough amount of data
            // that it's fine to reset both tables
            self.hash_table.fill(-1);
            self.chain_table.fill(0xFFFF);
        }
        self.bytes = bytes;
        self.base = off;
        self.next = off;
        self.end = off + len;

        Ok(())
    }

    fn init_dictionary(&mut self, dict_len: i32) -> Result<(), LuceneError> {
        todo!()
    }

    fn get(&mut self, off: i32) -> Result<i32, LuceneError> {
        debug_assert!(off >= self.next);
        debug_assert!(off < self.end);

        while self.next < off {
            self.add_hash(self.next)?;
            self.next += 1;
        }

        let mut bytes_guard = self
            .bytes
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        let v = LZ4::read_int(bytes_guard.as_mut_slice(), off);
        let h = LZ4::hash_hc(v);

        self.attempts = 0;
        let mut ref_idx = self.hash_table[h as usize];
        if ref_idx >= off {
            // remainder from a previous call to compress()
            return Ok(-1);
        }
        let min = std::cmp::max(self.base, off - LZ4::MAX_DISTANCE + 1);
        while ref_idx >= min && self.attempts < Self::MAX_ATTEMPTS {
            ref_idx -= self.chain_table[(ref_idx & Self::MASK) as usize] as i32 & 0xFFFF;
            self.attempts += 1;
            if LZ4::read_int(bytes_guard.as_mut_slice(), ref_idx) == v {
                return Ok(ref_idx);
            }
        }
        Ok(-1)
    }

    fn previous(&mut self, off: i32) -> Result<i32, LuceneError> {
        let mut bytes_guard = self
            .bytes
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        let v = LZ4::read_int(bytes_guard.as_mut_slice(), off);
        let mut ref_idx = off - self.chain_table[(off & Self::MASK) as usize] as i32;
        while ref_idx >= self.base && self.attempts < Self::MAX_ATTEMPTS {
            ref_idx -= self.chain_table[(ref_idx & Self::MASK) as usize] as i32 & 0xFFFF;
            self.attempts += 1;
            if LZ4::read_int(bytes_guard.as_mut_slice(), ref_idx) == v {
                return Ok(ref_idx);
            }
        }
        Ok(-1)
    }

    fn assert_reset(&self) -> bool {
        for i in 0..self.chain_table.len() {
            debug_assert!(self.chain_table[i] == 0xFFFF);
        }
        true
    }
}

pub enum HashTableEnum {
    FastCompressionHashTable(FastCompressionHashTable),
    HighCompressionHashTable(HighCompressionHashTable),
}
impl HashTable for HashTableEnum {
    fn reset(&mut self, b: Arc<Mutex<Vec<u8>>>, off: i32, len: i32) -> Result<(), LuceneError> {
        match self {
            HashTableEnum::FastCompressionHashTable(table) => table.reset(b, off, len),
            HashTableEnum::HighCompressionHashTable(table) => table.reset(b, off, len),
        }
    }

    fn init_dictionary(&mut self, dict_len: i32) -> Result<(), LuceneError> {
        match self {
            HashTableEnum::FastCompressionHashTable(table) => table.init_dictionary(dict_len),
            HashTableEnum::HighCompressionHashTable(table) => table.init_dictionary(dict_len),
        }
    }

    fn get(&mut self, off: i32) -> Result<i32, LuceneError> {
        match self {
            HashTableEnum::FastCompressionHashTable(table) => table.get(off),
            HashTableEnum::HighCompressionHashTable(table) => table.get(off),
        }
    }

    fn previous(&mut self, off: i32) -> Result<i32, LuceneError> {
        match self {
            HashTableEnum::FastCompressionHashTable(table) => table.previous(off),
            HashTableEnum::HighCompressionHashTable(table) => table.previous(off),
        }
    }

    fn assert_reset(&self) -> bool {
        match self {
            HashTableEnum::FastCompressionHashTable(table) => table.assert_reset(),
            HashTableEnum::HighCompressionHashTable(table) => table.assert_reset(),
        }
    }
}
