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
use crate::index::byte_slice_pool::ByteSlicePool;
use crate::index::byte_slice_reader::ByteSliceReader;
use crate::index::freq_prox_terms_writer_per_field::FreqProxTermsWriterPerField;
use crate::index::index_options::IndexOptions;
use crate::index::parallel_postings_array::PostingsArrayEnum;
use crate::index::term_vectors_consumer_per_field::TermVectorsConsumerPerField;
use crate::index::BytesRef;
use crate::util::bytes_ref_hash::{BytesRefHash, BytesStartArray};
use crate::util::error::lucene_error::LuceneError;
use crate::util::int_block_pool::IntBlockPool;
use crate::util::{ByteBlockPool, CounterEnum};
use std::sync::{Arc, Mutex};
/// This class stores streams of information per term without knowing the size of the stream ahead of
/// time. Each stream typically encodes one level of information, like term frequency per document or
/// term proximity.
///
/// Internally, this class allocates a linked list of slices that can be read by a [`ByteSliceReader`]
/// for each term. Terms are first deduplicated in a [`BytesRefHash`]. Once this is done, internal
/// data structures point to the current offset of each stream that can be written to.
pub struct TermsHashPerField {
    next_per_field: Option<Arc<Mutex<TermsHashPerFieldEnum>>>,
    int_pool: Arc<Mutex<IntBlockPool>>,
    byte_pool: Arc<Mutex<ByteBlockPool>>,
    slice_pool: ByteSlicePool,
    // for each term we store an integer per stream that points into the bytePool above
    // the address is updated once data is written to the stream to point to the next free offset
    // in the terms stream. The start address for the stream is stored in
    // postingsArray.byteStarts[termId]
    // This is initialized in the #addTerm method, either to a brand new per term stream if the term
    // is new or
    // to the addresses where the term stream was written to when we saw it the last time.    term_stream_address_buffer: Vec<i32>,
    term_stream_address_buffer_index: i32,
    stream_address_offset: i32,
    stream_count: i32,
    field_name: String,
    index_options: IndexOptions,
    // This stores the actual term bytes for postings and offsets into the parent hash
    // in the case that this TermsHashPerField is hashing term vectors.
    last_doc_id: i32, // only used with debug/asserts
    sorted_term_ids: bool,
    do_next_call: bool,
}
impl TermsHashPerField {
    const HASH_INIT_SIZE: i32 = 4;
    ///  streamCount: how many streams this field stores per term. E.g. doc(+freq) is 1 stream,
    ///prox+offset is a second.
    pub fn new(
        stream_count: i32,
        int_pool: Arc<Mutex<IntBlockPool>>,
        byte_pool: Arc<Mutex<ByteBlockPool>>,
        field_name: String,
        index_options: IndexOptions,
    ) -> Result<Self, LuceneError> {
        // In the original Java code, we assert that indexOptions != IndexOptions.NONE.
        debug_assert!(index_options != IndexOptions::None);
        let slice_pool = ByteSlicePool::new(byte_pool.clone());
        // Create the BytesRefHash.
        let result = TermsHashPerField {
            next_per_field: None,
            int_pool,
            byte_pool,
            slice_pool,
            term_stream_address_buffer_index: 0,
            stream_address_offset: 0,
            stream_count,
            field_name,
            index_options,
            last_doc_id: 0,
            sorted_term_ids: false,
            do_next_call: false,
        };

        Ok(result)
    }
    pub fn init_reader(
        &self,
        reader: &mut ByteSliceReader,
        postings_array: &mut PostingsArrayEnum,
        term_id: i32,
        stream: i32,
    ) -> Result<(), LuceneError> {
        debug_assert!(stream < self.stream_count);
        let term_id = term_id as usize;
        let stream_start_offset = postings_array.get_address_offset()[term_id];
        let buffer_index = stream_start_offset >> IntBlockPool::INT_BLOCK_SHIFT;
        let offset_in_address_buffer = stream_start_offset & IntBlockPool::INT_BLOCK_MASK;
        let addr;
        {
            let mut int_pool = self
                .int_pool
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
            let stream_address_buffer = int_pool.get_buffer(buffer_index);
            addr = stream_address_buffer[(offset_in_address_buffer + stream) as usize];
        }
        let init_offset =
            postings_array.get_byte_starts()[term_id] + stream * ByteSlicePool::FIRST_LEVEL_SIZE;
        reader.init(self.byte_pool.clone(), init_offset, addr)
    }
    /// Collapse the hash table and sort in-place; also sets this.sortedTermIDs to the results.
    /// This method must not be called twice unless [`reset()`](TermsHashPerFieldBase::reset) or [`reinit_hash()`](TermsHashPerFieldBase::reinit_hash) was called.
    pub(crate) fn sort_terms(&mut self, bytes_hash: &mut BytesRefHash) -> Result<(), LuceneError> {
        debug_assert!(!self.sorted_term_ids);
        bytes_hash.sort()?;
        self.sorted_term_ids = true;
        Ok(())
    }
    /// Returns the sorted term IDs. [`sort_terms()`](TermsHashPerField::sort_terms) must be called before.
    pub(crate) fn get_sorted_term_ids<'a>(&self, bytes_hash: &'a BytesRefHash) -> &'a [i32] {
        debug_assert!(!self.sorted_term_ids);
        bytes_hash.ids.as_slice()
    }
    fn assert_doc_id(&mut self, doc_id: i32) -> bool {
        debug_assert!(
            doc_id >= self.last_doc_id,
            "docID must be >= {} but was: {}",
            self.last_doc_id,
            doc_id
        );
        self.last_doc_id = doc_id;
        true
    }
    pub(crate) fn write_byte(&mut self, stream: i32, b: u8) -> Result<(), LuceneError> {
        let stream_address = (self.stream_address_offset + stream) as usize;
        let mut int_pool = self
            .int_pool
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        let term_stream_address_buffer = int_pool.get_buffer(self.term_stream_address_buffer_index);
        let upto = term_stream_address_buffer[stream_address];
        let mut byte_pool = self
            .byte_pool
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        let block_index = upto >> ByteBlockPool::BYTE_BLOCK_SHIFT;
        debug_assert!(block_index <= byte_pool.buffer_upto);
        let bytes = byte_pool.get_buffer(block_index);
        let offset = upto & ByteBlockPool::BYTE_BLOCK_MASK;
        let value = bytes[offset as usize];
        drop(byte_pool);
        let mut byte_pool;
        let new_offset =
            if value != 0 {
                // End of slice; allocate a new one
                let allocated_offset = self.slice_pool.alloc_slice(block_index, offset)?;
                byte_pool = self.byte_pool.lock().map_err(|_| {
                    LuceneError::illegal_state("Failed to acquire lock.".to_string())
                })?;
                term_stream_address_buffer[stream_address] = offset + byte_pool.byte_offset;
                allocated_offset
            } else {
                byte_pool = self.byte_pool.lock().map_err(|_| {
                    LuceneError::illegal_state("Failed to acquire lock.".to_string())
                })?;
                offset
            };
        let bytes = byte_pool.get_buffer(block_index);
        bytes[new_offset as usize] = b;
        term_stream_address_buffer[stream_address] += 1;
        Ok(())
    }
    pub(crate) fn write_bytes(
        &mut self,
        stream: i32,
        b: &[u8],
        offset: i32,
        len: i32,
    ) -> Result<(), LuceneError> {
        let mut offset = offset as usize;
        let end = offset + len as usize;
        let stream_address = (self.stream_address_offset + stream) as usize;

        let mut int_pool = self
            .int_pool
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        let term_stream_address_buffer = int_pool.get_buffer(self.term_stream_address_buffer_index);
        let upto = term_stream_address_buffer[stream_address];
        {
            let mut byte_pool = self
                .byte_pool
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
            let block_index = upto >> ByteBlockPool::BYTE_BLOCK_SHIFT;
            debug_assert!(block_index <= byte_pool.buffer_upto);
            let slice = byte_pool.get_buffer(block_index);
            let mut slice_offset = (upto & ByteBlockPool::BYTE_BLOCK_MASK) as usize;

            while offset < end && slice[slice_offset] == 0 {
                slice[slice_offset] = b[offset];
                slice_offset += 1;
                offset += 1;
                term_stream_address_buffer[stream_address] += 1;
            }

            drop(byte_pool);
            while offset < end {
                debug_assert!(slice_offset <= i32::MAX as usize);
                let offset_and_length = self
                    .slice_pool
                    .alloc_known_size_slice(block_index, slice_offset as i32)?;
                slice_offset = (offset_and_length >> 8) as usize;
                let slice_length = offset_and_length & 0xff;
                let mut byte_pool = self.byte_pool.lock().map_err(|_| {
                    LuceneError::illegal_state("Failed to acquire lock.".to_string())
                })?;
                let buffer_upto = byte_pool.buffer_upto;
                let slice = byte_pool.get_buffer(buffer_upto);
                let write_length = std::cmp::min(slice_length as usize - 1, end - offset);
                slice[slice_offset..].copy_from_slice(&b[offset..offset + write_length]);
                slice_offset += write_length;
                offset += write_length;
                debug_assert!(slice_offset <= i32::MAX as usize);
                term_stream_address_buffer[stream_address] =
                    slice_offset as i32 + byte_pool.byte_offset;
            }
        }
        Ok(())
    }
    pub(crate) fn write_vint(&mut self, stream: i32, mut i: i32) -> Result<(), LuceneError> {
        debug_assert!(stream < self.stream_count);
        while (i & !0x7F) != 0 {
            self.write_byte(stream, ((i & 0x7F) | 0x80) as u8)?;
            i = ((i as u32) >> 7) as i32 + i;
        }
        self.write_byte(stream, i as u8)?;
        Ok(())
    }

    pub(crate) fn get_next_per_field(&self) -> Arc<Mutex<TermsHashPerFieldEnum>> {
        debug_assert!(self.next_per_field.is_some());
        self.next_per_field.as_ref().unwrap().clone()
    }

    pub(crate) fn get_field_name(&self) -> &str {
        &self.field_name
    }
    pub(crate) fn get_num_terms(&self, bytes_ref_hash: &BytesRefHash) -> i32 {
        bytes_ref_hash.size()
    }
}
impl TermsHashPerFieldBase for TermsHashPerField {
    fn reset(&mut self, bytes_hash: &mut BytesRefHash) -> Result<(), LuceneError> {
        bytes_hash.clear()?;
        self.sorted_term_ids = false;
        if self.next_per_field.is_some() {
            let mut next_per_field =
                self.next_per_field.as_ref().unwrap().lock().map_err(|_| {
                    LuceneError::illegal_state("Failed to acquire lock.".to_string())
                })?;
            next_per_field.reset(bytes_hash)?;
        }
        Ok(())
    }

    fn reinit_hash(&mut self, bytes_hash: &mut BytesRefHash) -> Result<(), LuceneError> {
        self.sorted_term_ids = false;
        bytes_hash.reinit()
    }

    fn add_with_text_start(
        &mut self,
        bytes_hash: &mut BytesRefHash,
        text_start: i32,
        doc_id: i32,
        postings_array: &mut PostingsArrayEnum,
    ) -> Result<(), LuceneError> {
        let term_id = bytes_hash.add_by_pool_offset(text_start)?;
        if term_id >= 0 {
            self.init_stream_slices(term_id, doc_id, postings_array)?;
        } else {
            self.position_stream_slice(term_id, doc_id, postings_array)?;
        }
        Ok(())
    }
    /// Called once per inverted token. This is the primary entry point (for the first `TermsHash`);
    /// postings use this API.
    fn add_with_bytes_ref(
        &mut self,
        bytes_hash: &mut BytesRefHash,
        term_bytes: &BytesRef,
        doc_id: i32,
        postings_array: &mut PostingsArrayEnum,
    ) -> Result<(), LuceneError> {
        debug_assert!(self.assert_doc_id(doc_id));
        // We are first in the chain so we must "intern" the
        // term text into textStart address
        // Get the text & hash of this term.
        let mut term_id = bytes_hash.add(term_bytes)?;
        if term_id >= 0 {
            self.init_stream_slices(term_id, doc_id, postings_array)?;
        } else {
            term_id = self.position_stream_slice(term_id, doc_id, postings_array)?;
        }
        if self.do_next_call {
            debug_assert!(self.next_per_field.is_some());
            if let Some(ref next_per_field) = self.next_per_field {
                next_per_field
                    .lock()
                    .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?
                    .add_with_text_start(
                        bytes_hash,
                        postings_array.get_text_starts()[term_id as usize],
                        doc_id,
                        postings_array,
                    )?;
            }
        }
        Ok(())
    }

    /// Called when we first encounter a new term. We must allocate slices to store the postings
    /// (vInt compressed doc/freq/prox), and also the int pointers to where (in our [`ByteBlockPool`]
    /// storage) the postings for this term begin.
    fn init_stream_slices(
        &mut self,
        term_id: i32,
        _doc_id: i32,
        postings_array: &mut PostingsArrayEnum,
    ) -> Result<(), LuceneError> {
        let byte_offset;
        {
            let mut byte_pool = self.byte_pool.lock().unwrap();
            if ByteBlockPool::BYTE_BLOCK_SIZE - byte_pool.byte_upto
                < 2 * self.stream_count * ByteSlicePool::FIRST_LEVEL_SIZE
            {
                // can we fit at least one byte per stream in the current buffer, if not allocate a new one
                byte_pool.next_buffer()?;
            }
            byte_offset = byte_pool.byte_offset;
        }
        {
            let mut int_pool = self
                .int_pool
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
            if self.stream_count + int_pool.int_upto > IntBlockPool::INT_BLOCK_SIZE {
                int_pool.next_buffer()?;
            }
            self.term_stream_address_buffer_index = int_pool.buffer_upto;
            self.stream_address_offset = int_pool.int_upto;
            int_pool.int_upto += self.stream_count;
            postings_array.set_address_offset(
                term_id as usize,
                self.stream_address_offset + int_pool.int_offset,
            );

            let term_stream_address_buffer =
                int_pool.get_buffer(self.term_stream_address_buffer_index);
            for i in 0..self.stream_count as usize {
                let upto = self.slice_pool.new_slice(ByteSlicePool::FIRST_LEVEL_SIZE)?;
                term_stream_address_buffer[self.stream_address_offset as usize + i] =
                    upto + byte_offset;
            }
            postings_array.set_byte_starts(
                term_id as usize,
                term_stream_address_buffer[self.stream_address_offset as usize],
            );
        }
        Ok(())
    }

    fn position_stream_slice(
        &mut self,
        term_id: i32,
        _doc_id: i32,
        postings_array: &mut PostingsArrayEnum,
    ) -> Result<i32, LuceneError> {
        let term_id = (-term_id) - 1;
        let int_start = postings_array.get_address_offset()[term_id as usize];
        {
            let int_pool = self
                .int_pool
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
            let buffer_index = int_start >> IntBlockPool::INT_BLOCK_SHIFT;
            self.term_stream_address_buffer_index = int_pool.buffer_upto;
        }
        self.stream_address_offset = int_start & IntBlockPool::INT_BLOCK_MASK;
        Ok(term_id)
    }

    fn start(&mut self, field: &Fields, first: bool) -> Result<bool, LuceneError> {
        match self.next_per_field {
            Some(ref next_per_field) => {
                let mut next_per_field = next_per_field.lock().map_err(|_| {
                    LuceneError::illegal_state("Failed to acquire lock.".to_string())
                })?;
                next_per_field.start(field, first)
            }
            None => Ok(true),
        }
    }

    fn new_term(&mut self, _term_id: i32, _doc_id: i32) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented("should not be called"))
    }

    fn add_term(&mut self, _term_id: i32, _doc_id: i32) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented("should not be called"))
    }

    fn new_postings_array(&mut self) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented("should not be called"))
    }

    fn create_postings_array(&self, _size: usize) -> Result<PostingsArrayEnum, LuceneError> {
        Err(LuceneError::not_implemented("should not be called"))
    }

    fn finish(&mut self) -> Result<(), LuceneError> {
        match self.next_per_field {
            Some(ref next_per_field) => next_per_field
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?
                .finish(),
            None => Ok(()),
        }
    }
}

pub(crate) trait TermsHashPerFieldBase {
    fn reset(&mut self, bytes_hash: &mut BytesRefHash) -> Result<(), LuceneError>;
    fn reinit_hash(&mut self, bytes_hash: &mut BytesRefHash) -> Result<(), LuceneError>;
    fn add_with_text_start(
        &mut self,
        bytes_hash: &mut BytesRefHash,
        text_start: i32,
        doc_id: i32,
        postings_array: &mut PostingsArrayEnum,
    ) -> Result<(), LuceneError>;
    /// Called once per inverted token. This is the primary entry point (for first TermsHash); postings
    /// use this API.
    fn add_with_bytes_ref(
        &mut self,
        bytes_hash: &mut BytesRefHash,
        term_bytes: &BytesRef,
        doc_id: i32,
        postings_array: &mut PostingsArrayEnum,
    ) -> Result<(), LuceneError>;
    /// Called when we first encounter a new term. We must allocate slies to store the postings (vInt
    /// compressed doc/freq/prox), and also the int pointers to where (in our {@link ByteBlockPool}
    ///storage) the postings for this term begin.
    fn init_stream_slices(
        &mut self,
        term_id: i32,
        doc_id: i32,
        postings_array: &mut PostingsArrayEnum,
    ) -> Result<(), LuceneError>;
    fn position_stream_slice(
        &mut self,
        term_id: i32,
        doc_id: i32,
        postings_array_enum: &mut PostingsArrayEnum,
    ) -> Result<i32, LuceneError>;
    ///Start adding a new field instance; first is true if this is the first time this field name was
    ///seen in the document.
    fn start(&mut self, field: &Fields, first: bool) -> Result<bool, LuceneError>;
    /// Called when a term is seen for the first time.
    fn new_term(&mut self, term_id: i32, doc_id: i32) -> Result<(), LuceneError>;
    /// Called when a previously seen term is seen again.
    fn add_term(&mut self, term_id: i32, doc_id: i32) -> Result<(), LuceneError>;
    /// Called when the postings array is initialized or resized.
    fn new_postings_array(&mut self) -> Result<(), LuceneError>;
    /// Creates a new postings array of the specified size.
    fn create_postings_array(&self, size: usize) -> Result<PostingsArrayEnum, LuceneError>;
    /// Finish adding all instances of this field to the current document.
    fn finish(&mut self) -> Result<(), LuceneError>;
}
pub struct PostingsBytesStartArray {}
impl PostingsBytesStartArray {
    pub fn new(
        per_field: Arc<Mutex<TermsHashPerFieldEnum>>,
        bytes_used: Arc<Mutex<CounterEnum>>,
    ) -> Self {
        PostingsBytesStartArray {}
    }
}
impl BytesStartArray for PostingsBytesStartArray {
    fn init(&mut self) -> &Vec<i32> {
        todo!()
    }

    fn grow(&mut self) -> Result<(), LuceneError> {
        todo!()
    }

    fn clear(&mut self) -> Result<(), LuceneError> {
        todo!()
    }

    fn bytes_used(&mut self) -> Arc<Mutex<CounterEnum>> {
        todo!()
    }

    fn byte_start(&mut self) -> &mut Option<Vec<i32>> {
        todo!()
    }
}

pub(crate) enum TermsHashPerFieldEnum {
    TermVectorsConsumer(TermVectorsConsumerPerField),
    FreqProxTermsWriter(FreqProxTermsWriterPerField),
    #[cfg(test)]
    Mock(TermsHashPerFieldMock),
}
impl TermsHashPerFieldEnum {
    pub(crate) fn finish(&mut self) -> Result<(), LuceneError> {
        match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(t) => t.finish(),
            TermsHashPerFieldEnum::FreqProxTermsWriter(f) => f.finish(),
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(m) => m.finish(),
        }
    }
}
impl TermsHashPerFieldBase for TermsHashPerFieldEnum {
    fn reset(&mut self, bytes_hash: &mut BytesRefHash) -> Result<(), LuceneError> {
        todo!()
    }

    fn reinit_hash(&mut self, bytes_hash: &mut BytesRefHash) -> Result<(), LuceneError> {
        todo!()
    }

    fn add_with_text_start(
        &mut self,
        bytes_hash: &mut BytesRefHash,
        text_start: i32,
        doc_id: i32,
        postings_array: &mut PostingsArrayEnum,
    ) -> Result<(), LuceneError> {
        todo!()
    }

    fn add_with_bytes_ref(
        &mut self,
        bytes_hash: &mut BytesRefHash,
        term_bytes: &BytesRef,
        doc_id: i32,
        postings_array: &mut PostingsArrayEnum,
    ) -> Result<(), LuceneError> {
        todo!()
    }

    fn init_stream_slices(
        &mut self,
        term_id: i32,
        doc_id: i32,
        postings_array: &mut PostingsArrayEnum,
    ) -> Result<(), LuceneError> {
        todo!()
    }

    fn position_stream_slice(
        &mut self,
        term_id: i32,
        doc_id: i32,
        postings_array_enum: &mut PostingsArrayEnum,
    ) -> Result<i32, LuceneError> {
        todo!()
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

    fn create_postings_array(&self, size: usize) -> Result<PostingsArrayEnum, LuceneError> {
        todo!()
    }
    fn finish(&mut self) -> Result<(), LuceneError> {
        todo!()
    }
}
#[cfg(test)]
pub(crate) struct TermsHashPerFieldMock;
#[cfg(test)]
impl TermsHashPerFieldBase for TermsHashPerFieldMock {
    fn reset(&mut self, bytes_hash: &mut BytesRefHash) -> Result<(), LuceneError> {
        todo!()
    }

    fn reinit_hash(&mut self, bytes_hash: &mut BytesRefHash) -> Result<(), LuceneError> {
        todo!()
    }

    fn add_with_text_start(
        &mut self,
        bytes_hash: &mut BytesRefHash,
        text_start: i32,
        doc_id: i32,
        postings_array: &mut PostingsArrayEnum,
    ) -> Result<(), LuceneError> {
        todo!()
    }

    fn add_with_bytes_ref(
        &mut self,
        bytes_hash: &mut BytesRefHash,
        term_bytes: &BytesRef,
        doc_id: i32,
        postings_array: &mut PostingsArrayEnum,
    ) -> Result<(), LuceneError> {
        todo!()
    }

    fn init_stream_slices(
        &mut self,
        term_id: i32,
        doc_id: i32,
        postings_array: &mut PostingsArrayEnum,
    ) -> Result<(), LuceneError> {
        self.new_term(term_id, doc_id)
    }

    fn position_stream_slice(
        &mut self,
        term_id: i32,
        doc_id: i32,
        postings_array_enum: &mut PostingsArrayEnum,
    ) -> Result<i32, LuceneError> {
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

    fn create_postings_array(&self, size: usize) -> Result<PostingsArrayEnum, LuceneError> {
        todo!()
    }

    fn finish(&mut self) -> Result<(), LuceneError> {
        todo!()
    }
}
#[cfg(test)]
mod tests {
    
    
    
    
    
    
}
