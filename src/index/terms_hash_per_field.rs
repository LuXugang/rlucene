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
use std::cell::RefCell;
use std::rc::Rc;

use crate::document::fields::Fields;
use crate::index::byte_slice_pool::ByteSlicePool;
use crate::index::byte_slice_reader::ByteSliceReader;
use crate::index::freq_prox_terms_writer_per_field::{FreqProx, FreqProxPostingsArray};
use crate::index::index_options::IndexOptions;
use crate::index::parallel_postings_array::PostingsArrayEnum;
use crate::index::term_vectors_consumer_per_field::{
    TermVectorsConsumerPerField, TermVectorsPostingsArray,
};
use crate::index::BytesRef;
use crate::util::access::Access;
use crate::util::bytes_ref_hash::{BytesRefHash, BytesStartArray, STBytesRefHash};
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::int_block_pool::IntBlockPool;
use crate::util::{
    byte_block_pool_util, ByteBlockPoolBorrow, Counter, CounterEnumBorrow, SliceCopyOps,
};

/// This struct stores streams of information per term without knowing the size
/// of the stream ahead of time. Each stream typically encodes one level of
/// information, like term frequency per document or term proximity.
///
/// Internally, this struct allocates a linked list of slices that can be read
/// by a [`ByteSliceReader`] for each term. Terms are first deduplicated in a
/// [`BytesRefHash`]. Once this is done, internal data structures point to the
/// current offset of each stream that can be written to.
pub struct TermsHashPerField<S>
where
    S: TermsHashPerFieldBase,
{
    pub(crate) next_per_field: Option<Box<TermsHashPerField<TermVectorsConsumerPerField>>>,
    int_pool: Rc<RefCell<IntBlockPool>>,
    pub(crate) byte_pool: ByteBlockPoolBorrow,
    slice_pool: ByteSlicePool,
    // for each term we store an integer per stream that points into the
    // bytePool above the address is updated once data is written to the
    // stream to point to the next free offset in the terms stream. The
    // start address for the stream is stored in postingsArray.
    // byteStarts[termId] This is initialized in the #addTerm method,
    // either to a brand new per term stream if the term is new or
    // to the addresses where the term stream was written to when we saw it the
    // last time.    term_stream_address_buffer: Vec<i32>,
    term_stream_address_buffer_index: i32,
    stream_address_offset: i32,
    stream_count: i32,
    // This stores the actual term bytes for postings and offsets into the
    // parent hash in the case that this TermsHashPerField is hashing term
    // vectors.
    pub(crate) bytes_hash:
        BytesRefHash<CounterEnumBorrow, ByteBlockPoolBorrow, PostingsBytesStartArray>,
    last_doc_id: i32, // only used with debug/asserts
    sorted_term_ids: bool,
    pub(crate) do_next_call: bool,
    pub(crate) index_options: IndexOptions,
    // wrap with Option for `std::mem:take`
    pub(crate) sub: Option<S>,
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
pub mod terms_hash_per_field_util {
    pub(super) const HASH_INIT_SIZE: i32 = 4;
}
impl<S> TermsHashPerField<S>
where
    S: TermsHashPerFieldBase,
{
    ///  streamCount: how many streams this field stores per term. E.g.
    /// doc(+freq) is 1 stream, prox+offset is a second.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        stream_count: i32,
        int_pool: Rc<RefCell<IntBlockPool>>,
        byte_pool: ByteBlockPoolBorrow,
        term_byte_pool: ByteBlockPoolBorrow,
        bytes_used: CounterEnumBorrow,
        next_per_field: Option<Box<TermsHashPerField<TermVectorsConsumerPerField>>>,
        postings_array_wrapper: PostingsArrayWrapper,
        index_options: IndexOptions,
        sub: S,
    ) -> Self {
        // In the original Java code, we assert that indexOptions !=
        // IndexOptions.NONE.
        debug_assert!(index_options != IndexOptions::None);
        let slice_pool = ByteSlicePool;
        let byte_starts = PostingsBytesStartArray::new(postings_array_wrapper, bytes_used);

        let bytes_hash = BytesRefHash::from_bytes_start_array(
            term_byte_pool,
            terms_hash_per_field_util::HASH_INIT_SIZE,
            byte_starts,
        );

        TermsHashPerField {
            next_per_field,
            int_pool,
            byte_pool,
            slice_pool,
            term_stream_address_buffer_index: 0,
            stream_address_offset: 0,
            stream_count,
            bytes_hash,
            last_doc_id: 0,
            sorted_term_ids: false,
            do_next_call: false,
            index_options,
            sub: Some(sub),
        }
    }
    pub(crate) fn init_reader(&self, reader: &mut ByteSliceReader, term_id: i32, stream: i32) {
        debug_assert!(stream < self.stream_count);
        let term_id = term_id as usize;
        let postings_array_wrapper = &self.bytes_hash.bytes_start_array.per_field;
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
    /// Collapse the hash table and sort in-place; also sets this.sortedTermIDs
    /// to the results. This method must not be called twice unless
    /// [`reset()`](Self::reset) or
    /// [`reinit_hash()`](Self::reinit_hash) was called.
    pub(crate) fn sort_terms(&mut self, bytes_hash: &mut STBytesRefHash) -> Result<()> {
        debug_assert!(!self.sorted_term_ids);
        bytes_hash.sort()?;
        self.sorted_term_ids = true;
        Ok(())
    }
    /// Returns the sorted term IDs.
    /// [`sort_terms()`](TermsHashPerField::sort_terms) must be called before.
    pub(crate) fn get_sorted_term_ids<'a>(&self, bytes_hash: &'a STBytesRefHash) -> &'a [i32] {
        debug_assert!(!self.sorted_term_ids);
        bytes_hash.ids.as_slice()
    }

    pub(crate) fn write_byte(&mut self, stream: i32, b: u8) -> Result<()> {
        let stream_address = (self.stream_address_offset + stream) as usize;
        let mut int_pool = self.int_pool.borrow_mut();
        let term_stream_address_buffer = int_pool.get_buffer(self.term_stream_address_buffer_index);
        let upto = term_stream_address_buffer[stream_address];
        let mut byte_pool = self.byte_pool.borrow_mut();
        let block_index = upto >> byte_block_pool_util::BYTE_BLOCK_SHIFT;
        debug_assert!(block_index <= byte_pool.buffer_upto);
        let bytes = byte_pool.get_buffer(block_index);
        let offset = upto & byte_block_pool_util::BYTE_BLOCK_MASK;
        let value = bytes[offset as usize];
        drop(byte_pool);
        let mut byte_pool;
        let new_offset = if value != 0 {
            // End of slice; allocate a new one
            let allocated_offset = self.slice_pool.alloc_slice(
                block_index,
                offset,
                &mut self.byte_pool.borrow_mut(),
            )?;
            byte_pool = self.byte_pool.borrow_mut();
            term_stream_address_buffer[stream_address] = allocated_offset + byte_pool.byte_offset;
            allocated_offset
        } else {
            byte_pool = self.byte_pool.borrow_mut();
            offset
        };
        let bytes = byte_pool.get_buffer_mut(block_index);
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
            let mut block_index = upto >> byte_block_pool_util::BYTE_BLOCK_SHIFT;
            debug_assert!(block_index <= byte_pool.buffer_upto);
            let slice = byte_pool.get_buffer_mut(block_index);
            let mut slice_offset = (upto & byte_block_pool_util::BYTE_BLOCK_MASK) as usize;

            while offset < end && slice[slice_offset] == 0 {
                slice[slice_offset] = b[offset];
                slice_offset += 1;
                offset += 1;
                term_stream_address_buffer[stream_address] += 1;
            }

            drop(byte_pool);
            while offset < end {
                debug_assert!(slice_offset <= i32::MAX as usize);
                let offset_and_length = self.slice_pool.alloc_known_size_slice(
                    block_index,
                    slice_offset as i32,
                    &mut self.byte_pool.borrow_mut(),
                )?;
                slice_offset = (offset_and_length >> 8) as usize;
                let slice_length = offset_and_length & 0xff;
                let mut byte_pool = self.byte_pool.borrow_mut();
                let buffer_upto = byte_pool.buffer_upto;
                block_index = buffer_upto;
                let slice = byte_pool.get_buffer_mut(buffer_upto);
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
    pub(crate) fn write_vint(
        base: &mut TermsHashPerField<S>,
        stream: i32,
        mut i: i32,
    ) -> Result<()> {
        debug_assert!(stream < base.stream_count);
        while (i & !0x7F) != 0 {
            base.write_byte(stream, ((i & 0x7F) | 0x80) as u8)?;
            i = ((i as u32) >> 7) as i32;
        }
        base.write_byte(stream, i as u8)
    }

    pub(crate) fn get_next_per_field(&mut self) -> TermsHashPerField<TermVectorsConsumerPerField> {
        *self.next_per_field.take().unwrap()
    }

    pub(crate) fn get_field_name(&self) -> &str {
        self.sub.as_ref().unwrap().get_field_name()
    }
    fn finish(&mut self) {
        if let Some(ref mut next_per_field) = self.next_per_field {
            next_per_field.finish()
        }
        self.sub.as_mut().unwrap().finish()
    }
    pub(crate) fn get_num_terms(&self, bytes_ref_hash: &STBytesRefHash) -> i32 {
        bytes_ref_hash.size()
    }
    pub(crate) fn reset(&mut self) {
        self.bytes_hash.clear();
        self.sorted_term_ids = false;
        if self.next_per_field.is_some() {
            self.next_per_field.as_mut().unwrap().reset();
        }
    }

    pub(crate) fn reinit_hash(&mut self) {
        self.sorted_term_ids = false;
        self.bytes_hash.reinit()
    }
    // Secondary entry point (for 2nd & subsequent TermsHash),
    // because token text has already been "interned" into
    // textStart, so we hash by textStart.  term vectors use
    // this API.
    fn add_with_text_start(&mut self, text_start: i32, doc_id: i32) -> Result<()> {
        let term_id = self.bytes_hash.add_by_pool_offset(text_start)?;
        if term_id >= 0 {
            // First time we are seeing this token since we last
            // flushed the hash.
            self.init_stream_slices(term_id, doc_id)?;
        } else {
            self.position_stream_slice(term_id, doc_id)?;
        }
        Ok(())
    }
    /// Called when we first encounter a new term. We must allocate slices to
    /// store the postings (vInt compressed doc/freq/prox), and also the int
    /// pointers to where (in our [`ByteBlockPool`] storage) the postings
    /// for this term begin.
    pub(crate) fn init_stream_slices(&mut self, term_id: i32, doc_id: i32) -> Result<()> {
        let byte_offset;
        {
            let mut byte_pool = self.byte_pool.borrow_mut();
            if byte_block_pool_util::BYTE_BLOCK_SIZE - byte_pool.byte_upto
                < 2 * self.stream_count * ByteSlicePool::FIRST_LEVEL_SIZE
            {
                // can we fit at least one byte per stream in the current
                // buffer, if not allocate a new one
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
                int_pool.get_buffer(self.term_stream_address_buffer_index);
            for i in 0..self.stream_count as usize {
                let upto = self.slice_pool.new_slice(
                    ByteSlicePool::FIRST_LEVEL_SIZE,
                    &mut self.byte_pool.borrow_mut(),
                )?;
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
        let mut sub = std::mem::take(&mut self.sub);
        sub.as_mut().unwrap().new_term(term_id, doc_id, self)?;
        self.sub = sub;
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
    /// Called once per inverted token. This is the primary entry point (for
    /// first TermsHash); postings use this API.
    pub(crate) fn add_with_bytes_ref(
        &mut self,
        term_bytes: &BytesRef<Vec<u8>>,
        doc_id: i32,
    ) -> Result<()> {
        debug_assert!(self.assert_doc_id(doc_id));
        // We are first in the chain so we must "intern" the
        // term text into textStart address
        // Get the text & hash of this term.
        let mut term_id = self.bytes_hash.add(term_bytes)?;
        if term_id >= 0 {
            self.init_stream_slices(term_id, doc_id)?;
        } else {
            term_id = self.position_stream_slice(term_id, doc_id)?;
        }

        if self.do_next_call {
            if let Some(ref mut next_per_field) = self.next_per_field {
                let postings_array_wrapper = &self.bytes_hash.bytes_start_array.per_field;
                debug_assert!(postings_array_wrapper.postings_array.is_some());
                let text_start = postings_array_wrapper
                    .postings_array
                    .as_ref()
                    .unwrap()
                    .get_text_starts()[term_id as usize];
                next_per_field.add_with_text_start(text_start, doc_id)?;
            }
        }
        Ok(())
    }
    pub(crate) fn position_stream_slice(&mut self, term_id: i32, doc_id: i32) -> Result<i32> {
        let term_id = (-term_id) - 1;
        let postings_array_wrapper = &self.bytes_hash.bytes_start_array.per_field;
        debug_assert!(postings_array_wrapper.postings_array.is_some());
        let int_start = postings_array_wrapper
            .postings_array
            .as_ref()
            .unwrap()
            .get_address_offset()[term_id as usize];
        self.term_stream_address_buffer_index = int_start >> IntBlockPool::INT_BLOCK_SHIFT;
        self.stream_address_offset = int_start & IntBlockPool::INT_BLOCK_MASK;
        let mut sub = std::mem::take(&mut self.sub);
        sub.as_mut().unwrap().add_term(term_id, doc_id, self)?;
        self.sub = sub;
        Ok(term_id)
    }
    fn start(&mut self, field: &Fields, first: bool) -> Result<bool> {
        match self.next_per_field {
            Some(ref mut next_per_field) => next_per_field.start(field, first)?,
            None => true,
        };
        self.sub.as_mut().unwrap().start(field, first)
    }
}
pub(crate) trait TermsHashPerFieldBase {
    ///Start adding a new field instance; first is true if this is the first
    /// time this field name was seen in the document.
    fn start(&mut self, field: &Fields, first: bool) -> Result<bool>;
    /// Called when a term is seen for the first time.
    fn new_term<S: TermsHashPerFieldBase>(
        &mut self,
        term_id: i32,
        doc_id: i32,
        per_field: &mut TermsHashPerField<S>,
    ) -> Result<()>;
    /// Called when a previously seen term is seen again.
    fn add_term<S: TermsHashPerFieldBase>(
        &mut self,
        term_id: i32,
        doc_id: i32,
        per_field: &mut TermsHashPerField<S>,
    ) -> Result<()>;
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

    fn get_field_name(&self) -> &str;
}
pub(crate) struct PostingsBytesStartArray {
    pub(crate) per_field: PostingsArrayWrapper,
    bytes_used: CounterEnumBorrow,
}
#[allow(unused)]
impl PostingsBytesStartArray {
    pub(crate) fn new(per_field: PostingsArrayWrapper, bytes_used: CounterEnumBorrow) -> Self {
        Self {
            per_field,
            bytes_used,
        }
    }
}
impl BytesStartArray for PostingsBytesStartArray {
    fn init(&mut self) {
        if self.per_field.postings_array.is_none() {
            self.per_field.postings_array =
                Option::from(self.per_field.terms_hash_per_field_type.new_per_field(2));
            if let Some(ref mut postings_array) = self.per_field.postings_array {
                let byte_used = postings_array.bytes_per_posting() + postings_array.get_size();
                let _ = self
                    .bytes_used
                    .access_mut(|bytes_used| bytes_used.add_and_get(byte_used as i64));
            }
        }
    }

    fn grow(&mut self) -> Result<()> {
        debug_assert!(self.per_field.postings_array.is_some());
        let postings_array = self.per_field.postings_array.as_mut().unwrap();
        let old_size = postings_array.get_size();
        postings_array.grow()?;
        self.bytes_used.access_mut(|bytes_used| {
            bytes_used.add_and_get(
                (postings_array.bytes_per_posting() * (postings_array.get_size() - old_size))
                    as i64,
            )
        });
        Ok(())
    }

    fn clear(&mut self) {
        if self.per_field.postings_array.is_some() {
            let postings_array = self.per_field.postings_array.as_ref().unwrap();
            let byte_used = postings_array.bytes_per_posting() + postings_array.get_size();
            debug_assert!(byte_used <= i64::MAX as usize);
            let _ = self
                .bytes_used
                .access_mut(|bytes_used| bytes_used.add_and_get(-(byte_used as i64)));
            self.per_field.postings_array = None;
        }
    }

    type Counter = CounterEnumBorrow;

    fn bytes_used(&mut self) -> Self::Counter {
        self.bytes_used.clone()
    }

    fn get_value(&self, index: usize) -> i32 {
        debug_assert!(self.per_field.postings_array.is_some());
        self.per_field
            .postings_array
            .as_ref()
            .unwrap()
            .get_text_starts()[index]
    }

    fn set_value(&mut self, index: usize, value: i32) {
        debug_assert!(self.per_field.postings_array.is_some());
        self.per_field
            .postings_array
            .as_mut()
            .unwrap()
            .set_text_starts(index, value)
    }

    fn len(&self) -> usize {
        debug_assert!(self.per_field.postings_array.is_some());
        self.per_field
            .postings_array
            .as_ref()
            .unwrap()
            .get_text_starts()
            .len()
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
                let has_offsets =
                    f.index_options >= IndexOptions::DocsAndFreqsAndPositionsAndOffsets;
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

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::RefCell;
    use std::collections::{BTreeMap, HashMap};
    use std::rc::Rc;
    use std::sync::atomic::{AtomicI64, Ordering};

    use rand::distr::Alphanumeric;
    use rand::prelude::SliceRandom;
    use rand::Rng;

    use crate::document::fields::Fields;
    use crate::document::stored_field::StoredField;
    use crate::index::byte_slice_reader::ByteSliceReader;
    use crate::index::index_options::IndexOptions;
    use crate::index::parallel_postings_array::PostingsArrayEnum;
    use crate::index::terms_hash_per_field::{
        PostingsArrayWrapper, TermsHashPerField, TermsHashPerFieldBase, TermsHashPerFieldType,
    };
    use crate::index::BytesRef;
    use crate::store::DataInput;
    use crate::test::util::lucene_test_case::{new_bytes_ref_from_string, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::allocator_byte::{AllocatorByteEnum, DirectAllocatorByte};
    use crate::util::error::lucene_error::{LuceneError, Result};
    use crate::util::int_block_pool::IntBlockPool;
    use crate::util::{ByteBlockPool, CounterEnum};

    #[allow(dead_code)] // for quick search
    struct TestTermsHashPerField;

    fn create_new_hash(
        new_called: AtomicI64,
        add_called: AtomicI64,
    ) -> Result<TermsHashPerField<TermsHashPerFieldMock>> {
        let hash = TermsHashPerFieldMock::new(new_called, add_called)?;
        Ok(hash)
    }

    fn assert_doc_and_freq(
        reader: &mut ByteSliceReader,
        postings_array_wrapper: &PostingsArrayWrapper,
        prev_doc: i32,
        term_id: i32,
        doc: i32,
        frequency: i32,
    ) -> Result<bool> {
        assert!(term_id >= 0);
        let term_id = term_id as usize;
        let postings_array_enum = postings_array_wrapper.postings_array.as_ref().unwrap();
        let postings_array = match postings_array_enum {
            PostingsArrayEnum::FreqProx(freq_prox) => freq_prox,
            _ => {
                unreachable!()
            },
        };
        let mut doc_id = prev_doc;
        let freq: i32;
        let eof = reader.eof();
        if eof {
            doc_id = postings_array.last_doc_ids[term_id];
            match &postings_array.term_freqs {
                Some(term_freqs) => {
                    freq = term_freqs[term_id];
                },
                _ => {
                    return Err(LuceneError::illegal_state(
                        "term_freqs is None.".to_string(),
                    ));
                },
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

        assert_eq!(
            7,
            hash.sub.as_mut().unwrap().new_called.load(Ordering::SeqCst)
        );
        assert_eq!(
            6,
            hash.sub.as_mut().unwrap().add_called.load(Ordering::SeqCst)
        );

        let mut reader = ByteSliceReader::new();
        hash.init_reader(&mut reader, 0, 0);

        let postings_array_wrapper = &hash.bytes_hash.bytes_start_array.per_field;

        assert!(assert_doc_and_freq(
            &mut reader,
            postings_array_wrapper,
            0,
            0,
            0,
            1
        )?);
        hash.init_reader(&mut reader, 1, 0);
        assert!(assert_doc_and_freq(
            &mut reader,
            postings_array_wrapper,
            0,
            1,
            0,
            1
        )?);
        hash.init_reader(&mut reader, 2, 0);
        assert!(!assert_doc_and_freq(
            &mut reader,
            postings_array_wrapper,
            0,
            2,
            0,
            1
        )?);
        assert!(assert_doc_and_freq(
            &mut reader,
            postings_array_wrapper,
            2,
            2,
            1,
            3
        )?);
        hash.init_reader(&mut reader, 3, 0);
        assert!(assert_doc_and_freq(
            &mut reader,
            postings_array_wrapper,
            0,
            3,
            1,
            2
        )?);
        hash.init_reader(&mut reader, 4, 0);
        assert!(!assert_doc_and_freq(
            &mut reader,
            postings_array_wrapper,
            0,
            4,
            1,
            1
        )?);
        assert!(!assert_doc_and_freq(
            &mut reader,
            postings_array_wrapper,
            1,
            4,
            2,
            1
        )?);
        assert!(assert_doc_and_freq(
            &mut reader,
            postings_array_wrapper,
            2,
            4,
            3,
            1
        )?);
        hash.init_reader(&mut reader, 5, 0);
        assert!(assert_doc_and_freq(
            &mut reader,
            postings_array_wrapper,
            0,
            5,
            2,
            1
        )?);
        hash.init_reader(&mut reader, 6, 0);
        assert!(assert_doc_and_freq(
            &mut reader,
            postings_array_wrapper,
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

        let mut posting_map: HashMap<BytesRef<Vec<u8>>, Posting> = HashMap::new();
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

        let postings_array_wrapper = &hash.bytes_hash.bytes_start_array.per_field;
        for posting in values {
            hash.init_reader(&mut reader, posting.term_id, 0);

            let mut eof = false;
            let mut pref_doc = 0;

            for (doc, freq) in posting.doc_and_freq {
                assert!(!eof, "the reader must not be EOF here");

                eof = assert_doc_and_freq(
                    &mut reader,
                    postings_array_wrapper,
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
                let byte_block_pool = hash.byte_pool;
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
        new_called: AtomicI64,
        add_called: AtomicI64,
    }
    impl TermsHashPerFieldMock {
        #[allow(clippy::new_ret_no_self)]
        pub(crate) fn new(
            new_called: AtomicI64,
            add_called: AtomicI64,
        ) -> Result<TermsHashPerField<TermsHashPerFieldMock>> {
            let int_block_pool = Rc::new(RefCell::new(IntBlockPool::new()));

            let allocator = AllocatorByteEnum::DA(DirectAllocatorByte::new());
            let byte_block_pool = Rc::new(RefCell::new(ByteBlockPool::new(allocator)));
            let allocator1 = AllocatorByteEnum::DA(DirectAllocatorByte::new());
            let term_block_pool = Rc::new(RefCell::new(ByteBlockPool::new(allocator1)));
            let bytes_used = Rc::new(RefCell::new(CounterEnum::new_counter(false)));

            let postings_array_wrapper = PostingsArrayWrapper::new(TermsHashPerFieldType::Mock);

            let sub = TermsHashPerFieldMock {
                new_called,
                add_called,
            };
            Ok(TermsHashPerField::new(
                1,
                int_block_pool.clone(),
                byte_block_pool.clone(),
                term_block_pool,
                bytes_used,
                None,
                postings_array_wrapper,
                IndexOptions::DocsAndFreqs,
                sub,
            ))
        }
    }
    impl TermsHashPerFieldBase for TermsHashPerFieldMock {
        fn start(&mut self, _field: &Fields, _first: bool) -> Result<bool> {
            Ok(true)
        }

        fn new_term<S: TermsHashPerFieldBase>(
            &mut self,
            term_id: i32,
            doc_id: i32,
            per_filed: &mut TermsHashPerField<S>,
        ) -> Result<()> {
            self.new_called
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let term_id = term_id as usize;
            match per_filed
                .bytes_hash
                .bytes_start_array
                .per_field
                .postings_array
                .as_mut()
                .unwrap()
            {
                PostingsArrayEnum::FreqProx(f) => {
                    f.last_doc_ids[term_id] = doc_id;
                    f.last_doc_codes[term_id] = doc_id << 1;
                    match &mut f.term_freqs {
                        Some(term_freqs) => {
                            term_freqs[term_id] = 1;
                        },
                        None => unreachable!(),
                    }
                    Ok(())
                },
                _ => unreachable!(),
            }
        }

        fn add_term<S: TermsHashPerFieldBase>(
            &mut self,
            term_id: i32,
            doc_id: i32,
            per_field: &mut TermsHashPerField<S>,
        ) -> Result<()> {
            self.add_called
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let term_id = term_id as usize;
            let mut v = Vec::new();
            let mut need_write = false;
            match per_field
                .bytes_hash
                .bytes_start_array
                .per_field
                .postings_array
                .as_mut()
                .unwrap()
            {
                PostingsArrayEnum::FreqProx(postings) => {
                    if doc_id != postings.last_doc_ids[term_id] {
                        match &mut postings.term_freqs {
                            Some(term_freqs) => {
                                need_write = true;
                                if 1 == term_freqs[term_id] {
                                    v.push(postings.last_doc_codes[term_id] | 1);
                                } else {
                                    v.push(postings.last_doc_codes[term_id]);
                                    v.push(term_freqs[term_id]);
                                }
                                term_freqs[term_id] = 1;
                            },
                            None => unreachable!(),
                        }
                        postings.last_doc_codes[term_id] =
                            (doc_id - postings.last_doc_ids[term_id]) << 1;
                        postings.last_doc_ids[term_id] = doc_id;
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
                            },
                            None => unreachable!(),
                        }
                    }
                },
                _ => unreachable!(),
            }
            if need_write {
                for x in v {
                    TermsHashPerField::write_vint(per_field, 0, x)?;
                }
            }
            Ok(())
        }

        fn finish(&mut self) {}

        fn get_field_name(&self) -> &str {
            ""
        }
    }
}
