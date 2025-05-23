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
use std::borrow::Cow;
use std::fmt;
use std::rc::Rc;

use crate::codecs::block_term_state::BlockTermStateEnum;
use crate::codecs::block_tree::compression_algorithm::CompressionAlgorithm;
use crate::codecs::block_tree::lucene90_block_tree_terms_reader::lucene90_bttr_util;
use crate::codecs::postings_writer_base::PostingsWriterBase;
use crate::index::field_info::FieldInfo;
use crate::index::index_options::IndexOptions;
use crate::index::{BytesRef, BytesRefBuilder};
use crate::store::dummy::dummy_directory::DummyDirectory;
use crate::store::{
    ByteArrayDataOutput, ByteBuffersDataOutput, DataOutput, IndexInput, IndexOutput,
};
use crate::util::array_util::ArrayUtil;
use crate::util::bit_set::BitSet;
use crate::util::compress::lowercase_ascii_compression::LowercaseAsciiCompression;
use crate::util::compress::lz4::{HashTableEnum, HighCompressionHashTable, LZ4};
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fixed_bit_set::FixedBitSet;
use crate::util::fst_impl::byte_sequence_outputs::ByteSequenceOutputs;
use crate::util::fst_impl::bytes_ref_fst_enum::BytesRefFSTEnum;
use crate::util::fst_impl::fst::{fst_util, InputType, FST};
use crate::util::fst_impl::fst_compiler::{
    fst_compiler_util, Builder, DataOutputEnum, FSTCompiler,
};
use crate::util::fst_impl::util::Util;
use crate::util::ints_ref_builder::IntsRefBuilder;
use crate::util::packed::PackedInts;
use crate::util::to_string_utils::ToStringUtils;
use crate::util::{CoreHelper, SliceCopyOps, StringHelper};

pub struct Lucene90BlockTreeTermsWriter {
    first_pending_term: Option<PendingTerm>,
    last_pending_term: PendingTerm,
}

trait PendingEntry {
    fn is_term(&self) -> bool;
}
pub struct PendingTerm {
    pub term_bytes: Vec<u8>,
    pub state: BlockTermStateEnum,
}

impl PendingEntry for PendingTerm {
    fn is_term(&self) -> bool {
        true
    }
}

impl PendingTerm {
    pub fn new(term: &BytesRef<Vec<u8>>, state: BlockTermStateEnum) -> Self {
        Self {
            term_bytes: term.bytes[term.offset..term.offset + term.length].to_vec(),
            state,
        }
    }
}
impl fmt::Display for PendingTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = ToStringUtils::bytes_ref_to_string_from_bytes(self.term_bytes.clone());
        write!(f, "TERM: {}", s)
    }
}
pub struct PendingBlock {
    pub prefix: BytesRef<Vec<u8>>,
    pub fp: i64,
    pub index:
        Option<FST<BytesRef<Rc<Vec<u8>>>, ByteSequenceOutputs, DataOutputEnum<DummyDirectory>>>,
    pub sub_indices:
        Vec<FST<BytesRef<Rc<Vec<u8>>>, ByteSequenceOutputs, DataOutputEnum<DummyDirectory>>>,
    pub has_terms: bool,
    pub is_floor: bool,
    pub floor_lead_byte: i32,
}
impl PendingBlock {
    pub fn new(
        prefix: BytesRef<Vec<u8>>,
        fp: i64,
        has_terms: bool,
        is_floor: bool,
        floor_lead_byte: i32,
        sub_indices: Vec<
            FST<BytesRef<Rc<Vec<u8>>>, ByteSequenceOutputs, DataOutputEnum<DummyDirectory>>,
        >,
    ) -> Self {
        Self {
            prefix,
            fp,
            has_terms,
            is_floor,
            floor_lead_byte,
            index: None,
            sub_indices,
        }
    }
    fn compile_index(
        mut blocks: Vec<PendingBlock>,
        scratch_bytes: &mut ByteBuffersDataOutput,
        scratch_ints_ref: &mut IntsRefBuilder<Vec<i32>>,
        version: i32,
    ) -> Result<PendingBlock> {
        debug_assert!(
            (blocks.len() > 1 && blocks[0].is_floor) || (!blocks[0].is_floor && blocks.len() == 1),
            "is_floor={}, blocks.len()={}",
            blocks[0].is_floor,
            blocks.len()
        );
        debug_assert_eq!(scratch_bytes.size(), 0);

        let (is_floor, fp, prefix_len) = {
            let first_block = &mut blocks[0];
            let output = lucene90_bttw_util::encode_output(
                first_block.fp,
                first_block.has_terms,
                first_block.is_floor,
            );
            if version >= lucene90_bttr_util::VERSION_MSB_VLONG_OUTPUT {
                lucene90_bttw_util::write_msb_vlong(scratch_bytes, output)?;
            } else {
                scratch_bytes.write_vlong(output)?;
            }
            (
                first_block.is_floor,
                first_block.fp,
                first_block.prefix.length,
            )
        };

        if is_floor {
            debug_assert!((blocks.len() - 1) <= i32::MAX as usize);
            scratch_bytes.write_vint((blocks.len() - 1) as i32)?;
            for block in &blocks[1..] {
                debug_assert!(block.floor_lead_byte != -1);
                scratch_bytes.write_byte(block.floor_lead_byte as u8)?;
                debug_assert!(block.fp > fp);
                let delta_fp = ((block.fp - fp) << 1) | if block.has_terms { 1 } else { 0 };
                scratch_bytes.write_vlong(delta_fp)?;
            }
        }

        let mut estimate_size = prefix_len as i64;
        for block in blocks.iter() {
            for sub_index in &block.sub_indices {
                estimate_size += sub_index.num_bytes();
            }
        }

        let estimate_bits_required = PackedInts::bits_required(estimate_size)?;
        let page_bits = estimate_bits_required.clamp(6, 15);

        let outputs = ByteSequenceOutputs::get_singleton();
        let fst_version = if version >= lucene90_bttr_util::VERSION_CURRENT {
            fst_util::VERSION_CURRENT
        } else {
            fst_util::VERSION_90
        };

        let mut builder = Builder::new(InputType::Byte1, outputs.clone());
        // Disable suffixes sharing for block tree index because suffixes are mostly
        // dropped from the FST index and left in the term blocks.
        builder.suffix_ram_limit_mb(0.0)?;
        builder.data_output(DataOutputEnum::ReadWriter(
            fst_compiler_util::get_on_heap_reader_writer(page_bits)?,
        ));
        builder.with_version(fst_version)?;
        let mut fst_compiler = builder.build()?;

        let bytes = scratch_bytes.get_array_copy();
        let len = bytes.len();
        debug_assert!(!bytes.is_empty());

        Util::get_ints_ref(&blocks[0].prefix, scratch_ints_ref);
        fst_compiler.add(
            scratch_ints_ref.get(),
            BytesRef::from_slice(Rc::from(bytes), 0, len),
        )?;
        scratch_bytes.reset();

        for block in blocks.iter_mut() {
            for sub_index in std::mem::take(&mut block.sub_indices) {
                block.append(&mut fst_compiler, sub_index, scratch_ints_ref)?;
            }
        }
        let first_block = &mut blocks[0];
        first_block.index = Some(
            FST::from_fst_reader(
                fst_compiler.compile()?,
                Some(fst_compiler.inner.borrow_mut().get_fst_reader()?),
            )
            .unwrap(),
        );

        debug_assert!(first_block.sub_indices.is_empty());
        Ok(blocks.remove(0))
    }
    fn append(
        &self,
        fst_compiler: &mut FSTCompiler<BytesRef<Rc<Vec<u8>>>, ByteSequenceOutputs, DummyDirectory>,
        sub_index: FST<BytesRef<Rc<Vec<u8>>>, ByteSequenceOutputs, DataOutputEnum<DummyDirectory>>,
        scratch_ints_ref: &mut IntsRefBuilder<Vec<i32>>,
    ) -> Result<()> {
        let mut sub_index_enum = BytesRefFSTEnum::new(sub_index)?;

        while let Some(index_ent) = sub_index_enum.next()? {
            Util::get_ints_ref(&index_ent.input, scratch_ints_ref);
            fst_compiler.add(scratch_ints_ref.get(), index_ent.output.clone())?;
        }

        Ok(())
    }
}

struct StatsWriter {
    has_freqs: bool,
    singleton_count: i32,
}

impl StatsWriter {
    fn new(has_freqs: bool) -> Self {
        Self {
            has_freqs,
            singleton_count: 0,
        }
    }
    fn add(&mut self, out: &mut impl DataOutput, df: i32, ttf: i64) -> Result<()> {
        if df == 1 && (!self.has_freqs || ttf == 1) {
            self.singleton_count += 1;
        } else {
            self.finish(out)?;
            out.write_vint(df << 1)?;
            if self.has_freqs {
                out.write_vlong(ttf - df as i64)?;
            }
        }
        Ok(())
    }

    fn finish(&mut self, out: &mut impl DataOutput) -> Result<()> {
        if self.singleton_count > 0 {
            out.write_vint(((self.singleton_count - 1) << 1) | 1)?;
            self.singleton_count = 0;
        }
        Ok(())
    }
}
pub struct TermsWriter {
    field_info: FieldInfo,
    num_terms: i64,
    docs_seen: FixedBitSet,
    sum_total_term_freq: i64,
    sum_doc_freq: i64,
    // Records index into pending where the current prefix at that
    // length "started"; for example, if current term starts with 't',
    // startsByPrefix[0] is the index into pending for the first
    // term/sub-block starting with 't'.  We use this to figure out when
    // to write a new block:
    last_term: BytesRefBuilder<Vec<u8>>,
    prefix_starts: Vec<usize>,
    // Pending stack of terms and blocks.  As terms arrive (in sorted order)
    // we append to this stack, and once the top of the stack has enough
    // terms starting with a common prefix, we write a new block with
    // those terms and replace those terms in the stack with a new block:
    pending: Vec<PendingEntryEnum>,
    // Reused in writeBlocks:
    new_blocks: Vec<PendingBlock>,
    suffix_lengths_writer: ByteBuffersDataOutput,
    suffix_writer: BytesRefBuilder<Vec<u8>>,
    stats_writer: ByteBuffersDataOutput,
    meta_writer: ByteBuffersDataOutput,
    spare_writer: ByteBuffersDataOutput,
    spare_bytes: Vec<u8>,
    compression_hash_table: Option<HashTableEnum>,
    min_items_in_block: i32,
    max_items_in_block: i32,
    scratch_bytes: ByteBuffersDataOutput,
    scratch_ints_ref: IntsRefBuilder<Vec<i32>>,
    version: i32,
}
impl TermsWriter {
    pub fn write_blocks<O, PW>(
        &mut self,
        prefix_length: usize,
        count: usize,
        terms_out: &mut O,
        postings_writer: &mut PW,
    ) -> Result<()>
    where
        O: IndexOutput,
        PW: PostingsWriterBase,
    {
        debug_assert!(count > 0);
        debug_assert!(prefix_length > 0 || count == self.pending.len());

        let mut last_suffix_lead_label = -1;
        let mut has_terms = false;
        let mut has_sub_blocks = false;

        let start = self.pending.len() - count;
        let end = self.pending.len();
        let mut next_block_start = start;
        let mut next_floor_lead_label = -1;

        for i in start..end {
            let (suffix_lead_label, is_term) = {
                let ent = &self.pending[i];
                (
                    match ent {
                        PendingEntryEnum::Term(term) => {
                            if term.term_bytes.len() == prefix_length {
                                debug_assert_eq!(
                                    last_suffix_lead_label, -1,
                                    "i={} last_suffix_lead_label={}",
                                    i, last_suffix_lead_label
                                );
                                -1
                            } else {
                                term.term_bytes[prefix_length] as i32
                            }
                        },
                        PendingEntryEnum::Block(block) => {
                            debug_assert!(block.prefix.length > prefix_length);
                            block.prefix.bytes[block.prefix.offset + prefix_length] as i32
                        },
                    },
                    true,
                )
            };

            if suffix_lead_label != last_suffix_lead_label {
                let items_in_block = i - next_block_start;
                if items_in_block >= self.min_items_in_block as usize
                    && end - next_block_start > self.max_items_in_block as usize
                {
                    let is_floor = items_in_block < count;
                    let block = self.write_block(
                        prefix_length,
                        is_floor,
                        next_floor_lead_label,
                        next_block_start,
                        i,
                        has_terms,
                        has_sub_blocks,
                        terms_out,
                        postings_writer,
                    )?;
                    self.new_blocks.push(block);

                    has_terms = false;
                    has_sub_blocks = false;
                    next_floor_lead_label = suffix_lead_label;
                    next_block_start = i;
                }
                last_suffix_lead_label = suffix_lead_label;
            }
            if is_term {
                has_terms = true;
            } else {
                has_sub_blocks = true;
            }
        }

        if next_block_start < end {
            let items_in_block = end - next_block_start;
            let is_floor = items_in_block < count;
            let block = self.write_block(
                prefix_length,
                is_floor,
                next_floor_lead_label,
                next_block_start,
                end,
                has_terms,
                has_sub_blocks,
                terms_out,
                postings_writer,
            )?;
            self.new_blocks.push(block);
        }

        debug_assert!(!self.new_blocks.is_empty());

        debug_assert!(self.new_blocks[0].is_floor || self.new_blocks.len() == 1);

        let first_block = PendingBlock::compile_index(
            std::mem::take(&mut self.new_blocks),
            &mut self.scratch_bytes,
            &mut self.scratch_ints_ref,
            self.version,
        )?;

        let remove_start = self.pending.len() - count;
        self.pending.drain(remove_start..);
        self.pending.push(PendingEntryEnum::Block(first_block));

        Ok(())
    }

    fn all_equal(b: &[u8], start_offset: usize, end_offset: usize, value: u8) -> Result<bool> {
        CoreHelper::check_from_index_size(start_offset as i32, end_offset as i32, b.len() as i32)?;
        Ok(b[start_offset..end_offset].iter().all(|&x| x == value))
    }
    #[allow(clippy::too_many_arguments)]
    pub fn write_block<O, PW>(
        &mut self,
        prefix_length: usize,
        is_floor: bool,
        floor_lead_label: i32,
        start: usize,
        end: usize,
        has_terms: bool,
        has_sub_blocks: bool,
        terms_out: &mut O,
        postings_writer: &mut PW,
    ) -> Result<PendingBlock>
    where
        O: IndexOutput,
        PW: PostingsWriterBase,
    {
        debug_assert!(end > start);

        let start_fp = terms_out.get_file_pointer();
        let has_floor_lead = is_floor && floor_lead_label != -1;

        let mut prefix_bytes = vec![0u8; prefix_length + if has_floor_lead { 1 } else { 0 }];
        prefix_bytes.copy_from(&self.last_term.bytes_ref.bytes[0..prefix_length], 0);
        let mut prefix = BytesRef::from_bytes(prefix_bytes);
        prefix.length = prefix_length;

        let num_entries = end - start;
        let mut code = (num_entries << 1) as i32;
        if end == self.pending.len() {
            code |= 1;
        }
        terms_out.write_vint(code)?;

        let is_leaf = !has_sub_blocks;
        let mut sub_indices = Vec::new();
        let mut absolute = true;

        if is_leaf {
            let mut stats_writer =
                StatsWriter::new(*self.field_info.get_index_options() != IndexOptions::DOCS);
            for i in start..end {
                let term = match &self.pending[i] {
                    PendingEntryEnum::Term(term) => term,
                    _ => return Err(LuceneError::illegal_state("Expected PendingTerm")),
                };
                debug_assert!(StringHelper::starts_with_byte_array(
                    &term.term_bytes,
                    &prefix
                ));
                let state = &term.state;
                let suffix = term.term_bytes.len() - prefix_length;

                self.suffix_lengths_writer.write_vint(suffix as i32)?;
                self.suffix_writer
                    .append_with_range(&term.term_bytes, prefix_length, suffix);
                debug_assert!(
                    floor_lead_label == -1
                        || (term.term_bytes[prefix_length] as i32) >= floor_lead_label
                );

                match state {
                    BlockTermStateEnum::Block(block) => {
                        stats_writer.add(terms_out, block.doc_freq, block.total_term_freq)?;
                    },
                    BlockTermStateEnum::Int(int) => {
                        stats_writer.add(terms_out, int.base.doc_freq, int.base.total_term_freq)?;
                    },
                }

                postings_writer.encode_term(
                    terms_out,
                    &self.field_info,
                    Cow::Borrowed(state),
                    absolute,
                )?;
                absolute = false;
            }
            stats_writer.finish(terms_out)?;
        } else {
            let mut stats_writer =
                StatsWriter::new(*self.field_info.get_index_options() != IndexOptions::DOCS);
            for i in start..end {
                match &mut self.pending[i] {
                    PendingEntryEnum::Term(term) => {
                        debug_assert!(StringHelper::starts_with_byte_array(
                            &term.term_bytes,
                            &prefix
                        ));
                        let state = &term.state;
                        let suffix_len = term.term_bytes.len() - prefix_length;

                        self.suffix_lengths_writer
                            .write_vint((suffix_len << 1) as i32)?;
                        self.suffix_writer.append_with_range(
                            &term.term_bytes,
                            prefix_length,
                            suffix_len,
                        );
                        match state {
                            BlockTermStateEnum::Block(block) => {
                                stats_writer.add(
                                    terms_out,
                                    block.doc_freq,
                                    block.total_term_freq,
                                )?;
                            },
                            BlockTermStateEnum::Int(int) => {
                                stats_writer.add(
                                    terms_out,
                                    int.base.doc_freq,
                                    int.base.total_term_freq,
                                )?;
                            },
                        }
                        // meta
                        postings_writer.encode_term(
                            terms_out,
                            &self.field_info,
                            Cow::Borrowed(state),
                            absolute,
                        )?;
                        absolute = false;
                    },
                    PendingEntryEnum::Block(block) => {
                        debug_assert!(StringHelper::starts_with_byte_ref(&block.prefix, &prefix));
                        let suffix = block.prefix.length - prefix_length;
                        debug_assert!(suffix > 0);

                        // write block suffix
                        terms_out.write_vint(((suffix << 1) | 1) as i32)?;
                        self.suffix_writer.append_with_range(
                            &block.prefix.bytes,
                            prefix_length,
                            suffix,
                        );

                        debug_assert!(
                            floor_lead_label == -1
                                || (block.prefix.bytes[prefix_length] as i32) >= floor_lead_label
                        );
                        debug_assert!(block.fp < start_fp);

                        terms_out.write_vlong(start_fp - block.fp)?;
                        sub_indices.push(block.index.take().unwrap());
                    },
                }
            }
            stats_writer.finish(terms_out)?;
            debug_assert!(!sub_indices.is_empty());
        }
        // Write suffixes byte[] blob to terms dict output, either uncompressed,
        // compressed with LZ4 or with LowercaseAsciiCompression.
        let mut compression_alg = CompressionAlgorithm::NoCompression;
        let suffix_len = self.suffix_writer.length();
        // If there are 2 suffix bytes or less per term, then we don't bother
        // compressing as suffix are unlikely what
        // makes the terms dictionary large, and it also tends to be frequently the case
        // for dense IDs like
        // auto-increment IDs, so not compressing in that case helps not hurt ID lookups
        // by too much. We also only start compressing when the prefix length is
        // greater than 2 since blocks whose prefix length is
        // 1 or 2 always all get visited when running a fuzzy query whose max number of
        // edits is 2.
        if suffix_len > 2 * num_entries && prefix_length > 2 {
            if suffix_len > 6 * num_entries {
                if self.compression_hash_table.is_none() {
                    self.compression_hash_table =
                        Some(HashTableEnum::High(HighCompressionHashTable::default()));
                }
                let bytes = LZ4::compress(
                    std::mem::take(&mut self.suffix_writer.bytes_ref.bytes),
                    0,
                    suffix_len.try_into()?,
                    &mut self.spare_writer,
                    self.compression_hash_table.as_mut().unwrap(),
                )?;
                // take ownership back
                self.suffix_writer.bytes_ref.bytes = bytes;

                if self.spare_writer.size() < (suffix_len - (suffix_len >> 2)) as i64 {
                    compression_alg = CompressionAlgorithm::Lz4;
                }
            }

            if compression_alg == CompressionAlgorithm::NoCompression {
                self.spare_writer.reset();

                if self.spare_bytes.len() < suffix_len {
                    self.spare_bytes = vec![0u8; ArrayUtil::oversize(suffix_len, 1)];
                }

                if LowercaseAsciiCompression::compress(
                    &self.suffix_writer.bytes_ref.bytes,
                    suffix_len,
                    &mut self.spare_bytes,
                    &mut self.spare_writer,
                )? {
                    compression_alg = CompressionAlgorithm::LowercaseAscii;
                }
            }
        }

        let mut token = (suffix_len as u64) << 3;
        if is_leaf {
            token |= 0x04;
        }
        token |= compression_alg.code() as u64;
        terms_out.write_vlong(token.try_into()?)?;

        if compression_alg == CompressionAlgorithm::NoCompression {
            terms_out.write_bytes_with_len(
                &self.suffix_writer.bytes_ref.bytes,
                suffix_len.try_into()?,
            )?;
        } else {
            self.spare_writer.copy_to(terms_out)?;
        }
        self.suffix_writer.set_length(0);
        self.spare_writer.reset();

        // suffix lengths
        let num_suffix_bytes = self.suffix_lengths_writer.size().try_into()?;
        if let Some(v) = ArrayUtil::grow_no_copy(&self.spare_bytes, num_suffix_bytes) {
            self.spare_bytes = v
        }
        {
            let mut data_output =
                ByteArrayDataOutput::with_bytes(std::mem::take(&mut self.spare_bytes));
            self.suffix_lengths_writer.copy_to(&mut data_output)?;
            self.spare_bytes = std::mem::take(&mut data_output.bytes);
        }
        self.suffix_lengths_writer.reset();

        if Self::all_equal(&self.spare_bytes, 1, num_suffix_bytes, self.spare_bytes[0])? {
            debug_assert!(num_suffix_bytes <= i32::MAX as usize);
            terms_out.write_vint(((num_suffix_bytes << 1) | 1) as i32)?;
            terms_out.write_byte(self.spare_bytes[0])?;
        } else {
            debug_assert!(num_suffix_bytes <= i32::MAX as usize);
            terms_out.write_vint((num_suffix_bytes << 1) as i32)?;
            terms_out.write_bytes_with_len(&self.spare_bytes, num_suffix_bytes as i32)?;
        }

        // stats
        let num_stats_bytes = self.stats_writer.size() as i32;
        terms_out.write_vint(num_stats_bytes)?;
        self.stats_writer.copy_to(terms_out)?;
        self.stats_writer.reset();

        // meta
        terms_out.write_vint(self.meta_writer.size() as i32)?;
        self.meta_writer.copy_to(terms_out)?;
        self.meta_writer.reset();

        if has_floor_lead {
            prefix.bytes[prefix.length] = floor_lead_label as u8;
            prefix.length += 1;
        }

        Ok(PendingBlock::new(
            prefix,
            start_fp,
            has_terms,
            is_floor,
            floor_lead_label,
            sub_indices,
        ))
    }
    pub fn write<O, PW>(
        &mut self,
        text: &BytesRef<Vec<u8>>,
        terms_out: &mut O,
        terms_enum: &mut PW::TermsEnum,
        norms: &mut PW::Norms,
        postings_writer: &mut PW,
    ) -> Result<()>
    where
        O: IndexOutput,
        PW: PostingsWriterBase,
    {
        let state_opt = postings_writer.write_term(text, terms_enum, &mut self.docs_seen, norms)?;

        if let Some(state) = &state_opt {
            let (total_term_freq, doc_freq) = match state {
                BlockTermStateEnum::Block(block) => {
                    debug_assert!(block.doc_freq != 0);
                    (block.total_term_freq, block.doc_freq)
                },
                BlockTermStateEnum::Int(int) => {
                    debug_assert!(int.base.doc_freq != 0);
                    (int.base.total_term_freq, int.base.doc_freq)
                },
            };
            debug_assert!(
                *self.field_info.get_index_options() == IndexOptions::DOCS
                    || total_term_freq > doc_freq as i64
            );

            self.push_term(text, terms_out, postings_writer)?;

            let term = PendingTerm::new(text, state_opt.unwrap());
            self.pending.push(PendingEntryEnum::Term(term));

            self.sum_doc_freq += doc_freq as i64;
            self.sum_total_term_freq += total_term_freq;
            self.num_terms += 1;
        }

        Ok(())
    }
    fn push_term<O, PW>(
        &mut self,
        text: &BytesRef<Vec<u8>>,
        terms_out: &mut O,
        postings_writer: &mut PW,
    ) -> Result<()>
    where
        O: IndexOutput,
        PW: PostingsWriterBase,
    {
        let last_bytes = self.last_term.get_bytes_ref();
        let mut prefix_length = CoreHelper::miss_match(
            &last_bytes.bytes[..self.last_term.length()],
            &text.bytes[text.offset..text.offset + text.length],
        );
        if prefix_length == 1 {
            debug_assert!(self.last_term.length() == 0);
            prefix_length = 0;
        }

        for i in (prefix_length as usize..last_bytes.length).rev() {
            let prefix_top_size = self.pending.len() - self.prefix_starts[i];
            if prefix_top_size >= self.min_items_in_block as usize {
                self.write_blocks(i + 1, prefix_top_size, terms_out, postings_writer)?;
                self.prefix_starts[i] -= prefix_top_size - 1;
            }
        }

        if self.prefix_starts.len() < text.length {
            ArrayUtil::grow_with_len(&mut self.prefix_starts, text.length);
        }

        for i in prefix_length as usize..text.length {
            self.prefix_starts[i] = self.pending.len();
        }

        self.last_term.copy_bytes_with_ref(text);
        Ok(())
    }
    pub fn finish<O, PW>(
        &mut self,
        first_term_bytes: Vec<u8>,
        last_term_bytes: Vec<u8>,
        terms_out: &mut O,
        postings_writer: &mut PW,
        fields: &mut Vec<ByteBuffersDataOutput>,
        index_out: &mut O,
    ) -> Result<()>
    where
        O: IndexOutput,
        PW: PostingsWriterBase,
    {
        if self.num_terms > 0 {
            self.push_term(&BytesRef::new(), terms_out, postings_writer)?;
            self.push_term(&BytesRef::new(), terms_out, postings_writer)?;

            let pending_len = self.pending.len();
            self.write_blocks(0, pending_len, terms_out, postings_writer)?;

            debug_assert!(
                self.pending.len() == 1
                    && match self.pending[0] {
                        PendingEntryEnum::Block(_) => true,
                        PendingEntryEnum::Term(_) => false,
                    }
            );
            let mut root = match self.pending.pop().unwrap() {
                PendingEntryEnum::Block(b) => b,
                _ => return Err(LuceneError::illegal_state("expected final root block")),
            };
            debug_assert_eq!(root.prefix.length, 0);

            let root_code = root.index.as_ref().unwrap().get_empty_output();
            debug_assert!(root_code.is_some());

            let mut meta_out = ByteBuffersDataOutput::new();

            meta_out.write_vint(self.field_info.get_field_number())?;
            meta_out.write_vlong(self.num_terms)?;

            let root_code = root_code.unwrap();
            debug_assert!(root_code.length <= i32::MAX as usize);
            meta_out.write_vint(root_code.length as i32)?;
            debug_assert!(root_code.offset <= i32::MAX as usize);
            debug_assert!(root_code.length <= i32::MAX as usize);
            meta_out.write_bytes_range(
                &root_code.bytes,
                root_code.offset as i32,
                root_code.length as i32,
            )?;
            debug_assert!(*self.field_info.get_index_options() != IndexOptions::None);

            if *self.field_info.get_index_options() != IndexOptions::DOCS {
                meta_out.write_vlong(self.sum_total_term_freq)?;
            }
            meta_out.write_vlong(self.sum_doc_freq)?;
            meta_out.write_vint(self.docs_seen.cardinality())?;
            self.write_bytes_ref(&mut meta_out, &BytesRef::from_bytes(first_term_bytes))?;
            self.write_bytes_ref(&mut meta_out, &BytesRef::from_bytes(last_term_bytes))?;
            meta_out.write_vlong(index_out.get_file_pointer())?;
            root.index
                .as_mut()
                .unwrap()
                .save(&mut meta_out, index_out)?;

            fields.push(meta_out);
        } else {
            debug_assert!(
                self.sum_total_term_freq == 0
                    || (*self.field_info.get_index_options() == IndexOptions::DOCS
                        && self.sum_total_term_freq == -1)
            );
            debug_assert_eq!(self.sum_doc_freq, 0);
            debug_assert_eq!(self.docs_seen.cardinality(), 0);
        }

        Ok(())
    }
    fn write_bytes_ref(&self, out: &mut impl DataOutput, bytes: &BytesRef<Vec<u8>>) -> Result<()> {
        debug_assert!(bytes.length <= i32::MAX as usize);
        out.write_vint(bytes.length as i32)?;
        debug_assert!(bytes.offset <= i32::MAX as usize);
        out.write_bytes_range(&bytes.bytes, bytes.offset as i32, bytes.length as i32)?;
        Ok(())
    }
}

pub(crate) mod lucene90_bttw_util {
    use crate::codecs::block_tree::lucene90_block_tree_terms_reader::lucene90_bttr_util;
    use crate::store::DataOutput;
    use crate::util::error::lucene_error::Result;
    pub fn encode_output(fp: i64, has_terms: bool, is_floor: bool) -> i64 {
        debug_assert!(fp < (1i64 << 62));
        (fp << 2)
            | if has_terms {
                lucene90_bttr_util::OUTPUT_FLAG_HAS_TERMS as i64
            } else {
                0
            }
            | if is_floor {
                lucene90_bttr_util::OUTPUT_FLAG_IS_FLOOR as i64
            } else {
                0
            }
    }

    pub(crate) fn write_msb_vlong(out: &mut impl DataOutput, mut l: i64) -> Result<()> {
        debug_assert!(l >= 0);
        // Keep zero bits on most significant byte to have more chance to get prefix
        // bytes shared. e.g. we expect 0x7FFF stored as [0x81, 0xFF, 0x7F] but
        // not [0xFF, 0xFF, 0x40]
        let bits = 64 - l.leading_zeros();
        let bytes_needed = ((bits.saturating_sub(1)) / 7 + 1) as usize;
        l <<= 64 - bytes_needed * 7;
        for _ in 1..bytes_needed {
            let byte = ((l >> 57) & 0x7F) as u8 | 0x80;
            out.write_byte(byte)?;
            l <<= 7;
        }
        let last_byte = ((l >> 57) & 0x7F) as u8;
        out.write_byte(last_byte)?;
        Ok(())
    }
}

enum PendingEntryEnum {
    Term(PendingTerm),
    Block(PendingBlock),
}

#[cfg(test)]
mod tests {
    use crate::codecs::block_tree::field_reader::field_reader_util;
    use crate::codecs::block_tree::lucene90_block_tree_terms_writer::lucene90_bttw_util;
    use crate::store::{ByteArrayDataInput, ByteArrayDataOutput};
    use crate::test::util::lucene_test_case::{at_least, random};
    use crate::util::error::lucene_error::Result;
    #[allow(dead_code)] // for quick search
    struct TestMSBVLong;

    #[test]
    fn test_msb_vlong() -> Result<()> {
        assert_msb_vlong(i64::MAX)?;
        let mut random = random();
        let iter = at_least(&mut random, 10000) as i64;
        for i in 0..iter {
            assert_msb_vlong(i)?;
        }
        Ok(())
    }

    fn assert_msb_vlong(l: i64) -> Result<()> {
        let buffer = vec![0u8; 10];
        let mut output = ByteArrayDataOutput::with_bytes(buffer);
        lucene90_bttw_util::write_msb_vlong(&mut output, l)?;
        let buffer = output.bytes.clone();
        let len = output.get_position();
        let mut input = ByteArrayDataInput::with_range(buffer, 0, len);
        let recovered = field_reader_util::read_msb_vlong(&mut input)?;
        assert_eq!(
            recovered, l,
            "Mismatch in MSB VLong roundtrip: {} != {}",
            l, recovered
        );

        Ok(())
    }
}
