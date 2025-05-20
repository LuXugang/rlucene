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
use std::rc::Rc;

use crate::codecs::block_term_state::BlockTermStateEnum;
use crate::codecs::lucene90::block_tree::compression_algorithm::CompressionAlgorithm;
use crate::codecs::lucene90::block_tree::segment_terms_enum::{OutputAccumulator, SegmentTerms};
use crate::codecs::postings_reader_base::PostingsReaderBase;
use crate::index::index_options::IndexOptions;
use crate::index::terms_enum::SeekStatus;
use crate::index::BytesRef;
use crate::store::{ByteArrayDataInput, DataInput, IndexInput};
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::fst::Arc;
use crate::util::{SliceCopyOps, ToInt};

pub struct SegmentTermsEnumFrame<'a, I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    /// Our index in stack[]
    pub(crate) ord: i32,

    pub(crate) has_terms: bool,
    pub(crate) has_terms_orig: bool,
    pub(crate) is_floor: bool,

    pub(crate) arc: Option<Rc<RefCell<Arc<BytesRef<Rc<Vec<u8>>>>>>>,

    /// File pointer where this block was loaded from
    pub(crate) fp: i64,
    pub(crate) fp_orig: i64,
    pub(crate) fp_end: i64,
    pub(crate) total_suffix_bytes: i64, // for stats
    pub(crate) suffix_bytes: Vec<u8>,
    pub(crate) suffixes_reader: ByteArrayDataInput<Vec<u8>>,

    pub(crate) suffix_length_bytes: Vec<u8>,
    pub(crate) suffix_lengths_reader: ByteArrayDataInput<Vec<u8>>,
    pub(crate) stat_bytes: Vec<u8>,
    pub(crate) stats_singleton_run_length: i32,
    pub(crate) stats_reader: ByteArrayDataInput<Vec<u8>>,

    pub(crate) rewind_pos: i32,
    pub(crate) floor_data_reader: ByteArrayDataInput<Rc<Vec<u8>>>,

    // Length of prefix shared by all terms in this block
    pub(crate) prefix_length: i32,

    // Number of entries (term or sub-block) in this block
    pub(crate) ent_count: i32,

    // Which term we will next read, or -1 if the block isn't loaded yet
    pub(crate) next_ent: i32,

    // True if this block is either not a floor block, or it's the last sub-block of a floor block
    pub(crate) is_last_in_floor: bool,

    // True if all entries are terms
    pub(crate) is_leaf_block: bool,

    // True if all entries have the same length.
    pub(crate) all_equal: bool,

    pub(crate) last_sub_fp: i64,

    pub(crate) next_floor_label: i32,
    pub(crate) num_follow_floor_blocks: i32,

    // Next term to decode metaData; we decode metaData
    // lazily so that scanning to find the matching term is
    // fast and only if you find a match and app wants the
    // stats or docs/positions enums, will we decode the
    // metaData
    pub(crate) meta_data_upto: i32,

    pub(crate) state: BlockTermStateEnum,

    // metadata buffer
    pub(crate) bytes: Vec<u8>,
    pub(crate) bytes_reader: ByteArrayDataInput<Vec<u8>>,

    /// parent SegmentTerms
    ste: Rc<RefCell<SegmentTerms<'a, I, P>>>,

    start_byte_pos: i32,
    suffix_length: i32,
    sub_code: i64,
    compression_alg: CompressionAlgorithm,
}
impl<'a, I, P> SegmentTermsEnumFrame<'a, I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    pub fn new(ste: Rc<RefCell<SegmentTerms<'a, I, P>>>, ord: i32) -> Result<Self> {
        let mut state = ste
            .borrow()
            .fr
            .parent
            .borrow()
            .postings_reader
            .new_term_state()?;

        state.get_block_term_state().total_term_freq = -1;

        Ok(Self {
            ord,
            ste,
            state,

            arc: None,
            has_terms: false,
            has_terms_orig: false,
            is_floor: false,

            fp: 0,
            fp_orig: 0,
            fp_end: 0,
            total_suffix_bytes: 0,

            suffix_bytes: vec![0u8; 128],
            suffixes_reader: ByteArrayDataInput::new(),

            suffix_length_bytes: vec![0u8; 32],
            suffix_lengths_reader: ByteArrayDataInput::new(),

            stat_bytes: vec![0u8; 64],
            stats_singleton_run_length: 0,
            stats_reader: ByteArrayDataInput::new(),

            rewind_pos: 0,
            floor_data_reader: ByteArrayDataInput::new(),

            prefix_length: 0,
            ent_count: 0,
            next_ent: 0,

            is_last_in_floor: false,
            is_leaf_block: false,
            all_equal: false,

            last_sub_fp: 0,
            next_floor_label: 0,
            num_follow_floor_blocks: 0,

            meta_data_upto: 0,

            bytes: vec![0u8; 32],
            bytes_reader: ByteArrayDataInput::new(),
            start_byte_pos: 0,
            suffix_length: 0,
            sub_code: 0,
            compression_alg: CompressionAlgorithm::NoCompression,
        })
    }
    pub(crate) fn set_floor_data(&mut self, output_accumulator: &OutputAccumulator) -> Result<()> {
        output_accumulator.set_floor_data(&mut self.floor_data_reader);
        debug_assert!(self.floor_data_reader.get_position() <= i32::MAX as usize);
        self.rewind_pos = self.floor_data_reader.get_position() as i32;
        self.num_follow_floor_blocks = self.floor_data_reader.read_vint()?;
        self.next_floor_label = self.floor_data_reader.read_byte()? as i32;
        Ok(())
    }
    pub(crate) fn get_term_block_ord(&mut self) -> i32 {
        if self.is_leaf_block {
            self.next_ent
        } else {
            self.state.get_block_term_state().term_block_ord
        }
    }
    pub(crate) fn load_next_floor_block(&mut self) -> Result<()> {
        debug_assert!(
            self.arc.is_none() || self.is_floor,
            "arc= {:?} isFloor={}",
            self.arc,
            self.is_floor
        );

        self.fp = self.fp_end;
        self.next_ent = -1;

        self.load_block()
    }
    pub(crate) fn prefetch_block(&mut self) -> Result<()> {
        if self.next_ent != -1 {
            // Already loaded
            return Ok(());
        }

        // Clone the IndexInput lazily, so that consumers
        // that just pull a TermsEnum to
        // seekExact(TermState) don't pay this cost:
        self.ste.borrow_mut().init_index_input()?;

        // TODO: Could we know the number of bytes to prefetch?
        self.ste
            .borrow_mut()
            .input
            .as_mut()
            .unwrap()
            .prefetch(self.fp, 1)?;
        Ok(())
    }
    /* Does initial decode of next block of terms; this
    doesn't actually decode the docFreq, totalTermFreq,
    postings details (frq/prx offset, etc.) metadata;
    it just loads them as byte[] blobs which are then
    decoded on-demand if the metadata is ever requested
    for any term in this block.  This enables terms-only
    intensive consumes (eg certain MTQs, respelling) to
    not pay the price of decoding metadata they won't
    use. */
    pub(crate) fn load_block(&mut self) -> Result<()> {
        // Clone the IndexInput lazily, so that consumers
        // that just pull a TermsEnum to
        // seekExact(TermState) don't pay this cost:
        self.ste.borrow_mut().init_index_input()?;

        if self.next_ent != -1 {
            return Ok(()); // already loaded
        }

        let mut ste = self.ste.borrow_mut();
        let input = ste.input.as_mut().unwrap();

        input.seek(self.fp)?;
        let code = input.read_vint()?;
        self.ent_count = ((code as u32) >> 1) as i32;
        debug_assert!(self.ent_count > 0);
        self.is_last_in_floor = (code & 1) != 0;

        debug_assert!(
            self.arc.is_none() || self.is_last_in_floor || self.is_floor,
            "fp={} arc={:?} is_floor={} is_last_in_floor={}",
            self.fp,
            self.arc,
            self.is_floor,
            self.is_last_in_floor
        );
        // TODO: if suffixes were stored in random-access
        // array structure, then we could do binary search
        // instead of linear scan to find target term; eg
        // we could have simple array of offsets
        let start_suffix_fp = input.get_file_pointer();
        // term suffixes:
        let code_l = input.read_vlong()?;
        self.is_leaf_block = (code_l & 0x04) != 0;
        let num_suffix_bytes = ((code_l as u64) >> 3) as i32;

        if self.suffix_bytes.len() < num_suffix_bytes as usize {
            let new_len = ArrayUtil::oversize(num_suffix_bytes as usize, 1);
            self.suffix_bytes = vec![0u8; new_len];
        }

        let alg_code = (code_l & 0x03) as u8;
        self.compression_alg = CompressionAlgorithm::by_code(alg_code)?;

        self.compression_alg
            .read(input, &mut self.suffix_bytes, num_suffix_bytes)?;
        self.suffixes_reader.reset_with_range(
            std::mem::take(&mut self.suffix_bytes),
            0,
            num_suffix_bytes as usize,
        );

        let num_suffix_length_bytes = input.read_vint()?;
        debug_assert!(num_suffix_length_bytes >= 0);
        let mut num_suffix_length_bytes = num_suffix_length_bytes as usize;
        self.all_equal = (num_suffix_length_bytes & 0x01) != 0;
        num_suffix_length_bytes >>= 1;

        if self.suffix_length_bytes.len() < num_suffix_length_bytes {
            let new_len = ArrayUtil::oversize(num_suffix_length_bytes, 1);
            self.suffix_length_bytes = vec![0u8; new_len];
        }

        if self.all_equal {
            let fill_byte = input.read_byte()?;
            for i in 0..num_suffix_length_bytes {
                self.suffix_length_bytes[i] = fill_byte;
            }
        } else {
            input.read_bytes(
                &mut self.suffix_length_bytes,
                0,
                num_suffix_length_bytes as i32,
            )?;
        }

        self.suffix_lengths_reader.reset_with_range(
            std::mem::take(&mut self.suffix_length_bytes),
            0,
            num_suffix_length_bytes,
        );
        self.total_suffix_bytes = input.get_file_pointer() - start_suffix_fp;

        // stats
        let mut num_bytes = input.read_vint()?;
        debug_assert!(num_bytes >= 0);
        if self.stat_bytes.len() < num_bytes as usize {
            let new_len = ArrayUtil::oversize(num_bytes as usize, 1);
            self.stat_bytes = vec![0u8; new_len];
        }
        input.read_bytes(&mut self.stat_bytes, 0, num_bytes)?;
        self.stats_reader.reset_with_range(
            std::mem::take(&mut self.stat_bytes),
            0,
            num_bytes as usize,
        );
        self.stats_singleton_run_length = 0;
        self.meta_data_upto = 0;

        self.state.get_block_term_state().term_block_ord = 0;
        self.next_ent = 0;
        self.last_sub_fp = -1;
        // TODO: we could skip this if !hasTerms; but
        // that's rare so won't help much
        // metadata
        num_bytes = input.read_vint()?;
        if self.bytes.len() < num_bytes as usize {
            let new_len = ArrayUtil::oversize(num_bytes as usize, 1);
            self.bytes = vec![0u8; new_len];
        }
        input.read_bytes(&mut self.bytes, 0, num_bytes)?;
        self.bytes_reader
            .reset_with_range(std::mem::take(&mut self.bytes), 0, num_bytes as usize);

        self.fp_end = input.get_file_pointer();

        Ok(())
    }
    pub(crate) fn rewind(&mut self) -> Result<()> {
        // Force reload
        self.fp = self.fp_orig;
        self.next_ent = -1;
        self.has_terms = self.has_terms_orig;

        if self.is_floor {
            self.floor_data_reader
                .set_position(self.rewind_pos as usize);
            self.num_follow_floor_blocks = self.floor_data_reader.read_vint()?;
            debug_assert!(self.num_follow_floor_blocks > 0);
            self.next_floor_label = self.floor_data_reader.read_byte()? as i32;
        }

        Ok(())
    }
    pub fn next(&mut self) -> Result<bool> {
        if self.is_leaf_block {
            self.next_leaf()?;
            Ok(false)
        } else {
            self.next_non_leaf()
        }
    }
    pub fn next_leaf(&mut self) -> Result<()> {
        debug_assert!(
            self.next_ent != -1 && self.next_ent < self.ent_count,
            "next_ent={} ent_count={} fp={}",
            self.next_ent,
            self.ent_count,
            self.fp
        );

        self.next_ent += 1;
        self.suffix_length = self.suffix_lengths_reader.read_vint()?;
        debug_assert!(self.suffixes_reader.get_position() <= i32::MAX as usize);
        self.start_byte_pos = self.suffixes_reader.get_position() as i32;

        let mut ste = self.ste.borrow_mut();
        let term_len = self.prefix_length + self.suffix_length;
        ste.term.set_length(term_len as usize);
        let len = ste.term.length();
        ste.term.grow(len);

        self.suffixes_reader.read_bytes(
            ste.term.get_bytes_mut_ref().bytes.as_mut(),
            self.prefix_length,
            self.suffix_length,
        )?;

        ste.term_exists = true;
        Ok(())
    }
    pub(crate) fn next_non_leaf(&mut self) -> Result<bool> {
        loop {
            if self.next_ent == self.ent_count {
                debug_assert!(
                    self.arc.is_none() || (self.is_floor && !self.is_last_in_floor),
                    "is_floor={}, is_last_in_floor={}",
                    self.is_floor,
                    self.is_last_in_floor
                );

                self.load_next_floor_block()?;

                if self.is_leaf_block {
                    self.next_leaf()?;
                    return Ok(false);
                } else {
                    continue;
                }
            }

            debug_assert!(
                self.next_ent != -1 && self.next_ent < self.ent_count,
                "next_ent={} ent_count={} fp={}",
                self.next_ent,
                self.ent_count,
                self.fp
            );

            self.next_ent += 1;

            let code = self.suffix_lengths_reader.read_vint()?;
            self.suffix_length = ((code as u32) >> 1) as i32;
            debug_assert!(self.suffixes_reader.get_position() <= i32::MAX as usize);
            self.start_byte_pos = self.suffixes_reader.get_position() as i32;

            let mut ste = self.ste.borrow_mut();
            let term_len = self.prefix_length + self.suffix_length;
            ste.term.set_length(term_len as usize);
            let len = ste.term.length();
            ste.term.grow(len);

            self.suffixes_reader.read_bytes(
                ste.term.get_bytes_mut_ref().bytes.as_mut(),
                self.prefix_length,
                self.suffix_length,
            )?;

            if (code & 1) == 0 {
                // Normal term
                ste.term_exists = true;
                self.sub_code = 0;
                self.state.get_block_term_state().term_block_ord += 1;
                return Ok(false);
            } else {
                // A sub-block; make sub-FP absolute:
                ste.term_exists = false;
                self.sub_code = self.suffix_lengths_reader.read_vlong()?;
                self.last_sub_fp = self.fp - self.sub_code;
                return Ok(true);
            }
        }
    }
    pub fn scan_to_floor_frame(&mut self, target: &BytesRef<Vec<u8>>) -> Result<()> {
        if !self.is_floor || target.length <= self.prefix_length as usize {
            return Ok(());
        }

        let target_label = target.bytes[target.offset + self.prefix_length as usize] as i32;

        if target_label < self.next_floor_label {
            return Ok(());
        }

        debug_assert!(self.num_follow_floor_blocks != 0);

        let mut new_fp = self.fp_orig;

        loop {
            let code = self.floor_data_reader.read_vlong()?;
            new_fp = self.fp_orig + ((code as u64) >> 1) as i64;
            self.has_terms = (code & 1) != 0;

            self.is_last_in_floor = self.num_follow_floor_blocks == 0;
            self.num_follow_floor_blocks -= 1;

            if self.is_last_in_floor {
                self.next_floor_label = 256;
                break;
            } else {
                self.next_floor_label = self.floor_data_reader.read_byte()? as i32;
                if target_label < self.next_floor_label {
                    break;
                }
            }
        }

        if new_fp != self.fp {
            self.next_ent = -1;
            self.fp = new_fp;
        }

        Ok(())
    }
    pub fn decode_meta_data(&mut self) -> Result<()> {
        let limit = self.get_term_block_ord();
        let mut absolute = self.meta_data_upto == 0;
        debug_assert!(limit > 0);

        while self.meta_data_upto < limit {
            let ste = self.ste.borrow_mut();
            let state = self.state.get_block_term_state();

            if self.stats_singleton_run_length > 0 {
                state.doc_freq = 1;
                state.total_term_freq = 1;
                self.stats_singleton_run_length -= 1;
            } else {
                let token = self.stats_reader.read_vint()?;
                if (token & 1) == 1 {
                    state.doc_freq = 1;
                    state.total_term_freq = 1;
                    self.stats_singleton_run_length = (token as u32 >> 1) as i32;
                } else {
                    state.doc_freq = (token as u32 >> 1) as i32;
                    if *ste.fr.field_info.get_index_options() == IndexOptions::DOCS {
                        state.total_term_freq = state.doc_freq as i64;
                    } else {
                        state.total_term_freq =
                            state.doc_freq as i64 + self.stats_reader.read_vlong()?;
                    }
                }
            }

            ste.fr.parent.borrow_mut().postings_reader.decode_term(
                &mut self.bytes_reader,
                &ste.fr.field_info,
                &mut self.state,
                absolute,
            )?;

            self.meta_data_upto += 1;
            absolute = false;
        }

        self.state.get_block_term_state().term_block_ord = self.meta_data_upto;

        Ok(())
    }
    /// Used only in debug assertions: does target prefix match the current
    /// term?
    fn prefix_matches(&self, target: &BytesRef<Vec<u8>>) -> bool {
        let ste = self.ste.borrow();
        for byte_pos in 0..self.prefix_length as usize {
            if target.bytes[target.offset + byte_pos] != ste.term.byte_at(byte_pos) {
                return false;
            }
        }
        true
    }
    // Scans to sub-block that has this target fp; only
    // called by next(); NOTE: does not set
    // startBytePos/suffix as a side effect
    pub fn scan_to_sub_block(&mut self, sub_fp: i64) -> Result<()> {
        debug_assert!(!self.is_leaf_block);
        if self.last_sub_fp == sub_fp {
            return Ok(());
        }

        debug_assert!(sub_fp < self.fp, "fp={} sub_fp={}", self.fp, sub_fp);
        let target_sub_code = self.fp - sub_fp;

        loop {
            debug_assert!(self.next_ent < self.ent_count);
            self.next_ent += 1;

            let code = self.suffix_lengths_reader.read_vint()?;
            self.suffixes_reader.skip_bytes((code as u64 >> 1) as i64)?;

            if (code & 1) != 0 {
                let sub_code = self.suffix_lengths_reader.read_vlong()?;
                if target_sub_code == sub_code {
                    self.last_sub_fp = sub_fp;
                    return Ok(());
                }
            } else {
                self.state.get_block_term_state().term_block_ord += 1;
            }
        }
    }
    /// Scan to a specific target term within the block. May update
    /// suffix/startBytePos.
    pub(crate) fn scan_to_term(
        &mut self,
        target: &BytesRef<Vec<u8>>,
        exact_only: bool,
    ) -> Result<SeekStatus> {
        if self.is_leaf_block {
            if self.all_equal {
                self.binary_search_term_leaf(target, exact_only)
            } else {
                self.scan_to_term_leaf(target, exact_only)
            }
        } else {
            self.scan_to_term_non_leaf(target, exact_only)
        }
    }
    // Target's prefix matches this block's prefix; we
    // scan the entries to check if the suffix matches.
    pub fn scan_to_term_leaf(
        &mut self,
        target: &BytesRef<Vec<u8>>,
        exact_only: bool,
    ) -> Result<SeekStatus> {
        debug_assert!(self.next_ent != -1);

        {
            let mut ste = self.ste.borrow_mut();
            ste.term_exists = true;
        }

        self.sub_code = 0;

        if self.next_ent == self.ent_count {
            if exact_only {
                self.fill_term();
            }
            return Ok(SeekStatus::End);
        }

        debug_assert!(self.prefix_matches(target));

        loop {
            self.next_ent += 1;
            self.suffix_length = self.suffix_lengths_reader.read_vint()?;
            debug_assert!(self.suffixes_reader.get_position() <= i32::MAX as usize);
            self.start_byte_pos = self.suffixes_reader.get_position() as i32;
            self.suffixes_reader.skip_bytes(self.suffix_length as i64)?;

            let suffix_start = self.start_byte_pos as usize;
            let suffix_end = suffix_start + self.suffix_length as usize;

            let cmp = self.suffix_bytes[suffix_start..suffix_end]
                .cmp(
                    &target.bytes[target.offset + self.prefix_length as usize
                        ..target.offset + target.length],
                )
                .to_int();

            if cmp < 0 {
                // Current entry is still before the target;
                // keep scanning
            } else if cmp > 0 {
                // Done!  Current entry is after target --
                // return NOT_FOUND:
                self.fill_term();
                return Ok(SeekStatus::NotFound);
            } else {
                // Exact match!

                // This cannot be a sub-block because we
                // would have followed the index to this
                // sub-block from the start:
                self.fill_term();
                return Ok(SeekStatus::Found);
            }
            if self.next_ent < self.ent_count {
                break;
            }
        }
        // It is possible (and OK) that terms index pointed us
        // at this block, but, we scanned the entire block and
        // did not find the term to position to.  This happens
        // when the target is after the last term in the block
        // (but, before the next term in the index).  EG
        // target could be foozzz, and terms index pointed us
        // to the foo* block, but the last term in this block
        // was fooz (and, eg, first term in the next block will
        // bee fop).
        if exact_only {
            self.fill_term();
        }
        // TODO: not consistent that in the
        // not-exact case we don't next() into the next
        // frame here
        Ok(SeekStatus::End)
    }

    // Target's prefix matches this block's prefix;
    // And all suffixes have the same length in this block,
    // we binary search the entries to check if the suffix matches.
    pub fn binary_search_term_leaf(
        &mut self,
        target: &BytesRef<Vec<u8>>,
        exact_only: bool,
    ) -> Result<SeekStatus> {
        debug_assert!(self.next_ent != -1);

        {
            let mut ste = self.ste.borrow_mut();
            ste.term_exists = true;
        }
        self.sub_code = 0;

        if self.next_ent == self.ent_count {
            if exact_only {
                self.fill_term();
            }
            return Ok(SeekStatus::End);
        }

        debug_assert!(self.prefix_matches(target));

        self.suffix_length = self.suffix_lengths_reader.read_vint()?;

        let mut start = self.next_ent;
        let mut end = self.ent_count - 1;
        let mut cmp = 0;

        while start <= end {
            let mid = ((start + end) as u32 >> 1) as i32;
            self.next_ent = mid + 1;
            self.start_byte_pos = mid * self.suffix_length;

            let suffix_start = self.start_byte_pos as usize;
            let suffix_end = suffix_start + self.suffix_length as usize;

            cmp = self.suffix_bytes[suffix_start..suffix_end]
                .cmp(
                    &target.bytes[target.offset + self.prefix_length as usize
                        ..target.offset + target.length],
                )
                .to_int();

            if cmp < 0 {
                start = mid + 1;
            } else if cmp > 0 {
                end = mid - 1;
            } else {
                // match
                self.suffixes_reader
                    .set_position((self.start_byte_pos + self.suffix_length) as usize);
                self.fill_term();
                return Ok(SeekStatus::Found);
            }
        }
        // It is possible (and OK) that terms index pointed us
        // at this block, but, we searched the entire block and
        // did not find the term to position to.  This happens
        // when the target is after the last term in the block
        // (but, before the next term in the index).  EG
        // target could be foozzz, and terms index pointed us
        // to the foo* block, but the last term in this block
        // was fooz (and, eg, first term in the next block will
        // bee fop).
        let seek_status;

        if end < self.ent_count - 1 {
            seek_status = SeekStatus::NotFound;
            if cmp < 0 {
                self.start_byte_pos += self.suffix_length;
                self.next_ent += 1;
            }
            self.suffixes_reader
                .set_position((self.start_byte_pos + self.suffix_length) as usize);
            self.fill_term();
        } else {
            seek_status = SeekStatus::End;
            self.suffixes_reader
                .set_position((self.start_byte_pos + self.suffix_length) as usize);
            if exact_only {
                self.fill_term();
            }
        }

        Ok(seek_status)
    }
    // Target's prefix matches this block's prefix; we
    // scan the entries to check if the suffix matches.
    pub fn scan_to_term_non_leaf(
        &mut self,
        target: &BytesRef<Vec<u8>>,
        exact_only: bool,
    ) -> Result<SeekStatus> {
        debug_assert!(self.next_ent != -1);

        if self.next_ent == self.ent_count {
            if exact_only {
                self.fill_term();
                self.ste.borrow_mut().term_exists = self.sub_code == 0;
            }
            return Ok(SeekStatus::End);
        }

        debug_assert!(self.prefix_matches(target));

        while self.next_ent < self.ent_count {
            self.next_ent += 1;
            let code = self.suffix_lengths_reader.read_vint()?;
            self.suffix_length = (code as u32 >> 1) as i32;
            debug_assert!(self.suffixes_reader.get_position() <= i32::MAX as usize);
            self.start_byte_pos = self.suffixes_reader.get_position() as i32;
            self.suffixes_reader.skip_bytes(self.suffix_length as i64)?;

            let exists = {
                let mut ste = self.ste.borrow_mut();
                ste.term_exists = (code & 1) == 0;
                ste.term_exists
            };
            if exists {
                self.state.get_block_term_state().term_block_ord += 1;
                self.sub_code = 0;
            } else {
                self.sub_code = self.suffix_lengths_reader.read_vlong()?;
                self.last_sub_fp = self.fp - self.sub_code;
            }

            let suffix_start = self.start_byte_pos as usize;
            let suffix_end = suffix_start + self.suffix_length as usize;

            let cmp = self.suffix_bytes[suffix_start..suffix_end]
                .cmp(
                    &target.bytes[target.offset + self.prefix_length as usize
                        ..target.offset + target.length],
                )
                .to_int();

            if cmp < 0 {
                // Current entry is still before the target;
                // keep scanning
            } else if cmp > 0 {
                // TODO: 等完成segment_terms_enum再来写
                self.fill_term();
                let ste = self.ste.borrow();
                // if !exact_only && !ste.term_exists {
                //     let prefix_len = self.prefix_length + self.suffix_length;
                //     let mut new_frame = ste.push_frame(None, self.last_sub_fp, prefix_len);
                //     new_frame.load_block()?;
                //     while new_frame.next()? {
                //         let next_prefix = ste.term.length();
                //         new_frame = ste.push_frame(None, new_frame.last_sub_fp, next_prefix);
                //         new_frame.load_block()?;
                //     }
                //     ste.current_frame = new_frame;
                // }

                return Ok(SeekStatus::NotFound);
            } else {
                debug_assert!(self.ste.borrow_mut().term_exists);
                self.fill_term();
                return Ok(SeekStatus::Found);
            }
        }
        // It is possible (and OK) that terms index pointed us
        // at this block, but, we scanned the entire block and
        // did not find the term to position to.  This happens
        // when the target is after the last term in the block
        // (but, before the next term in the index).  EG
        // target could be foozzz, and terms index pointed us
        // to the foo* block, but the last term in this block
        // was fooz (and, eg, first term in the next block will
        // bee fop).
        if exact_only {
            self.fill_term();
        }

        Ok(SeekStatus::End)
    }
    pub(crate) fn fill_term(&mut self) {
        let term_length = self.prefix_length + self.suffix_length;
        let mut ste = self.ste.borrow_mut();

        ste.term.set_length(term_length as usize);
        ste.term.grow(term_length as usize);

        let dest: &mut [u8] = ste.term.get_bytes_mut_ref().bytes.as_mut();
        let src = &self.suffix_bytes;
        let start = self.start_byte_pos as usize;
        let len = start + self.suffix_length as usize;
        dest.copy_from(&src[start..start + len], self.prefix_length as usize);
    }
}
