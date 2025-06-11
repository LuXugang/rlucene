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
use crate::codecs::compressing::lucene90_compressing_term_vectors_writer::lucene90_ctvw_util::{
    OFFSETS, PAYLOADS, POSITIONS,
};
use crate::index::automaton_terms_enum::AutomatonTermsEnum;
use crate::index::base_terms_enum::BaseTermsEnum;
use crate::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::index::dummy::dummy_term_state_type::DummyTermState;
use crate::index::field_infos::FieldInfos;
use crate::index::fields::Fields;
use crate::index::filtered_terms_enum::{FilteredTermsEnum, FilteredTermsEnumBase};
use crate::index::postings_enum::PostingsEnum;
use crate::index::terms::Terms;
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::{ByteArrayDataInput, DataInput};
use crate::util::array_util::ArrayUtil;
use crate::util::automation::compiled_automaton::CompiledAutomaton;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::ToInt;
use std::borrow::Cow;
use std::rc::Rc;

pub struct Lucene90CompressingTermVectorsReader;

impl Lucene90CompressingTermVectorsReader {
    fn sum(arr: &[i32]) -> i32 {
        let mut sum = 0;
        for &el in arr {
            sum += el;
        }
        sum
    }
}
pub(crate) struct TVFields {
    field_nums: Vec<i32>,
    field_flags: Vec<i32>,
    field_num_offs: Vec<i32>,
    num_terms: Vec<i32>,
    field_lengths: Vec<i32>,

    prefix_lengths: Vec<Rc<Vec<i32>>>,
    suffix_lengths: Vec<Rc<Vec<i32>>>,
    term_freqs: Vec<Rc<Vec<i32>>>,
    position_index: Vec<Rc<Vec<i32>>>,
    positions: Vec<Rc<Vec<i32>>>,
    start_offsets: Vec<Rc<Vec<i32>>>,
    lengths: Vec<Rc<Vec<i32>>>,

    payload_bytes: BytesRef<Rc<Vec<u8>>>,
    payload_index: Vec<Rc<Vec<i32>>>,
    suffix_bytes: BytesRef<Rc<Vec<u8>>>,

    names: Vec<String>,
    field_infos: Rc<FieldInfos>,
}
impl TVFields {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        field_nums: Vec<i32>,
        field_flags: Vec<i32>,
        field_num_offs: Vec<i32>,
        num_terms: Vec<i32>,
        field_lengths: Vec<i32>,
        prefix_lengths: Vec<Rc<Vec<i32>>>,
        suffix_lengths: Vec<Rc<Vec<i32>>>,
        term_freqs: Vec<Rc<Vec<i32>>>,
        position_index: Vec<Rc<Vec<i32>>>,
        positions: Vec<Rc<Vec<i32>>>,
        start_offsets: Vec<Rc<Vec<i32>>>,
        lengths: Vec<Rc<Vec<i32>>>,
        payload_bytes: BytesRef<Rc<Vec<u8>>>,
        payload_index: Vec<Rc<Vec<i32>>>,
        suffix_bytes: BytesRef<Rc<Vec<u8>>>,
        field_infos: Rc<FieldInfos>,
    ) -> Result<Self> {
        let mut names = Vec::new();
        for i in 0..field_num_offs.len() {
            let field_num = field_nums[field_num_offs[i] as usize];
            let field_info = field_infos.field_info_by_number(field_num)?;
            match field_info {
                Some(fi) => {
                    names.push(fi.name.clone());
                },
                None => {
                    return Err(LuceneError::illegal_state(format!(
                        "Field number {} not found in field infos",
                        field_num
                    )));
                },
            }
        }

        Ok(Self {
            field_nums,
            field_flags,
            field_num_offs,
            num_terms,
            field_lengths,

            prefix_lengths,
            suffix_lengths,
            term_freqs,
            position_index,
            positions,
            start_offsets,
            lengths,
            payload_bytes,
            payload_index,
            suffix_bytes,
            names,
            field_infos,
        })
    }
}
impl Fields for TVFields {
    fn iterator(&self) -> impl Iterator<Item = &String> {
        self.names.iter()
    }

    type Terms = TVTerms;

    fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
        let field_info = match self.field_infos.field_info_by_name(field) {
            Some(info) => info,
            None => return Ok(None),
        };

        let mut idx = -1;
        for (i, &off) in self.field_num_offs.iter().enumerate() {
            if self.field_nums[off as usize] == field_info.number {
                idx = i as i32;
                break;
            }
        }
        if idx == -1 || self.num_terms[idx as usize] != 0 {
            // no term
            return Ok(None);
        }

        let mut field_off = 0;
        let mut field_len = -1_i32;
        for (i, &len) in self.field_lengths.iter().enumerate() {
            if i < idx as usize {
                field_off += len;
            } else {
                field_len = len;
                break;
            }
        }
        debug_assert!(field_len >= 0);

        let term_bytes = BytesRef::from_slice(
            self.suffix_bytes.bytes.clone(),
            self.suffix_bytes.offset + field_off as usize,
            field_len as usize,
        );

        let idx = idx as usize;
        let tv_terms = TVTerms::new(
            self.num_terms[idx],
            self.field_flags[idx],
            self.prefix_lengths[idx].clone(),
            self.suffix_lengths[idx].clone(),
            self.term_freqs[idx].clone(),
            self.position_index[idx].clone(),
            self.positions[idx].clone(),
            self.start_offsets[idx].clone(),
            self.lengths[idx].clone(),
            self.payload_index[idx].clone(),
            self.payload_bytes.clone(),
            term_bytes,
        );
        Ok(Some(tv_terms))
    }

    fn size(&self) -> Result<i32> {
        debug_assert!(self.field_num_offs.len() <= i32::MAX as usize);
        Ok(self.field_num_offs.len() as i32)
    }
}

pub(crate) struct TVTerms {
    num_terms: i32,
    flags: i32,
    total_term_freq: i64,

    prefix_lengths: Rc<Vec<i32>>,
    suffix_lengths: Rc<Vec<i32>>,
    term_freqs: Rc<Vec<i32>>,
    position_index: Rc<Vec<i32>>,
    positions: Rc<Vec<i32>>,
    start_offsets: Rc<Vec<i32>>,
    lengths: Rc<Vec<i32>>,
    payload_index: Rc<Vec<i32>>,

    payload_bytes: BytesRef<Rc<Vec<u8>>>,
    term_bytes: BytesRef<Rc<Vec<u8>>>,
}
impl TVTerms {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        num_terms: i32,
        flags: i32,
        prefix_lengths: Rc<Vec<i32>>,
        suffix_lengths: Rc<Vec<i32>>,
        term_freqs: Rc<Vec<i32>>,
        position_index: Rc<Vec<i32>>,
        positions: Rc<Vec<i32>>,
        start_offsets: Rc<Vec<i32>>,
        lengths: Rc<Vec<i32>>,
        payload_index: Rc<Vec<i32>>,
        payload_bytes: BytesRef<Rc<Vec<u8>>>,
        term_bytes: BytesRef<Rc<Vec<u8>>>,
    ) -> Self {
        let total_term_freq = term_freqs.iter().map(|&x| x as i64).sum();

        TVTerms {
            num_terms,
            flags,
            prefix_lengths,
            suffix_lengths,
            term_freqs,
            position_index,
            positions,
            start_offsets,
            lengths,
            payload_index,
            payload_bytes,
            term_bytes,
            total_term_freq,
        }
    }
}
impl Terms for TVTerms {
    type TermsEnum = BaseTermsEnum<TVTermsEnum>;

    fn iterator(&self) -> Result<Self::TermsEnum> {
        let terms_enum = TVTermsEnum::new(
            self.num_terms,
            self.flags,
            self.prefix_lengths.clone(),
            self.suffix_lengths.clone(),
            self.term_freqs.clone(),
            self.position_index.clone(),
            self.positions.clone(),
            self.start_offsets.clone(),
            self.lengths.clone(),
            self.payload_index.clone(),
            self.payload_bytes.clone(),
            ByteArrayDataInput::with_range(
                self.term_bytes.bytes.clone(),
                self.term_bytes.offset,
                self.term_bytes.length,
            ),
        );
        Ok(BaseTermsEnum::new(terms_enum))
    }

    type IntersectIter
        = FilteredTermsEnum<Self::TermsEnum, AutomatonTermsEnum>
    where
        Self::TermsEnum: BytesRefIterator,
        AutomatonTermsEnum: FilteredTermsEnumBase;
    fn intersect(
        &self,
        compiled: &mut CompiledAutomaton,
        start_term: Option<BytesRef<Vec<u8>>>,
    ) -> Result<Self::IntersectIter> {
        self.default_intersect(compiled, start_term)
    }

    fn size(&self) -> Result<i64> {
        Ok(self.num_terms as i64)
    }

    fn get_sum_total_term_freq(&self) -> Result<i64> {
        Ok(self.total_term_freq)
    }

    fn get_sum_doc_freq(&self) -> Result<i64> {
        Ok(self.num_terms as i64)
    }

    fn get_doc_count(&self) -> Result<i32> {
        Ok(1)
    }

    fn has_freqs(&self) -> bool {
        true
    }

    fn has_offsets(&self) -> bool {
        (self.flags & OFFSETS) != 0
    }

    fn has_positions(&self) -> bool {
        (self.flags & POSITIONS) != 0
    }

    fn has_payloads(&self) -> bool {
        (self.flags & PAYLOADS) != 0
    }
}

pub(crate) struct TVTermsEnum {
    num_terms: i32,
    start_pos: i32,
    ord: i32,

    prefix_lengths: Rc<Vec<i32>>,
    suffix_lengths: Rc<Vec<i32>>,
    term_freqs: Rc<Vec<i32>>,
    position_index: Rc<Vec<i32>>,
    positions: Rc<Vec<i32>>,
    start_offsets: Rc<Vec<i32>>,
    lengths: Rc<Vec<i32>>,
    payload_index: Rc<Vec<i32>>,

    input: ByteArrayDataInput<Rc<Vec<u8>>>,
    payloads: BytesRef<Rc<Vec<u8>>>,
    term: BytesRef<Vec<u8>>,
}
impl TVTermsEnum {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        num_terms: i32,
        _flags: i32,
        prefix_lengths: Rc<Vec<i32>>,
        suffix_lengths: Rc<Vec<i32>>,
        term_freqs: Rc<Vec<i32>>,
        position_index: Rc<Vec<i32>>,
        positions: Rc<Vec<i32>>,
        start_offsets: Rc<Vec<i32>>,
        lengths: Rc<Vec<i32>>,
        payload_index: Rc<Vec<i32>>,
        payloads: BytesRef<Rc<Vec<u8>>>,
        input: ByteArrayDataInput<Rc<Vec<u8>>>,
    ) -> Self {
        let start_pos = input.get_position();
        debug_assert!(start_pos <= i32::MAX as usize);

        let mut term_enum = TVTermsEnum {
            num_terms,
            prefix_lengths,
            suffix_lengths,
            term_freqs,
            position_index,
            positions,
            start_offsets,
            lengths,
            payload_index,
            payloads,
            input,
            start_pos: start_pos as i32,
            ord: -1,
            term: BytesRef::with_capacity(16),
        };

        term_enum.reset();
        term_enum
    }
    pub fn reset(&mut self) {
        self.term.length = 0;
        self.input.set_position(self.start_pos as usize);
        self.ord = -1;
    }
}

impl BytesRefIterator for TVTermsEnum {
    fn next(&mut self) -> Result<Option<Cow<BytesRef<Vec<u8>>>>> {
        if self.ord == self.num_terms - 1 {
            return Ok(None);
        } else {
            debug_assert!(self.ord < self.num_terms);
            self.ord += 1;
        }

        let prefix_len = self.prefix_lengths[self.ord as usize];
        let suffix_len = self.suffix_lengths[self.ord as usize];
        let total_len = prefix_len + suffix_len;

        self.term.offset = 0;
        self.term.length = total_len as usize;

        if self.term.bytes.len() < self.term.length {
            ArrayUtil::grow_with_len(&mut self.term.bytes, self.term.length);
        }

        self.input
            .read_bytes(&mut self.term.bytes, prefix_len, suffix_len)?;

        Ok(Option::from(Cow::Borrowed(&self.term)))
    }
}

impl TermsEnum for TVTermsEnum {
    fn seek_ceil(&mut self, text: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
        if self.ord < self.num_terms && self.ord >= 0 {
            let cmp = self.term.cmp(text).to_int();
            if cmp == 0 {
                return Ok(SeekStatus::Found);
            } else if cmp > 0 {
                self.reset();
            }
        }

        // linear scan
        loop {
            let term = self.next()?;
            match term {
                None => return Ok(SeekStatus::End),
                Some(t) => {
                    let cmp = (*t).cmp(text).to_int();
                    if cmp > 0 {
                        return Ok(SeekStatus::NotFound);
                    } else if cmp == 0 {
                        return Ok(SeekStatus::Found);
                    }
                },
            }
        }
    }

    fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn term(&self) -> Result<Cow<BytesRef<Vec<u8>>>> {
        Ok(Cow::Borrowed(&self.term))
    }

    fn ord(&self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn doc_freq(&mut self) -> Result<i32> {
        Ok(1)
    }

    fn total_term_freq(&mut self) -> Result<i64> {
        Ok(self.term_freqs[self.ord as usize] as i64)
    }

    type PostingsEnum = TVPostingsEnum;

    fn postings_with_flags(
        &mut self,
        reuse: Option<Self::PostingsEnum>,
        _flags: i32,
    ) -> Result<Self::PostingsEnum> {
        let docs_enum = match reuse {
            Some(mut postings_enum) => {
                postings_enum.reset();
                postings_enum
            },
            None => TVPostingsEnum::new(
                self.term_freqs[self.ord as usize],
                self.position_index[self.ord as usize],
                self.positions.clone(),
                self.start_offsets.clone(),
                self.lengths.clone(),
                self.payloads.clone(),
                self.payload_index.clone(),
            ),
        };
        Ok(docs_enum)
    }

    type ImpactsEnum = DummyImpactsEnum;

    fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
        todo!()
    }

    type TermState = DummyTermState;
}

pub(crate) struct TVPostingsEnum {
    doc: i32,
    term_freq: i32,
    position_index: i32,

    positions: Rc<Vec<i32>>,
    start_offsets: Rc<Vec<i32>>,
    lengths: Rc<Vec<i32>>,
    payload: BytesRef<Rc<Vec<u8>>>,
    payload_index: Rc<Vec<i32>>,
    base_payload_offset: usize,
    i: i32,

    payload_length: usize,
    payload_offset: usize,
}
impl TVPostingsEnum {
    fn new(
        freq: i32,
        position_index: i32,
        positions: Rc<Vec<i32>>,
        start_offsets: Rc<Vec<i32>>,
        lengths: Rc<Vec<i32>>,
        payload: BytesRef<Rc<Vec<u8>>>,
        payload_index: Rc<Vec<i32>>,
    ) -> Self {
        let base_payload_offset = payload.offset;
        TVPostingsEnum {
            doc: -1,
            term_freq: freq,
            position_index,
            positions,
            start_offsets,
            lengths,
            payload,
            payload_index,
            base_payload_offset,
            i: -1,
            payload_length: 0,
            payload_offset: 0,
        }
    }
    fn reset(&mut self) {
        self.base_payload_offset = self.payload_offset;
        self.payload_length = 0;

        self.payload_offset = 0;
        self.i = -1;
        self.doc = -1;
    }

    fn check_doc(&self) -> Result<()> {
        if self.doc == NO_MORE_DOCS {
            Err(LuceneError::illegal_state("DocsEnum exhausted"))
        } else if self.doc == -1 {
            Err(LuceneError::illegal_state("DocsEnum not started"))
        } else {
            Ok(())
        }
    }
    fn check_position(&self) -> Result<()> {
        self.check_doc()?;
        if self.i < 0 {
            Err(LuceneError::illegal_state("Position enum not started"))
        } else if self.i >= self.term_freq {
            Err(LuceneError::illegal_state("Read past last position"))
        } else {
            Ok(())
        }
    }
}

impl DocIdSetIterator for TVPostingsEnum {
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        if self.doc == -1 {
            self.doc = 0;
        } else {
            self.doc = NO_MORE_DOCS;
        }
        Ok(self.doc)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.slow_advance(target)
    }

    fn cost(&self) -> Result<i64> {
        Ok(1)
    }
}

impl PostingsEnum for TVPostingsEnum {
    fn freq(&mut self) -> Result<i32> {
        self.check_doc()?;
        Ok(self.doc)
    }

    fn next_position(&mut self) -> Result<i32> {
        if self.doc != 0 {
            return Err(LuceneError::illegal_state(""));
        } else if self.i >= self.term_freq - 1 {
            return Err(LuceneError::illegal_state("Read past last position"));
        }
        self.i += 1;
        if self.payload_index.is_empty() {
            let index = (self.position_index + self.i) as usize;
            self.payload_offset = self.base_payload_offset + self.payload_index[index] as usize;
            self.payload_length =
                (self.payload_index[index + 1] - self.payload_index[index]) as usize;
        }
        if self.positions.is_empty() {
            Ok(-1)
        } else {
            Ok(self.positions[(self.position_index + self.i) as usize])
        }
    }

    fn start_offset(&self) -> Result<i32> {
        self.check_position()?;
        if self.start_offsets.is_empty() {
            Ok(-1)
        } else {
            Ok(self.start_offsets[(self.position_index + self.i) as usize])
        }
    }

    fn end_offset(&self) -> Result<i32> {
        self.check_position()?;
        if self.start_offsets.is_empty() {
            Ok(-1)
        } else {
            let index = (self.position_index + self.i) as usize;
            Ok(self.start_offsets[index] + self.lengths[index])
        }
    }

    fn get_payload(&self) -> Result<Option<Cow<BytesRef<Vec<u8>>>>> {
        self.check_position()?;
        if self.payload_index.is_empty() || self.payload.length == 0 {
            Ok(None)
        } else {
            // TODO: always data copy here
            let v = self.payload.bytes
                [self.payload_offset..self.payload_offset + self.payload_length]
                .to_vec();
            let v = BytesRef::from_slice(v, self.payload_offset, self.payload_length);
            Ok(Some(Cow::Owned(v)))
        }
    }
}
