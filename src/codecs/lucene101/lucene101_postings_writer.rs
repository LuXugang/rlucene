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
use crate::codecs::competitive_impact_accumulator::CompetitiveImpactAccumulator;
use crate::codecs::lucene101::for_delta_util::ForDeltaUtil;
use crate::codecs::lucene101::lucene101_postings_format::{
    IntBlockTermState, Lucene101PostingsFormat,
};
use crate::codecs::lucene101::pfor_util::PForUtil;
use crate::codecs::lucene101::postings_util::PostingsUtil;
use crate::codecs::norms_producer::NormsProducer;
use crate::codecs::push_postings_writer_base::PushPostingsWriterBase;
use crate::codecs::CodecUtil;
use crate::index::field_info::FieldInfo;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::terms_enum::TermsEnum;
use crate::index::IndexFileNames;
use crate::store::directory::Directory;
use crate::store::{ByteBuffersDataOutput, DataOutput, IndexOutput};
use crate::util::error::lucene_error::{LuceneError, Result};
use std::default::Default;
use crate::util::bit_util::BitUtil;

/// Writer for [`Lucene101PostingsFormat`](crate::codecs::lucene101::lucene101_postings_format)
pub struct Lucene101PostingsWriter<O, T, N>
where
    O: IndexOutput,
    T: TermsEnum,
    N: NormsProducer,
{
    pub(crate) base: PushPostingsWriterBase<T>,
    pub(crate) meta_out: O,
    pub(crate) doc_out: O,
    pub(crate) pos_out: Option<O>,
    pub(crate) pay_out: Option<O>,
    pub(crate) last_state: IntBlockTermState,
    /// Holds starting file pointers for current term:
    doc_start_fp: i64,
    pos_start_fp: i64,
    pay_start_fp: i64,

    pub(crate) doc_delta_buffer: Vec<i32>,
    pub(crate) freq_buffer: Vec<i32>,
    doc_buffer_upto: i32,

    pub(crate) pos_delta_buffer: Vec<i32>,
    pub(crate) payload_length_buffer: Vec<i32>,
    pub(crate) offset_start_delta_buffer: Vec<i32>,
    pub(crate) offset_length_buffer: Vec<i32>,
    pos_buffer_upto: i32,

    payload_bytes: Vec<u8>,
    payload_byte_upto: i32,

    level0_last_doc_id: i32,
    level0_last_pos_fp: i64,
    level0_last_pay_fp: i64,

    level1_last_doc_id: i32,
    level1_last_pos_fp: i64,
    level1_last_pay_fp: i64,

    doc_id: i32,
    last_doc_id: i32,
    last_position: i32,
    last_start_offset: i32,
    doc_count: i32,

    pfor_util: PForUtil,
    for_delta_util: ForDeltaUtil,

    field_has_norms: bool,
    norms: Option<N::NumericDocValues>,
    level0_freq_norm_accumulator: CompetitiveImpactAccumulator,
    level1_competitive_freq_norm_accumulator: CompetitiveImpactAccumulator,

    max_num_impacts_at_level0: i32,
    max_impact_num_bytes_at_level0: i32,
    max_num_impacts_at_level1: i32,
    max_impact_num_bytes_at_level1: i32,
    /// Scratch output that we use to be able to prepend the encoded length, e.g. impacts.
    scratch_output: ByteBuffersDataOutput,
    /// Output for a single block. This is useful to be able to prepend skip data before each block,
    /// which can only be computed once the block is encoded. The content is then typically copied to
    /// [`level1Output`].
    level0_output: ByteBuffersDataOutput,
    /// Output for groups of 32 blocks. This is useful to prepend skip data for these 32 blocks, which
    /// can only be done once we have encoded these 32 blocks. The content is then typically copied to
    /// [`docCount`].
    level1_output: ByteBuffersDataOutput,
}
impl<O, T, N> Lucene101PostingsWriter<O, T, N>
where
    O: IndexOutput,
    T: TermsEnum,
    N: NormsProducer,
{
    pub fn new<D>(state: &SegmentWriteState<D>) -> Result<Self>
    where
        D: Directory<IndexOutputType = O>,
    {
        let meta_file = IndexFileNames::segment_file_name(
            &state.segment_info.name,
            &state.segment_suffix,
            Lucene101PostingsFormat::META_EXTENSION,
        );
        let doc_file = IndexFileNames::segment_file_name(
            &state.segment_info.name,
            &state.segment_suffix,
            Lucene101PostingsFormat::DOC_EXTENSION,
        );

        let mut directory = state
            .directory
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire  lock.".to_string()))?;
        let mut meta_out = directory.create_output(&meta_file, &state.context)?;
        let mut doc_out = directory.create_output(&doc_file, &state.context)?;
        CodecUtil::write_index_header(
            &mut meta_out,
            Lucene101PostingsFormat::META_CODEC,
            Lucene101PostingsFormat::VERSION_CURRENT,
            &state.segment_info.get_id(),
            &state.segment_suffix,
        )?;
        CodecUtil::write_index_header(
            &mut doc_out,
            Lucene101PostingsFormat::DOC_CODEC,
            Lucene101PostingsFormat::VERSION_CURRENT,
            &state.segment_info.get_id(),
            &state.segment_suffix,
        )?;

        let for_delta_util = ForDeltaUtil::new();
        let pfor_util = PForUtil::new();

        let mut pos_out: Option<O> = None;
        let mut pay_out: Option<O> = None;

        let mut pos_delta_buffer = Vec::new();
        let mut payload_length_buffer = Vec::new();
        let mut offset_start_delta_buffer = Vec::new();
        let mut offset_length_buffer = Vec::new();
        let mut payload_bytes = Vec::new();

        if state.field_infos.has_prox() {
            let pos_file = IndexFileNames::segment_file_name(
                &state.segment_info.name,
                &state.segment_suffix,
                Lucene101PostingsFormat::POS_EXTENSION,
            );
            let mut pos_out_opt = directory.create_output(&pos_file, &state.context)?;
            CodecUtil::write_index_header(
                &mut pos_out_opt,
                Lucene101PostingsFormat::POS_CODEC,
                Lucene101PostingsFormat::VERSION_CURRENT,
                &state.segment_info.get_id(),
                &state.segment_suffix,
            )?;
            pos_out = Some(pos_out_opt);
            pos_delta_buffer = vec![0; Lucene101PostingsFormat::BLOCK_SIZE];

            if state.field_infos.has_payloads() {
                payload_bytes = vec![0; 128];
                payload_length_buffer = vec![0; Lucene101PostingsFormat::BLOCK_SIZE];
            }

            if state.field_infos.has_offsets() {
                offset_start_delta_buffer = vec![0; Lucene101PostingsFormat::BLOCK_SIZE];
                offset_length_buffer = vec![0; Lucene101PostingsFormat::BLOCK_SIZE];
            }

            if state.field_infos.has_payloads() || state.field_infos.has_offsets() {
                let pay_file = IndexFileNames::segment_file_name(
                    &state.segment_info.name,
                    &state.segment_suffix,
                    Lucene101PostingsFormat::PAY_EXTENSION,
                );
                let mut pay_out_opt = directory.create_output(&pay_file, &state.context)?;
                CodecUtil::write_index_header(
                    &mut pay_out_opt,
                    Lucene101PostingsFormat::PAY_CODEC,
                    Lucene101PostingsFormat::VERSION_CURRENT,
                    &state.segment_info.get_id(),
                    &state.segment_suffix,
                )?;
                pay_out = Some(pay_out_opt);
            }
        }

        let doc_delta_buffer = vec![0; Lucene101PostingsFormat::BLOCK_SIZE];
        let freq_buffer = vec![0; Lucene101PostingsFormat::BLOCK_SIZE];

        Ok(Self {
            base: PushPostingsWriterBase::new(FieldInfo::default()),
            meta_out,
            doc_out,
            pos_out,
            pay_out,
            last_state: IntBlockTermState::default(),
            doc_start_fp: 0,
            pos_start_fp: 0,
            pay_start_fp: 0,
            doc_delta_buffer,
            freq_buffer,
            doc_buffer_upto: 0,
            pos_delta_buffer,
            payload_length_buffer,
            offset_start_delta_buffer,
            offset_length_buffer,
            pos_buffer_upto: 0,
            payload_bytes,
            payload_byte_upto: 0,
            level0_last_doc_id: 0,
            level0_last_pos_fp: 0,
            level0_last_pay_fp: 0,
            level1_last_doc_id: 0,
            level1_last_pos_fp: 0,
            level1_last_pay_fp: 0,
            doc_id: 0,
            last_doc_id: 0,
            last_position: 0,
            last_start_offset: 0,
            doc_count: 0,
            pfor_util,
            for_delta_util,
            field_has_norms: false,
            norms: None,
            level0_freq_norm_accumulator: CompetitiveImpactAccumulator::new(),
            level1_competitive_freq_norm_accumulator: CompetitiveImpactAccumulator::new(),
            max_num_impacts_at_level0: 0,
            max_impact_num_bytes_at_level0: 0,
            max_num_impacts_at_level1: 0,
            max_impact_num_bytes_at_level1: 0,
            scratch_output: ByteBuffersDataOutput::with_resettable_instance(),
            level0_output: ByteBuffersDataOutput::with_resettable_instance(),
            level1_output: ByteBuffersDataOutput::with_resettable_instance(),
        })
    }
    fn flush_doc_block(&mut self, finish_term: bool) -> Result<()> {
        debug_assert!(self.doc_buffer_upto != 0);

        if (self.doc_buffer_upto as usize) < Lucene101PostingsFormat::BLOCK_SIZE {
            debug_assert!(finish_term);
            PostingsUtil::write_vint_block(
                &mut self.level0_output,
                &mut self.doc_delta_buffer,
                &self.freq_buffer,
                self.doc_buffer_upto ,
                self.base.write_freqs,
            )?;
        } else {
            if self.base.write_freqs {
                let impacts = self
                    .level0_freq_norm_accumulator
                    .get_competitive_freq_norm_pairs();
                let n = impacts.len() as i32;
                if n > self.max_num_impacts_at_level0 {
                    self.max_num_impacts_at_level0 = n;
                }
                lucene101_pw_util::write_impacts(&impacts, &mut self.scratch_output)?;
                debug_assert!(self.level0_output.size()  == 0);
                let scratch_len = self.scratch_output.size();
                if scratch_len> self.max_impact_num_bytes_at_level0 as i64{
                    self.max_impact_num_bytes_at_level0 = scratch_len.try_into()?;
                }
                self.level0_output.write_vlong(scratch_len )?;
                self.scratch_output.copy_to(&mut self.level0_output)?;
                self.scratch_output.reset();

                if self.base.write_positions {
                    let pos_out = self.pos_out.as_ref().unwrap();
                    self.level0_output
                        .write_vlong(pos_out.get_file_pointer() - self.level0_last_pos_fp)?;
                    self.level0_output.write_byte(self.pos_buffer_upto as u8)?;
                    self.level0_last_pos_fp = pos_out.get_file_pointer();

                    if self.base.write_offsets || self.base.write_payloads {
                        let pay_out = self.pay_out.as_ref().unwrap();
                        self.level0_output
                            .write_vlong(pay_out.get_file_pointer() - self.level0_last_pay_fp)?;
                        self.level0_output.write_vint(self.payload_byte_upto)?;
                        self.level0_last_pay_fp = pay_out.get_file_pointer();
                    }
                }
            }

            let mut num_skip_bytes = self.level0_output.size();
            self.for_delta_util
                .encode_deltas(&mut self.doc_delta_buffer, &mut self.level0_output)?;
            if self.base.write_freqs {
                self.pfor_util
                    .encode(&mut self.freq_buffer, &mut self.level0_output)?;
            }
            // docID - lastBlockDocID is at least 128, so it can never fit a single byte with a vint
            // Even if we subtracted 128, only extremely dense blocks would be eligible to a single byte
            // so let's go with 2 bytes right away
            lucene101_pw_util::write_vint15(
                &mut self.scratch_output,
                self.doc_id - self.level0_last_doc_id,
            )?;
            lucene101_pw_util::write_vlong15(&mut self.scratch_output, self.level0_output.size())?;
            num_skip_bytes += self.scratch_output.size() ;

            self.level1_output.write_vlong(num_skip_bytes)?;
            self.scratch_output.copy_to(&mut self.level1_output)?;
            self.scratch_output.reset();
        }

        self.level0_output.copy_to(&mut self.level1_output)?;
        self.level0_output.reset();

        self.level0_last_doc_id = self.doc_id;
        if self.base.write_freqs {
            self.level1_competitive_freq_norm_accumulator
                .add_all(&self.level0_freq_norm_accumulator);
            self.level0_freq_norm_accumulator.clear();
        }

        if (self.doc_count & Lucene101PostingsFormat::LEVEL1_MASK) == 0 {// true every 32 blocks (4,096 docs)
            self.write_level1_skip_data()?;
            self.level1_last_doc_id = self.doc_id;
            self.level1_competitive_freq_norm_accumulator.clear();
        } else if finish_term {
            self.level1_output.copy_to(&mut self.doc_out)?;
            self.level1_output.reset();
            self.level1_competitive_freq_norm_accumulator.clear();
        }

        Ok(())
    }
    fn write_level1_skip_data(&mut self) -> Result<()> {
        self.doc_out
            .write_vint((self.doc_id - self.level1_last_doc_id))?;
        let level1_end: i64;

        if self.base.write_freqs {
            let impacts = self
                .level1_competitive_freq_norm_accumulator
                .get_competitive_freq_norm_pairs();
            let n = impacts.len() as i32;
            if n > self.max_num_impacts_at_level1 {
                self.max_num_impacts_at_level1 = n;
            }
            lucene101_pw_util::write_impacts(&impacts, &mut self.scratch_output)?;
            let num_impact_bytes = self.scratch_output.size() ;
            if num_impact_bytes > self.max_impact_num_bytes_at_level1 as i64{
                self.max_impact_num_bytes_at_level1 = num_impact_bytes.try_into()?;
            }
            if self.base.write_positions {
                let pos_fp = self.pos_out.as_ref().unwrap().get_file_pointer();
                self.scratch_output.write_vlong(pos_fp - self.level1_last_pos_fp)?;
                self.scratch_output.write_byte(self.pos_buffer_upto as u8)?;
                self.level1_last_pos_fp = pos_fp;
                if self.base.write_offsets || self.base.write_payloads {
                    let pay_fp = self.pay_out.as_ref().unwrap().get_file_pointer();
                    self.scratch_output.write_vlong(pay_fp - self.level1_last_pay_fp)?;
                    self.scratch_output.write_vint(self.payload_byte_upto)?;
                    self.level1_last_pay_fp = pay_fp;
                }
            }
            let level1_len = 2 * BitUtil::SHORT_BYTES as i64
                + self.scratch_output.size() 
                + self.level1_output.size() ;
            self.doc_out.write_vlong(level1_len)?;
            level1_end = self.doc_out.get_file_pointer() + level1_len;
            // There are at most 128 impacts, that require at most 2 bytes each
            debug_assert!(self.scratch_output.size()<= i16::MAX as i64);
            // Like impacts plus a few vlongs, still way under the max short value
            debug_assert!(
                (self.scratch_output.size() + BitUtil::SHORT_BYTES as i64 )
                    <= i16::MAX as i64
            );
            self.doc_out
                .write_short((self.scratch_output.size() + BitUtil::SHORT_BYTES as i64) as i16)?;
            self.doc_out
                .write_short(self.scratch_output.size() as i16)?;
            self.scratch_output.copy_to(&mut self.doc_out)?;
            self.scratch_output.reset();
        } else {
            self.doc_out
                .write_vlong(self.level1_output.size() )?;
            level1_end = self.doc_out.get_file_pointer() + self.level1_output.size() ;
        }

        self.level1_output.copy_to(&mut self.doc_out)?;
        self.level1_output.reset();
        debug_assert_eq!(self.doc_out.get_file_pointer(), level1_end);
        Ok(())
    }
}

pub mod lucene101_pw_util {
    use crate::index::impact::Impact;
    use crate::store::DataOutput;
    use crate::util::error::lucene_error::Result;

    /// Special vints that are encoded on 2 bytes if they require 15 bits or less.
    /// VInt becomes especially slow when the number of bytes is variable, so this
    /// special layout helps in the case when the number likely requires 15 bits or less.
    pub(crate) fn write_vint15(out: &mut impl DataOutput, v: i32) -> Result<()> {
        debug_assert!(v >= 0);
        write_vlong15(out, v as i64)
    }

    /// @see [`write_vint15`]
    pub(crate) fn write_vlong15(out: &mut impl DataOutput, v: i64) -> Result<()> {
        debug_assert!(v >= 0);
        if v & !0x7FFF == 0 {
            out.write_short(v as i16)?;
        } else {
            let prefix = 0x8000 | (v & 0x7FFF);
            out.write_short(prefix as i16)?;
            out.write_vlong(v >> 15)?;
        }
        Ok(())
    }
    pub(crate) fn write_impacts(impacts: &[Impact], out: &mut impl DataOutput) -> Result<()> {
        let mut previous = Impact { freq: 0, norm: 0 };
        for impact in impacts {
            debug_assert!(impact.freq > previous.freq);
            debug_assert!((impact.norm as u64) > (previous.norm as u64));
            let freq_delta = impact.freq - previous.freq - 1;
            let norm_delta = impact.norm - previous.norm - 1;
            if norm_delta == 0 {
                // most of time, norm only increases by 1, so we can fold everything in a single byte
                out.write_vint(freq_delta << 1)?;
            } else {
                out.write_vint((freq_delta << 1) | 1)?;
                out.write_zlong(norm_delta)?;
            }
            previous = impact.clone();
        }
        Ok(())
    }
}
