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
use crate::core::index::freq_prox_terms_writer_per_field::FreqProxPostingsArray;
use crate::core::index::term_vectors_consumer_per_field::TermVectorsPostingsArray;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::Result;

pub(crate) struct ParallelPostingsArray {
  size: usize,
  pub(crate) text_starts: Vec<i32>, /* maps term ID to the term's text start in the
                                     * bytesHash  */
  pub(crate) address_offset: Vec<i32>, // maps term ID to current stream address
  pub(crate) byte_starts: Vec<i32>,    /* maps term ID to stream start offset in the
                                        * byte pool  */
}

impl ParallelPostingsArray {
  pub(crate) const BYTES_PER_POSTING: usize = 3 * BitUtil::INT_BYTES;

  pub(crate) fn new(size: usize) -> Self {
    Self {
      size,
      text_starts: vec![0; size],
      address_offset: vec![0; size],
      byte_starts: vec![0; size],
    }
  }
}
impl PostingsArrayBase for ParallelPostingsArray {
  fn bytes_per_posting(&self) -> usize {
    Self::BYTES_PER_POSTING
  }
  fn copy_to(&mut self, new_size: usize) -> Result<()> {
    self.size = new_size;
    ArrayUtil::grow_exact(&mut self.text_starts, new_size)?;
    ArrayUtil::grow_exact(&mut self.address_offset, new_size)?;
    ArrayUtil::grow_exact(&mut self.byte_starts, new_size)?;
    Ok(())
  }
}

pub(crate) trait PostingsArrayBase {
  fn bytes_per_posting(&self) -> usize;
  /// # Note
  /// Diff to Java Lucene, this method used for array growing in Java Lucene,
  /// But In Rust Lucene we do not need to init a new array instead we can
  /// just grow the vector.
  #[allow(dead_code)]
  fn new_instance(&self, _size: i32) {}
  /// # Note
  /// Diff to Java Lucene, In Rust Lucene we do not need to init a new array
  /// instead we can just grow the vector. But we still keep this method
  /// with same function name for consistent.
  fn copy_to(&mut self, new_size: usize) -> Result<()>;
}
pub(crate) enum PostingsArrayEnum {
  Parallel(ParallelPostingsArray),
  FreqProx(FreqProxPostingsArray),
  TermVectors(TermVectorsPostingsArray),
}
impl PostingsArrayEnum {
  pub(crate) fn bytes_per_posting(&self) -> usize {
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
      PostingsArrayEnum::FreqProx(f) => f.parent.address_offset.as_slice(),
      PostingsArrayEnum::TermVectors(t) => t.parent.address_offset.as_slice(),
    }
  }
  pub(crate) fn set_address_offset(&mut self, index: usize, value: i32) {
    match self {
      PostingsArrayEnum::Parallel(p) => p.address_offset[index] = value,
      PostingsArrayEnum::FreqProx(f) => f.parent.address_offset[index] = value,
      PostingsArrayEnum::TermVectors(t) => t.parent.address_offset[index] = value,
    }
  }
  pub(crate) fn get_text_starts(&self) -> &[i32] {
    match self {
      PostingsArrayEnum::Parallel(p) => p.text_starts.as_slice(),
      PostingsArrayEnum::FreqProx(f) => f.parent.text_starts.as_slice(),
      PostingsArrayEnum::TermVectors(t) => t.parent.text_starts.as_slice(),
    }
  }
  pub(crate) fn set_text_starts(&mut self, index: usize, value: i32) {
    match self {
      PostingsArrayEnum::Parallel(p) => p.text_starts[index] = value,
      PostingsArrayEnum::FreqProx(f) => f.parent.text_starts[index] = value,
      PostingsArrayEnum::TermVectors(t) => t.parent.text_starts[index] = value,
    }
  }
  pub(crate) fn get_byte_starts(&self) -> &[i32] {
    match self {
      PostingsArrayEnum::Parallel(p) => p.byte_starts.as_slice(),
      PostingsArrayEnum::FreqProx(f) => f.parent.byte_starts.as_slice(),
      PostingsArrayEnum::TermVectors(t) => t.parent.byte_starts.as_slice(),
    }
  }
  pub(crate) fn set_byte_starts(&mut self, index: usize, value: i32) {
    match self {
      PostingsArrayEnum::Parallel(p) => p.byte_starts[index] = value,
      PostingsArrayEnum::FreqProx(f) => f.parent.byte_starts[index] = value,
      PostingsArrayEnum::TermVectors(t) => t.parent.byte_starts[index] = value,
    }
  }
  pub(crate) fn get_size(&self) -> usize {
    match self {
      PostingsArrayEnum::Parallel(p) => p.size,
      PostingsArrayEnum::FreqProx(f) => f.size,
      PostingsArrayEnum::TermVectors(t) => t.size,
    }
  }
}
