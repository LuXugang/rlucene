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
use std::cmp::Ordering;
use std::rc::Rc;

use crate::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::codecs::term_vectors_writer::TermVectorsWriter;
use crate::codecs::Codec;
use crate::document::fields::Fields;
use crate::index::field_info::FieldInfo;
use crate::index::field_invert_state::FieldInvertState;
use crate::index::freq_prox_terms_writer::FreqProxTermsWriter;
use crate::index::index_options::IndexOptions;
use crate::index::parallel_postings_array::{
    ParallelPostingsArray, PostingsArrayBase, PostingsArrayEnum,
};
use crate::index::term_vectors_consumer_per_field::TermVectorsConsumerPerField;
use crate::index::terms_hash_per_field::{
    PostingsArrayWrapper, TermsHashPerField, TermsHashPerFieldBase, TermsHashPerFieldType,
};
use crate::index::BytesRef;
use crate::store::directory::Directory;
use crate::util::array_util::ArrayUtil;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::ToInt;
// TODO: break into separate freq and prox writers as
// codecs; make separate container (tii/tis/skip/*) that can
// be configured as any number of files 1..N
pub(crate) struct FreqProxTermsWriterPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    field_info: Rc<FieldInfo>,
    pub(crate) has_freq: bool,
    pub(crate) has_prox: bool,
    pub(crate) has_offsets: bool,
    // Set to true if any token had a payload in the current segment.
    pub(crate) saw_payloads: bool,
    field_state: Rc<FieldInvertState<O, P, T>>,

    pub(crate) next_per_field: Option<TermVectorsConsumerPerField<O, P, T>>,

    pub(crate) base: TermsHashPerField,
}
impl<O, P, T> FreqProxTermsWriterPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    pub fn new<D, C, TVW>(
        field_state: Rc<FieldInvertState<O, P, T>>,
        terms_hash: &mut FreqProxTermsWriter<D, C, TVW, O, P, T>,
        field_info: Rc<FieldInfo>,
        next_per_field: TermVectorsConsumerPerField<O, P, T>,
    ) -> FreqProxTermsWriterPerField<O, P, T>
    where
        D: Directory,
        C: Codec,
        TVW: TermVectorsWriter,
    {
        let index_options = *field_info.get_index_options();

        let has_freq = index_options >= IndexOptions::DocsAndFreqs;
        let has_prox = index_options >= IndexOptions::DocsAndFreqsAndPositions;
        let has_offsets = index_options >= IndexOptions::DocsAndFreqsAndPositionsAndOffsets;

        let saw_payloads = false;

        let stream_count = if index_options
            .cmp(&IndexOptions::DocsAndFreqsAndPositions)
            .to_int()
            >= 0
        {
            2
        } else {
            1
        };
        let name = field_info.get_name().to_string();
        let postings_array_wrapper = PostingsArrayWrapper::new(TermsHashPerFieldType::FreqProx(
            FreqProx::new(index_options),
        ));
        let base = TermsHashPerField::new(
            stream_count,
            terms_hash.int_pool.clone(),
            terms_hash.byte_pool.clone(),
            terms_hash.term_byte_pool.as_mut().unwrap().clone(),
            terms_hash.bytes_used.clone(),
            postings_array_wrapper,
            name,
            index_options,
        );
        FreqProxTermsWriterPerField {
            field_info,
            has_freq,
            has_prox,
            has_offsets,
            saw_payloads,
            next_per_field: Option::from(next_per_field),
            field_state,
            base,
        }
    }
    pub(crate) fn write_prox(&mut self, term_id: usize, prox_code: i32) -> Result<()> {
        if let Some(payload_attr) = &self.field_state.pay_load_attribute {
            let payload = payload_attr.get_payload();
            if payload.length > 0 {
                self.base.write_vint(1, (prox_code << 1) | 1)?;
                self.base.write_vint(1, payload.length as i32)?;
                self.base.write_bytes(
                    1,
                    &payload.bytes,
                    payload.offset as i32,
                    payload.length as i32,
                )?;
                self.saw_payloads = true;
            } else {
                self.base.write_vint(1, prox_code << 1)?;
            }
        } else {
            self.base.write_vint(1, prox_code << 1)?;
        }
        let postings_array_enum = self
            .base
            .bytes_hash
            .bytes_start_array
            .per_field
            .postings_array
            .as_mut()
            .expect("postings_array must be Some");
        match postings_array_enum {
            PostingsArrayEnum::FreqProx(f) => {
                f.last_positions.as_mut().unwrap()[term_id] = self.field_state.position();
            },
            _ => unreachable!("should not be here"),
        }

        Ok(())
    }
    pub(crate) fn write_offsets(&mut self, term_id: usize, offset_accum: i32) -> Result<()> {
        let offset_attribute = &self.field_state.off_set_attribute.as_ref().unwrap();
        let start_offset = offset_accum + offset_attribute.start_offset();
        let end_offset = offset_accum + offset_attribute.end_offset();

        let postings_array = self
            .base
            .bytes_hash
            .bytes_start_array
            .per_field
            .postings_array
            .as_mut()
            .expect("postings_array must be Some");

        let (v1, v2) = match postings_array {
            PostingsArrayEnum::FreqProx(f) => {
                let last_offsets = f.last_offsets.as_mut().expect("last_offsets must be Some");
                let last_offset = last_offsets[term_id];

                debug_assert!(
                    start_offset - last_offset >= 0,
                    "start_offset must not go backwards"
                );
                let v1 = start_offset - last_offset;
                let v2 = end_offset - start_offset;

                last_offsets[term_id] = start_offset;

                (v1, v2)
            },
            _ => unreachable!("expected FreqProx posting array"),
        };
        self.base.write_vint(1, v1)?;
        self.base.write_vint(1, v2)?;

        Ok(())
    }
    fn get_term_freq(&self, field_state: &FieldInvertState<O, P, T>) -> Result<i32>
    where
        O: OffsetAttribute,
        P: PayloadAttribute,
        T: TermFrequencyAttribute,
    {
        let freq = if let Some(attr) = &field_state.term_freq_attribute {
            attr.get_term_frequency()
        } else {
            1
        };

        if freq != 1 && self.has_prox {
            return Err(LuceneError::illegal_state(format!(
                "field \"{}\": cannot index positions while using custom TermFrequencyAttribute",
                self.field_info.name
            )));
        }

        Ok(freq)
    }
    pub(crate) fn get_next_per_field(&mut self) -> TermVectorsConsumerPerField<O, P, T> {
        self.next_per_field.take().unwrap()
    }
    fn finish(&mut self) {
        if self.saw_payloads {
            self.field_info
                .set_store_payloads()
                .expect("should not failed")
        }
        if self.saw_payloads {
            self.field_info
                .set_store_payloads()
                .expect("should not failed")
        }
    }
    pub(crate) fn reset(&mut self) {
        self.base.reset();
        if self.next_per_field.is_some() {
            self.next_per_field.as_mut().unwrap().reset();
        }
    }
    /// Called once per inverted token. This is the primary entry point (for
    /// first TermsHash); postings use this API.
    pub(crate) fn add_with_bytes_ref(
        &mut self,
        term_bytes: &BytesRef<Vec<u8>>,
        doc_id: i32,
    ) -> Result<()> {
        debug_assert!(self.base.assert_doc_id(doc_id));
        // We are first in the chain so we must "intern" the
        // term text into textStart address
        // Get the text & hash of this term.
        let mut term_id = self.base.bytes_hash.add(term_bytes)?;
        if term_id >= 0 {
            self.base.init_stream_slices(term_id, doc_id)?;
            self.new_term(term_id, doc_id)?;
        } else {
            term_id = self.base.position_stream_slice(term_id, doc_id)?;
            self.add_term(term_id, doc_id)?;
        }

        if let Some(ref mut next_per_field) = self.next_per_field {
            let postings_array_wrapper = &self.base.bytes_hash.bytes_start_array.per_field;
            debug_assert!(postings_array_wrapper.postings_array.is_some());
            let text_start = postings_array_wrapper
                .postings_array
                .as_ref()
                .unwrap()
                .get_text_starts()[term_id as usize];
            next_per_field.add_with_text_start(text_start, doc_id)?;
        }
        Ok(())
    }
    fn start(&mut self, field: &Fields, first: bool) -> Result<bool> {
        match self.next_per_field {
            Some(ref mut next_per_field) => next_per_field.start(field, first)?,
            None => true,
        };
        Ok(true)
    }
}
impl<O, P, T> TermsHashPerFieldBase for FreqProxTermsWriterPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn new_term(&mut self, term_id: i32, doc_id: i32) -> Result<()> {
        let term_id = term_id as usize;
        // First time we're seeing this term since the last
        // flush
        let tf = self.get_term_freq(&self.field_state)?;
        let postings_array_enum = self
            .base
            .bytes_hash
            .bytes_start_array
            .per_field
            .postings_array
            .as_mut()
            .expect("postings_array must be Some");

        match postings_array_enum {
            PostingsArrayEnum::FreqProx(postings) => {
                postings.last_doc_ids[term_id] = doc_id;

                if !self.has_freq {
                    debug_assert!(postings.term_freqs.is_none());
                    postings.last_doc_codes[term_id] = doc_id;
                    let mut inner = self.field_state.inner.borrow_mut();
                    inner.max_term_frequency = inner.max_term_frequency.max(1);
                } else {
                    postings.last_doc_codes[term_id] = doc_id << 1;
                    postings
                        .term_freqs
                        .as_mut()
                        .expect("term_freqs must be Some")[term_id] = tf;

                    if self.has_prox {
                        self.write_prox(term_id, self.field_state.position)?;
                        if self.has_offsets {
                            self.write_offsets(term_id, self.field_state.offset)?;
                        }
                    } else {
                        debug_assert!(!self.has_offsets);
                    }

                    let mut inner = self.field_state.inner.borrow_mut();
                    inner.max_term_frequency = inner.max_term_frequency.max(tf);
                }
                {
                    let mut inner = self.field_state.inner.borrow_mut();
                    inner.unique_term_count += 1;
                }
            },
            _ => unreachable!("expected FreqProx posting array"),
        }

        Ok(())
    }

    fn add_term(&mut self, term_id: i32, doc_id: i32) -> Result<()> {
        let term_id = term_id as usize;

        let tf = self.get_term_freq(&self.field_state)?;
        let postings_enum = self
            .base
            .bytes_hash
            .bytes_start_array
            .per_field
            .postings_array
            .as_mut()
            .expect("postings_array must be Some");
        let mut v = Vec::new();
        match postings_enum {
            PostingsArrayEnum::FreqProx(postings) => {
                if self.has_freq {
                    debug_assert!(postings.term_freqs.as_ref().unwrap()[term_id] > 0);
                }

                if !self.has_freq {
                    debug_assert!(postings.term_freqs.is_none());

                    if let Some(attr) = &self.field_state.term_freq_attribute {
                        if attr.get_term_frequency() != 1 {
                            return Err(LuceneError::illegal_state(format!(
                                "field \"{}\": must index term freq while using custom TermFrequencyAttribute",
                                self.field_info.name
                            )));
                        }
                    }

                    if doc_id != postings.last_doc_ids[term_id] {
                        debug_assert!(doc_id > postings.last_doc_ids[term_id]);
                        v.push(postings.last_doc_codes[term_id]);
                        postings.last_doc_codes[term_id] = doc_id - postings.last_doc_ids[term_id];
                        postings.last_doc_ids[term_id] = doc_id;
                        {
                            let mut inner = self.field_state.inner.borrow_mut();
                            inner.unique_term_count += 1;
                        }
                    }
                } else if doc_id != postings.last_doc_ids[term_id] {
                    debug_assert!(
                        doc_id > postings.last_doc_ids[term_id],
                        "docID = {}, postingsID = {}, termID = {}",
                        doc_id,
                        postings.last_doc_ids[term_id],
                        term_id
                    );

                    let freq = postings.term_freqs.as_ref().unwrap()[term_id];
                    // Term not yet seen in the current doc but previously
                    // seen in other doc(s) since the last flush

                    // Now that we know doc freq for previous doc,
                    // write it & lastDocCode
                    if freq == 1 {
                        v.push(postings.last_doc_codes[term_id] | 1);
                    } else {
                        v.push(postings.last_doc_codes[term_id]);
                        v.push(freq);
                    }
                    // Init freq for the current document
                    postings.term_freqs.as_mut().unwrap()[term_id] = tf;

                    {
                        let mut inner = self.field_state.inner.borrow_mut();
                        inner.max_term_frequency = inner.max_term_frequency.max(tf);
                    }

                    postings.last_doc_codes[term_id] =
                        (doc_id - postings.last_doc_ids[term_id]) << 1;
                    postings.last_doc_ids[term_id] = doc_id;

                    if self.has_prox && self.has_offsets {
                        postings.last_offsets.as_mut().unwrap()[term_id] = 0;
                    }
                    if self.has_prox {
                        self.write_prox(term_id, self.field_state.position)?;
                        if self.has_offsets {
                            self.write_offsets(term_id, self.field_state.offset)?;
                        }
                    } else {
                        debug_assert!(!self.has_offsets);
                    }
                    {
                        let mut inner = self.field_state.inner.borrow_mut();
                        inner.unique_term_count += 1;
                    }
                } else {
                    let term_freqs = postings.term_freqs.as_mut().unwrap();
                    term_freqs[term_id] = term_freqs[term_id].checked_add(tf).ok_or_else(|| {
                        LuceneError::illegal_state("term frequency overflow".to_string())
                    })?;

                    {
                        let mut inner = self.field_state.inner.borrow_mut();
                        inner.max_term_frequency =
                            inner.max_term_frequency.max(term_freqs[term_id]);
                    }

                    if self.has_prox {
                        let delta = self.field_state.position
                            - postings.last_positions.as_ref().unwrap()[term_id];
                        self.write_prox(term_id, delta)?;
                        if self.has_offsets {
                            self.write_offsets(term_id, self.field_state.offset)?;
                        }
                    }
                }
            },
            _ => unreachable!("expected FreqProx posting array"),
        }
        for x in v {
            self.base.write_vint(0, x)?
        }
        Ok(())
    }

    fn get_field_name(&self) -> &str {
        &self.field_info.name
    }
}

impl<O, P, T> Eq for FreqProxTermsWriterPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
}

impl<O, P, T> PartialEq<Self> for FreqProxTermsWriterPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl<O, P, T> PartialOrd<Self> for FreqProxTermsWriterPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<O, P, T> Ord for FreqProxTermsWriterPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.base.field_name.cmp(&other.base.field_name)
    }
}

pub(crate) struct FreqProxPostingsArray {
    pub(crate) size: usize,
    pub(crate) term_freqs: Option<Vec<i32>>, /* # times this term occurs in
                                              * the current doc */
    pub(crate) last_doc_ids: Vec<i32>, // Last docID where this term occurred
    pub(crate) last_doc_codes: Vec<i32>, // Code for prior doc
    pub(crate) last_positions: Option<Vec<i32>>, /* Last position where this term
                                        * occurred */
    pub(crate) last_offsets: Option<Vec<i32>>, // Last endOffset where this term occurred
    pub(crate) parent: ParallelPostingsArray,
}
impl FreqProxPostingsArray {
    // Constructor for FreqProxPostingsArray
    pub(crate) fn new(
        size: usize,
        write_freqs: bool,
        write_prox: bool,
        write_offsets: bool,
    ) -> Self {
        let mut term_freqs = None;
        if write_freqs {
            term_freqs = Some(vec![0; size]);
        }
        let last_positions = if write_prox {
            Some(vec![0; size])
        } else {
            None
        };
        let last_offsets = if write_offsets {
            Some(vec![0; size])
        } else {
            None
        };
        FreqProxPostingsArray {
            size,
            term_freqs,
            last_doc_ids: vec![0; size],
            last_doc_codes: vec![0; size],
            last_positions,
            last_offsets,
            parent: ParallelPostingsArray::new(size),
        }
    }
}

impl PostingsArrayBase for FreqProxPostingsArray {
    fn bytes_per_posting(&self) -> usize {
        let i32_bytes = BitUtil::INT_BYTES;
        let mut bytes = ParallelPostingsArray::BYTES_PER_POSTING + 2 * i32_bytes;

        if self.last_positions.is_some() {
            bytes += i32_bytes;
        }
        if self.last_offsets.is_some() {
            bytes += i32_bytes;
        }
        if self.term_freqs.is_some() {
            bytes += i32_bytes;
        }
        bytes
    }

    fn copy_to(&mut self, new_size: usize) -> Result<()> {
        self.parent.copy_to(new_size)?;
        self.size = new_size;
        ArrayUtil::grow_exact(&mut self.last_doc_ids, new_size)?;
        ArrayUtil::grow_exact(&mut self.last_doc_codes, new_size)?;
        if self.last_positions.is_some() {
            ArrayUtil::grow_exact(self.last_positions.as_mut().unwrap(), new_size)?;
        }
        if self.last_offsets.is_some() {
            ArrayUtil::grow_exact(self.last_offsets.as_mut().unwrap(), new_size)?;
        }
        if self.term_freqs.is_some() {
            ArrayUtil::grow_exact(self.term_freqs.as_mut().unwrap(), new_size)?;
        }
        Ok(())
    }
}

pub(crate) struct FreqProx {
    pub(crate) index_options: IndexOptions,
}
impl FreqProx {
    pub fn new(index_options: IndexOptions) -> Self {
        FreqProx { index_options }
    }
}
