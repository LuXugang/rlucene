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
use crate::document::fields::Fields;
use crate::index::index_options::IndexOptions;
use crate::index::parallel_postings_array::{
    ParallelPostingsArray, PostingsArrayBase, PostingsArrayEnum,
};
use crate::index::terms_hash_per_field::{TermsHashPerField, TermsHashPerFieldBase};
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::LuceneError;
use crate::util::VecCopyOps;

pub(crate) struct FreqProxTermsWriterPerField {
    pub(crate) parent_per_field: TermsHashPerField,
    pub(crate) postings_array: Option<PostingsArrayEnum>,
}
impl FreqProxTermsWriterPerField {}
impl TermsHashPerFieldBase for FreqProxTermsWriterPerField {
    fn init_stream_slices(&mut self, term_id: i32, doc_id: i32) -> Result<(), LuceneError> {
        self.parent_per_field.init_stream_slices(term_id, doc_id)?;
        self.new_term(term_id, doc_id)
    }

    fn position_stream_slice(&mut self, term_id: i32, doc_id: i32) -> Result<i32, LuceneError> {
        let term_id = self
            .parent_per_field
            .position_stream_slice(term_id, doc_id)?;
        self.add_term(term_id, doc_id)?;
        Ok(term_id)
    }

    fn start(&mut self, field: &Fields, first: bool) -> Result<bool, LuceneError> {
        todo!()
    }

    fn new_term(&mut self, term_id: i32, doc_id: i32) -> Result<(), LuceneError> {
        todo!()
    }

    fn add_term(&mut self, term_id: i32, doc_id: i32) -> Result<(), LuceneError> {
        todo!()
    }

    fn new_postings_array(&mut self) -> Result<(), LuceneError> {
        todo!()
    }

    fn create_postings_array(&self, size: i32) -> Result<PostingsArrayEnum, LuceneError> {
        todo!()
    }

    fn finish(&mut self) -> Result<(), LuceneError> {
        todo!()
    }
}

pub(crate) struct FreqProxPostingsArray {
    pub(crate) size: i32,
    pub(crate) term_freqs: Option<Vec<i32>>, // # times this term occurs in the current doc
    pub(crate) last_doc_ids: Vec<i32>,       // Last docID where this term occurred
    pub(crate) last_doc_codes: Vec<i32>,     // Code for prior doc
    last_positions: Option<Vec<i32>>,        // Last position where this term occurred
    last_offsets: Option<Vec<i32>>,          // Last endOffset where this term occurred
    pub(crate) parent_postings_array: ParallelPostingsArray,
}
impl FreqProxPostingsArray {
    // Constructor for FreqProxPostingsArray
    pub(crate) fn new(size: i32, write_freqs: bool, write_prox: bool, write_offsets: bool) -> Self {
        let vec_size = size as usize;
        let mut term_freqs = None;
        if write_freqs {
            term_freqs = Some(vec![0; vec_size]);
        }
        let last_positions = if write_prox {
            Some(vec![0; vec_size])
        } else {
            None
        };
        let last_offsets = if write_offsets {
            Some(vec![0; vec_size])
        } else {
            None
        };
        debug_assert!(vec_size <= i32::MAX as usize);
        FreqProxPostingsArray {
            size,
            term_freqs,
            last_doc_ids: vec![0; vec_size],
            last_doc_codes: vec![0; vec_size],
            last_positions,
            last_offsets,
            parent_postings_array: ParallelPostingsArray::new(size),
        }
    }
}

impl PostingsArrayBase for FreqProxPostingsArray {
    fn bytes_per_posting(&self) -> i32 {
        let i32_bytes = BitUtil::INT_BYTES as i32;
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

    fn new_instance(&self, size: i32) -> Self {
        FreqProxPostingsArray::new(
            size,
            self.term_freqs.is_some(),
            self.last_positions.is_some(),
            self.last_offsets.is_some(),
        )
    }

    fn copy_to(&self, to_array: &mut PostingsArrayEnum, num_to_copy: i32) {
        self.parent_postings_array.copy_to(to_array, num_to_copy);
        if let PostingsArrayEnum::FreqProx(to) = to_array {
            let num_to_copy = num_to_copy as usize;
            to.last_doc_ids
                .copy_from(&self.last_doc_ids[..num_to_copy], 0);
            to.last_doc_codes
                .copy_from(&self.last_doc_codes[..num_to_copy], 0);

            if let Some(ref last_positions) = self.last_positions {
                if let Some(ref mut to_positions) = to.last_positions {
                    to_positions.copy_from(&last_positions[..num_to_copy], 0);
                } else {
                    debug_assert!(false, "should never happen");
                }
            }

            if let Some(ref last_offsets) = self.last_offsets {
                if let Some(ref mut to_offsets) = to.last_offsets {
                    to_offsets.copy_from(&last_offsets[..num_to_copy], 0);
                } else {
                    debug_assert!(false, "should never happen");
                }
            }

            if let Some(ref term_freqs) = self.term_freqs {
                if let Some(ref mut to_term_freqs) = to.term_freqs {
                    to_term_freqs.copy_from(&term_freqs[..num_to_copy], 0);
                } else {
                    debug_assert!(false, "should never happen");
                }
            }
        } else {
            debug_assert!(false, "should never happen");
        }
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
