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
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::CoreHelper;

/// LZ4 compression and decompression routines.
///
/// <https://github.com/lz4/lz4/tree/dev/lib> <http://fastcompression.blogspot.fr/p/lz4.html>
///
/// The high-compression option is a simpler version of the one of the original algorithm, and
/// only retains a better hash table that remembers about more occurrences of a previous 4-bytes
/// sequence, and removes all the logic about handling of the case when overlapping matches are
/// found.
#[allow(unused)]
pub struct LZ4;
#[allow(unused)]
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
        debug_assert!(i >= 0);
        // According to LZ4's algorithm the endianness does not matter at all:
        BitUtil::get_i32_le(buf, i as usize)
    }

    fn common_bytes(b: &[u8], o1: i32, o2: i32, limit: i32) -> i32 {
        debug_assert!(o1 < o2);
        // never -1 because lengths always differ
        CoreHelper::miss_match(
            &b[(o1 as usize)..(limit as usize)],
            &b[(o2 as usize)..(limit as usize)],
        )
    }

    /// Decompress at least `decompressed_len` bytes into `dest[d_off..]`.
    /// Please note that `dest` must be large enough to hold **all** decompressed data
    /// (meaning that you need to know the total decompressed length). If the given bytes were
    /// compressed using a preset dictionary, the same dictionary must be provided in
    /// `dest[d_off-dict_len..d_off]`.
    pub fn decompress(
        compressed: &mut impl DataInput,
        decompressed_len: i32,
        dest: &mut [u8],
        d_off: i32,
    ) -> Result<i32> {
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
            let match_dec = compressed.read_short()? as u16 as i32;

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
                let mut start = (d_off - match_dec) as usize;
                let end = d_off + match_len;

                while d_off < end {
                    dest[d_off as usize] = dest[start];
                    start += 1;
                    d_off += 1;
                }
            } else {
                let start = (d_off - match_dec) as usize;
                let end = d_off as usize;
                let fast_len = fast_len as usize;
                dest.copy_within(start..start + fast_len, end);
                d_off += match_len;
            }

            if d_off >= dest_end {
                break;
            }
        }
        Ok(d_off)
    }
    fn encode_len(mut l: i32, out: &mut impl DataOutput) -> Result<()> {
        while l >= 0xFF {
            out.write_byte(0xFF)?;
            l -= 0xFF;
        }
        out.write_byte(l as u8)?;
        Ok(())
    }
    fn encode_literals(
        bytes: &[u8],
        token: i32,
        anchor: i32,
        literal_len: i32,
        out: &mut impl DataOutput,
    ) -> Result<()> {
        out.write_byte(token as u8)?;

        // encode literal length
        if literal_len >= 0x0F {
            Self::encode_len(literal_len - 0x0F, out)?;
        }

        // encode literals
        out.write_bytes_range(bytes, anchor, literal_len)?;

        Ok(())
    }
    fn encode_last_literals(
        bytes: &[u8],
        anchor: i32,
        literal_len: i32,
        out: &mut impl DataOutput,
    ) -> Result<()> {
        let token = std::cmp::min(literal_len, 0x0F) << 4;
        Self::encode_literals(bytes, token, anchor, literal_len, out)
    }

    fn encode_sequence(
        bytes: &[u8],
        anchor: i32,
        match_ref: i32,
        match_off: i32,
        match_len: i32,
        out: &mut impl DataOutput,
    ) -> Result<()> {
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
    pub fn compress(
        bytes: Vec<u8>,
        off: i32,
        len: i32,
        out: &mut impl DataOutput,
        ht: &mut HashTableEnum,
    ) -> Result<()> {
        Self::compress_with_dictionary(bytes, off, 0, len, out, ht)
    }
    /// Compress `[dictOff+dictLen:dictOff+dictLen+len]` into `out` using at most 16kB
    /// of memory. `[dictOff:dictOff+dictLen]` will be used as a dictionary. `dictLen`
    /// must not be greater than `MAX_DISTANCE 64kB`, the maximum window size.
    /// `ht` shouldn't be shared across threads but can safely be reused.
    pub fn compress_with_dictionary(
        bytes: Vec<u8>,
        dict_off: i32,
        dict_len: i32,
        len: i32,
        out: &mut impl DataOutput,
        ht: &mut HashTableEnum,
    ) -> Result<()> {
        // Ensure the indices are valid
        CoreHelper::check_from_index_size(dict_off, dict_len, bytes.len() as i32)?;
        CoreHelper::check_from_index_size(dict_off + dict_len, len, bytes.len() as i32)?;

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
            ht.init_dictionary(dict_len);

            'outer: while off <= limit {
                // find a match
                let mut ref_idx;
                while {
                    if off >= match_limit {
                        break 'outer;
                    }
                    ref_idx = ht.get(off);
                    ref_idx == -1
                } {
                    off += 1;
                }

                // Compute match length
                let mut match_len = LZ4::MIN_MATCH
                    + LZ4::common_bytes(
                        bytes.as_slice(),
                        ref_idx + LZ4::MIN_MATCH,
                        off + LZ4::MIN_MATCH,
                        limit,
                    );

                // Try to find a better match
                let min = (off - LZ4::MAX_DISTANCE + 1).max(dict_off);
                let mut r = ht.previous(ref_idx);
                while r >= min {
                    assert_eq!(
                        LZ4::read_int(bytes.as_slice(), r),
                        LZ4::read_int(bytes.as_slice(), off)
                    );
                    let r_match_len = LZ4::MIN_MATCH
                        + LZ4::common_bytes(
                            bytes.as_slice(),
                            r + LZ4::MIN_MATCH,
                            off + LZ4::MIN_MATCH,
                            limit,
                        );
                    if r_match_len > match_len {
                        ref_idx = r;
                        match_len = r_match_len;
                    }

                    r = ht.previous(r);
                }

                // Encode match
                LZ4::encode_sequence(bytes.as_slice(), anchor, ref_idx, off, match_len, out)?;
                off += match_len;
                anchor = off;
            }
        }

        // Handle last literals
        let literal_len = end - anchor;
        assert!(literal_len >= LZ4::LAST_LITERALS || literal_len == len);
        LZ4::encode_last_literals(bytes.as_slice(), anchor, literal_len, out)?;

        Ok(())
    }
}

/// A record of previous occurrences of sequences of 4 bytes.
pub trait HashTable {
    /// Reset this hash table in order to compress the given content.
    fn reset(&mut self, b: Vec<u8>, off: i32, len: i32) -> Result<()>;

    /// Init `dict_len` bytes to be used as a dictionary.
    fn init_dictionary(&mut self, dict_len: i32);

    /// Advance the cursor to `off` and return an index that stored the same 4 bytes as `b[off:off+4]`.
    /// This may only be called on strictly increasing sequences of offsets.
    /// A return value of `-1` indicates that no other index could be found.
    fn get(&mut self, off: i32) -> i32;

    /// Return an index that is less than `off` and stores the same 4 bytes.
    /// Unlike `get`, it doesn't need to be called on increasing offsets.
    /// A return value of `-1` indicates that no other index could be found.
    fn previous(&mut self, off: i32) -> i32;

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
pub struct FastCompressionHashTable {
    bytes: Vec<u8>,
    base: i32,
    last_off: i32,
    end: i32,
    hash_log: i32,
    hash_table: Option<TableEnum>,
}

impl Default for FastCompressionHashTable {
    fn default() -> Self {
        Self::new()
    }
}

impl FastCompressionHashTable {
    /// Sole constructor
    pub fn new() -> Self {
        FastCompressionHashTable {
            bytes: vec![],
            base: 0,
            last_off: 0,
            end: 0,
            hash_log: 0,
            hash_table: None,
        }
    }
}
impl HashTable for FastCompressionHashTable {
    fn reset(&mut self, bytes: Vec<u8>, off: i32, len: i32) -> Result<()> {
        CoreHelper::check_from_index_size(off, len, bytes.len() as i32)?;
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

    fn init_dictionary(&mut self, dict_len: i32) {
        for i in 0..dict_len {
            let v = LZ4::read_int(self.bytes.as_slice(), self.base + i);
            let h = LZ4::hash(v, self.hash_log);
            debug_assert!(self.hash_table.is_some());
            if let Some(table) = &mut self.hash_table {
                table.set(h, i);
            }
        }
        self.last_off += dict_len;
    }

    fn get(&mut self, off: i32) -> i32 {
        debug_assert!(off > self.last_off);
        debug_assert!(off < self.end);
        let v = LZ4::read_int(self.bytes.as_slice(), off);
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
            && LZ4::read_int(self.bytes.as_slice(), ref_idx) == v
        {
            ref_idx
        } else {
            -1
        }
    }

    fn previous(&mut self, _off: i32) -> i32 {
        -1
    }

    fn assert_reset(&self) -> bool {
        true
    }
}
/// A higher-precision `HashTable`. It stores up to 256 occurrences of 4-bytes sequences in
/// the last 2^16 bytes, which makes it much more likely to find matches than FastCompressionHashTable.
pub struct HighCompressionHashTable {
    bytes: Vec<u8>,
    base: i32,
    next: i32,
    end: i32,
    hash_table: Vec<i32>,
    chain_table: Vec<u16>,
    attempts: i32,
}

impl Default for HighCompressionHashTable {
    fn default() -> Self {
        Self::new()
    }
}

impl HighCompressionHashTable {
    const MAX_ATTEMPTS: i32 = 256;
    pub const MASK: i32 = LZ4::MAX_DISTANCE - 1;
    /// Sole constructor
    pub fn new() -> Self {
        HighCompressionHashTable {
            bytes: vec![],
            base: 0,
            next: 0,
            end: 0,
            hash_table: vec![-1; LZ4::HASH_TABLE_SIZE_HC as usize],
            chain_table: vec![0xFFFF; LZ4::MAX_DISTANCE as usize],
            attempts: 0,
        }
    }
    fn add_hash(&mut self, off: i32) {
        let v = LZ4::read_int(self.bytes.as_slice(), off);
        let h = LZ4::hash_hc(v);
        let mut delta = off - self.hash_table[h as usize];
        if delta <= 0 || delta >= LZ4::MAX_DISTANCE {
            delta = LZ4::MAX_DISTANCE - 1;
        }
        self.chain_table[(off & Self::MASK) as usize] = delta as u16;
        self.hash_table[h as usize] = off;
    }
}
impl HashTable for HighCompressionHashTable {
    fn reset(&mut self, bytes: Vec<u8>, off: i32, len: i32) -> Result<()> {
        CoreHelper::check_from_index_size(off, len, bytes.len() as i32)?;

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

    fn init_dictionary(&mut self, dict_len: i32) {
        debug_assert!(self.next == self.base);
        for i in 0..dict_len {
            self.add_hash(self.base + i);
        }
        self.next += dict_len;
    }

    fn get(&mut self, off: i32) -> i32 {
        debug_assert!(off >= self.next);
        debug_assert!(off < self.end);

        while self.next < off {
            self.add_hash(self.next);
            self.next += 1;
        }
        let v = LZ4::read_int(self.bytes.as_slice(), off);
        let h = LZ4::hash_hc(v);

        self.attempts = 0;
        let mut ref_idx = self.hash_table[h as usize];
        if ref_idx >= off {
            // remainder from a previous call to compress()
            return -1;
        }
        let min = std::cmp::max(self.base, off - LZ4::MAX_DISTANCE + 1);
        while ref_idx >= min && self.attempts < Self::MAX_ATTEMPTS {
            if LZ4::read_int(self.bytes.as_slice(), ref_idx) == v {
                return ref_idx;
            }
            ref_idx -= self.chain_table[(ref_idx & Self::MASK) as usize] as i32;
            self.attempts += 1;
        }
        -1
    }

    fn previous(&mut self, off: i32) -> i32 {
        let v = LZ4::read_int(self.bytes.as_slice(), off);
        let mut ref_idx = off - ((self.chain_table[(off & Self::MASK) as usize] as i32) & 0xFFFF);
        while ref_idx >= self.base && self.attempts < Self::MAX_ATTEMPTS {
            if LZ4::read_int(self.bytes.as_slice(), ref_idx) == v {
                return ref_idx;
            }
            ref_idx -= self.chain_table[(ref_idx & Self::MASK) as usize] as i32 & 0xFFFF;
            self.attempts += 1;
        }
        -1
    }

    fn assert_reset(&self) -> bool {
        for i in 0..self.chain_table.len() {
            debug_assert!(self.chain_table[i] == 0xFFFF);
        }
        true
    }
}

pub enum HashTableEnum {
    Fast(FastCompressionHashTable),
    High(HighCompressionHashTable),
}
impl HashTable for HashTableEnum {
    fn reset(&mut self, b: Vec<u8>, off: i32, len: i32) -> Result<()> {
        match self {
            HashTableEnum::Fast(table) => table.reset(b, off, len),
            HashTableEnum::High(table) => table.reset(b, off, len),
        }
    }

    fn init_dictionary(&mut self, dict_len: i32) {
        match self {
            HashTableEnum::Fast(table) => table.init_dictionary(dict_len),
            HashTableEnum::High(table) => table.init_dictionary(dict_len),
        }
    }

    fn get(&mut self, off: i32) -> i32 {
        match self {
            HashTableEnum::Fast(table) => table.get(off),
            HashTableEnum::High(table) => table.get(off),
        }
    }

    fn previous(&mut self, off: i32) -> i32 {
        match self {
            HashTableEnum::Fast(table) => table.previous(off),
            HashTableEnum::High(table) => table.previous(off),
        }
    }

    fn assert_reset(&self) -> bool {
        match self {
            HashTableEnum::Fast(table) => table.assert_reset(),
            HashTableEnum::High(table) => table.assert_reset(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::store::{ByteArrayDataInput, ByteBuffersDataOutput, DataOutput};
    use crate::test::util::lucene_test_case::random;
    use crate::test::util::test_util::TestUtil;
    use crate::util::array_util::ArrayUtil;
    use crate::util::compress::lz4::{FastCompressionHashTable, HighCompressionHashTable, LZ4};
    use crate::util::compress::lz4::{HashTable, HashTableEnum};
    use crate::util::error::lucene_error::Result;
    use crate::util::SliceCopyOps;
    use rand::rngs::StdRng;
    use rand::{Rng, RngCore};

    struct TestFastLZ4;
    impl LZ4TestCase for TestFastLZ4 {
        fn new_hash_table(&self) -> AssertingHashTable {
            AssertingHashTable::new(HashTableEnum::Fast(FastCompressionHashTable::new()))
        }
    }
    #[test]
    fn test_empty_fast() -> Result<()> {
        let mut random = random();
        let case = TestFastLZ4;
        case.test_empty(&mut random)
    }
    #[test]
    fn test_short_literals_and_matches_fast() -> Result<()> {
        let mut random = random();
        let case = TestFastLZ4;
        case.test_short_literals_and_matches(&mut random)
    }
    #[test]
    fn test_long_matches_fast() -> Result<()> {
        let mut random = random();
        let case = TestFastLZ4;
        case.test_long_matches(&mut random)
    }
    #[test]
    fn test_long_literals_fast() -> Result<()> {
        let mut random = random();
        let case = TestFastLZ4;
        case.test_long_literals(&mut random)
    }
    #[test]
    fn test_match_right_before_last_literals_fast() -> Result<()> {
        let mut random = random();
        let case = TestFastLZ4;
        case.test_match_right_before_last_literals(&mut random)
    }
    #[test]
    fn test_incompressible_random_fast() -> Result<()> {
        let mut random = random();
        let case = TestFastLZ4;
        case.test_incompressible_random(&mut random)
    }
    #[test]
    fn test_compressible_random_fast() -> Result<()> {
        let mut random = random();
        let case = TestFastLZ4;
        case.test_compressible_random(&mut random)
    }
    #[test]
    fn test_lucene5201_fast() -> Result<()> {
        let mut random = random();
        let case = TestFastLZ4;
        case.test_lucene5201(&mut random)
    }
    #[test]
    fn test_use_dictionary_fast() -> Result<()> {
        let mut random = random();
        let case = TestFastLZ4;
        case.test_use_dictionary(&mut random)
    }

    #[allow(dead_code)] // for quick search
    struct TestHighLZ4;
    impl LZ4TestCase for TestHighLZ4 {
        fn new_hash_table(&self) -> AssertingHashTable {
            AssertingHashTable::new(HashTableEnum::High(HighCompressionHashTable::new()))
        }
    }
    #[test]
    fn test_empty_high() -> Result<()> {
        let mut random = random();
        let case = TestHighLZ4;
        case.test_empty(&mut random)
    }
    #[test]
    fn test_short_literals_and_matches_high() -> Result<()> {
        let mut random = random();
        let case = TestHighLZ4;
        case.test_short_literals_and_matches(&mut random)
    }
    #[test]
    fn test_long_matches_high() -> Result<()> {
        let mut random = random();
        let case = TestHighLZ4;
        case.test_long_matches(&mut random)
    }
    #[test]
    fn test_long_literals_high() -> Result<()> {
        let mut random = random();
        let case = TestHighLZ4;
        case.test_long_literals(&mut random)
    }
    #[test]
    fn test_match_right_before_last_literals_high() -> Result<()> {
        let mut random = random();
        let case = TestHighLZ4;
        case.test_match_right_before_last_literals(&mut random)
    }
    #[test]
    fn test_incompressible_random_high() -> Result<()> {
        let mut random = random();
        let case = TestHighLZ4;
        case.test_incompressible_random(&mut random)
    }
    #[test]
    fn test_compressible_random_high() -> Result<()> {
        let mut random = random();
        let case = TestHighLZ4;
        case.test_compressible_random(&mut random)
    }
    #[test]
    fn test_lucene5201_high() -> Result<()> {
        let mut random = random();
        let case = TestHighLZ4;
        case.test_lucene5201(&mut random)
    }
    #[test]
    fn test_use_dictionary_high() -> Result<()> {
        let mut random = random();
        let case = TestHighLZ4;
        case.test_use_dictionary(&mut random)
    }

    trait LZ4TestCase {
        fn new_hash_table(&self) -> AssertingHashTable;

        fn do_test(
            random: &mut StdRng,
            data: Vec<u8>,
            hash_table: &mut AssertingHashTable,
        ) -> Result<()> {
            // this triggers special reset logic for high compression
            let offset = if data.len() >= (1 << 16) || random.random_bool(0.5) {
                random.random_range(0..10)
            } else {
                (1 << 16) - data.len() as i32 / 2
            };

            let mut copy = vec![0; data.len() + offset as usize + random.random_range(0..10)];
            copy.copy_from(&data, offset as usize);
            Self::do_test_with_offset(random, copy, offset, data.len() as i32, hash_table)
        }

        fn do_test_with_offset(
            random: &mut StdRng,
            data: Vec<u8>,
            offset: i32,
            length: i32,
            hash_table: &mut AssertingHashTable,
        ) -> Result<()> {
            let mut out = ByteBuffersDataOutput::new();
            LZ4::compress(data.clone(), offset, length, &mut out, &mut hash_table.ht)?;

            let compressed = out.try_get_array_ownership();
            let mut off = 0;
            let mut decompressed_off = 0;

            loop {
                let token = compressed[off];
                off += 1;
                let mut literal_len = (token >> 4) as i32;

                if literal_len == 0x0F {
                    while compressed[off] == 0xFF {
                        literal_len += 0xFF;
                        off += 1;
                    }
                    literal_len += compressed[off] as i32;
                    off += 1;
                }
                // skip literals
                off += literal_len as usize;
                decompressed_off += literal_len;
                // check that the stream ends with literals and that there are at least
                // 5 of them
                if off == compressed.len() {
                    assert_eq!(length, decompressed_off);
                    assert!(literal_len >= LZ4::LAST_LITERALS || literal_len == length);
                    break;
                }

                let match_dec = (compressed[off] as i32) | ((compressed[off + 1] as i32) << 8);
                off += 2;

                assert!(match_dec > 0 && match_dec <= decompressed_off);

                let mut match_len = token as i32 & 0x0F;
                if match_len == 0x0F {
                    while compressed[off] == 0xFF {
                        match_len += 0xFF;
                        off += 1;
                    }
                    match_len += compressed[off] as i32;
                    off += 1;
                }
                match_len += LZ4::MIN_MATCH;
                {
                    // if the match ends prematurely, the next sequence should not have
                    // literals or this means we are wasting space
                    if decompressed_off + match_len < length - LZ4::LAST_LITERALS {
                        let more_common_bytes = data
                            [offset as usize + decompressed_off as usize + match_len as usize]
                            == data[offset as usize + decompressed_off as usize
                                - match_dec as usize
                                + match_len as usize];
                        let next_sequence_has_literals = compressed[off] >> 4 != 0;
                        assert!(!(more_common_bytes && next_sequence_has_literals));
                    }
                }

                decompressed_off += match_len;
            }

            assert_eq!(length, decompressed_off);

            // Compress once again with the same hash table to test reuse
            let mut out2 = ByteBuffersDataOutput::new();
            LZ4::compress(data.clone(), offset, length, &mut out2, &mut hash_table.ht)?;
            assert_eq!(compressed, out2.try_get_array_ownership());

            let compressed_clone = compressed.clone();
            // Now restore and compare bytes
            let mut restored = vec![0; length as usize + random.random_range(0..10)];
            let mut input = ByteArrayDataInput::with_bytes(compressed);
            LZ4::decompress(&mut input, length, &mut restored, 0)?;

            assert!(off <= i32::MAX as usize);
            let left = ArrayUtil::copy_of_sub_array(data.as_slice(), offset, offset + length);
            let right = ArrayUtil::copy_of_sub_array(&restored, 0, length);
            assert_eq!(left, right);

            // Now restore with an offset
            let restore_offset: i32 = random.random_range(1..10);
            restored =
                vec![0; restore_offset as usize + length as usize + random.random_range(0..10)];
            let mut input = ByteArrayDataInput::with_bytes(compressed_clone);
            LZ4::decompress(&mut input, length, &mut restored, restore_offset)?;

            let left = ArrayUtil::copy_of_sub_array(data.as_slice(), offset, offset + length);
            let right =
                ArrayUtil::copy_of_sub_array(&restored, restore_offset, restore_offset + length);
            assert_eq!(left, right);

            Ok(())
        }

        fn do_test_with_dictionary(
            random: &mut StdRng,
            data: Vec<u8>,
            hash_table: &mut AssertingHashTable,
        ) -> Result<()> {
            let mut copy = ByteBuffersDataOutput::new();
            let dict_off = random.random_range(0..10);
            copy.write_bytes(vec![0u8; dict_off as usize])?;

            // Create a dictionary from substrings of the input to compress
            let mut dict_len = 0;
            let mut i = TestUtil::next_int(random, 0, data.len() as i32);
            while i < data.len() as i32 && dict_len < LZ4::MAX_DISTANCE {
                let l = std::cmp::min(
                    data.len() - i as usize,
                    TestUtil::next_int(random, 1, 32) as usize,
                );
                let l = std::cmp::min(l, (LZ4::MAX_DISTANCE - dict_len) as usize);
                debug_assert!(l <= i32::MAX as usize);
                copy.write_bytes_range(&data, i, l as i32)?;
                dict_len += l as i32;
                i += l as i32;
                i += TestUtil::next_int(random, 1, 32);
            }

            let data_length = data.len();
            assert!(data_length <= i32::MAX as usize);
            copy.write_bytes(data)?;
            copy.write_bytes(vec![0u8; random.random_range(0..10)])?;

            let copy_bytes = copy.try_get_array_ownership();
            Self::do_test_with_dictionary_inner(
                random,
                copy_bytes,
                dict_off,
                dict_len,
                data_length as i32,
                hash_table,
            )
        }

        fn do_test_with_dictionary_inner(
            random: &mut StdRng,
            data: Vec<u8>,
            dict_off: i32,
            dict_len: i32,
            length: i32,
            hash_table: &mut AssertingHashTable,
        ) -> Result<()> {
            let mut out = ByteBuffersDataOutput::new();
            LZ4::compress_with_dictionary(
                data.clone(),
                dict_off,
                dict_len,
                length,
                &mut out,
                &mut hash_table.ht,
            )?;
            let compressed = out.try_get_array_ownership();

            // Compress once again with the same hash table to test reuse
            let mut out2 = ByteBuffersDataOutput::new();
            LZ4::compress_with_dictionary(
                data.clone(),
                dict_off,
                dict_len,
                length,
                &mut out2,
                &mut hash_table.ht,
            )?;
            assert_eq!(compressed, out2.try_get_array_ownership());

            // Now restore and compare bytes
            let restore_offset = TestUtil::next_int(random, 1, 10);
            let mut restored =
                vec![0; (restore_offset + dict_len + length + random.random_range(0..10)) as usize];
            restored.copy_from(
                &data[dict_off as usize..(dict_off + dict_len) as usize],
                restore_offset as usize,
            );

            let mut input = ByteArrayDataInput::with_bytes(compressed);
            LZ4::decompress(&mut input, length, &mut restored, dict_len + restore_offset)?;

            let left = ArrayUtil::copy_of_sub_array(
                data.as_slice(),
                dict_off + dict_len,
                dict_off + dict_len + length,
            );
            let right = ArrayUtil::copy_of_sub_array(
                &restored,
                dict_len + restore_offset,
                dict_len + restore_offset + length,
            );
            assert_eq!(left, right);

            Ok(())
        }
        fn test_empty(&self, random: &mut StdRng) -> Result<()> {
            // literals and match lengths <= 15
            let data: Vec<u8> = "".to_string().into_bytes();
            Self::do_test(random, data, &mut self.new_hash_table())
        }

        fn test_short_literals_and_matches(&self, random: &mut StdRng) -> Result<()> {
            // literals and match lengths <= 15
            let data: Vec<u8> = "1234562345673456745678910123".to_string().into_bytes();
            Self::do_test(random, data.clone(), &mut self.new_hash_table())?;
            Self::do_test_with_dictionary(random, data.clone(), &mut self.new_hash_table())?;
            Ok(())
        }

        fn test_long_matches(&self, random: &mut StdRng) -> Result<()> {
            // match length >= 20
            let len = random.random_range(300..1024);
            let mut data = vec![0u8; len];
            for (index, element) in data.iter_mut().enumerate() {
                *element = index as u8;
            }
            Self::do_test(random, data, &mut self.new_hash_table())?;
            Ok(())
        }
        fn test_long_literals(&self, random: &mut StdRng) -> Result<()> {
            // long literals (length >= 16) which are not the last literals
            let len = random.random_range(400..1024);
            let mut data = vec![0u8; len];
            random.fill_bytes(&mut data);
            let match_ref = random.random_range(0..30);
            let match_off = random.random_range(len - 40..len - 20);
            let match_length = random.random_range(4..10);
            data.copy_within(match_ref..match_ref + match_length, match_off);
            Self::do_test(random, data, &mut self.new_hash_table())?;
            Ok(())
        }

        fn test_match_right_before_last_literals(&self, random: &mut StdRng) -> Result<()> {
            let data = vec![1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 5];
            Self::do_test(random, data, &mut self.new_hash_table())?;
            Ok(())
        }

        fn test_incompressible_random(&self, random: &mut StdRng) -> Result<()> {
            let len = random.random_range(1..1 << 18);
            let mut b = vec![0u8; len];
            random.fill_bytes(&mut b);
            let b_clone = b.clone();
            Self::do_test(random, b, &mut self.new_hash_table())?;
            Self::do_test_with_dictionary(random, b_clone, &mut self.new_hash_table())?;
            Ok(())
        }

        fn test_compressible_random(&self, random: &mut StdRng) -> Result<()> {
            let len = random.random_range(1..1 << 18);
            let mut b = vec![0u8; len];
            let base = random.random_range(0..256);
            let max_delta = 1 + random.random_range(0..8);
            for elem in b.iter_mut() {
                *elem = (base + random.random_range(0..max_delta)) as u8;
            }
            let b_clone = b.clone();
            Self::do_test(random, b, &mut self.new_hash_table())?;
            Self::do_test_with_dictionary(random, b_clone, &mut self.new_hash_table())?;
            Ok(())
        }
        fn test_lucene5201(&self, random: &mut StdRng) -> Result<()> {
            let data: Vec<i8> = vec![
                14, 72, 14, 85, 3, 72, 14, 85, 3, 72, 14, 72, 14, 72, 14, 85, 3, 72, 14, 72, 14,
                72, 14, 72, 14, 72, 14, 72, 14, 85, 3, 72, 14, 85, 3, 72, 14, 85, 3, 72, 14, 85, 3,
                72, 14, 85, 3, 72, 14, 85, 3, 72, 14, 50, 64, 0, 46, -1, 0, 0, 0, 29, 3, 85, 8,
                -113, 0, 68, -97, 3, 0, 2, 3, -97, 6, 0, 68, -113, 0, 2, 3, -97, 6, 0, 68, -113, 0,
                2, 3, -97, 6, 0, 68, -113, 0, 2, 3, -97, 6, 0, 68, -113, 0, 2, 3, -97, 6, 0, 68,
                -113, 0, 50, 64, 0, 47, -105, 0, 0, 0, 30, 3, -97, 6, 0, 68, -113, 0, 2, 3, -97, 6,
                0, 68, -113, 0, 2, 3, 85, 8, -113, 0, 68, -97, 3, 0, 2, 3, 85, 8, -113, 0, 68,
                -113, 0, 2, 3, -97, 6, 0, 68, -113, 0, 2, 3, -97, 6, 0, 68, -113, 0, 2, 3, -97, 6,
                0, 68, -113, 0, 2, 3, 85, 8, -113, 0, 68, -97, 3, 0, 2, 3, -97, 6, 0, 68, -113, 0,
                2, 3, -97, 6, 0, 68, -113, 0, 2, 3, -97, 6, 0, 68, -113, 0, 2, 3, -97, 6, 0, 68,
                -113, 0, 2, 3, -97, 6, 0, 68, -113, 0, 2, 3, -97, 6, 0, 68, -113, 0, 2, 3, -97, 6,
                0, 68, -113, 0, 2, 3, 85, 8, -113, 0, 68, -113, 0, 2, 3, -97, 6, 0, 68, -113, 0, 2,
                3, 85, 8, -113, 0, 68, -97, 3, 0, 2, 3, -97, 6, 0, 68, -113, 0, 2, 3, -97, 6, 0,
                68, -113, 0, 2, 3, -97, 6, 0, 68, -113, 0, 2, 3, -97, 6, 0, 50, 64, 0, 50, 53, 0,
                0, 0, 34, 3, 85, 8, -113, 0, 68, -97, 3, 0, 2, 3, -97, 6, 0, 68, -113, 0, 2, 3,
                -97, 6, 0, 68, -113, 0, 2, 3, -97, 6, 0, 68, -113, 0, 2, 3, -97, 6, 0, 68, -113, 0,
                2, 3, -97, 6, 0, 68, -113, 0, 2, 3, 85, 8, -113, 0, 68, -113, 0, 2, 3, -97, 6, 0,
                68, -113, 0, 2, 3, 85, 8, -113, 0, 68, -97, 3, 0, 2, 3, 85, 8, -113, 0, 68, -97, 3,
                0, 120, 64, 0, 52, -88, 0, 0, 0, 39, 13, 85, 5, 72, 13, 85, 5, 72, 13, 85, 5, 72,
                13, 72, 13, 85, 5, 72, 13, 85, 5, 72, 13, 85, 5, 72, 13, 85, 5, 72, 13, 85, 5, 72,
                13, 72, 13, 85, 5, 72, 13, 85, 5, 72, 13, 72, 13, 72, 13, 85, 5, 72, 13, 85, 5, 72,
                13, 85, 5, 72, 13, 85, 5, 72, 13, 85, 5, 72, 13, 85, 5, 72, 13, 85, 5, 72, 13, 85,
                5, 72, 13, 85, 5, 72, 13, 85, 5, 72, 13, 85, 5, 72, 13, 85, 5, 72, 13, 85, 5, 72,
                13, 85, 5, 72, 13, 72, 13, 72, 13, 72, 13, 85, 5, 72, 13, 85, 5, 72, 13, 72, 13,
                85, 5, 72, 13, 85, 5, 72, 13, -19, -24, -101, -35,
            ];
            let len = data.len() as i32;
            let data_u8: Vec<u8> = data.iter().map(|&x| x as u8).collect();
            Self::do_test_with_offset(random, data_u8, 9, len - 9, &mut self.new_hash_table())
        }

        fn test_use_dictionary(&self, random: &mut StdRng) -> Result<()> {
            let b: Vec<i8> = vec![1, 2, 3, 4, 5, 6, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
            let dict_off = 0;
            let dict_len = 6;
            let len = (b.len() - dict_len) as i32;
            let byte: Vec<u8> = b.iter().map(|&x| x as u8).collect();

            Self::do_test_with_dictionary_inner(
                random,
                byte.clone(),
                dict_off,
                dict_len as i32,
                len,
                &mut self.new_hash_table(),
            )?;
            let mut out = ByteBuffersDataOutput::new();
            LZ4::compress_with_dictionary(
                byte.clone(),
                dict_off,
                dict_len as i32,
                len,
                &mut out,
                &mut self.new_hash_table().ht,
            )?;

            // The compressed output is smaller than the original input despite being incompressible on its
            // own
            assert!(out.size() < len as i64);
            Ok(())
        }
    }

    struct AssertingHashTable {
        ht: HashTableEnum,
    }
    impl AssertingHashTable {
        fn new(ht: HashTableEnum) -> Self {
            AssertingHashTable { ht }
        }
    }
    impl HashTable for AssertingHashTable {
        fn reset(&mut self, b: Vec<u8>, off: i32, len: i32) -> Result<()> {
            self.ht.reset(b, off, len)?;
            assert!(self.ht.assert_reset());
            Ok(())
        }

        fn init_dictionary(&mut self, dict_len: i32) {
            assert!(self.ht.assert_reset());
            self.ht.init_dictionary(dict_len)
        }

        fn get(&mut self, off: i32) -> i32 {
            self.ht.get(off)
        }

        fn previous(&mut self, off: i32) -> i32 {
            self.ht.previous(off)
        }

        fn assert_reset(&self) -> bool {
            unreachable!()
        }
    }
}
