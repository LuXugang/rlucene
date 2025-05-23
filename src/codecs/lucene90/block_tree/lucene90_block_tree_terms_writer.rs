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
use crate::codecs::block_tree::lucene90_block_tree_terms_reader::lucene90_bttr_util;
use crate::codecs::postings_writer_base::PostingsWriterBase;
use crate::index::field_info::FieldInfo;
use crate::index::index_options::IndexOptions;
use crate::index::{BytesRef, BytesRefBuilder};
use crate::store::dummy::dummy_directory::DummyDirectory;
use crate::store::{ByteBuffersDataOutput, DataOutput, IndexInput, IndexOutput};
use crate::util::compress::lz4;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fixed_bit_set::FixedBitSet;
use crate::util::fst_impl::byte_sequence_outputs::ByteSequenceOutputs;
use crate::util::fst_impl::bytes_ref_fst_enum::BytesRefFSTEnum;
use crate::util::fst_impl::fst::{fst_util, InputType, FST};
use crate::util::fst_impl::fst_compiler::{
    fst_compiler_util, Builder, DataOutputEnum, FSTCompiler,
};
use crate::util::fst_impl::off_heap_fst_store::OffHeapFSTStore;
use crate::util::fst_impl::util::Util;
use crate::util::ints_ref_builder::IntsRefBuilder;
use crate::util::packed::PackedInts;
use crate::util::to_string_utils::ToStringUtils;
use crate::util::{CoreHelper, SliceCopyOps, StringHelper};

pub struct Lucene90BlockTreeTermsWriter;

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
pub struct PendingBlock<I>
where
    I: IndexInput,
{
    pub prefix: BytesRef<Vec<u8>>,
    pub fp: i64,
    pub index:
        Option<FST<BytesRef<Rc<Vec<u8>>>, ByteSequenceOutputs, DataOutputEnum<DummyDirectory>>>,
    pub sub_indices: Vec<FST<BytesRef<Rc<Vec<u8>>>, ByteSequenceOutputs, OffHeapFSTStore<I>>>,
    pub has_terms: bool,
    pub is_floor: bool,
    pub floor_lead_byte: i32,
}
impl<I> PendingBlock<I>
where
    I: IndexInput,
{
    pub fn new(
        prefix: BytesRef<Vec<u8>>,
        fp: i64,
        has_terms: bool,
        is_floor: bool,
        floor_lead_byte: i32,
        sub_indices: Vec<FST<BytesRef<Rc<Vec<u8>>>, ByteSequenceOutputs, OffHeapFSTStore<I>>>,
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
        &mut self,
        mut blocks: Vec<PendingBlock<I>>,
        scratch_bytes: &mut ByteBuffersDataOutput,
        scratch_ints_ref: &mut IntsRefBuilder<Vec<i32>>,
        version: i32,
    ) -> Result<()> {
        debug_assert!(
            (self.is_floor && blocks.len() > 1) || (!self.is_floor && blocks.len() == 1),
            "is_floor={}, blocks.len()={}",
            self.is_floor,
            blocks.len()
        );
        debug_assert!(std::ptr::eq(self, &blocks[0]));
        debug_assert_eq!(scratch_bytes.size(), 0);

        let output = lucene90_bttw_util::encode_output(self.fp, self.has_terms, self.is_floor);
        if version >= lucene90_bttr_util::VERSION_MSB_VLONG_OUTPUT {
            lucene90_bttw_util::write_msb_vlong(scratch_bytes, output)?;
        } else {
            scratch_bytes.write_vlong(output)?;
        }

        if self.is_floor {
            debug_assert!((blocks.len() - 1) <= i32::MAX as usize);
            scratch_bytes.write_vint((blocks.len() - 1) as i32)?;
            for block in &blocks[1..] {
                debug_assert!(block.floor_lead_byte != -1);
                scratch_bytes.write_byte(block.floor_lead_byte as u8)?;
                debug_assert!(block.fp > self.fp);
                let delta_fp = ((block.fp - self.fp) << 1) | if block.has_terms { 1 } else { 0 };
                scratch_bytes.write_vlong(delta_fp)?;
            }
        }

        let mut estimate_size = self.prefix.length as i64;
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

        Util::get_ints_ref(&self.prefix, scratch_ints_ref);
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

        self.index = Some(
            FST::from_fst_reader(
                fst_compiler.compile()?,
                Some(fst_compiler.inner.borrow_mut().get_fst_reader()?),
            )
            .unwrap(),
        );

        debug_assert!(self.sub_indices.is_empty());

        Ok(())
    }
    fn append(
        &self,
        fst_compiler: &mut FSTCompiler<BytesRef<Rc<Vec<u8>>>, ByteSequenceOutputs, DummyDirectory>,
        sub_index: FST<BytesRef<Rc<Vec<u8>>>, ByteSequenceOutputs, OffHeapFSTStore<I>>,
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
pub struct TermsWriter<I: IndexInput> {
    pub field_info: FieldInfo,
    pub num_terms: i64,
    pub docs_seen: FixedBitSet,
    pub sum_total_term_freq: i64,
    pub sum_doc_freq: i64,
    // Records index into pending where the current prefix at that
    // length "started"; for example, if current term starts with 't',
    // startsByPrefix[0] is the index into pending for the first
    // term/sub-block starting with 't'.  We use this to figure out when
    // to write a new block:
    pub last_term: BytesRefBuilder<Vec<u8>>,
    pub prefix_starts: Vec<usize>,
    // Pending stack of terms and blocks.  As terms arrive (in sorted order)
    // we append to this stack, and once the top of the stack has enough
    // terms starting with a common prefix, we write a new block with
    // those terms and replace those terms in the stack with a new block:
    pub pending: Vec<PendingEntryEnum<I>>,
    // Reused in writeBlocks:
    pub new_blocks: Vec<PendingBlock<I>>,

    pub first_pending_term: Option<PendingTerm>,
    pub last_pending_term: Option<PendingTerm>,

    pub suffix_lengths_writer: ByteBuffersDataOutput,
    pub suffix_writer: BytesRefBuilder<Vec<u8>>,
    pub stats_writer: ByteBuffersDataOutput,
    pub meta_writer: ByteBuffersDataOutput,
    pub spare_writer: ByteBuffersDataOutput,
    pub spare_bytes: Vec<u8>,
    pub compression_hash_table: lz4::HighCompressionHashTable,
}
impl<I: IndexInput> TermsWriter<I>
where
    I: IndexInput,
{
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
    ) -> Result<PendingBlock<I>>
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
        // let mut sub_indices = Vec::new();
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
                        // sub_indices.push(block.index.clone().unwrap());
                    },
                }
            }
            stats_writer.finish(terms_out)?;
            // debug_assert!(!sub_indices.is_empty());
        }
        todo!()
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

enum PendingEntryEnum<I>
where
    I: IndexInput,
{
    Term(PendingTerm),
    Block(PendingBlock<I>),
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
