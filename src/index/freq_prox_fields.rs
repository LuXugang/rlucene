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
use crate::index::automaton_terms_enum::AutomatonTermsEnum;
use crate::index::base_terms_enum::BaseTermsEnum;
use crate::index::byte_slice_reader::ByteSliceReader;
use crate::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::index::dummy::dummy_term_state_type::DummyTermState;
use crate::index::filtered_terms_enum::FilteredTermsEnum;
use crate::index::freq_prox_terms_writer_per_field::FreqProxTermsWriterPerField;
use crate::index::index_options::IndexOptions;
use crate::index::parallel_postings_array::PostingsArrayEnum;
use crate::index::postings_enum::{postings_enum_util, PostingsEnum};
use crate::index::terms::Terms;
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::terms_hash_per_field::{TermsHashPerField, TermsHashPerFieldBase};
use crate::index::{BytesRef, BytesRefBuilder};
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::DataInput;
use crate::util::automation::compiled_automaton::CompiledAutomaton;
use crate::util::bytes_ref_block_pool::BytesRefBlockPool;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::either_enums::EitherPostingsEnum;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{ByteBlockPoolBorrow, CounterEnumBorrow, ToInt};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
/// Implements limited (iterators only, no stats) [`Fields`](crate::index::fields::Fields) interface over the in-RAM buffered
/// fields/terms/postings, to flush postings through the PostingsFormat.
pub(crate) struct FreqProxFields<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fields: HashMap<String, TermsHashPerField<FreqProxTermsWriterPerField<O, P, T>>>,
}
impl<O, P, T> FreqProxFields<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    pub fn new(field_list: Vec<TermsHashPerField<FreqProxTermsWriterPerField<O, P, T>>>) -> Self {
        // NOTE: fields are already sorted by field name
        let mut fields = HashMap::with_capacity(field_list.len());
        for field in field_list {
            let field_name = field.get_field_name().to_string();
            fields.insert(field_name, field);
        }
        Self { fields }
    }
}

struct FreqProxTerms<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    terms: Rc<RefCell<TermsHashPerField<FreqProxTermsWriterPerField<O, P, T>>>>,
}
impl<O, P, T> FreqProxTerms<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    pub fn new(
        terms: Rc<RefCell<TermsHashPerField<FreqProxTermsWriterPerField<O, P, T>>>>,
    ) -> Self {
        Self { terms }
    }
}
impl<O, P, T> Terms for FreqProxTerms<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    type TermsEnum<'a>
        = BaseTermsEnum<FreqProxTermsEnum<O, P, T>>
    where
        Self: 'a;

    fn iterator(&self) -> Result<Self::TermsEnum<'_>> {
        Ok(FreqProxTermsEnum::new(self.terms.clone()))
    }

    type IntersectIter<'a>
        = FilteredTermsEnum<Self::TermsEnum<'a>, AutomatonTermsEnum>
    where
        O: 'a,
        P: 'a,
        T: 'a;

    fn intersect(
        &self,
        compiled: &mut CompiledAutomaton,
        start_term: Option<BytesRef<Vec<u8>>>,
    ) -> Result<Self::IntersectIter<'_>> {
        self.default_intersect(compiled, start_term)
    }

    fn size(&self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn get_sum_total_term_freq(&self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn get_sum_doc_freq(&self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn get_doc_count(&self) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn has_freqs(&self) -> bool {
        self.terms
            .borrow()
            .index_options
            .cmp(&IndexOptions::DocsAndFreqs)
            .to_int()
            >= 0
    }

    fn has_offsets(&self) -> bool {
        // NOTE: the in-memory buffer may have indexed offsets
        // because that's what FieldInfo said when we started,
        // but during indexing this may have been downgraded:
        self.terms
            .borrow()
            .index_options
            .cmp(&IndexOptions::DocsAndFreqsAndPositionsAndOffsets)
            .to_int()
            >= 0
    }

    fn has_positions(&self) -> bool {
        // NOTE: the in-memory buffer may have indexed positions
        // because that's what FieldInfo said when we started,
        // but during indexing this may have been downgraded:
        self.terms
            .borrow()
            .index_options
            .cmp(&IndexOptions::DocsAndFreqsAndPositions)
            .to_int()
            >= 0
    }

    fn has_payloads(&self) -> bool {
        self.terms.borrow().sub.as_ref().unwrap().saw_payloads
    }
}

struct FreqProxTermsEnum<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    terms: Rc<RefCell<TermsHashPerField<FreqProxTermsWriterPerField<O, P, T>>>>,
    terms_pool: BytesRefBlockPool<CounterEnumBorrow, ByteBlockPoolBorrow>,
    scratch: BytesRef<Vec<u8>>,
    num_terms: i32,
    ord: i32,
}
impl<O, P, T> FreqProxTermsEnum<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn new(
        terms: Rc<RefCell<TermsHashPerField<FreqProxTermsWriterPerField<O, P, T>>>>,
    ) -> BaseTermsEnum<Self> {
        let (num_terms, terms_pool) = {
            let terms_b = terms.borrow();
            let num_terms = terms_b.get_num_terms();
            let terms_pool = BytesRefBlockPool::from_byte_block_pool(terms_b.byte_pool.clone());
            (num_terms, terms_pool)
        };
        let sub = Self {
            terms,
            terms_pool,
            scratch: BytesRef::new(),
            num_terms,
            ord: 0,
        };
        BaseTermsEnum::new(sub)
    }
    pub fn reset(&mut self) {
        self.ord = -1;
    }
}

impl<O, P, T> BytesRefIterator for FreqProxTermsEnum<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn next(&mut self) -> Result<Option<Cow<BytesRef<Vec<u8>>>>> {
        self.ord += 1;
        if self.ord >= self.num_terms {
            return Ok(None);
        }

        let term_id = self.terms.borrow().get_sorted_term_ids()[self.ord as usize];

        let postings_array_enum = &self
            .terms
            .borrow()
            .bytes_hash
            .bytes_start_array
            .per_field
            .postings_array;

        let Some(PostingsArrayEnum::FreqProx(p)) = postings_array_enum else {
            return Err(LuceneError::illegal_state(
                "Expected FreqProx postings array",
            ));
        };

        let text_start = p.parent.text_starts[term_id as usize];
        self.terms_pool
            .fill_bytes_ref(&mut self.scratch, text_start);

        Ok(Some(Cow::Borrowed(&self.scratch)))
    }
}

impl<O, P, T> TermsEnum for FreqProxTermsEnum<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn seek_ceil(&mut self, text: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
        let terms = self.terms.borrow();
        let sub = terms.sub.as_ref().expect("sub must be initialized");
        let postings_array_enum = &self
            .terms
            .borrow()
            .bytes_hash
            .bytes_start_array
            .per_field
            .postings_array;
        let Some(postings_array) = postings_array_enum else {
            return Err(LuceneError::illegal_state("Postings array is none"));
        };

        let PostingsArrayEnum::FreqProx(postings_array) = postings_array else {
            return Err(LuceneError::illegal_state("Unexpected postings array type"));
        };

        let terms_b = self.terms.borrow();
        let sorted_term_ids = terms_b.get_sorted_term_ids();

        let mut lo = 0;
        let mut hi = self.num_terms - 1;

        while hi >= lo {
            let mid = (lo + hi) >> 1;
            let term_id = sorted_term_ids[mid as usize];
            let text_start = postings_array.parent.text_starts[term_id as usize];

            self.terms_pool
                .fill_bytes_ref(&mut self.scratch, text_start);
            let cmp = self.scratch.cmp(text).to_int();

            if cmp < 0 {
                lo = mid + 1;
            } else if cmp > 0 {
                hi = mid - 1;
            } else {
                // found
                self.ord = mid;
                debug_assert_eq!((*self.term()?).cmp(text).to_int(), 0);
                return Ok(SeekStatus::Found);
            }
        }

        // not found
        self.ord = lo;
        if self.ord >= self.num_terms {
            Ok(SeekStatus::End)
        } else {
            let term_id = sorted_term_ids[self.ord as usize];
            let text_start = postings_array.parent.text_starts[term_id as usize];
            self.terms_pool
                .fill_bytes_ref(&mut self.scratch, text_start);
            debug_assert!((*self.term()?).cmp(text).to_int() > 0);
            Ok(SeekStatus::NotFound)
        }
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        let ord = ord as i32;
        self.ord = ord;

        let term_id = self.terms.borrow().get_sorted_term_ids()[ord as usize];

        let postings_array_enum = &self
            .terms
            .borrow()
            .bytes_hash
            .bytes_start_array
            .per_field
            .postings_array;

        let Some(PostingsArrayEnum::FreqProx(p)) = postings_array_enum else {
            return Err(LuceneError::illegal_state(
                "Expected FreqProx postings array",
            ));
        };

        let text_start = p.parent.text_starts[term_id as usize];
        self.terms_pool
            .fill_bytes_ref(&mut self.scratch, text_start);

        Ok(())
    }

    fn term(&self) -> Result<Cow<BytesRef<Vec<u8>>>> {
        Ok(Cow::Borrowed(&self.scratch))
    }

    fn ord(&self) -> Result<i64> {
        Ok(self.ord as i64)
    }

    fn doc_freq(&mut self) -> Result<i32> {
        // We do not store this per-term, and we cannot
        // implement this at merge time w/o an added pass
        // through the postings:
        Err(LuceneError::unsupported_operation(""))
    }

    fn total_term_freq(&mut self) -> Result<i64> {
        // We do not store this per-term, and we cannot
        // implement this at merge time w/o an added pass
        // through the postings:
        Err(LuceneError::unsupported_operation(""))
    }

    type PostingsEnum =
        EitherPostingsEnum<FreqProxPostingsEnum<O, P, T>, FreqProxDocsEnum<O, P, T>>;

    fn postings_with_flags(
        &mut self,
        reuse: Option<Self::PostingsEnum>,
        flags: i32,
    ) -> Result<Self::PostingsEnum> {
        let terms_b = self.terms.borrow();
        let sorted_term_ids = terms_b.get_sorted_term_ids();
        if postings_enum_util::feature_requested(flags, postings_enum_util::POSITIONS) {
            let terms_borrow = self.terms.borrow();
            let (has_prox, has_offsets, has_freq) = {
                let sub = terms_borrow.sub.as_ref().expect("sub must be initialized");
                (sub.has_prox, sub.has_offsets, sub.has_freq)
            };

            if !has_prox {
                // Caller wants positions but we didn't index them;
                // don't lie:
                return Err(LuceneError::illegal_state("did not index positions"));
            }
            if !has_offsets
                && postings_enum_util::feature_requested(flags, postings_enum_util::OFFSETS)
            {
                // Caller wants offsets but we didn't index them;
                // don't lie:
                return Err(LuceneError::illegal_state("did not index offsets"));
            }

            let mut pos_enum = match reuse {
                Some(EitherPostingsEnum::F(p)) => p,
                Some(EitherPostingsEnum::S(_)) => FreqProxPostingsEnum::new(self.terms.clone()),
                None => return Err(LuceneError::illegal_state("reuse is none")),
            };
            pos_enum.reset(sorted_term_ids[self.ord as usize]);
            return Ok(EitherPostingsEnum::F(pos_enum));
        }

        if !postings_enum_util::feature_requested(flags, postings_enum_util::OFFSETS) {
            // Caller wants offsets but we didn't index them;
            // don't lie:
            return Err(LuceneError::illegal_state("did not index offsets"));
        };
        let mut docs_enum = match reuse {
            Some(EitherPostingsEnum::S(p)) => p,
            Some(EitherPostingsEnum::F(_)) => FreqProxDocsEnum::new(self.terms.clone()),
            None => return Err(LuceneError::illegal_state("reuse is none")),
        };
        docs_enum.reset(sorted_term_ids[self.ord as usize]);
        Ok(EitherPostingsEnum::S(docs_enum))
    }

    type ImpactsEnum = DummyImpactsEnum;

    fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
        Err(LuceneError::unsupported_operation(""))
    }

    type TermState = DummyTermState;
}

struct FreqProxDocsEnum<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    pub terms: Rc<RefCell<TermsHashPerField<FreqProxTermsWriterPerField<O, P, T>>>>,
    pub reader: ByteSliceReader,
    pub read_term_freq: bool,
    pub doc_id: i32,
    pub freq: i32,
    pub ended: bool,
    pub term_id: i32,
}
impl<O, P, T> FreqProxDocsEnum<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    pub fn new(
        terms: Rc<RefCell<TermsHashPerField<FreqProxTermsWriterPerField<O, P, T>>>>,
    ) -> Self {
        let read_term_freq = terms.borrow().sub.as_ref().unwrap().has_freq;
        Self {
            terms,
            reader: ByteSliceReader::new(),
            read_term_freq,
            doc_id: -1,
            freq: 0,
            ended: false,
            term_id: -1,
        }
    }
    pub fn reset(&mut self, term_id: i32) {
        self.term_id = term_id;
        let terms = self.terms.borrow_mut();
        terms.init_reader(&mut self.reader, term_id, 0);
        self.ended = false;
        self.doc_id = -1;
    }
}

impl<O, P, T> DocIdSetIterator for FreqProxDocsEnum<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        if self.doc_id == -1 {
            self.doc_id = 0;
        }

        if self.reader.eof() {
            if self.ended {
                return Ok(NO_MORE_DOCS);
            } else {
                self.ended = true;
                {
                    let postings_array_enum = &self
                        .terms
                        .borrow()
                        .bytes_hash
                        .bytes_start_array
                        .per_field
                        .postings_array;
                    let Some(postings_array) = postings_array_enum else {
                        return Err(LuceneError::illegal_state("Postings array is none"));
                    };

                    let PostingsArrayEnum::FreqProx(p) = postings_array else {
                        return Err(LuceneError::illegal_state("Unexpected postings array type"));
                    };
                    self.doc_id = p.last_doc_ids[self.term_id as usize];
                    if self.read_term_freq {
                        self.freq = p.term_freqs.as_ref().expect("term_freqs must exist")
                            [self.term_id as usize];
                    }
                }
            }
        } else {
            let code = self.reader.read_vint()?;
            if !self.read_term_freq {
                self.doc_id += code;
            } else {
                self.doc_id += (code as u32 >> 1) as i32;
                if (code & 1) != 0 {
                    self.freq = 1;
                } else {
                    self.freq = self.reader.read_vint()?;
                }
            }
        }

        Ok(self.doc_id)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn cost(&self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl<O, P, T> PostingsEnum for FreqProxDocsEnum<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn freq(&mut self) -> Result<i32> {
        // Don't lie here ... don't want codecs writings lots
        // of wasted 1s into the index:
        if !self.read_term_freq {
            return Err(LuceneError::illegal_state("freq was not indexed"));
        }
        Ok(self.freq)
    }

    fn next_position(&mut self) -> Result<i32> {
        Ok(-1)
    }

    fn start_offset(&self) -> Result<i32> {
        Ok(-1)
    }

    fn end_offset(&self) -> Result<i32> {
        Ok(-1)
    }

    fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
        Ok(None)
    }
}

struct FreqProxPostingsEnum<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    terms: Rc<RefCell<TermsHashPerField<FreqProxTermsWriterPerField<O, P, T>>>>,
    reader: ByteSliceReader,
    pos_reader: ByteSliceReader,
    read_offsets: bool,
    doc_id: i32,
    freq: i32,
    pos: i32,
    start_offset: i32,
    end_offset: i32,
    pos_left: i32,
    term_id: i32,
    ended: bool,
    has_payload: bool,
    payload: BytesRefBuilder<Vec<u8>>,
}
impl<O, P, T> FreqProxPostingsEnum<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    pub fn new(
        terms: Rc<RefCell<TermsHashPerField<FreqProxTermsWriterPerField<O, P, T>>>>,
    ) -> Self {
        let has_offsets = terms.borrow().sub.as_ref().unwrap().has_offsets;
        Self {
            terms,
            reader: ByteSliceReader::new(),
            pos_reader: ByteSliceReader::new(),
            read_offsets: has_offsets,
            doc_id: -1,
            freq: 0,
            pos: 0,
            start_offset: 0,
            end_offset: 0,
            pos_left: 0,
            term_id: -1,
            ended: false,
            has_payload: false,
            payload: BytesRefBuilder::new(),
        }
    }
    pub fn reset(&mut self, term_id: i32) {
        self.term_id = term_id;
        let terms = self.terms.borrow_mut();
        terms.init_reader(&mut self.reader, term_id, 0);
        terms.init_reader(&mut self.pos_reader, term_id, 1);
        self.ended = false;
        self.doc_id = -1;
        self.pos_left = 0;
    }
}

impl<O, P, T> DocIdSetIterator for FreqProxPostingsEnum<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        if self.doc_id == -1 {
            self.doc_id = 0;
        }

        while self.pos_left != 0 {
            self.next_position()?;
        }

        if self.reader.eof() {
            if self.ended {
                return Ok(NO_MORE_DOCS);
            } else {
                self.ended = true;
                {
                    let postings_array_enum = &self
                        .terms
                        .borrow()
                        .bytes_hash
                        .bytes_start_array
                        .per_field
                        .postings_array;
                    let Some(postings_array) = postings_array_enum else {
                        return Err(LuceneError::illegal_state("Postings array is none"));
                    };

                    let PostingsArrayEnum::FreqProx(p) = postings_array else {
                        return Err(LuceneError::illegal_state("Unexpected postings array type"));
                    };

                    self.doc_id = p.last_doc_codes[self.term_id as usize];
                    self.freq = p.term_freqs.as_ref().unwrap()[self.term_id as usize];
                }
            }
        } else {
            let code = self.reader.read_vint()?;
            self.doc_id += ((code as u32) >> 1) as i32;
            if (code & 1) != 0 {
                self.freq = 1;
            } else {
                self.freq = self.reader.read_vint()?;
            }
        }

        self.pos_left = self.freq;
        self.pos = 0;
        self.start_offset = 0;

        Ok(self.doc_id)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn cost(&self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl<O, P, T> PostingsEnum for FreqProxPostingsEnum<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn freq(&mut self) -> Result<i32> {
        Ok(self.freq)
    }

    fn next_position(&mut self) -> Result<i32> {
        debug_assert!(self.pos_left > 0);
        self.pos_left -= 1;

        let code = self.pos_reader.read_vint()?;
        self.pos += (code as u32 >> 1) as i32;

        if (code & 1) != 0 {
            self.has_payload = true;
            // has a payload
            let payload_len = self.pos_reader.read_vint()? as usize;
            self.payload.set_length(payload_len);
            self.payload.grow_no_copy(payload_len);

            debug_assert!(payload_len <= i32::MAX as usize);
            self.pos_reader
                .read_bytes(&mut self.payload.bytes_ref.bytes, 0, payload_len as i32)?;
        } else {
            self.has_payload = false;
        }

        if self.read_offsets {
            self.start_offset += self.pos_reader.read_vint()?;
            self.end_offset = self.start_offset + self.pos_reader.read_vint()?;
        }
        Ok(self.pos)
    }

    fn start_offset(&self) -> Result<i32> {
        if !self.read_offsets {
            return Err(LuceneError::unsupported_operation(
                "Offsets not indexed".to_string(),
            ));
        }
        Ok(self.start_offset)
    }

    fn end_offset(&self) -> Result<i32> {
        if !self.read_offsets {
            return Err(LuceneError::unsupported_operation(
                "Offsets not indexed".to_string(),
            ));
        }
        Ok(self.end_offset)
    }

    fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
        if !self.has_payload {
            return Err(LuceneError::unsupported_operation(
                "Payloads not indexed".to_string(),
            ));
        }
        Ok(Some(&self.payload.bytes_ref))
    }
}
