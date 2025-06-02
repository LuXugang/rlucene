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
use crate::util::array_util::ArrayUtil;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::Result;
#[allow(unused)]
pub(crate) struct FreqProxTermsWriterPerField {
    pub(crate) postings_array: Option<PostingsArrayEnum>,
}
impl FreqProxTermsWriterPerField {}
#[allow(unused)]
impl TermsHashPerFieldBase for FreqProxTermsWriterPerField {
    fn start(&mut self, _field: &Fields, _first: bool) -> Result<bool> {
        todo!()
    }

    fn new_term<S: TermsHashPerFieldBase>(
        &mut self,
        term_id: i32,
        doc_id: i32,
        per_filed: &mut TermsHashPerField<S>,
    ) -> Result<()> {
        todo!()
    }

    fn add_term<S: TermsHashPerFieldBase>(
        &mut self,
        term_id: i32,
        doc_id: i32,
        per_field: &mut TermsHashPerField<S>,
    ) -> Result<()> {
        todo!()
    }

    fn finish(&mut self) {
        todo!()
    }
}

pub(crate) struct FreqProxPostingsArray {
    pub(crate) size: i32,
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

    fn copy_to(&mut self, new_size: i32) -> Result<()> {
        self.parent_postings_array.copy_to(new_size)?;
        self.size = new_size;
        let new_size = new_size as usize;
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
#[allow(unused)]
impl FreqProx {
    pub fn new(index_options: IndexOptions) -> Self {
        FreqProx { index_options }
    }
}
