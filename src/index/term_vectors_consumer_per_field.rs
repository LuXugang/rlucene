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
use crate::util::array_util::ArrayUtil;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::Result;
#[allow(unused)]
pub(crate) struct TermVectorsConsumerPerField {
    pub(crate) parent_per_field: TermsHashPerField,
    pub(crate) postings_array: Option<PostingsArrayEnum>,
}
#[allow(unused)]
impl TermVectorsConsumerPerField {
    pub(crate) fn new(_size: i32) -> Self {
        todo!()
    }
}
impl TermsHashPerFieldBase for TermVectorsConsumerPerField {
    fn init_stream_slices(&mut self, term_id: i32, doc_id: i32) -> Result<()> {
        self.parent_per_field.init_stream_slices(term_id, doc_id)?;
        self.new_term(term_id, doc_id)
    }

    fn position_stream_slice(&mut self, term_id: i32, doc_id: i32) -> Result<i32> {
        let term_id = self.parent_per_field.position_stream_slice(term_id, doc_id);
        self.add_term(term_id, doc_id)?;
        Ok(term_id)
    }

    fn start(&mut self, _field: &Fields, _first: bool) -> Result<bool> {
        todo!()
    }

    fn new_term(&mut self, _term_id: i32, _doc_id: i32) -> Result<()> {
        todo!()
    }

    fn add_term(&mut self, _term_id: i32, _doc_id: i32) -> Result<()> {
        todo!()
    }

    fn finish(&mut self) {
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
    fn copy_to(&mut self, new_size: i32) -> Result<()> {
        self.parent_postings_array.copy_to(new_size)?;
        self.size = new_size;
        let new_size = new_size as usize;
        ArrayUtil::grow_exact(&mut self.freqs, new_size)?;
        ArrayUtil::grow_exact(&mut self.last_offsets, new_size)?;
        ArrayUtil::grow_exact(&mut self.last_positions, new_size)?;
        Ok(())
    }
}
