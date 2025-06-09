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
use crate::codecs::compression::compression_mode::{CompressionModeEnum, CompressorEnum};
use crate::codecs::lucene90::fields_index_writer::FieldsIndexWriter;
use crate::index::BytesRef;
use crate::store::directory::Directory;
use crate::store::{ByteBuffersDataOutput, DataOutput};
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::packed::abstract_block_packed_writer::AbstractBlockPackedWriter;
use crate::util::packed::block_packed_writer::BlockPackedWriter;
use crate::util::packed::direct_writer::{direct_writer_util, DirectWriter};
use crate::util::packed::Format::Packed;
use crate::util::packed::{PackedImpl, PackedInts, Writer};
use once_cell::sync::Lazy;
use std::collections::{HashSet, VecDeque};

pub(crate) static FLAGS_BITS: Lazy<i32> = Lazy::new(|| {
    PackedInts::bits_required(
        (lucene90_ctvw_util::POSITIONS | lucene90_ctvw_util::OFFSETS | lucene90_ctvw_util::PAYLOADS)
            as i64,
    )
    .unwrap()
});

pub struct Lucene90CompressingTermVectorsWriter<D>
where
    D: Directory,
{
    segment: String,
    index_writer: FieldsIndexWriter<D>,
    meta_stream: D::IndexOutputType,
    vectors_stream: D::IndexOutputType,
    compression_mode: CompressionModeEnum,
    compressor: CompressorEnum,
    chunk_size: i32,
    // number of chunks
    num_chunks: i64,
    // number of incomplete compressed blocks written
    num_dirty_chunks: i64,
    // cumulative number of docs in incomplete chunks
    num_dirty_docs: i64,

    // total number of docs seen
    num_docs: i32,
    // pending docs
    pending_docs: VecDeque<DocData>,
    // current document
    cur_doc: usize,
    // current field
    cur_field: usize,
    last_term: BytesRef<Vec<u8>>,

    positions_buf: Vec<i32>,
    start_offsets_buf: Vec<i32>,
    lengths_buf: Vec<i32>,
    payload_lengths_buf: Vec<i32>,
    // buffered term suffixes
    term_suffixes: ByteBuffersDataOutput,
    // buffered term payloads
    payload_bytes: ByteBuffersDataOutput,
    writer: AbstractBlockPackedWriter<BlockPackedWriter>,
    max_docs_per_chunk: i32,
    scratch_buffer: ByteBuffersDataOutput,
}
impl<D> Lucene90CompressingTermVectorsWriter<D>
where
    D: Directory,
{
    fn add_doc_data(&mut self, num_vector_fields: i32) -> usize {
        let mut last: Option<&FieldData> = None;

        for doc in self.pending_docs.iter().rev() {
            if let Some(field) = doc.fields.back() {
                last = Some(field);
                break;
            }
        }

        let (pos_start, off_start, pay_start) = if let Some(last_field) = last {
            let total = last_field.total_positions;
            let pos_start = last_field.pos_start + if last_field.has_positions { total } else { 0 };
            let off_start = last_field.off_start + if last_field.has_offsets { total } else { 0 };
            let pay_start = last_field.pay_start + if last_field.has_payloads { total } else { 0 };
            (pos_start, off_start, pay_start)
        } else {
            (0, 0, 0)
        };

        let doc = DocData::new(num_vector_fields, pos_start, off_start, pay_start);

        let index = self.pending_docs.len();
        self.pending_docs.push_back(doc);
        index
    }
    fn trigger_flush(&self) -> bool {
        self.term_suffixes.size() >= self.chunk_size as i64
            || self.pending_docs.len() >= self.max_docs_per_chunk as usize
    }
    fn flush_num_fields(&mut self, chunk_docs: usize) -> Result<i32> {
        if chunk_docs == 1 {
            let num_fields = self.pending_docs.front().unwrap().num_fields;
            self.vectors_stream.write_vint(num_fields)?;
            Ok(num_fields)
        } else {
            self.writer.reset();
            let mut total_fields = 0;
            for doc in &self.pending_docs {
                self.writer
                    .add(doc.num_fields as i64, &mut self.vectors_stream)?;
                total_fields += doc.num_fields;
            }
            self.writer.finish(&mut self.vectors_stream)?;
            Ok(total_fields)
        }
    }
    /// Returns a sorted array containing unique field numbers
    pub(crate) fn flush_field_nums(&mut self) -> Result<Vec<i32>> {
        // 1. Collect unique field numbers
        let mut field_nums_set = HashSet::new();
        for doc in &self.pending_docs {
            for field in &doc.fields {
                field_nums_set.insert(field.field_num);
            }
        }

        let mut field_nums: Vec<i32> = field_nums_set.into_iter().collect();
        field_nums.sort_unstable();

        let num_distinct_fields = field_nums.len();
        assert!(num_distinct_fields > 0);

        let max_field_num = field_nums[num_distinct_fields - 1];
        let bits_required = PackedInts::bits_required(max_field_num as i64)?;

        let token = ((num_distinct_fields - 1).min(0x07) << 5) as u8 | (bits_required as u8);
        self.vectors_stream.write_byte(token)?;

        if num_distinct_fields > 0x07 {
            self.vectors_stream
                .write_vint((num_distinct_fields - 1 - 0x07) as i32)?;
        }
        debug_assert!(num_distinct_fields <= i32::MAX as usize);
        let mut writer = PackedInts::get_writer_no_header(
            &mut self.vectors_stream,
            Packed(PackedImpl::new(0)),
            num_distinct_fields as i32,
            bits_required,
            1,
        );

        for &field_num in &field_nums {
            writer.add(field_num as i64)?;
        }
        writer.finish()?;

        Ok(field_nums)
    }
    fn flush_fields(&mut self, total_fields: i32, field_nums: &[i32]) -> Result<()> {
        self.scratch_buffer.reset();

        let bits_required = direct_writer_util::bits_required((field_nums.len() - 1) as i64)?;
        let mut writer = DirectWriter::get_instance(
            &mut self.scratch_buffer,
            total_fields as i64,
            bits_required,
        )?;

        for doc in &self.pending_docs {
            for field in &doc.fields {
                let field_num = field.field_num;
                let field_index = match field_nums.binary_search(&field_num) {
                    Ok(index) => index,
                    Err(_) => {
                        return Err(LuceneError::illegal_state(format!(
                            "Field number {} not found in field_nums",
                            field_num
                        )));
                    },
                };
                writer.add(field_num as i64)?
            }
        }

        writer.finish()?;

        self.vectors_stream
            .write_vlong(self.scratch_buffer.size())?;
        self.scratch_buffer.copy_to(&mut self.vectors_stream)?;

        Ok(())
    }
    fn flush_flags(&mut self, total_fields: i32, field_nums: &[i32]) -> Result<()> {
        // check if fields always have the same flags
        let mut non_changing_flags = true;
        let mut field_flags = vec![-1; field_nums.len()];

        'outer: for doc in &self.pending_docs {
            for fd in &doc.fields {
                let field_num = fd.field_num;
                let field_index = match field_nums.binary_search(&field_num) {
                    Ok(index) => index,
                    Err(_) => {
                        return Err(LuceneError::illegal_state(format!(
                            "field_num {} not found in field_nums",
                            field_num
                        )));
                    },
                };

                if field_flags[field_index] == -1 {
                    field_flags[field_index] = fd.flags;
                } else if field_flags[field_index] != fd.flags {
                    non_changing_flags = false;
                    break 'outer;
                }
            }
        }

        self.scratch_buffer.reset();

        if non_changing_flags {
            // write one flag per field num
            self.vectors_stream.write_vint(0)?;
            let mut writer = DirectWriter::get_instance(
                &mut self.scratch_buffer,
                field_flags.len() as i64,
                *FLAGS_BITS,
            )?;

            for &flags in &field_flags {
                debug_assert!(flags >= 0);
                writer.add(flags as i64)?;
            }

            writer.finish()?;

            self.vectors_stream
                .write_vint(self.scratch_buffer.size().try_into()?)?;
            self.scratch_buffer.copy_to(&mut self.vectors_stream)?;
        } else {
            // write one flag for every field instance
            self.vectors_stream.write_vint(1)?;
            let mut writer = DirectWriter::get_instance(
                &mut self.scratch_buffer,
                total_fields as i64,
                *FLAGS_BITS,
            )?;

            for doc in &self.pending_docs {
                for field in &doc.fields {
                    writer.add(field.flags as i64)?;
                }
            }

            writer.finish()?;

            self.vectors_stream
                .write_vint(self.scratch_buffer.size() as i32)?;
            self.scratch_buffer.copy_to(&mut self.vectors_stream)?;
        }

        Ok(())
    }
    fn flush_num_terms(&mut self, total_fields: i32) -> Result<()> {
        let mut max_num_terms = 0;
        for doc in &self.pending_docs {
            for field in &doc.fields {
                max_num_terms |= field.num_terms;
            }
        }

        let bits_required = direct_writer_util::bits_required(max_num_terms as i64)?;
        self.vectors_stream.write_vint(bits_required)?;

        self.scratch_buffer.reset();

        let mut writer = DirectWriter::get_instance(
            &mut self.scratch_buffer,
            total_fields as i64,
            bits_required,
        )?;

        for doc in &self.pending_docs {
            for field in &doc.fields {
                writer.add(field.num_terms as i64)?;
            }
        }

        writer.finish()?;

        self.vectors_stream
            .write_vint(self.scratch_buffer.size().try_into()?)?;
        self.scratch_buffer.copy_to(&mut self.vectors_stream)?;

        Ok(())
    }
    fn flush_term_lengths(&mut self) -> Result<()> {
        self.writer.reset();

        for doc in &self.pending_docs {
            for field in &doc.fields {
                for i in 0..field.num_terms {
                    self.writer
                        .add(field.prefix_lengths[i] as i64, &mut self.vectors_stream)?;
                }
            }
        }

        self.writer.finish(&mut self.vectors_stream)?;

        self.writer.reset();

        for doc in &self.pending_docs {
            for field in &doc.fields {
                for i in 0..field.num_terms {
                    self.writer
                        .add(field.suffix_lengths[i] as i64, &mut self.vectors_stream)?;
                }
            }
        }

        self.writer.finish(&mut self.vectors_stream)
    }
    fn flush_term_freqs(&mut self) -> Result<()> {
        self.writer.reset();

        for doc in &self.pending_docs {
            for field in &doc.fields {
                for i in 0..field.num_terms {
                    self.writer
                        .add((field.freqs[i] - 1) as i64, &mut self.vectors_stream)?;
                }
            }
        }

        self.writer.finish(&mut self.vectors_stream)
    }
    fn flush_positions(&mut self) -> Result<()> {
        self.writer.reset();

        for doc in &self.pending_docs {
            for field in &doc.fields {
                if field.has_positions {
                    let mut pos = 0;
                    for i in 0..field.num_terms {
                        let freq = field.freqs[i];
                        let mut previous_position = 0;
                        for _ in 0..freq {
                            let index = field.pos_start + pos;
                            let position = self.positions_buf[index];
                            self.writer.add(
                                (position - previous_position) as i64,
                                &mut self.vectors_stream,
                            )?;
                            previous_position = position;
                            pos += 1;
                        }
                    }
                    debug_assert_eq!(pos, field.total_positions);
                }
            }
        }

        self.writer.finish(&mut self.vectors_stream)
    }
    fn flush_offsets(&mut self, field_nums: &[i32]) -> Result<()> {
        let mut has_offsets = false;
        let mut sum_pos = vec![0u64; field_nums.len()];
        let mut sum_offsets = vec![0u64; field_nums.len()];

        for doc in &self.pending_docs {
            for field in &doc.fields {
                has_offsets |= field.has_offsets;
                if field.has_offsets && field.has_positions {
                    let field_index = match field_nums.binary_search(&field.field_num) {
                        Ok(idx) => idx,
                        Err(_) => {
                            return Err(LuceneError::illegal_state(format!(
                                "field_num {} not found in field_nums",
                                field.field_num
                            )))
                        },
                    };

                    let mut pos = 0;
                    for i in 0..field.num_terms {
                        let freq = field.freqs[i] as usize;
                        let position_idx = field.pos_start + pos + freq - 1;
                        let offset_idx = field.off_start + pos + freq - 1;
                        sum_pos[field_index] += self.positions_buf[position_idx] as u64;
                        sum_offsets[field_index] += self.start_offsets_buf[offset_idx] as u64;
                        pos += freq;
                    }
                    debug_assert_eq!(pos, field.total_positions);
                }
            }
        }

        if !has_offsets {
            // nothing to do
            return Ok(());
        }

        let mut chars_per_term = vec![0f32; field_nums.len()];
        for i in 0..field_nums.len() {
            chars_per_term[i] = if sum_pos[i] == 0 || sum_offsets[i] == 0 {
                0.0
            } else {
                (sum_offsets[i] as f64 / sum_pos[i] as f64) as f32
            };
            // start offsets
            self.vectors_stream
                .write_int(chars_per_term[i].to_bits() as i32)?;
        }

        // start offsets
        self.writer.reset();

        for doc in &self.pending_docs {
            for field in &doc.fields {
                if field.flags & lucene90_ctvw_util::OFFSETS != 0 {
                    let field_num_off = match field_nums.binary_search(&field.field_num) {
                        Ok(idx) => idx,
                        Err(_) => {
                            return Err(LuceneError::illegal_state(format!(
                                "field_num {} not found",
                                field.field_num
                            )))
                        },
                    };
                    let cpt = chars_per_term[field_num_off];
                    let mut pos = 0;
                    for i in 0..field.num_terms {
                        let freq = field.freqs[i];
                        let mut previous_pos = 0;
                        let mut previous_off = 0;
                        for _ in 0..freq {
                            let position = if field.has_positions {
                                self.positions_buf[field.pos_start + pos]
                            } else {
                                0
                            };
                            let start_offset = self.start_offsets_buf[field.off_start + pos];
                            let predicted = (cpt * (position - previous_pos) as f32) as i32;
                            self.writer.add(
                                (start_offset - previous_off - predicted) as i64,
                                &mut self.vectors_stream,
                            )?;
                            previous_pos = position;
                            previous_off = start_offset;
                            pos += 1;
                        }
                    }
                }
            }
        }

        self.writer.finish(&mut self.vectors_stream)?;

        // lengths
        self.writer.reset();

        for doc in &self.pending_docs {
            for field in &doc.fields {
                if field.flags & lucene90_ctvw_util::OFFSETS != 0 {
                    let mut pos = 0;
                    for i in 0..field.num_terms {
                        let prefix = field.prefix_lengths[i];
                        let suffix = field.suffix_lengths[i];
                        for _ in 0..field.freqs[i] {
                            let length = self.lengths_buf[field.off_start + pos];
                            self.writer.add(
                                (length as usize - prefix - suffix) as i64,
                                &mut self.vectors_stream,
                            )?;
                            pos += 1;
                        }
                    }
                    debug_assert_eq!(pos, field.total_positions);
                }
            }
        }

        self.writer.finish(&mut self.vectors_stream)
    }
    fn flush_payload_lengths(&mut self) -> Result<()> {
        self.writer.reset();

        for doc in &self.pending_docs {
            for field in &doc.fields {
                if field.has_payloads {
                    for i in 0..field.total_positions {
                        let value = self.payload_lengths_buf[field.pay_start + i];
                        self.writer.add(value as i64, &mut self.vectors_stream)?;
                    }
                }
            }
        }

        self.writer.finish(&mut self.vectors_stream)
    }
}

pub mod lucene90_ctvw_util {
    pub(crate) const VECTORS_EXTENSION: &str = "tvd";
    pub(crate) const VECTORS_INDEX_EXTENSION: &str = "tvx";
    pub(crate) const VECTORS_META_EXTENSION: &str = "tvm";
    pub(crate) const VECTORS_INDEX_CODEC_NAME: &str = "Lucene90TermVectorsIndex";

    pub(crate) const VERSION_START: i32 = 0;
    pub(crate) const VERSION_CURRENT: i32 = VERSION_START;
    pub(crate) const META_VERSION_START: i32 = 0;

    pub(crate) const PACKED_BLOCK_SIZE: i32 = 64;

    pub(crate) const POSITIONS: i32 = 0x01;
    pub(crate) const OFFSETS: i32 = 0x02;
    pub(crate) const PAYLOADS: i32 = 0x04;
}
/// a pending doc
pub(crate) struct DocData {
    pub num_fields: i32,
    pub fields: VecDeque<FieldData>,
    pub pos_start: usize,
    pub off_start: usize,
    pub pay_start: usize,
}
impl DocData {
    pub(crate) fn new(
        num_fields: i32,
        pos_start: usize,
        off_start: usize,
        pay_start: usize,
    ) -> Self {
        Self {
            num_fields,
            fields: VecDeque::with_capacity(num_fields as usize),
            pos_start,
            off_start,
            pay_start,
        }
    }
    fn add_field(
        &mut self,
        field_num: i32,
        num_terms: usize,
        positions: bool,
        offsets: bool,
        payloads: bool,
    ) -> usize {
        let (pos_start, off_start, pay_start) = if let Some(last) = self.fields.back() {
            let total = last.total_positions;
            let pos_start = last.pos_start + if last.has_positions { total } else { 0 };
            let off_start = last.off_start + if last.has_offsets { total } else { 0 };
            let pay_start = last.pay_start + if last.has_payloads { total } else { 0 };
            (pos_start, off_start, pay_start)
        } else {
            (self.pos_start, self.off_start, self.pay_start)
        };

        let field = FieldData::new(
            field_num, num_terms, positions, offsets, payloads, pos_start, off_start, pay_start,
        );
        let index = self.fields.len();
        self.fields.push_back(field);
        index
    }
}
/// a pending field
pub(crate) struct FieldData {
    pub has_positions: bool,
    pub has_offsets: bool,
    pub has_payloads: bool,
    pub field_num: i32,
    pub flags: i32,
    pub num_terms: usize,
    pub freqs: Vec<i32>,
    pub prefix_lengths: Vec<usize>,
    pub suffix_lengths: Vec<usize>,
    pub pos_start: usize,
    pub off_start: usize,
    pub pay_start: usize,
    pub total_positions: usize,
    pub ord: usize,
}
impl FieldData {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        field_num: i32,
        num_terms: usize,
        positions: bool,
        offsets: bool,
        payloads: bool,
        pos_start: usize,
        off_start: usize,
        pay_start: usize,
    ) -> Self {
        let flags = (if positions {
            lucene90_ctvw_util::POSITIONS
        } else {
            0
        }) | (if offsets {
            lucene90_ctvw_util::OFFSETS
        } else {
            0
        }) | (if payloads {
            lucene90_ctvw_util::PAYLOADS
        } else {
            0
        });

        Self {
            has_positions: positions,
            has_offsets: offsets,
            has_payloads: payloads,
            field_num,
            flags,
            num_terms,
            freqs: vec![0; num_terms],
            prefix_lengths: vec![0; num_terms],
            suffix_lengths: vec![0; num_terms],
            pos_start,
            off_start,
            pay_start,
            total_positions: 0,
            ord: 0,
        }
    }
    pub(crate) fn add_term(&mut self, freq: i32, prefix_length: usize, suffix_length: usize) {
        self.freqs[self.ord] = freq;
        self.prefix_lengths[self.ord] = prefix_length;
        self.suffix_lengths[self.ord] = suffix_length;
        self.ord += 1;
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_position(
        &mut self,
        position: i32,
        start_offset: i32,
        length: i32,
        payload_length: i32,
        positions_buf: &mut Vec<i32>,
        start_offsets_buf: &mut Vec<i32>,
        lengths_buf: &mut Vec<i32>,
        payload_lengths_buf: &mut Vec<i32>,
    ) -> Result<()> {
        if self.has_positions {
            let required = self.pos_start + self.total_positions;
            if required == positions_buf.len() {
                ArrayUtil::grow(positions_buf)?;
            }
            positions_buf[required] = position;
        }

        if self.has_offsets {
            let required = self.off_start + self.total_positions;
            if required == start_offsets_buf.len() {
                let new_len = ArrayUtil::oversize(required, 4);
                ArrayUtil::grow_exact(start_offsets_buf, new_len)?;
                ArrayUtil::grow_exact(lengths_buf, new_len)?;
            }
            start_offsets_buf[required] = start_offset;
            lengths_buf[required] = length;
        }

        if self.has_payloads {
            let required = self.pay_start + self.total_positions;
            if required == payload_lengths_buf.len() {
                ArrayUtil::grow(payload_lengths_buf)?;
            }
            payload_lengths_buf[required] = payload_length;
        }
        self.total_positions += 1;
        Ok(())
    }
}
