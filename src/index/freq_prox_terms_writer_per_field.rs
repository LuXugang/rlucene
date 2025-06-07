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
use std::rc::Rc;

use crate::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
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
    field_state: FieldInvertState<O, P, T>,
    field_info: Rc<FieldInfo>,
    pub(crate) has_freq: bool,
    has_prox: bool,
    pub(crate) has_offsets: bool,
    // Set to true if any token had a payload in the current segment.
    pub(crate) saw_payloads: bool,
}
impl<O, P, T> FreqProxTermsWriterPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    pub fn new(
        field_state: FieldInvertState<O, P, T>,
        terms_hash: &mut FreqProxTermsWriter,
        field_info: Rc<FieldInfo>,
        next_per_field: TermsHashPerField<TermVectorsConsumerPerField>,
    ) -> TermsHashPerField<FreqProxTermsWriterPerField<O, P, T>> {
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
        let sub = FreqProxTermsWriterPerField {
            field_state,
            field_info,
            has_freq,
            has_prox,
            has_offsets,
            saw_payloads,
        };
        let postings_array_wrapper = PostingsArrayWrapper::new(TermsHashPerFieldType::FreqProx(
            FreqProx::new(index_options),
        ));
        TermsHashPerField::new(
            stream_count,
            terms_hash.base.int_pool.clone(),
            terms_hash.base.byte_pool.clone(),
            terms_hash.base.term_byte_pool.as_mut().unwrap().clone(),
            terms_hash.base.bytes_used.clone(),
            Some(Box::new(next_per_field)),
            postings_array_wrapper,
            index_options,
            sub,
        )
    }
    pub(crate) fn write_prox<S>(
        &mut self,
        term_id: usize,
        prox_code: i32,
        per_filed: &mut TermsHashPerField<S>,
    ) -> Result<()>
    where
        S: TermsHashPerFieldBase,
    {
        if let Some(payload_attr) = &self.field_state.pay_load_attribute {
            let payload = payload_attr.get_payload();
            if payload.length > 0 {
                TermsHashPerField::write_vint(per_filed, 1, (prox_code << 1) | 1)?;
                TermsHashPerField::write_vint(per_filed, 1, payload.length as i32)?;
                TermsHashPerField::write_bytes(
                    per_filed,
                    1,
                    &payload.bytes,
                    payload.offset as i32,
                    payload.length as i32,
                )?;
                self.saw_payloads = true;
            } else {
                TermsHashPerField::write_vint(per_filed, 1, prox_code << 1)?;
            }
        } else {
            TermsHashPerField::write_vint(per_filed, 1, prox_code << 1)?;
        }
        let postings_array_enum = per_filed
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
    pub(crate) fn write_offsets<S>(
        &mut self,
        term_id: usize,
        offset_accum: i32,
        per_field: &mut TermsHashPerField<S>,
    ) -> Result<()>
    where
        S: TermsHashPerFieldBase,
    {
        let offset_attribute = self.field_state.off_set_attribute.as_ref().unwrap();
        let start_offset = offset_accum + offset_attribute.start_offset();
        let end_offset = offset_accum + offset_attribute.end_offset();

        let postings_array = per_field
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
        TermsHashPerField::write_vint(per_field, 1, v1)?;
        TermsHashPerField::write_vint(per_field, 1, v2)?;

        Ok(())
    }
    fn get_term_freq(&self) -> Result<i32> {
        let freq = if let Some(attr) = &self.field_state.term_freq_attribute {
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
}
impl<O, P, T> TermsHashPerFieldBase for FreqProxTermsWriterPerField<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn start(&mut self, _field: &Fields, _first: bool) -> Result<bool> {
        Ok(true)
    }

    fn new_term<S: TermsHashPerFieldBase>(
        &mut self,
        term_id: i32,
        doc_id: i32,
        per_field: &mut TermsHashPerField<S>,
    ) -> Result<()> {
        let term_id = term_id as usize;
        // First time we're seeing this term since the last
        // flush
        let postings_array_enum = per_field
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
                    self.field_state.max_term_frequency =
                        self.field_state.max_term_frequency.max(1);
                } else {
                    postings.last_doc_codes[term_id] = doc_id << 1;
                    let tf = self.get_term_freq()?;
                    postings
                        .term_freqs
                        .as_mut()
                        .expect("term_freqs must be Some")[term_id] = tf;

                    if self.has_prox {
                        self.write_prox(term_id, self.field_state.position, per_field)?;
                        if self.has_offsets {
                            self.write_offsets(term_id, self.field_state.offset, per_field)?;
                        }
                    } else {
                        debug_assert!(!self.has_offsets);
                    }

                    self.field_state.max_term_frequency =
                        self.field_state.max_term_frequency.max(tf);
                }

                self.field_state.unique_term_count += 1;
            },
            _ => unreachable!("expected FreqProx posting array"),
        }

        Ok(())
    }

    fn add_term<S: TermsHashPerFieldBase>(
        &mut self,
        term_id: i32,
        doc_id: i32,
        per_field: &mut TermsHashPerField<S>,
    ) -> Result<()> {
        let term_id = term_id as usize;

        let postings_enum = per_field
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
                        self.field_state.unique_term_count += 1;
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
                    let tf = self.get_term_freq()?;
                    postings.term_freqs.as_mut().unwrap()[term_id] = tf;

                    self.field_state.max_term_frequency =
                        self.field_state.max_term_frequency.max(tf);

                    postings.last_doc_codes[term_id] =
                        (doc_id - postings.last_doc_ids[term_id]) << 1;
                    postings.last_doc_ids[term_id] = doc_id;

                    if self.has_prox && self.has_offsets {
                        postings.last_offsets.as_mut().unwrap()[term_id] = 0;
                    }
                    if self.has_prox {
                        self.write_prox(term_id, self.field_state.position, per_field)?;
                        if self.has_offsets {
                            self.write_offsets(term_id, self.field_state.offset, per_field)?;
                        }
                    } else {
                        debug_assert!(!self.has_offsets);
                    }

                    self.field_state.unique_term_count += 1;
                } else {
                    let tf = self.get_term_freq()?;
                    let term_freqs = postings.term_freqs.as_mut().unwrap();
                    term_freqs[term_id] = term_freqs[term_id].checked_add(tf).ok_or_else(|| {
                        LuceneError::illegal_state("term frequency overflow".to_string())
                    })?;

                    self.field_state.max_term_frequency =
                        self.field_state.max_term_frequency.max(term_freqs[term_id]);

                    if self.has_prox {
                        let delta = self.field_state.position
                            - postings.last_positions.as_ref().unwrap()[term_id];
                        self.write_prox(term_id, delta, per_field)?;
                        if self.has_offsets {
                            self.write_offsets(term_id, self.field_state.offset, per_field)?;
                        }
                    }
                }
            },
            _ => unreachable!("expected FreqProx posting array"),
        }
        for x in v {
            TermsHashPerField::write_vint(per_field, 0, x)?
        }
        Ok(())
    }

    fn finish(&mut self) {
        if self.saw_payloads {
            self.field_info
                .set_store_payloads()
                .expect("should not failed")
        }
    }

    fn get_field_name(&self) -> &str {
        &self.field_info.name
    }
}

pub(crate) struct FreqProxPostingsArray {
    pub(crate) size: usize,
    pub(crate) term_freqs: Option<Vec<i32>>, /* # times this term occurs in
                                              * the current doc */
    pub(crate) last_doc_ids: Vec<i32>, // Last docID where this term occurred
    pub(crate) last_doc_codes: Vec<i32>, // Code for prior doc
    last_positions: Option<Vec<i32>>,  /* Last position where this term
                                        * occurred */
    last_offsets: Option<Vec<i32>>, // Last endOffset where this term occurred
    pub(crate) parent_postings_array: ParallelPostingsArray,
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
            parent_postings_array: ParallelPostingsArray::new(size),
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
        self.parent_postings_array.copy_to(new_size)?;
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
