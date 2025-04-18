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
use crate::index::freq_prox_terms_writer_per_field::{FreqProx, FreqProxPostingsArray};
use crate::index::index_options::IndexOptions;
use crate::index::parallel_postings_array::PostingsArrayEnum;
use crate::index::term_vectors_consumer_per_field::TermVectorsPostingsArray;
use crate::index::terms_hash_per_field_enum::TermsHashPerFieldEnum;
use crate::util::access::Access;
use crate::util::bytes_ref_hash::{
    BytesRefHash, BytesStartArray, BytesStartArrayEnum, STBytesRefHash,
};
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::int_block_pool::IntBlockPool;
use crate::util::{ByteBlockPool, ByteBlockPoolBorrow, Counter, CounterEnum, SliceCopyOps};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// This struct stores streams of information per term without knowing the size of the stream ahead of
/// time. Each stream typically encodes one level of information, like term frequency per document or
/// term proximity.
///
/// Internally, this struct allocates a linked list of slices that can be read by a [`ByteSliceReader`]
/// for each term. Terms are first deduplicated in a [`BytesRefHash`]. Once this is done, internal
/// data structures point to the current offset of each stream that can be written to.
#[allow(unused)]
pub struct TermsHashPerField {
    pub(crate) next_per_field: Option<Rc<RefCell<TermsHashPerFieldEnum>>>,
    int_pool: Rc<RefCell<IntBlockPool>>,
    pub(crate) byte_pool: ByteBlockPoolBorrow,
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
    pub(crate) bytes_hash: STBytesRefHash,
    last_doc_id: i32, // only used with debug/asserts
    sorted_term_ids: bool,
    pub(crate) do_next_call: bool,
    pub(crate) postings_array_wrapper: Rc<RefCell<PostingsArrayWrapper>>,
}
pub(crate) struct PostingsArrayWrapper {
    pub(crate) postings_array: Option<PostingsArrayEnum>,
    pub(crate) terms_hash_per_field_type: TermsHashPerFieldType,
}
/// for multi-threaded scenarios
pub type MTPostingsArrayWrapper = Arc<Mutex<PostingsArrayWrapper>>;
/// for single-threaded scenarios
pub type STPostingsArrayWrapper = Rc<RefCell<PostingsArrayWrapper>>;
#[allow(unused)]
impl PostingsArrayWrapper {
    pub fn new(terms_hash_per_field_type: TermsHashPerFieldType) -> Self {
        Self {
            postings_array: None,
            terms_hash_per_field_type,
        }
    }
}
#[allow(unused)]
impl TermsHashPerField {
    const HASH_INIT_SIZE: i32 = 4;
    ///  streamCount: how many streams this field stores per term. E.g. doc(+freq) is 1 stream,
    ///prox+offset is a second.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        stream_count: i32,
        int_pool: Rc<RefCell<IntBlockPool>>,
        byte_pool: ByteBlockPoolBorrow,
        term_byte_pool: ByteBlockPoolBorrow,
        bytes_used: Rc<RefCell<CounterEnum>>,
        next_per_field: Option<Rc<RefCell<TermsHashPerFieldEnum>>>,
        field_name: String,
        index_options: IndexOptions,
        postings_array_wrapper: Rc<RefCell<PostingsArrayWrapper>>,
    ) -> Result<Self> {
        // In the original Java code, we assert that indexOptions != IndexOptions.NONE.
        debug_assert!(index_options != IndexOptions::None);
        let slice_pool = ByteSlicePool::new(byte_pool.clone());
        let byte_starts = Rc::new(RefCell::new(BytesStartArrayEnum::Postings(
            PostingsBytesStartArray {
                per_field: postings_array_wrapper.clone(),
                bytes_used,
            },
        )));

        let bytes_hash = BytesRefHash::from_bytes_start_array(
            term_byte_pool,
            TermsHashPerField::HASH_INIT_SIZE,
            byte_starts,
        )?;

        Ok(TermsHashPerField {
            next_per_field,
            int_pool,
            byte_pool,
            slice_pool,
            term_stream_address_buffer_index: 0,
            stream_address_offset: 0,
            stream_count,
            field_name,
            index_options,
            bytes_hash,
            last_doc_id: 0,
            sorted_term_ids: false,
            do_next_call: false,
            postings_array_wrapper,
        })
    }
    pub(crate) fn init_reader(&self, reader: &mut ByteSliceReader, term_id: i32, stream: i32) {
        debug_assert!(stream < self.stream_count);
        let term_id = term_id as usize;
        let postings_array_wrapper = self.postings_array_wrapper.borrow_mut();
        let stream_start_offset = postings_array_wrapper
            .postings_array
            .as_ref()
            .unwrap()
            .get_address_offset()[term_id];
        let buffer_index = stream_start_offset >> IntBlockPool::INT_BLOCK_SHIFT;
        let offset_in_address_buffer = stream_start_offset & IntBlockPool::INT_BLOCK_MASK;
        let addr;
        {
            let mut int_pool = self.int_pool.borrow_mut();
            let stream_address_buffer = int_pool.get_buffer(buffer_index);
            addr = stream_address_buffer[(offset_in_address_buffer + stream) as usize];
        }
        let init_offset = postings_array_wrapper
            .postings_array
            .as_ref()
            .unwrap()
            .get_byte_starts()[term_id]
            + stream * ByteSlicePool::FIRST_LEVEL_SIZE;
        reader.init(self.byte_pool.clone(), init_offset, addr)
    }
    /// Collapse the hash table and sort in-place; also sets this.sortedTermIDs to the results.
    /// This method must not be called twice unless [`reset()`](TermsHashPerFieldBase::reset) or [`reinit_hash()`](TermsHashPerFieldBase::reinit_hash) was called.
    pub(crate) fn sort_terms(&mut self, bytes_hash: &mut STBytesRefHash) -> Result<()> {
        debug_assert!(!self.sorted_term_ids);
        bytes_hash.sort()?;
        self.sorted_term_ids = true;
        Ok(())
    }
    /// Returns the sorted term IDs. [`sort_terms()`](TermsHashPerField::sort_terms) must be called before.
    pub(crate) fn get_sorted_term_ids<'a>(&self, bytes_hash: &'a STBytesRefHash) -> &'a [i32] {
        debug_assert!(!self.sorted_term_ids);
        bytes_hash.ids.as_slice()
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
    pub(crate) fn write_byte(&mut self, stream: i32, b: u8) -> Result<()> {
        let stream_address = (self.stream_address_offset + stream) as usize;
        let mut int_pool = self.int_pool.borrow_mut();
        let term_stream_address_buffer = int_pool.get_buffer(self.term_stream_address_buffer_index);
        let upto = term_stream_address_buffer[stream_address];
        let mut byte_pool = self.byte_pool.borrow_mut();
        let block_index = upto >> ByteBlockPool::BYTE_BLOCK_SHIFT;
        debug_assert!(block_index <= byte_pool.buffer_upto);
        let bytes = byte_pool.get_buffer(block_index);
        let offset = upto & ByteBlockPool::BYTE_BLOCK_MASK;
        let value = bytes[offset as usize];
        drop(byte_pool);
        let mut byte_pool;
        let new_offset = if value != 0 {
            // End of slice; allocate a new one
            let allocated_offset = self.slice_pool.alloc_slice(block_index, offset)?;
            byte_pool = self.byte_pool.borrow_mut();
            term_stream_address_buffer[stream_address] = allocated_offset + byte_pool.byte_offset;
            allocated_offset
        } else {
            byte_pool = self.byte_pool.borrow_mut();
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
    ) -> Result<()> {
        let mut offset = offset as usize;
        let end = offset + len as usize;
        let stream_address = (self.stream_address_offset + stream) as usize;

        let mut int_pool = self.int_pool.borrow_mut();
        let term_stream_address_buffer = int_pool.get_buffer(self.term_stream_address_buffer_index);
        let upto = term_stream_address_buffer[stream_address];
        {
            let mut byte_pool = self.byte_pool.borrow_mut();
            let mut block_index = upto >> ByteBlockPool::BYTE_BLOCK_SHIFT;
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
                let mut byte_pool = self.byte_pool.borrow_mut();
                let buffer_upto = byte_pool.buffer_upto;
                block_index = buffer_upto;
                let slice = byte_pool.get_buffer(buffer_upto);
                let write_length = std::cmp::min(slice_length as usize - 1, end - offset);
                slice.copy_from(&b[offset..offset + write_length], slice_offset);
                slice_offset += write_length;
                offset += write_length;
                debug_assert!(slice_offset <= i32::MAX as usize);
                term_stream_address_buffer[stream_address] =
                    slice_offset as i32 + byte_pool.byte_offset;
            }
        }
        Ok(())
    }
    pub(crate) fn write_vint(&mut self, stream: i32, mut i: i32) -> Result<()> {
        debug_assert!(stream < self.stream_count);
        while (i & !0x7F) != 0 {
            self.write_byte(stream, ((i & 0x7F) | 0x80) as u8)?;
            i = ((i as u32) >> 7) as i32;
        }
        self.write_byte(stream, i as u8)
    }

    pub(crate) fn get_next_per_field(&self) -> Rc<RefCell<TermsHashPerFieldEnum>> {
        debug_assert!(self.next_per_field.is_some());
        self.next_per_field.as_ref().unwrap().clone()
    }

    pub(crate) fn get_field_name(&self) -> &str {
        &self.field_name
    }
    fn finish(&mut self) {
        if let Some(ref next_per_field) = self.next_per_field {
            next_per_field.borrow_mut().finish()
        }
    }
    pub(crate) fn get_num_terms(&self, bytes_ref_hash: &STBytesRefHash) -> i32 {
        bytes_ref_hash.size()
    }
    pub(crate) fn reset(&mut self) {
        self.bytes_hash.clear();
        self.sorted_term_ids = false;
        if self.next_per_field.is_some() {
            let mut next_per_field = self.next_per_field.as_ref().unwrap().borrow_mut();
            next_per_field.reset();
        }
    }

    pub(crate) fn reinit_hash(&mut self) -> Result<()> {
        self.sorted_term_ids = false;
        self.bytes_hash.reinit()
    }
    /// Called when we first encounter a new term. We must allocate slices to store the postings
    /// (vInt compressed doc/freq/prox), and also the int pointers to where (in our [`ByteBlockPool`]
    /// storage) the postings for this term begin.
    pub(crate) fn init_stream_slices(&mut self, term_id: i32, _doc_id: i32) -> Result<()> {
        let byte_offset;
        {
            let mut byte_pool = self.byte_pool.borrow_mut();
            if ByteBlockPool::BYTE_BLOCK_SIZE - byte_pool.byte_upto
                < 2 * self.stream_count * ByteSlicePool::FIRST_LEVEL_SIZE
            {
                // can we fit at least one byte per stream in the current buffer, if not allocate a new one
                byte_pool.next_buffer()?;
            }
            byte_offset = byte_pool.byte_offset;
        }
        {
            let mut int_pool = self.int_pool.borrow_mut();
            if self.stream_count + int_pool.int_upto > IntBlockPool::INT_BLOCK_SIZE {
                int_pool.next_buffer()?;
            }
            self.term_stream_address_buffer_index = int_pool.buffer_upto;
            self.stream_address_offset = int_pool.int_upto;
            int_pool.int_upto += self.stream_count;
            let mut postings_array_wrapper = self.postings_array_wrapper.borrow_mut();
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
                int_pool.get_buffer(self.term_stream_address_buffer_index);
            for i in 0..self.stream_count as usize {
                let upto = self.slice_pool.new_slice(ByteSlicePool::FIRST_LEVEL_SIZE)?;
                term_stream_address_buffer[self.stream_address_offset as usize + i] =
                    upto + byte_offset;
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

    pub(crate) fn position_stream_slice(&mut self, term_id: i32, _doc_id: i32) -> i32 {
        let term_id = (-term_id) - 1;
        let postings_array_wrapper = self.postings_array_wrapper.borrow_mut();
        debug_assert!(postings_array_wrapper.postings_array.is_some());
        let int_start = postings_array_wrapper
            .postings_array
            .as_ref()
            .unwrap()
            .get_address_offset()[term_id as usize];
        self.term_stream_address_buffer_index = int_start >> IntBlockPool::INT_BLOCK_SHIFT;
        self.stream_address_offset = int_start & IntBlockPool::INT_BLOCK_MASK;
        term_id
    }
    fn start(&mut self, field: &Fields, first: bool) -> Result<bool> {
        match self.next_per_field {
            Some(ref next_per_field) => {
                let mut next_per_field = next_per_field.borrow_mut();
                next_per_field.start(field, first)
            }
            None => Ok(true),
        }
    }
}
#[allow(unused)]
pub(crate) trait TermsHashPerFieldBase {
    /// Called when we first encounter a new term. We must allocate slies to store the postings (vInt
    /// compressed doc/freq/prox), and also the int pointers to where (in our {@link ByteBlockPool}
    ///storage) the postings for this term begin.
    fn init_stream_slices(&mut self, term_id: i32, doc_id: i32) -> Result<()>;
    fn position_stream_slice(&mut self, term_id: i32, doc_id: i32) -> Result<i32>;
    ///Start adding a new field instance; first is true if this is the first time this field name was
    ///seen in the document.
    fn start(&mut self, field: &Fields, first: bool) -> Result<bool>;
    /// Called when a term is seen for the first time.
    fn new_term(&mut self, term_id: i32, doc_id: i32) -> Result<()>;
    /// Called when a previously seen term is seen again.
    fn add_term(&mut self, term_id: i32, doc_id: i32) -> Result<()>;
    /// Called when the postings array is initialized or resized.
    /// # Note
    /// In rust Lucene, we do not need to init new postings array
    /// But we still keep this method for consistent with the original Java code
    #[allow(dead_code)]
    fn new_postings_array(&mut self) -> Result<()> {
        Err(LuceneError::not_implemented(
            "should nerve called".to_string(),
        ))
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
    /// Finish adding all instances of this field to the current document.
    fn finish(&mut self);
}
pub(crate) struct PostingsBytesStartArray<C, P>
where
    C: Access<CounterEnum>,
    P: Access<PostingsArrayWrapper>,
{
    per_field: P,
    bytes_used: C,
}
#[allow(unused)]
impl<C, P> PostingsBytesStartArray<C, P>
where
    C: Access<CounterEnum>,
    P: Access<PostingsArrayWrapper>,
{
    pub(crate) fn new(per_field: P, bytes_used: C) -> Self {
        Self {
            per_field,
            bytes_used,
        }
    }
}
impl<C, P> BytesStartArray for PostingsBytesStartArray<C, P>
where
    C: Access<CounterEnum>,
    P: Access<PostingsArrayWrapper>,
{
    fn init(&mut self) -> Result<()> {
        self.per_field.access_mut(|postings_array_wrapper| {
            if postings_array_wrapper.postings_array.is_none() {
                postings_array_wrapper.postings_array = Option::from(
                    postings_array_wrapper
                        .terms_hash_per_field_type
                        .new_per_field(2),
                );
                if let Some(ref mut postings_array) = postings_array_wrapper.postings_array {
                    let byte_used = postings_array.bytes_per_posting() + postings_array.get_size();
                    self.bytes_used
                        .access_mut(|bytes_used| Ok(bytes_used.add_and_get(byte_used as i64)))?;
                }
            }
            Ok(())
        })
    }

    fn grow(&mut self) -> Result<()> {
        self.per_field.access_mut(|postings_array_wrapper| {
            debug_assert!(postings_array_wrapper.postings_array.is_some());
            let postings_array = postings_array_wrapper.postings_array.as_mut().unwrap();
            let old_size = postings_array.get_size();
            postings_array.grow()?;
            self.bytes_used.access_mut(|bytes_used| {
                Ok(bytes_used.add_and_get(
                    (postings_array.bytes_per_posting() * (postings_array.get_size() - old_size))
                        as i64,
                ))
            })?;
            Ok(())
        })
    }

    fn clear(&mut self) -> Result<()> {
        self.per_field.access_mut(|postings_array_wrapper| {
            if postings_array_wrapper.postings_array.is_some() {
                let postings_array = postings_array_wrapper.postings_array.as_ref().unwrap();
                let byte_used = postings_array.bytes_per_posting() + postings_array.get_size();
                self.bytes_used
                    .access_mut(|bytes_used| Ok(bytes_used.add_and_get(-byte_used as i64)))?;
                postings_array_wrapper.postings_array = None;
            }
            Ok(())
        })
    }

    type Counter = C;

    fn bytes_used(&mut self) -> Self::Counter {
        self.bytes_used.clone()
    }

    fn get_value(&self, index: usize) -> Result<i32> {
        self.per_field.access_mut(|postings_array_wrapper| {
            debug_assert!(postings_array_wrapper.postings_array.is_some());
            Ok(postings_array_wrapper
                .postings_array
                .as_ref()
                .unwrap()
                .get_text_starts()[index])
        })
    }

    fn set_value(&mut self, index: usize, value: i32) -> Result<()> {
        self.per_field.access_mut(|postings_array_wrapper| {
            debug_assert!(postings_array_wrapper.postings_array.is_some());
            postings_array_wrapper
                .postings_array
                .as_mut()
                .unwrap()
                .set_text_starts(index, value);
            Ok(())
        })
    }

    fn len(&self) -> Result<usize> {
        self.per_field.access_mut(|postings_array_wrapper| {
            debug_assert!(postings_array_wrapper.postings_array.is_some());
            Ok(postings_array_wrapper
                .postings_array
                .as_ref()
                .unwrap()
                .get_text_starts()
                .len())
        })
    }
}
#[allow(unused)]
pub(crate) enum TermsHashPerFieldType {
    TermVectors,
    FreqProx(FreqProx),
    #[cfg(test)]
    Mock,
}
impl TermsHashPerFieldType {
    pub(crate) fn new_per_field(&self, size: i32) -> PostingsArrayEnum {
        match self {
            TermsHashPerFieldType::TermVectors => {
                PostingsArrayEnum::TermVectors(TermVectorsPostingsArray::new(size))
            }
            TermsHashPerFieldType::FreqProx(f) => {
                let has_freq = f.index_options >= IndexOptions::DocsAndFreqs;
                let has_prox = f.index_options >= IndexOptions::DocsAndFreqsAndPositions;
                let has_offsets =
                    f.index_options >= IndexOptions::DocsAndFreqsAndPositionsAndOffsets;
                PostingsArrayEnum::FreqProx(FreqProxPostingsArray::new(
                    size,
                    has_freq,
                    has_prox,
                    has_offsets,
                ))
            }
            #[cfg(test)]
            TermsHashPerFieldType::Mock => {
                PostingsArrayEnum::FreqProx(FreqProxPostingsArray::new(size, true, false, false))
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::document::fields::Fields;
    use crate::document::stored_field::StoredField;
    use crate::index::byte_slice_reader::ByteSliceReader;
    use std::cell::RefCell;

    use crate::index::index_options::IndexOptions;
    use crate::index::parallel_postings_array::PostingsArrayEnum;
    use crate::index::terms_hash_per_field::{
        PostingsArrayWrapper, TermsHashPerField, TermsHashPerFieldBase, TermsHashPerFieldType,
    };
    use crate::index::terms_hash_per_field_enum::TermsHashPerFieldEnum;
    use crate::index::BytesRef;
    use crate::store::DataInput;
    use crate::test::util::lucene_test_case::{new_bytes_ref_from_string, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::allocator_byte::{AllocatorByteEnum, DirectAllocatorByte};
    use crate::util::error::lucene_error::{LuceneError, Result};
    use crate::util::int_block_pool::IntBlockPool;
    use crate::util::{ByteBlockPool, CounterEnum};
    use rand::distr::Alphanumeric;
    use rand::prelude::SliceRandom;
    use rand::Rng;
    use std::collections::{BTreeMap, HashMap};
    use std::rc::Rc;
    use std::sync::atomic::{AtomicI64, Ordering};

    #[allow(dead_code)] // for quick search
    struct TestTermsHashPerField;

    fn create_new_hash(
        new_called: AtomicI64,
        add_called: AtomicI64,
    ) -> Result<TermsHashPerFieldEnum> {
        let hash = TermsHashPerFieldMock::new(new_called, add_called)?;
        Ok(hash)
    }

    fn assert_doc_and_freq(
        reader: &mut ByteSliceReader,
        parent: Rc<RefCell<PostingsArrayWrapper>>,
        prev_doc: i32,
        term_id: i32,
        doc: i32,
        frequency: i32,
    ) -> Result<bool> {
        assert!(term_id >= 0);
        let term_id = term_id as usize;
        let mut postings_array_enum = parent.borrow_mut();
        let postings_array_enum = postings_array_enum.postings_array.as_mut().unwrap();
        let postings_array = match postings_array_enum {
            PostingsArrayEnum::FreqProx(freq_prox) => freq_prox,
            _ => {
                unreachable!()
            }
        };
        let mut doc_id = prev_doc;
        let freq: i32;
        let eof = reader.eof();
        if eof {
            doc_id = postings_array.last_doc_ids[term_id];
            match &mut postings_array.term_freqs {
                Some(term_freqs) => {
                    freq = term_freqs[term_id];
                }
                _ => {
                    return Err(LuceneError::illegal_state(
                        "term_freqs is None.".to_string(),
                    ));
                }
            }
        } else {
            let code = reader.read_vint()?;
            doc_id += code >> 1;
            if (code & 1) != 0 {
                freq = 1;
            } else {
                freq = reader.read_vint()?;
            }
        }
        assert_eq!(doc, doc_id, "docID mismatch eof: {}", eof);
        assert_eq!(frequency, freq, "freq mismatch eof: {}", eof);
        Ok(eof)
    }
    #[test]
    fn test_add_and_update_term() -> Result<()> {
        let mut random = random();
        let new_called = AtomicI64::new(0);
        let add_called = AtomicI64::new(0);
        let mut hash = create_new_hash(new_called, add_called)?;
        let dummy_value = "dummy";
        let dummy_filed = Fields::Stored(StoredField::with_binary(
            "binary",
            dummy_value.as_bytes().to_vec(),
        )?);
        hash.start(&dummy_filed, true)?;
        // Pass `None` for the field as in the Java version (null)

        hash.add_with_bytes_ref(&new_bytes_ref_from_string(&mut random, "start")?, 0)?;
        hash.add_with_bytes_ref(&new_bytes_ref_from_string(&mut random, "foo")?, 0)?;
        hash.add_with_bytes_ref(&new_bytes_ref_from_string(&mut random, "bar")?, 0)?;
        hash.finish();
        hash.add_with_bytes_ref(&new_bytes_ref_from_string(&mut random, "bar")?, 1)?;
        hash.add_with_bytes_ref(&new_bytes_ref_from_string(&mut random, "foobar")?, 1)?;
        hash.add_with_bytes_ref(&new_bytes_ref_from_string(&mut random, "bar")?, 1)?;
        hash.add_with_bytes_ref(&new_bytes_ref_from_string(&mut random, "bar")?, 1)?;
        hash.add_with_bytes_ref(&new_bytes_ref_from_string(&mut random, "foobar")?, 1)?;
        hash.add_with_bytes_ref(
            &new_bytes_ref_from_string(&mut random, "verylongfoobarbaz")?,
            1,
        )?;
        hash.finish();
        hash.add_with_bytes_ref(
            &new_bytes_ref_from_string(&mut random, "verylongfoobarbaz")?,
            2,
        )?;
        hash.add_with_bytes_ref(&new_bytes_ref_from_string(&mut random, "boom")?, 2)?;
        hash.finish();
        hash.add_with_bytes_ref(
            &new_bytes_ref_from_string(&mut random, "verylongfoobarbaz")?,
            3,
        )?;
        hash.add_with_bytes_ref(&new_bytes_ref_from_string(&mut random, "end")?, 3)?;
        hash.finish();

        match &hash {
            TermsHashPerFieldEnum::Mock(hash) => {
                assert_eq!(7, hash.new_called.load(Ordering::SeqCst));
                assert_eq!(6, hash.add_called.load(Ordering::SeqCst));
            }
            _ => {
                unreachable!();
            }
        }

        let mut reader = ByteSliceReader::new();
        let parent = match &hash {
            TermsHashPerFieldEnum::Mock(inner) => &mut inner.postings_array_wrapper.clone(),
            _ => {
                unreachable!();
            }
        };
        hash.init_reader(&mut reader, 0, 0);

        assert!(assert_doc_and_freq(
            &mut reader,
            parent.clone(),
            0,
            0,
            0,
            1
        )?);
        hash.init_reader(&mut reader, 1, 0);
        assert!(assert_doc_and_freq(
            &mut reader,
            parent.clone(),
            0,
            1,
            0,
            1
        )?);
        hash.init_reader(&mut reader, 2, 0);
        assert!(!assert_doc_and_freq(
            &mut reader,
            parent.clone(),
            0,
            2,
            0,
            1
        )?);
        assert!(assert_doc_and_freq(
            &mut reader,
            parent.clone(),
            2,
            2,
            1,
            3
        )?);
        hash.init_reader(&mut reader, 3, 0);
        assert!(assert_doc_and_freq(
            &mut reader,
            parent.clone(),
            0,
            3,
            1,
            2
        )?);
        hash.init_reader(&mut reader, 4, 0);
        assert!(!assert_doc_and_freq(
            &mut reader,
            parent.clone(),
            0,
            4,
            1,
            1
        )?);
        assert!(!assert_doc_and_freq(
            &mut reader,
            parent.clone(),
            1,
            4,
            2,
            1
        )?);
        assert!(assert_doc_and_freq(
            &mut reader,
            parent.clone(),
            2,
            4,
            3,
            1
        )?);
        hash.init_reader(&mut reader, 5, 0);
        assert!(assert_doc_and_freq(
            &mut reader,
            parent.clone(),
            0,
            5,
            2,
            1
        )?);
        hash.init_reader(&mut reader, 6, 0);
        assert!(assert_doc_and_freq(
            &mut reader,
            parent.clone(),
            0,
            6,
            3,
            1
        )?);
        Ok(())
    }
    #[test]
    fn test_add_and_update_random() -> Result<()> {
        let mut random = random();
        let new_called = AtomicI64::new(0);
        let add_called = AtomicI64::new(0);
        let mut hash = create_new_hash(new_called, add_called)?;
        let dummy_value = "dummy";
        let dummy_filed = Fields::Stored(StoredField::with_binary(
            "binary",
            dummy_value.as_bytes().to_vec(),
        )?);
        hash.start(&dummy_filed, true)?;

        #[derive(Clone)]
        struct Posting {
            term_id: i32,
            doc_and_freq: BTreeMap<i32, i32>,
        }
        impl Posting {
            fn new() -> Self {
                Self {
                    term_id: -1,
                    doc_and_freq: BTreeMap::new(),
                }
            }
        }

        let mut posting_map: HashMap<BytesRef, Posting> = HashMap::new();
        let num_strings = 1 + random.random_range(0..200);

        let random_length = random.random_range(1..100);
        for _ in 0..num_strings {
            let random_string = (&mut random)
                .sample_iter(&Alphanumeric)
                .take(random_length)
                .map(char::from)
                .collect::<String>();
            posting_map
                .entry(new_bytes_ref_from_string(&mut random, &random_string)?)
                .or_insert_with(Posting::new);
        }

        let mut bytes_refs: Vec<_> = posting_map.keys().cloned().collect();
        let vec_len = bytes_refs.len();
        bytes_refs.sort();

        let num_docs = 1 + random.random_range(0..200);
        let mut term_ord = 0;
        for doc in 0..num_docs {
            let num_terms = 1 + random.random_range(0..200);
            for _ in 0..num_terms {
                let ref_ = bytes_refs.get(random.random_range(0..vec_len)).unwrap();
                let posting = posting_map.get_mut(ref_).unwrap();

                if posting.term_id == -1 {
                    posting.term_id = term_ord;
                    term_ord += 1;
                }

                posting
                    .doc_and_freq
                    .entry(doc)
                    .and_modify(|v| *v += 1)
                    .or_insert(1);
                hash.add_with_bytes_ref(ref_, doc)?;
            }
            hash.finish();
        }

        let mut values: Vec<_> = posting_map
            .values()
            .filter(|x| x.term_id != -1)
            .cloned()
            .collect();
        values.shuffle(&mut random);
        let mut reader = ByteSliceReader::new();
        let parent = match &hash {
            TermsHashPerFieldEnum::Mock(inner) => &mut inner.postings_array_wrapper.clone(),
            _ => {
                unreachable!();
            }
        };
        for posting in values {
            hash.init_reader(&mut reader, posting.term_id, 0);

            let mut eof = false;
            let mut pref_doc = 0;

            for (doc, freq) in posting.doc_and_freq {
                assert!(!eof, "the reader must not be EOF here");

                eof = assert_doc_and_freq(
                    &mut reader,
                    parent.clone(),
                    pref_doc,
                    posting.term_id,
                    doc,
                    freq,
                )?;

                pref_doc = doc;
            }

            assert!(eof, "the last posting must be EOF on the reader");
        }

        Ok(())
    }
    #[test]
    fn test_write_bytes() -> Result<()> {
        let mut random = random();

        for _ in 0..100 {
            let new_called = AtomicI64::new(0);
            let add_called = AtomicI64::new(0);
            let mut hash = create_new_hash(new_called, add_called)?;
            let dummy_value = "dummy";
            let dummy_filed = Fields::Stored(StoredField::with_binary(
                "binary",
                dummy_value.as_bytes().to_vec(),
            )?);
            hash.start(&dummy_filed, true)?;
            hash.add_with_bytes_ref(&new_bytes_ref_from_string(&mut random, "start")?, 0)?;

            let size = random.random_range(50_000..=100_000);
            let mut random_data = vec![0u8; size];
            random.fill(&mut random_data[..]);

            let mut offset = 0;
            while offset < random_data.len() {
                let write_length = std::cmp::min(
                    random_data.len() - offset,
                    TestUtil::next_int(&mut random, 1, 200) as usize,
                );
                debug_assert!(offset <= i32::MAX as usize);
                debug_assert!(write_length <= i32::MAX as usize);
                hash.write_bytes(0, &random_data, offset as i32, write_length as i32)?;
                offset += write_length;
            }

            let mut reader = ByteSliceReader::new();
            {
                let byte_block_pool = hash.get_byte_block_pool();
                let byte_offset;
                let byte_upto;
                {
                    let byte_pool = byte_block_pool.borrow_mut();
                    byte_offset = byte_pool.byte_offset;
                    byte_upto = byte_pool.byte_upto;
                }
                reader.init(byte_block_pool, 0, byte_offset + byte_upto);
            }

            for &expected in &random_data {
                assert_eq!(expected, reader.read_byte()?);
            }
        }
        Ok(())
    }

    pub(crate) struct TermsHashPerFieldMock {
        pub(crate) postings_array_wrapper: Rc<RefCell<PostingsArrayWrapper>>,
        pub(crate) parent_per_field: TermsHashPerField,
        new_called: AtomicI64,
        add_called: AtomicI64,
    }
    impl TermsHashPerFieldMock {
        #[allow(clippy::new_ret_no_self)]
        pub(crate) fn new(
            new_called: AtomicI64,
            add_called: AtomicI64,
        ) -> Result<TermsHashPerFieldEnum> {
            let int_block_pool = Rc::new(RefCell::new(IntBlockPool::new()));

            let allocator = AllocatorByteEnum::DA(DirectAllocatorByte::new());
            let byte_block_pool = Rc::new(RefCell::new(ByteBlockPool::new(allocator)));
            let allocator1 = AllocatorByteEnum::DA(DirectAllocatorByte::new());
            let term_block_pool = Rc::new(RefCell::new(ByteBlockPool::new(allocator1)));
            let bytes_used = Rc::new(RefCell::new(CounterEnum::new_counter(false)));

            let postings_array_wrapper = Rc::new(RefCell::new(PostingsArrayWrapper::new(
                TermsHashPerFieldType::Mock,
            )));

            let parent_per_filed = TermsHashPerField::new(
                1,
                int_block_pool.clone(),
                byte_block_pool.clone(),
                term_block_pool,
                bytes_used,
                None,
                "field_name".to_string(),
                IndexOptions::DocsAndFreqs,
                postings_array_wrapper.clone(),
            )?;
            Ok(TermsHashPerFieldEnum::Mock(TermsHashPerFieldMock {
                postings_array_wrapper,
                parent_per_field: parent_per_filed,
                new_called,
                add_called,
            }))
        }
    }
    impl TermsHashPerFieldBase for TermsHashPerFieldMock {
        fn init_stream_slices(&mut self, term_id: i32, doc_id: i32) -> Result<()> {
            self.parent_per_field.init_stream_slices(term_id, doc_id)?;
            self.new_term(term_id, doc_id)
        }

        fn position_stream_slice(&mut self, term_id: i32, doc_id: i32) -> Result<i32> {
            let term_id = self.parent_per_field.position_stream_slice(term_id, doc_id);
            self.add_term(term_id, doc_id)?;
            Ok(term_id)
        }

        fn start(&mut self, field: &Fields, first: bool) -> Result<bool> {
            self.parent_per_field.start(field, first)
        }

        fn new_term(&mut self, term_id: i32, doc_id: i32) -> Result<()> {
            self.new_called
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let term_id = term_id as usize;
            let mut postings_array_wrapper = self.postings_array_wrapper.borrow_mut();
            debug_assert!(postings_array_wrapper.postings_array.is_some());
            match &mut postings_array_wrapper.postings_array {
                Some(postings_array) => match postings_array {
                    PostingsArrayEnum::FreqProx(f) => {
                        f.last_doc_ids[term_id] = doc_id;
                        f.last_doc_codes[term_id] = doc_id << 1;
                        match &mut f.term_freqs {
                            Some(term_freqs) => {
                                term_freqs[term_id] = 1;
                            }
                            None => unreachable!(),
                        }
                        Ok(())
                    }
                    _ => unreachable!(),
                },
                None => {
                    unreachable!()
                }
            }
        }

        fn add_term(&mut self, term_id: i32, doc_id: i32) -> Result<()> {
            self.add_called
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let term_id = term_id as usize;
            let mut postings_array_wrapper = self.postings_array_wrapper.borrow_mut();
            debug_assert!(postings_array_wrapper.postings_array.is_some());
            match &mut postings_array_wrapper.postings_array {
                Some(postings_array) => match postings_array {
                    PostingsArrayEnum::FreqProx(postings) => {
                        if doc_id != postings.last_doc_ids[term_id] {
                            match &mut postings.term_freqs {
                                Some(term_freqs) => {
                                    if 1 == term_freqs[term_id] {
                                        self.parent_per_field
                                            .write_vint(0, postings.last_doc_codes[term_id] | 1)?;
                                    } else {
                                        self.parent_per_field
                                            .write_vint(0, postings.last_doc_codes[term_id])?;
                                        self.parent_per_field.write_vint(0, term_freqs[term_id])?;
                                    }
                                    term_freqs[term_id] = 1;
                                }
                                None => unreachable!(),
                            }
                            postings.last_doc_codes[term_id] =
                                (doc_id - postings.last_doc_ids[term_id]) << 1;
                            postings.last_doc_ids[term_id] = doc_id;
                            Ok(())
                        } else {
                            match &mut postings.term_freqs {
                                Some(term_freqs) => {
                                    let value = term_freqs[term_id] as i64 + 1;
                                    if value > i32::MAX as i64 {
                                        return Err(LuceneError::number_overflow(
                                            "term_freqs".to_string(),
                                        ));
                                    }
                                    term_freqs[term_id] += 1;
                                    Ok(())
                                }
                                None => unreachable!(),
                            }
                        }
                    }
                    _ => unreachable!(),
                },
                None => {
                    unreachable!()
                }
            }
        }

        fn finish(&mut self) {
            self.parent_per_field.finish()
        }
    }
}
