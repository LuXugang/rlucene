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
    fn new_instance(&self, size: i32) -> ParallelPostingsArray {
        ParallelPostingsArray::new(size)
    }
    fn copy_to(&self, to_array: &mut PostingsArrayEnum, num_to_copy: i32) {
        let num_to_copy = num_to_copy as usize;
        if let PostingsArrayEnum::Parallel(to_array) = to_array {
            to_array.size = self.size;
            to_array.text_starts[..num_to_copy].copy_from_slice(&self.text_starts[..num_to_copy]);
            to_array.address_offset[..num_to_copy]
                .copy_from_slice(&self.address_offset[..num_to_copy]);
            to_array.byte_starts[..num_to_copy].copy_from_slice(&self.byte_starts[..num_to_copy]);
        } else {
            debug_assert!(false, "should never happen");
        }
    }
}

pub(crate) trait PostingsArrayBase {
    fn bytes_per_posting(&self) -> i32;
    fn new_instance(&self, size: i32) -> Self;
    fn copy_to(&self, to_array: &mut PostingsArrayEnum, num_to_copy: i32);
}

pub(crate) enum PostingsArrayEnum {
    Parallel(ParallelPostingsArray),
    FreqProx(FreqProxPostingsArray),
    TermVectors(TermVectorsPostingsArray),
}
impl PostingsArrayEnum {
    pub(crate) fn grow(&mut self, size: i32) -> PostingsArrayEnum {
        let bytes_per_posting = match self {
            PostingsArrayEnum::Parallel(p) => p.bytes_per_posting(),
            PostingsArrayEnum::FreqProx(f) => f.bytes_per_posting(),
            PostingsArrayEnum::TermVectors(t) => t.bytes_per_posting(),
        };
        let new_size = ArrayUtil::oversize(size + 1, bytes_per_posting);
        let mut new_array = match self {
            PostingsArrayEnum::Parallel(p) => PostingsArrayEnum::Parallel(p.new_instance(new_size)),
            PostingsArrayEnum::FreqProx(f) => PostingsArrayEnum::FreqProx(f.new_instance(new_size)),
            PostingsArrayEnum::TermVectors(t) => {
                PostingsArrayEnum::TermVectors(t.new_instance(new_size))
            }
        };
        self.copy_to(&mut new_array, size);
        new_array
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
}
impl PostingsArrayBase for PostingsArrayEnum {
    fn bytes_per_posting(&self) -> i32 {
        match self {
            PostingsArrayEnum::Parallel(p) => p.bytes_per_posting(),
            PostingsArrayEnum::FreqProx(p) => p.bytes_per_posting(),
            PostingsArrayEnum::TermVectors(p) => p.bytes_per_posting(),
        }
    }

    fn new_instance(&self, size: i32) -> Self {
        match self {
            PostingsArrayEnum::Parallel(p) => PostingsArrayEnum::Parallel(p.new_instance(size)),
            PostingsArrayEnum::FreqProx(f) => PostingsArrayEnum::FreqProx(f.new_instance(size)),
            PostingsArrayEnum::TermVectors(t) => {
                PostingsArrayEnum::TermVectors(t.new_instance(size))
            }
        }
    }

    fn copy_to(&self, to_array: &mut PostingsArrayEnum, num_to_copy: i32) {
        match self {
            PostingsArrayEnum::Parallel(p) => p.copy_to(to_array, num_to_copy),
            PostingsArrayEnum::FreqProx(f) => f.copy_to(to_array, num_to_copy),
            PostingsArrayEnum::TermVectors(t) => t.copy_to(to_array, num_to_copy),
        }
    }
}
