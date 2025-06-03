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
use crate::index::parallel_postings_array::{ParallelPostingsArray, PostingsArrayBase};
use crate::index::terms_hash_per_field::{TermsHashPerField, TermsHashPerFieldBase};
use crate::util::array_util::ArrayUtil;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::Result;
pub(crate) struct TermVectorsConsumerPerField {}
impl TermVectorsConsumerPerField {
    pub(crate) fn new(_size: i32) -> Self {
        todo!()
    }
}
impl TermsHashPerFieldBase for TermVectorsConsumerPerField {
    fn start(&mut self, _field: &Fields, _first: bool) -> Result<bool> {
        todo!()
    }

    fn new_term<S: TermsHashPerFieldBase>(
        &mut self,
        term_id: i32,
        doc_id: i32,
        per_field: &mut TermsHashPerField<S>,
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

    fn field_name(&self) -> &str {
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
