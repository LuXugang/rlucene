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
use crate::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::codecs::term_vectors_writer::TermVectorsWriter;
use crate::codecs::Codec;
use crate::document::fields::Fields;
use crate::index::field_info::FieldInfo;
use crate::index::field_invert_state::FieldInvertState;
use crate::index::index_options::IndexOptions;
use crate::index::indexable_field::IndexableField;
use crate::index::indexable_field_type::IndexableFieldType;
use crate::index::parallel_postings_array::{
    ParallelPostingsArray, PostingsArrayBase, PostingsArrayEnum,
};
use crate::index::term_vectors_consumer::TermVectorsConsumer;
use crate::index::terms_hash_per_field::{
    PostingsArrayWrapper, TermsHashPerField, TermsHashPerFieldBase, TermsHashPerFieldType,
};
use crate::store::directory::Directory;
use crate::util::array_util::ArrayUtil;
use crate::util::bit_util::BitUtil;
use crate::util::bytes_ref_block_pool::BytesRefBlockPool;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{ByteBlockPoolBorrow, CounterEnumBorrow};
use std::cmp::Ordering;
use std::rc::Rc;

pub(crate) struct TermVectorsConsumerPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    field_info: Rc<FieldInfo>,
    do_vectors: bool,
    do_vector_positions: bool,
    do_vector_offsets: bool,
    do_vector_payloads: bool,
    term_byte_pool: BytesRefBlockPool<CounterEnumBorrow, ByteBlockPoolBorrow>,
    has_payloads: bool,
    field_name: String,
    field_state: Rc<FieldInvertState<O, P, T>>,
    base: TermsHashPerField,
}
impl<O, P, T> Default for TermVectorsConsumerPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn default() -> Self {
        TermVectorsConsumerPerField {
            field_info: Rc::new(FieldInfo::default()),
            do_vectors: false,
            do_vector_positions: false,
            do_vector_offsets: false,
            do_vector_payloads: false,
            term_byte_pool: BytesRefBlockPool::default(),
            has_payloads: false,
            field_name: String::new(),
            field_state: Rc::new(FieldInvertState::default()),
            base: TermsHashPerField::default(),
        }
    }
}
impl<O, P, T> Clone for TermVectorsConsumerPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    // for padding
    fn clone(&self) -> Self {
        TermVectorsConsumerPerField {
            field_info: Rc::clone(&self.field_info),
            do_vectors: self.do_vectors,
            do_vector_positions: self.do_vector_positions,
            do_vector_offsets: self.do_vector_offsets,
            do_vector_payloads: self.do_vector_payloads,
            term_byte_pool: BytesRefBlockPool::default(),
            has_payloads: self.has_payloads,
            field_name: self.field_name.clone(),
            field_state: Rc::clone(&self.field_state),
            base: TermsHashPerField::default(),
        }
    }
}

impl<O, P, T> TermVectorsConsumerPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    pub(crate) fn new<D, C>(
        field_invert_state: Rc<FieldInvertState<O, P, T>>,
        terms_hash: &mut TermVectorsConsumer<D, C, O, P, T>,
        field_info: Rc<FieldInfo>,
    ) -> Self
    where
        D: Directory,
        C: Codec,
    {
        let postings_array_wrapper = PostingsArrayWrapper::new(TermsHashPerFieldType::TermVectors);
        let base = TermsHashPerField::new(
            2,
            terms_hash.base.int_pool.clone(),
            terms_hash.base.byte_pool.clone(),
            terms_hash.base.term_byte_pool.as_mut().unwrap().clone(),
            terms_hash.base.bytes_used.clone(),
            postings_array_wrapper,
            field_info.name.clone(),
            field_info.index_options,
        );
        let field_name = field_info.name.clone();
        Self {
            field_info,
            do_vectors: false,
            do_vector_positions: false,
            do_vector_offsets: false,
            do_vector_payloads: false,
            term_byte_pool: BytesRefBlockPool::from_byte_block_pool(
                terms_hash.base.term_byte_pool.as_mut().unwrap().clone(),
            ),
            has_payloads: false,
            field_name,
            field_state: field_invert_state,
            base,
        }
    }

    pub(crate) fn finish_document<D, C>(
        &mut self,
        term_vectors_consumer: &mut TermVectorsConsumer<D, C, O, P, T>,
    ) -> Result<()>
    where
        D: Directory,
        C: Codec,
    {
        if !self.do_vectors {
            return Ok(());
        }
        self.do_vectors = false;

        let num_postings = self.base.get_num_terms();
        debug_assert!(num_postings >= 0);

        let tv = term_vectors_consumer.writer.as_mut().unwrap();

        self.base.sort_terms()?;
        let term_ids = self.base.get_sorted_term_ids();

        tv.start_field(
            &self.field_info,
            num_postings as usize,
            self.do_vector_positions,
            self.do_vector_offsets,
            self.has_payloads,
        )?;

        let mut pos_reader = if self.do_vector_positions {
            Some(
                term_vectors_consumer
                    .vector_slice_reader_pos
                    .take()
                    .unwrap(),
            )
        } else {
            None
        };
        let mut off_reader = if self.do_vector_offsets {
            Some(
                term_vectors_consumer
                    .vector_slice_reader_off
                    .take()
                    .unwrap(),
            )
        } else {
            None
        };

        let postings_array_enum = self
            .base
            .bytes_hash
            .bytes_start_array
            .per_field
            .postings_array
            .as_ref()
            .expect("postings_array must be Some");
        match postings_array_enum {
            PostingsArrayEnum::TermVectors(postings) => {
                for &term_id in term_ids {
                    let freq = postings.freqs[term_id as usize];
                    self.term_byte_pool.fill_bytes_ref(
                        &mut term_vectors_consumer.flush_term,
                        postings.parent.text_starts[term_id as usize],
                    );

                    tv.start_term(&term_vectors_consumer.flush_term, freq)?;

                    if self.do_vector_positions || self.do_vector_offsets {
                        if pos_reader.is_some() {
                            self.base
                                .init_reader(pos_reader.as_mut().unwrap(), term_id, 0);
                        }
                        if off_reader.is_some() {
                            self.base
                                .init_reader(off_reader.as_mut().unwrap(), term_id, 1);
                        }
                        tv.add_prox(freq as usize, &mut pos_reader, &mut off_reader)?;
                    }

                    tv.finish_term()?;
                }
            },
            _ => unreachable!("Expected TermVectors postings"),
        }

        tv.finish_field()?;

        self.reset();
        self.field_info.set_store_term_vectors()?;
        term_vectors_consumer.vector_slice_reader_off = off_reader;
        term_vectors_consumer.vector_slice_reader_pos = pos_reader;
        Ok(())
    }
    pub(crate) fn reset(&mut self) {
        self.base.reset();
    }
    // Secondary entry point (for 2nd & subsequent TermsHash),
    // because token text has already been "interned" into
    // textStart, so we hash by textStart.  term vectors use
    // this API.
    pub(crate) fn add_with_text_start(&mut self, text_start: i32, doc_id: i32) -> Result<()> {
        let term_id = self.base.bytes_hash.add_by_pool_offset(text_start)?;
        if term_id >= 0 {
            // First time we are seeing this token since we last
            // flushed the hash.
            self.base.init_stream_slices(term_id, doc_id)?;
            self.new_term(term_id, doc_id)?;
        } else {
            self.base.position_stream_slice(term_id, doc_id)?;
            self.add_term(term_id, doc_id)?;
        }
        Ok(())
    }
    pub(crate) fn start(&mut self, field: &Fields, first: bool) -> Result<bool> {
        debug_assert!(*field.field_type().index_options() != IndexOptions::None);

        if first {
            if self.base.get_num_terms() != 0 {
                // Only necessary if previous doc hit a
                // non-aborting exception while writing vectors in
                // this field:
                self.base.reset();
            }

            self.base.reinit_hash();

            self.has_payloads = false;

            self.do_vectors = field.field_type().store_term_vectors();

            if self.do_vectors {
                self.do_vector_positions = field.field_type().store_term_vector_positions();
                // Somewhat confusingly, unlike postings, you are
                // allowed to index TV offsets without TV positions:
                self.do_vector_offsets = field.field_type().store_term_vector_offsets();

                if self.do_vector_positions {
                    self.do_vector_payloads = field.field_type().store_term_vector_payloads();
                } else {
                    self.do_vector_payloads = false;
                    if field.field_type().store_term_vector_payloads() {
                        return Err(LuceneError::illegal_argument(format!(
                            "cannot index term vector payloads without term vector positions (field=\"{}\")",
                            field.name()
                        )));
                    }
                }
            } else {
                if field.field_type().store_term_vector_offsets() {
                    return Err(LuceneError::illegal_argument(format!(
                        "cannot index term vector offsets when term vectors are not indexed (field=\"{}\")",
                        field.name()
                    )));
                }
                if field.field_type().store_term_vector_positions() {
                    return Err(LuceneError::illegal_argument(format!(
                        "cannot index term vector positions when term vectors are not indexed (field=\"{}\")",
                        field.name()
                    )));
                }
                if field.field_type().store_term_vector_payloads() {
                    return Err(LuceneError::illegal_argument(format!(
                        "cannot index term vector payloads when term vectors are not indexed (field=\"{}\")",
                        field.name()
                    )));
                }
            }
        } else {
            if self.do_vectors != field.field_type().store_term_vectors() {
                return Err(LuceneError::illegal_argument(format!(
                    "all instances of a given field name must have the same term vectors settings (storeTermVectors changed for field=\"{}\")",
                    field.name()
                )));
            }
            if self.do_vector_positions != field.field_type().store_term_vector_positions() {
                return Err(LuceneError::illegal_argument(format!(
                    "all instances of a given field name must have the same term vectors settings (storeTermVectorPositions changed for field=\"{}\")",
                    field.name()
                )));
            }
            if self.do_vector_offsets != field.field_type().store_term_vector_offsets() {
                return Err(LuceneError::illegal_argument(format!(
                    "all instances of a given field name must have the same term vectors settings (storeTermVectorOffsets changed for field=\"{}\")",
                    field.name()
                )));
            }
            if self.do_vector_payloads != field.field_type().store_term_vector_payloads() {
                return Err(LuceneError::illegal_argument(format!(
                    "all instances of a given field name must have the same term vectors settings (storeTermVectorPayloads changed for field=\"{}\")",
                    field.name()
                )));
            }
        }

        if self.do_vectors && self.do_vector_offsets {
            debug_assert!(self.field_state.off_set_attribute.is_some());
        }
        Ok(self.do_vectors)
    }
    pub(crate) fn write_prox(&mut self, term_id: usize) -> Result<()>
    where
        O: OffsetAttribute,
        P: PayloadAttribute,
        T: TermFrequencyAttribute,
    {
        let postings = self
            .base
            .bytes_hash
            .bytes_start_array
            .per_field
            .postings_array
            .as_ref()
            .unwrap();
        let mut last_offset = None;
        let mut last_position = None;
        match postings {
            PostingsArrayEnum::TermVectors(postings) => {
                if self.do_vector_offsets {
                    let offset_attr = self.field_state.off_set_attribute.as_ref().unwrap();
                    let start_offset = self.field_state.offset + offset_attr.start_offset();
                    let end_offset = self.field_state.offset + offset_attr.end_offset();

                    self.base
                        .write_vint(1, start_offset - postings.last_offsets[term_id])?;
                    self.base.write_vint(1, end_offset - start_offset)?;
                    last_offset = Some(end_offset);
                }

                if self.do_vector_positions {
                    let payload_attribute = &self.field_state.pay_load_attribute;

                    let pos = self.field_state.position - postings.last_positions[term_id];

                    if let Some(v) = payload_attribute {
                        let payload = v.get_payload();
                        if payload.length > 0 {
                            self.base.write_vint(0, (pos << 1) | 1)?;
                            self.base.write_vint(0, payload.length as i32)?;
                            self.base.write_bytes(
                                0,
                                &payload.bytes,
                                payload.offset,
                                payload.length,
                            )?;
                            self.has_payloads = true;
                        } else {
                            self.base.write_vint(0, pos << 1)?;
                        }
                    } else {
                        self.base.write_vint(0, pos << 1)?;
                    }

                    last_position = Some(self.field_state.position);
                }
            },
            _ => unreachable!("should not be here"),
        }
        let postings = self
            .base
            .bytes_hash
            .bytes_start_array
            .per_field
            .postings_array
            .as_mut()
            .unwrap();
        match postings {
            PostingsArrayEnum::TermVectors(postings) => {
                if last_offset.is_some() {
                    postings.last_offsets[term_id] = last_offset.unwrap();
                }
                if last_position.is_some() {
                    postings.last_positions[term_id] = last_position.unwrap();
                }
            },
            _ => unreachable!("should not be here"),
        }

        Ok(())
    }
    pub(crate) fn get_term_freq(&self) -> Result<i32> {
        let freq = if let Some(att) = &self.field_state.term_freq_attribute {
            att.get_term_frequency()
        } else {
            return Ok(1);
        };

        if freq != 1 {
            if self.do_vector_positions {
                return Err(LuceneError::illegal_argument(format!(
                    "field \"{}\": cannot index term vector positions while using custom TermFrequencyAttribute",
                    self.field_name
                )));
            }
            if self.do_vector_offsets {
                return Err(LuceneError::illegal_argument(format!(
                    "field \"{}\": cannot index term vector offsets while using custom TermFrequencyAttribute",
                    self.field_name
                )));
            }
        }

        Ok(freq)
    }
    pub(crate) fn finish<D, C>(self, term_vectors_consumer: &mut TermVectorsConsumer<D, C, O, P, T>)
    where
        D: Directory,
        C: Codec,
    {
        if !self.do_vectors || self.base.get_num_terms() == 0 {
            return;
        }
        term_vectors_consumer.add_field_to_flush(self)
    }
}

impl<O, P, T> Eq for TermVectorsConsumerPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
}

impl<O, P, T> PartialEq<Self> for TermVectorsConsumerPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl<O, P, T> PartialOrd<Self> for TermVectorsConsumerPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<O, P, T> Ord for TermVectorsConsumerPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.field_name.cmp(&other.field_name)
    }
}
impl<O, P, T> TermsHashPerFieldBase for TermVectorsConsumerPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn new_term(&mut self, term_id: i32, doc_id: i32) -> Result<()> {
        let term_id = term_id as usize;
        let freq = self.get_term_freq()?;
        let postings_enum = self
            .base
            .bytes_hash
            .bytes_start_array
            .per_field
            .postings_array
            .as_mut()
            .unwrap();
        if let PostingsArrayEnum::TermVectors(postings) = postings_enum {
            postings.freqs[term_id] = freq;
            postings.last_offsets[term_id] = 0;
            postings.last_positions[term_id] = 0;

            self.write_prox(term_id)?;
        } else {
            unreachable!("Expected TermVectors postings");
        }
        Ok(())
    }

    fn add_term(&mut self, term_id: i32, doc_id: i32) -> Result<()> {
        let term_id = term_id as usize;
        let freq = self.get_term_freq()?;
        let postings_enum = self
            .base
            .bytes_hash
            .bytes_start_array
            .per_field
            .postings_array
            .as_mut()
            .unwrap();

        if let PostingsArrayEnum::TermVectors(postings) = postings_enum {
            postings.freqs[term_id] += freq;
            self.write_prox(term_id)?;
        } else {
            unreachable!("Expected TermVectors postings");
        }

        Ok(())
    }

    fn get_field_name(&self) -> &str {
        self.field_name.as_str()
    }
}

pub(crate) struct TermVectorsPostingsArray {
    pub(crate) size: usize,
    freqs: Vec<i32>,          // How many times this term occurred in the current doc
    last_offsets: Vec<i32>,   // Last offset we saw
    last_positions: Vec<i32>, // Last position where this term occurred
    pub(crate) parent: ParallelPostingsArray,
}

impl TermVectorsPostingsArray {
    pub fn new(size: usize) -> Self {
        TermVectorsPostingsArray {
            size,
            freqs: vec![0; size],
            last_offsets: vec![0; size],
            last_positions: vec![0; size],
            parent: ParallelPostingsArray::new(size),
        }
    }
}

impl PostingsArrayBase for TermVectorsPostingsArray {
    fn bytes_per_posting(&self) -> usize {
        self.parent.bytes_per_posting() + 3 * BitUtil::INT_BYTES
    }
    fn copy_to(&mut self, new_size: usize) -> Result<()> {
        self.parent.copy_to(new_size)?;
        self.size = new_size;
        ArrayUtil::grow_exact(&mut self.freqs, new_size)?;
        ArrayUtil::grow_exact(&mut self.last_offsets, new_size)?;
        ArrayUtil::grow_exact(&mut self.last_positions, new_size)?;
        Ok(())
    }
}
