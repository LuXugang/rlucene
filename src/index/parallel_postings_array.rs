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
use crate::index::freq_prox_terms_writer_per_field::FreqProxPostingsArray;
use crate::index::term_vectors_consumer_per_field::TermVectorsPostingsArray;
use crate::util::array_util::ArrayUtil;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::Result;

pub(crate) struct ParallelPostingsArray {
    size: i32,
    text_starts: Vec<i32>, // maps term ID to the term's text start in the bytesHash
    address_offset: Vec<i32>, // maps term ID to current stream address
    byte_starts: Vec<i32>, // maps term ID to stream start offset in the byte pool
}

impl ParallelPostingsArray {
    pub(crate) const BYTES_PER_POSTING: i32 = 3 * BitUtil::INT_BYTES as i32;

    pub(crate) fn new(size: i32) -> Self {
        let vec_size = size as usize;
        Self {
            size,
            text_starts: vec![0; vec_size],
            address_offset: vec![0; vec_size],
            byte_starts: vec![0; vec_size],
        }
    }
}
impl PostingsArrayBase for ParallelPostingsArray {
    fn bytes_per_posting(&self) -> i32 {
        Self::BYTES_PER_POSTING
    }
    fn copy_to(&mut self, new_size: i32) -> Result<()> {
        self.size = new_size;
        ArrayUtil::grow_exact(&mut self.text_starts, new_size)?;
        ArrayUtil::grow_exact(&mut self.address_offset, new_size)?;
        ArrayUtil::grow_exact(&mut self.byte_starts, new_size)?;
        Ok(())
    }
}

pub(crate) trait PostingsArrayBase {
    fn bytes_per_posting(&self) -> i32;
    /// # Note
    /// Diff to Java Lucene, this method used for array growing in Java Lucene,
    /// But In Rust Lucene we do not need to init a new array instead we can just grow the vector.
    #[allow(dead_code)]
    fn new_instance(&self, _size: i32) {}
    /// # Note
    /// Diff to Java Lucene, In Rust Lucene we do not need to init a new array instead we can just grow the vector.
    /// But we still keep this method with same function name for consistent.
    fn copy_to(&mut self, new_size: i32) -> Result<()>;
}
#[allow(unused)]
pub enum PostingsArrayEnum {
    Parallel(ParallelPostingsArray),
    FreqProx(FreqProxPostingsArray),
    TermVectors(TermVectorsPostingsArray),
}
#[allow(unused)]
impl PostingsArrayEnum {
    pub(crate) fn bytes_per_posting(&self) -> i32 {
        match self {
            PostingsArrayEnum::Parallel(p) => p.bytes_per_posting(),
            PostingsArrayEnum::FreqProx(p) => p.bytes_per_posting(),
            PostingsArrayEnum::TermVectors(p) => p.bytes_per_posting(),
        }
    }
    pub(crate) fn grow(&mut self) -> Result<()> {
        let bytes_per_posting = match self {
            PostingsArrayEnum::Parallel(p) => p.bytes_per_posting(),
            PostingsArrayEnum::FreqProx(f) => f.bytes_per_posting(),
            PostingsArrayEnum::TermVectors(t) => t.bytes_per_posting(),
        };
        let size = self.get_size();
        let new_size = ArrayUtil::oversize(size + 1, bytes_per_posting);
        match self {
            PostingsArrayEnum::Parallel(p) => p.copy_to(new_size)?,
            PostingsArrayEnum::FreqProx(f) => f.copy_to(new_size)?,
            PostingsArrayEnum::TermVectors(t) => t.copy_to(new_size)?,
        };
        Ok(())
    }
    pub(crate) fn get_address_offset(&self) -> &[i32] {
        match self {
            PostingsArrayEnum::Parallel(p) => p.address_offset.as_slice(),
            PostingsArrayEnum::FreqProx(f) => f.parent_postings_array.address_offset.as_slice(),
            PostingsArrayEnum::TermVectors(t) => t.parent_postings_array.address_offset.as_slice(),
        }
    }
    pub(crate) fn set_address_offset(&mut self, index: usize, value: i32) {
        match self {
            PostingsArrayEnum::Parallel(p) => p.address_offset[index] = value,
            PostingsArrayEnum::FreqProx(f) => f.parent_postings_array.address_offset[index] = value,
            PostingsArrayEnum::TermVectors(t) => {
                t.parent_postings_array.address_offset[index] = value
            }
        }
    }
    pub(crate) fn get_text_starts(&self) -> &[i32] {
        match self {
            PostingsArrayEnum::Parallel(p) => p.text_starts.as_slice(),
            PostingsArrayEnum::FreqProx(f) => f.parent_postings_array.text_starts.as_slice(),
            PostingsArrayEnum::TermVectors(t) => t.parent_postings_array.text_starts.as_slice(),
        }
    }
    pub(crate) fn set_text_starts(&mut self, index: usize, value: i32) {
        match self {
            PostingsArrayEnum::Parallel(p) => p.text_starts[index] = value,
            PostingsArrayEnum::FreqProx(f) => f.parent_postings_array.text_starts[index] = value,
            PostingsArrayEnum::TermVectors(t) => t.parent_postings_array.text_starts[index] = value,
        }
    }
    pub(crate) fn get_byte_starts(&self) -> &[i32] {
        match self {
            PostingsArrayEnum::Parallel(p) => p.byte_starts.as_slice(),
            PostingsArrayEnum::FreqProx(f) => f.parent_postings_array.byte_starts.as_slice(),
            PostingsArrayEnum::TermVectors(t) => t.parent_postings_array.byte_starts.as_slice(),
        }
    }
    pub(crate) fn set_byte_starts(&mut self, index: usize, value: i32) {
        match self {
            PostingsArrayEnum::Parallel(p) => p.byte_starts[index] = value,
            PostingsArrayEnum::FreqProx(f) => f.parent_postings_array.byte_starts[index] = value,
            PostingsArrayEnum::TermVectors(t) => t.parent_postings_array.byte_starts[index] = value,
        }
    }
    pub(crate) fn get_size(&self) -> i32 {
        match self {
            PostingsArrayEnum::Parallel(p) => p.size,
            PostingsArrayEnum::FreqProx(f) => f.size,
            PostingsArrayEnum::TermVectors(t) => t.size,
        }
    }
}
