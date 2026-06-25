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
use crate::core::index::byte_slice_pool::ByteSlicePool;
use crate::core::index::byte_slice_reader::ByteSliceReader;
use crate::core::index::field_invert_state::FieldInvertState;
use crate::core::index::freq_prox_terms_writer_per_field::{FreqProx, FreqProxPostingsArray};
use crate::core::index::index_options::IndexOptions;
use crate::core::index::parallel_postings_array::PostingsArrayEnum;
use crate::core::index::term_vectors_consumer_per_field::TermVectorsPostingsArray;
use crate::core::util::attribute_source::AttributeSource;
use crate::core::util::bytes_ref_hash::{BytesRefHash, BytesStartArray};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::int_block_pool::{
  INT_BLOCK_MASK, INT_BLOCK_SHIFT, INT_BLOCK_SIZE, IntBlockPool,
};
use crate::core::util::{
  BYTE_BLOCK_MASK, BYTE_BLOCK_SHIFT, BYTE_BLOCK_SIZE, ByteBlockPool, Counter, SharedCounter,
  SliceCopyOps, size_of_slice,
};
use std::ops::Deref;

/// This struct stores streams of information per term without knowing the size
/// of the stream ahead of time. Each stream typically encodes one level of
/// information, like term frequency per document or term proximity.
///
/// Internally, this struct allocates a linked list of slices that can be read
/// by a [`ByteSliceReader`] for each term. Terms are first deduplicated in a
/// [`BytesRefHash`]. Once this is done, internal data structures point to the
/// current offset of each stream that can be written to.
pub struct TermsHashPerField {
  slice_pool: ByteSlicePool,
  // for each term we store an integer per stream that points into the
  // bytePool above the address is updated once data is written to the
  // stream to point to the next free offset in the terms stream. The
  // start address for the stream is stored in postingsArray.
  // `byte_starts[term_id]` is initialized in `add_term`,
  // either to a brand new per term stream if the term is new or
  // to the addresses where the term stream was written to when we saw it the
  // last time.    term_stream_address_buffer: Vec<i32>,
  term_stream_address_buffer_index: i32,
  stream_address_offset: i32,
  stream_count: i32,
  // This stores the actual term bytes for postings and offsets into the
  // parent hash in the case that this TermsHashPerField is hashing term
  // vectors.
  pub(crate) bytes_hash: BytesRefHash<PostingsBytesStartArray>,
  last_doc_id: i32, // only used with debug/asserts
  pub(crate) do_next_call: bool,
  pub(crate) field_name: String,
  pub(crate) index_options: IndexOptions,
}
impl TermsHashPerField {
  ///  streamCount: how many streams this field stores per term. E.g.
  /// doc(+freq) is 1 stream, prox+offset is a second.
  pub(crate) fn new(
    stream_count: i32,
    bytes_used: SharedCounter,
    postings_array_wrapper: PostingsArrayWrapper,
    field_name: String,
    index_options: IndexOptions,
  ) -> Result<Self> {
    // In the original Java code, we assert that indexOptions !=
    // IndexOptions.NONE.
    debug_assert!(index_options != IndexOptions::None);
    let slice_pool = ByteSlicePool;
    let byte_starts = PostingsBytesStartArray::new(postings_array_wrapper, bytes_used);

    let bytes_hash = BytesRefHash::from_bytes_start_array(HASH_INIT_SIZE, byte_starts)?;
    Ok(TermsHashPerField {
      slice_pool,
      term_stream_address_buffer_index: 0,
      stream_address_offset: 0,
      stream_count,
      bytes_hash,
      last_doc_id: 0,
      do_next_call: false,
      field_name,
      index_options,
    })
  }
  pub(crate) fn init_reader<P>(
    &self,
    reader: &mut ByteSliceReader<P>,
    term_id: i32,
    stream: i32,
    int_pool: &IntBlockPool,
  ) where
    P: Deref<Target = ByteBlockPool>,
  {
    debug_assert!(stream < self.stream_count);
    let term_id = term_id as usize;
    let postings_array_wrapper = &self.bytes_hash.bytes_start_array.per_field;
    let stream_start_offset = postings_array_wrapper
      .postings_array
      .as_ref()
      .unwrap()
      .get_address_offset()[term_id];
    let buffer_index = stream_start_offset >> INT_BLOCK_SHIFT;
    let offset_in_address_buffer = stream_start_offset & INT_BLOCK_MASK;
    let addr;
    {
      let stream_address_buffer = int_pool.get_buffer(buffer_index);
      addr = stream_address_buffer[(offset_in_address_buffer + stream) as usize];
    }
    let init_offset = postings_array_wrapper
      .postings_array
      .as_ref()
      .unwrap()
      .get_byte_starts()[term_id]
      + stream * ByteSlicePool::FIRST_LEVEL_SIZE;
    reader.init(init_offset as usize, addr as usize)
  }
  /// Collapse the hash table and sort in-place; also sets this.sortedTermIDs
  /// to the results. This method must not be called twice unless
  /// [`reset()`](Self::reset) or
  /// [`reinit_hash()`](Self::reinit_hash) was called.
  pub(crate) fn sort_terms(&mut self, byte_pool: &ByteBlockPool) -> Result<()> {
    self.bytes_hash.sort(byte_pool)?;
    Ok(())
  }
  /// Returns the sorted term IDs.
  /// [`sort_terms()`](TermsHashPerField::sort_terms) must be called before.
  pub(crate) fn get_sorted_term_ids(&self) -> &[i32] {
    self.bytes_hash.ids.as_slice()
  }

  pub(crate) fn write_byte(
    &self,
    stream: i32,
    b: u8,
    int_pool: &mut IntBlockPool,
    byte_pool: &mut ByteBlockPool,
  ) -> Result<()> {
    let stream_address = (self.stream_address_offset + stream) as usize;
    let term_stream_address_buffer = int_pool.get_buffer_mut(self.term_stream_address_buffer_index);
    let upto = term_stream_address_buffer[stream_address];
    let mut block_index = (upto >> BYTE_BLOCK_SHIFT) as usize;
    debug_assert!(block_index <= byte_pool.buffer_upto()?);
    let mut bytes = byte_pool.get_buffer_mut(block_index);
    let mut offset = upto & BYTE_BLOCK_MASK;
    if bytes[offset as usize] != 0 {
      // End of slice; allocate a new one
      offset = self
        .slice_pool
        .alloc_slice(block_index, offset, byte_pool)?;
      term_stream_address_buffer[stream_address] = offset + byte_pool.byte_offset;
      // try update bytes
      block_index = byte_pool.buffer_upto()?;
      bytes = byte_pool.get_buffer_mut(block_index);
    }
    bytes[offset as usize] = b;
    term_stream_address_buffer[stream_address] += 1;
    Ok(())
  }
  pub(crate) fn write_bytes(
    &self,
    stream: i32,
    b: &[u8],
    mut offset: usize,
    len: usize,
    int_pool: &mut IntBlockPool,
    byte_pool: &mut ByteBlockPool,
  ) -> Result<()> {
    let end = offset + len;
    let stream_address = (self.stream_address_offset + stream) as usize;

    let term_stream_address_buffer = int_pool.get_buffer_mut(self.term_stream_address_buffer_index);
    let upto = term_stream_address_buffer[stream_address];
    {
      let mut block_index = (upto >> BYTE_BLOCK_SHIFT) as usize;
      debug_assert!(block_index <= byte_pool.buffer_upto()?);
      let slice = byte_pool.get_buffer_mut(block_index);
      let mut slice_offset = (upto & BYTE_BLOCK_MASK) as usize;

      while offset < end && slice[slice_offset] == 0 {
        slice[slice_offset] = b[offset];
        slice_offset += 1;
        offset += 1;
        term_stream_address_buffer[stream_address] += 1;
      }

      while offset < end {
        debug_assert!(slice_offset <= i32::MAX as usize);
        let offset_and_length =
          self
            .slice_pool
            .alloc_known_size_slice(block_index, slice_offset as i32, byte_pool)?;
        slice_offset = (offset_and_length >> 8) as usize;
        let slice_length = offset_and_length & 0xff;
        let buffer_upto = byte_pool.buffer_upto()?;
        block_index = buffer_upto;
        let slice = byte_pool.get_buffer_mut(buffer_upto);
        let write_length = std::cmp::min(slice_length as usize - 1, end - offset);
        slice.copy_from(&b[offset..offset + write_length], slice_offset);
        slice_offset += write_length;
        offset += write_length;
        debug_assert!(slice_offset <= i32::MAX as usize);
        term_stream_address_buffer[stream_address] = slice_offset as i32 + byte_pool.byte_offset;
      }
    }
    Ok(())
  }
  pub(crate) fn write_vint(
    &self,
    stream: i32,
    mut i: i32,
    int_pool: &mut IntBlockPool,
    byte_pool: &mut ByteBlockPool,
  ) -> Result<()> {
    debug_assert!(stream < self.stream_count);
    while (i & !0x7F) != 0 {
      self.write_byte(stream, ((i & 0x7F) | 0x80) as u8, int_pool, byte_pool)?;
      i = ((i as u32) >> 7) as i32;
    }
    self.write_byte(stream, i as u8, int_pool, byte_pool)
  }

  pub(crate) fn get_field_name(&self) -> &str {
    self.field_name.as_str()
  }

  pub(crate) fn get_num_terms(&self) -> i32 {
    self.bytes_hash.size()
  }
  pub(crate) fn reset(&mut self, byte_pool: &mut ByteBlockPool) {
    self.bytes_hash.clear_with_reset_pool(false, byte_pool);
  }

  pub(crate) fn reinit_hash(&mut self) -> Result<()> {
    self.bytes_hash.reinit()
  }

  /// Called when we first encounter a new term. We must allocate slices to
  /// store the postings (vInt compressed doc/freq/prox), and also the int
  /// pointers to where (in our [`ByteBlockPool`] storage) the postings
  /// for this term begin.
  pub(crate) fn init_stream_slices(
    &mut self,
    term_id: i32,
    _doc_id: i32,
    int_pool: &mut IntBlockPool,
    byte_pool: &mut ByteBlockPool,
  ) -> Result<()> {
    if BYTE_BLOCK_SIZE - byte_pool.byte_upto
      < 2 * self.stream_count * ByteSlicePool::FIRST_LEVEL_SIZE
    {
      // can we fit at least one byte per stream in the current
      // buffer, if not allocate a new one
      byte_pool.next_buffer()?;
    }
    let byte_offset = byte_pool.byte_offset;
    {
      if self.stream_count + int_pool.int_upto > INT_BLOCK_SIZE {
        int_pool.next_buffer()?;
      }
      self.term_stream_address_buffer_index = int_pool.buffer_upto;
      self.stream_address_offset = int_pool.int_upto;
      int_pool.int_upto += self.stream_count;
      let postings_array_wrapper = &mut self.bytes_hash.bytes_start_array.per_field;
      debug_assert!(postings_array_wrapper.postings_array.is_some());
      postings_array_wrapper
        .postings_array
        .as_mut()
        .unwrap()
        .set_address_offset(
          term_id as usize,
          self.stream_address_offset + int_pool.int_offset,
        );

      let term_stream_address_buffer =
        int_pool.get_buffer_mut(self.term_stream_address_buffer_index);
      for i in 0..self.stream_count as usize {
        let upto = self
          .slice_pool
          .new_slice(ByteSlicePool::FIRST_LEVEL_SIZE, byte_pool)?;
        term_stream_address_buffer[self.stream_address_offset as usize + i] = upto + byte_offset;
      }
      postings_array_wrapper
        .postings_array
        .as_mut()
        .unwrap()
        .set_byte_starts(
          term_id as usize,
          term_stream_address_buffer[self.stream_address_offset as usize],
        );
    }
    Ok(())
  }

  pub(crate) fn assert_doc_id(&mut self, doc_id: i32) -> bool {
    debug_assert!(
      doc_id >= self.last_doc_id,
      "docID must be >= {} but was: {}",
      self.last_doc_id,
      doc_id
    );
    self.last_doc_id = doc_id;
    true
  }

  pub(crate) fn position_stream_slice(&mut self, term_id: i32, _doc_id: i32) -> Result<i32> {
    let term_id = (-term_id) - 1;
    let postings_array_wrapper = &self.bytes_hash.bytes_start_array.per_field;
    debug_assert!(postings_array_wrapper.postings_array.is_some());
    let int_start = postings_array_wrapper
      .postings_array
      .as_ref()
      .unwrap()
      .get_address_offset()[term_id as usize];
    self.term_stream_address_buffer_index = int_start >> INT_BLOCK_SHIFT;
    self.stream_address_offset = int_start & INT_BLOCK_MASK;
    Ok(term_id)
  }
}

pub(crate) trait TermsHashPerFieldBase {
  /// Called when a term is seen for the first time.
  fn new_term(
    &mut self,
    term_id: i32,
    doc_id: i32,
    state: &mut FieldInvertState,
    attribute_source: &impl AttributeSource,
    int_pool: &mut IntBlockPool,
    byte_pool: &mut ByteBlockPool,
  ) -> Result<()>;
  /// Called when a previously seen term is seen again.
  fn add_term(
    &mut self,
    term_id: i32,
    doc_id: i32,
    state: &mut FieldInvertState,
    attribute_source: &impl AttributeSource,
    int_pool: &mut IntBlockPool,
    byte_pool: &mut ByteBlockPool,
  ) -> Result<()>;
  /// Called when the postings array is initialized or resized.
  /// # Note
  /// In rust Lucene, we do not need to init new postings array
  /// But we still keep this method for consistent with the original Java code
  #[allow(dead_code)]
  fn new_postings_array(&mut self) -> Result<()> {
    Err(LuceneError::not_implemented("should nerve called"))
  }
  /// Creates a new postings array of the specified size.
  /// # Note
  /// In rust Lucene, we do not need to init new postings array
  /// But we still keep this method for consistent with the original Java code
  #[allow(dead_code)]
  fn create_postings_array(&self, _size: i32) -> Result<PostingsArrayEnum> {
    Err(LuceneError::not_implemented(
      "should nerve called".to_string(),
    ))
  }
  fn get_field_name(&self) -> &str;
}
pub(crate) struct PostingsArrayWrapper {
  pub(crate) postings_array: Option<PostingsArrayEnum>,
  pub(crate) terms_hash_per_field_type: TermsHashPerFieldType,
}

impl PostingsArrayWrapper {
  pub fn new(terms_hash_per_field_type: TermsHashPerFieldType) -> Self {
    Self {
      postings_array: None,
      terms_hash_per_field_type,
    }
  }
}

pub(crate) struct PostingsBytesStartArray {
  pub(crate) per_field: PostingsArrayWrapper,
  bytes_used: SharedCounter,
}

impl PostingsBytesStartArray {
  pub(crate) fn new(per_field: PostingsArrayWrapper, bytes_used: SharedCounter) -> Self {
    Self {
      per_field,
      bytes_used,
    }
  }
}
impl BytesStartArray for PostingsBytesStartArray {
  fn init(&mut self) -> Result<()> {
    if self.per_field.postings_array.is_none() {
      self.per_field.postings_array =
        Option::from(self.per_field.terms_hash_per_field_type.new_per_field(2));
      if let Some(ref mut postings_array) = self.per_field.postings_array {
        let byte_used = postings_array.bytes_per_posting() * postings_array.get_size();
        let _ = self.bytes_used.add_and_get(byte_used as i64);
      }
    }
    Ok(())
  }

  fn grow(&mut self) -> Result<()> {
    debug_assert!(self.per_field.postings_array.is_some());
    let postings_array = self.per_field.postings_array.as_mut().unwrap();
    let old_size = postings_array.get_size();
    postings_array.grow()?;
    self.bytes_used.add_and_get(
      (postings_array.bytes_per_posting() * (postings_array.get_size() - old_size)) as i64,
    );
    Ok(())
  }

  fn clear(&mut self) {
    if let Some(postings_array) = self.per_field.postings_array.take() {
      let byte_used = postings_array.bytes_per_posting() * postings_array.get_size();
      debug_assert!(byte_used <= i64::MAX as usize);
      let _ = self.bytes_used.add_and_get(-(byte_used as i64));
    }
  }

  fn bytes_used(&mut self) -> SharedCounter {
    self.bytes_used.clone()
  }

  fn get_value(&self, index: usize) -> i32 {
    debug_assert!(self.per_field.postings_array.is_some());
    self
      .per_field
      .postings_array
      .as_ref()
      .unwrap()
      .get_text_starts()[index]
  }

  fn set_value(&mut self, index: usize, value: i32) {
    debug_assert!(self.per_field.postings_array.is_some());
    self
      .per_field
      .postings_array
      .as_mut()
      .unwrap()
      .set_text_starts(index, value)
  }

  fn len(&self) -> usize {
    debug_assert!(self.per_field.postings_array.is_some());
    self
      .per_field
      .postings_array
      .as_ref()
      .unwrap()
      .get_text_starts()
      .len()
  }

  fn need_init(&self) -> bool {
    self.per_field.postings_array.is_none()
  }

  fn ram_bytes_used(&self) -> Result<i64> {
    match self.per_field.postings_array.as_ref() {
      Some(postings_array) => Ok(size_of_slice(postings_array.get_text_starts())),
      None => Ok(0),
    }
  }
}
pub(crate) enum TermsHashPerFieldType {
  TermVectors,
  FreqProx(FreqProx),
  #[cfg(test)]
  Mock,
}
impl TermsHashPerFieldType {
  pub(crate) fn new_per_field(&self, size: usize) -> PostingsArrayEnum {
    match self {
      TermsHashPerFieldType::TermVectors => {
        PostingsArrayEnum::TermVectors(TermVectorsPostingsArray::new(size))
      },
      TermsHashPerFieldType::FreqProx(f) => {
        let has_freq = f.index_options >= IndexOptions::DocsAndFreqs;
        let has_prox = f.index_options >= IndexOptions::DocsAndFreqsAndPositions;
        let has_offsets = f.index_options >= IndexOptions::DocsAndFreqsAndPositionsAndOffsets;
        PostingsArrayEnum::FreqProx(FreqProxPostingsArray::new(
          size,
          has_freq,
          has_prox,
          has_offsets,
        ))
      },
      #[cfg(test)]
      TermsHashPerFieldType::Mock => {
        PostingsArrayEnum::FreqProx(FreqProxPostingsArray::new(size, true, false, false))
      },
    }
  }
}

pub(crate) const HASH_INIT_SIZE: i32 = 4;
