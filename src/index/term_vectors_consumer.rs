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
use crate::codecs::term_vectors_writer::TermVectorsWriter;
use crate::codecs::Codec;
use crate::index::byte_slice_reader::ByteSliceReader;
use crate::index::field_info::FieldInfo;
use crate::index::field_invert_state::FieldInvertState;
use crate::index::segment_info::SegmentInfo;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::sorter::DocMap;
use crate::index::term_vectors_consumer_per_field::TermVectorsConsumerPerField;
use crate::index::BytesRef;
use crate::store::directory::Directory;
use crate::store::flush_info::FlushInfo;
use crate::store::IOContext;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{Counter, CounterEnumBorrow};
use parking_lot::Mutex;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct TermVectorsConsumer<D, C, TVW, O, P, T>
where
    D: Directory,
    C: Codec,
    TVW: TermVectorsWriter,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    directory: Arc<Mutex<D>>,
    info: Rc<SegmentInfo<D>>,
    code: Arc<Mutex<C>>,
    writer: Option<TVW>,
    // Scratch term used by TermVectorsConsumerPerField.finishDocument.
    flush_term: BytesRef<Vec<u8>>,
    // Used by TermVectorsConsumerPerField when serializing the term vectors.
    vector_slice_reader_pos: ByteSliceReader,
    vector_slice_reader_off: ByteSliceReader,
    has_vectors: bool,
    num_vector_fields: i32,
    last_doc_id: i32,
    per_fields: Vec<TermVectorsConsumerPerField<O, P, T>>,
}
impl<D, C, TVW, O, P, T> TermVectorsConsumer<D, C, TVW, O, P, T>
where
    D: Directory,
    C: Codec,
    TVW: TermVectorsWriter,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    pub(crate) fn reset_fields(&mut self) {
        self.per_fields.clear(); // don't hang onto stuff from previous doc
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
    ) -> Result<()> {
        if self.writer.is_none() {
            let flush_info = FlushInfo::new(self.last_doc_id, bytes_used.borrow().get());
            let context = IOContext::with_flush(flush_info)?;

            let writer = self.code.lock().term_vectors_format().vectors_writer(
                Arc::clone(&self.directory),
                Rc::clone(&self.info),
                &context,
            )?;

            self.last_doc_id = 0;
        }
        Ok(())
    }
    pub(crate) fn flush<DM>(
        &mut self,
        state: &mut SegmentWriteState<D>,
        sort_map: &Option<Rc<DM>>,
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
    ) -> Result<()> {
        // if !self.has_vectors {
        //     return Ok(());
        // }
        //
        // ArrayUtil::intro_sort_with_range(&mut self.per_fields, 0, self.num_vector_fields)?;
        //
        // self.init_term_vectors_writer(bytes_used)?;
        // self.fill(doc_id)?;
        // // Append term vectors to the real outputs:
        // if let Some(ref mut writer) = self.writer {
        //     writer.start_document(self.num_vector_fields)?;
        //
        //     for i in 0..self.num_vector_fields as usize {
        //         self.per_fields[i].finish_document()?;
        //     }
        //
        //     writer.finish_document()?;
        // } else {
        //     return Err(LuceneError::illegal_state(
        //         "TermVectorsConsumer writer was not initialized",
        //     ));
        // }
        //
        // debug_assert_eq!(
        //     self.last_doc_id, doc_id,
        //     "last_doc_id = {}, doc_id = {}",
        //     self.last_doc_id, doc_id
        // );
        //
        // self.last_doc_id += 1;
        // self.reset_fields();

        Ok(())
    }
    pub(crate) fn start_document(&mut self) -> Result<()> {
        self.reset_fields();
        self.num_vector_fields = 0;
        Ok(())
    }
    pub(crate) fn add_field(
        &mut self,
        _field_invert_state: Rc<FieldInvertState<O, P, T>>,
        _field_info: Rc<FieldInfo>,
    ) -> TermVectorsConsumerPerField<O, P, T> {
        todo!()
    }
}
