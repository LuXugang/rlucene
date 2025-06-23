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
use crate::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::codecs::term_vectors_format::TermVectorsFormat;
use crate::codecs::term_vectors_writer::{TermVectorsWriter, TermVectorsWriterEnum};
use crate::codecs::Codec;
use crate::index::byte_slice_reader::ByteSliceReader;
use crate::index::field_info::FieldInfo;
use crate::index::field_invert_state::FieldInvertState;
use crate::index::segment_info::SegmentInfo;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::sorter::DocMap;
use crate::index::term_vectors_consumer_per_field::TermVectorsConsumerPerField;
use crate::index::terms_hash::TermsHash;
use crate::index::BytesRef;
use crate::store::directory::Directory;
#[cfg(test)]
use crate::store::dummy::dummy_directory::DummyDirectory;
use crate::store::flush_info::FlushInfo;
use crate::store::IOContext;
use crate::util::allocator_byte::AllocatorByteEnum;
#[cfg(test)]
use crate::util::allocator_byte::DirectAllocatorByte;
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::int_block_pool::AllocatorIntEnum;
#[cfg(test)]
use crate::util::int_block_pool::DirectAllocatorI32;
use crate::util::{Counter, CounterEnum, CounterEnumBorrow};
use parking_lot::Mutex;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct TermVectorsConsumer<D1, D2, O, P, T>
where
    D1: Directory,
    D2: Directory,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    directory: Arc<Mutex<D1>>,
    pub(crate) info: Rc<SegmentInfo<D2>>,
    pub(crate) writer: Option<TermVectorsWriterEnum<D1>>,
    // Scratch term used by TermVectorsConsumerPerField.finishDocument.
    pub(crate) flush_term: BytesRef<Vec<u8>>,
    // Used by TermVectorsConsumerPerField when serializing the term vectors.
    pub(crate) vector_slice_reader_pos: Option<ByteSliceReader>,
    pub(crate) vector_slice_reader_off: Option<ByteSliceReader>,
    has_vectors: bool,
    num_vector_fields: i32,
    pub(crate) last_doc_id: i32,
    per_fields: Vec<TermVectorsConsumerPerField<O, P, T>>,

    pub(crate) base: TermsHash,
}

#[cfg(test)]
impl<O, P, T> Default for TermVectorsConsumer<DummyDirectory, DummyDirectory, O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn default() -> Self {
        let int_block_allocator = AllocatorIntEnum::DA(DirectAllocatorI32::new());
        let byte_block_allocator = AllocatorByteEnum::DA(DirectAllocatorByte::new());
        let directory = Arc::new(Mutex::new(DummyDirectory));
        let info = Rc::new(SegmentInfo::default());
        TermVectorsConsumer::new(int_block_allocator, byte_block_allocator, directory, info)
    }
}

impl<D1, D2, O, P, T> TermVectorsConsumer<D1, D2, O, P, T>
where
    D1: Directory,
    D2: Directory,

    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    pub(crate) fn new(
        int_block_allocator: AllocatorIntEnum<CounterEnumBorrow>,
        byte_block_allocator: AllocatorByteEnum<CounterEnumBorrow>,
        directory: Arc<Mutex<D1>>,
        info: Rc<SegmentInfo<D2>>,
    ) -> Self {
        let base = TermsHash::new(
            int_block_allocator,
            byte_block_allocator,
            Rc::new(RefCell::new(CounterEnum::new_counter(false))),
        );

        let per_fields = vec![TermVectorsConsumerPerField::default(); 1];

        TermVectorsConsumer {
            directory,
            info,
            writer: None,
            flush_term: BytesRef::default(),
            vector_slice_reader_pos: Some(ByteSliceReader::new()),
            vector_slice_reader_off: Some(ByteSliceReader::new()),
            has_vectors: false,
            num_vector_fields: 0,
            last_doc_id: 0,
            per_fields,
            base,
        }
    }
    pub(crate) fn reset_fields(&mut self) {
        self.per_fields.clear();
        self.num_vector_fields = 0;
    }
    pub(crate) fn fill(&mut self, doc_id: i32) -> Result<()> {
        while self.last_doc_id < doc_id {
            if let Some(ref mut w) = self.writer {
                w.start_document(0)?;
                w.finish_document()?;
            } else {
                Err(LuceneError::illegal_state(
                    "TermVectorsConsumer writer is not initialized",
                ))?;
            }
            self.last_doc_id += 1;
        }
        Ok(())
    }
    pub(crate) fn init_term_vectors_writer(
        &mut self,
        bytes_used: &CounterEnumBorrow,
        codec: &impl Codec,
    ) -> Result<()> {
        if self.writer.is_none() {
            let flush_info = FlushInfo::new(self.last_doc_id, bytes_used.borrow().get());
            let context = IOContext::with_flush(flush_info)?;

            self.writer = Option::from(codec.term_vectors_format().vectors_writer(
                Arc::clone(&self.directory),
                Rc::clone(&self.info),
                &context,
            )?);

            self.last_doc_id = 0;
        }
        Ok(())
    }
    pub(crate) fn flush<DM>(
        &mut self,
        state: &mut SegmentWriteState<D2>,
        _sort_map: &Option<Rc<DM>>,
        _codec: &impl Codec,
    ) -> Result<()>
    where
        DM: DocMap,
    {
        if self.writer.is_some() {
            let num_docs = state.segment_info.max_doc()?;
            debug_assert!(num_docs > 0);
            // At least one doc in this run had term vectors enabled
            self.fill(num_docs)?;
            self.writer.as_mut().unwrap().finish(num_docs)?;
        }
        Ok(())
    }
    pub(crate) fn set_has_vectors(&mut self) {
        self.has_vectors = true;
    }
    pub(crate) fn finish_document(
        &mut self,
        doc_id: i32,
        bytes_used: &CounterEnumBorrow,
        codec: &impl Codec,
    ) -> Result<()> {
        if !self.has_vectors {
            return Ok(());
        }

        ArrayUtil::intro_sort_with_range(&mut self.per_fields, 0, self.num_vector_fields)?;

        self.init_term_vectors_writer(bytes_used, codec)?;
        self.fill(doc_id)?;
        // Append term vectors to the real outputs:
        self.writer
            .as_mut()
            .unwrap()
            .start_document(self.num_vector_fields)?;
        let mut per_fields = std::mem::take(&mut self.per_fields);
        for i in 0..self.num_vector_fields as usize {
            per_fields[i].finish_document(self)?;
        }
        self.writer.as_mut().unwrap().finish_document()?;

        debug_assert_eq!(
            self.last_doc_id, doc_id,
            "last_doc_id = {}, doc_id = {}",
            self.last_doc_id, doc_id
        );

        self.last_doc_id += 1;
        self.reset_fields();
        Ok(())
    }
    pub(crate) fn start_document(&mut self) -> Result<()> {
        self.reset_fields();
        self.num_vector_fields = 0;
        Ok(())
    }
    pub(crate) fn add_field(
        &mut self,
        field_invert_state: Rc<FieldInvertState<O, P, T>>,
        field_info: Rc<FieldInfo>,
    ) -> TermVectorsConsumerPerField<O, P, T> {
        TermVectorsConsumerPerField::new(field_invert_state, self, field_info)
    }
    pub(crate) fn add_field_to_flush(
        &mut self,
        field_to_flush: TermVectorsConsumerPerField<O, P, T>,
    ) {
        let num_vector_fields = self.num_vector_fields as usize;
        if num_vector_fields == self.per_fields.len() {
            let new_size = ArrayUtil::oversize(num_vector_fields + 1, 0);
            ArrayUtil::grow_with_len(&mut self.per_fields, new_size);
        }
        self.per_fields[num_vector_fields] = field_to_flush;
        self.num_vector_fields += 1;
    }
    pub(crate) fn abort(&mut self) {
        self.base.reset();
    }
}
