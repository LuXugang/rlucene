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
use crate::document::fields::Fields;
use crate::index::field_info::FieldInfo;
use crate::index::field_invert_state::FieldInvertState;
use crate::index::parallel_postings_array::{ParallelPostingsArray, PostingsArrayBase};
use crate::index::terms_hash_per_field::{TermsHashPerField, TermsHashPerFieldBase};
use crate::index::BytesRef;
use crate::util::array_util::ArrayUtil;
use crate::util::bit_util::BitUtil;
use crate::util::bytes_ref_block_pool::BytesRefBlockPoolBorrow;
use crate::util::error::lucene_error::Result;
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
    term_byte_pool: BytesRefBlockPoolBorrow,
    has_payloads: bool,
    field_name: String,
    field_state: Rc<FieldInvertState<O, P, T>>,

    base: TermsHashPerField,
}
impl<O, P, T> TermVectorsConsumerPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    pub(crate) fn new(_size: i32) -> Self {
        todo!()
    }

    pub(crate) fn finish_document(&mut self, flush_term: &mut BytesRef<Vec<u8>>) -> Result<()> {
        // if !self.do_vectors {
        //     return Ok(());
        // }
        // self.do_vectors = false;
        //
        // let num_postings = self.get_num_terms();
        // debug_assert!(num_postings >= 0);
        //
        // let postings = &self.term_vectors_postings_array;
        // let tv = &mut self.terms_writer.writer;
        //
        // self.sort_terms();
        // let term_ids = self.get_sorted_term_ids();
        //
        // tv.start_field(
        //     &*self.field_info,
        //     num_postings as usize,
        //     self.do_vector_positions,
        //     self.do_vector_offsets,
        //     self.has_payloads,
        // )?;
        //
        // let mut pos_reader = if self.do_vector_positions {
        //     Some(self.terms_writer.vector_slice_reader_pos.clone())
        // } else {
        //     None
        // };
        // let mut off_reader = if self.do_vector_offsets {
        //     Some(self.terms_writer.vector_slice_reader_off.clone())
        // } else {
        //     None
        // };
        //
        // for &term_id in &term_ids {
        //     let freq = postings.freqs[term_id];
        //     self.term_byte_pool.fill_bytes_ref(flush_term, postings.text_starts[term_id]);
        //
        //     tv.start_term(flush_term, freq)?;
        //
        //     if self.do_vector_positions || self.do_vector_offsets {
        //         if let Some(ref mut pr) = pos_reader {
        //             self.init_reader(pr, term_id, 0)?;
        //         }
        //         if let Some(ref mut or) = off_reader {
        //             self.init_reader(or, term_id, 1)?;
        //         }
        //         tv.add_prox(freq, pos_reader.as_mut(), off_reader.as_mut())?;
        //     }
        //
        //     tv.finish_term()?;
        // }
        //
        // tv.finish_field()?;
        //
        // self.reset();
        // self.field_info.set_store_term_vectors()?;
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
        todo!()
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
        todo!()
    }

    fn add_term(&mut self, term_id: i32, doc_id: i32) -> Result<()> {
        todo!()
    }

    fn get_field_name(&self) -> &str {
        todo!()
    }
}

pub(crate) struct TermVectorsPostingsArray {
    pub(crate) size: usize,
    freqs: Vec<i32>,          // How many times this term occurred in the current doc
    last_offsets: Vec<i32>,   // Last offset we saw
    last_positions: Vec<i32>, // Last position where this term occurred
    pub(crate) parent_postings_array: ParallelPostingsArray,
}

impl TermVectorsPostingsArray {
    pub fn new(size: usize) -> Self {
        TermVectorsPostingsArray {
            size,
            freqs: vec![0; size],
            last_offsets: vec![0; size],
            last_positions: vec![0; size],
            parent_postings_array: ParallelPostingsArray::new(size),
        }
    }
}

impl PostingsArrayBase for TermVectorsPostingsArray {
    fn bytes_per_posting(&self) -> usize {
        self.parent_postings_array.bytes_per_posting() + 3 * BitUtil::INT_BYTES
    }
    fn copy_to(&mut self, new_size: usize) -> Result<()> {
        self.parent_postings_array.copy_to(new_size)?;
        self.size = new_size;
        ArrayUtil::grow_exact(&mut self.freqs, new_size)?;
        ArrayUtil::grow_exact(&mut self.last_offsets, new_size)?;
        ArrayUtil::grow_exact(&mut self.last_positions, new_size)?;
        Ok(())
    }
}
