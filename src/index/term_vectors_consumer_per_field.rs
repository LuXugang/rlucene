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
use crate::index::parallel_postings_array::{
    ParallelPostingsArray, PostingsArrayBase, PostingsArrayEnum,
};
use crate::index::terms_hash_per_field::{TermsHashPerField, TermsHashPerFieldBase};
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::LuceneError;
use crate::util::VecCopyOps;

pub(crate) struct TermVectorsConsumerPerField {
    pub(crate) parent_per_field: TermsHashPerField,
    pub(crate) postings_array: Option<PostingsArrayEnum>,
}
impl TermVectorsConsumerPerField {
    pub(crate) fn new(size: i32) -> Self {
        todo!()
    }
}
impl TermsHashPerFieldBase for TermVectorsConsumerPerField {
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
pub(crate) struct TermVectorsPostingsArray {
    pub(crate) size: i32,
    freqs: Vec<i32>,          // How many times this term occurred in the current doc
    last_offsets: Vec<i32>,   // Last offset we saw
    last_positions: Vec<i32>, // Last position where this term occurred
    pub(crate) parent_postings_array: ParallelPostingsArray,
}

impl TermVectorsPostingsArray {
    pub fn new(size: i32) -> Self {
        let vec_size = size as usize;
        debug_assert!(vec_size <= i32::MAX as usize);

        TermVectorsPostingsArray {
            size,
            freqs: vec![0; vec_size],
            last_offsets: vec![0; vec_size],
            last_positions: vec![0; vec_size],
            parent_postings_array: ParallelPostingsArray::new(size),
        }
    }
}

impl PostingsArrayBase for TermVectorsPostingsArray {
    fn bytes_per_posting(&self) -> i32 {
        self.parent_postings_array.bytes_per_posting() + 3 * BitUtil::INT_BYTES as i32
    }

    fn new_instance(&self, size: i32) -> Self {
        TermVectorsPostingsArray::new(size)
    }

    fn copy_to(&self, to_array: &mut PostingsArrayEnum, num_to_copy: i32) {
        self.parent_postings_array.copy_to(to_array, num_to_copy);
        if let PostingsArrayEnum::TermVectors(to) = to_array {
            let size = self.size as usize;
            to.freqs.copy_from(&self.freqs[..size], size);
            to.last_offsets.copy_from(&self.last_offsets[..size], size);
            to.last_positions
                .copy_from(&self.last_positions[..size], size);
        } else {
            debug_assert!(false, "should never happen");
        }
    }
}
