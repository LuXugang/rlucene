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
use crate::index::terms_hash::TermsHashBase;
use crate::index::terms_hash_per_field::TermsHashPerField;
use crate::index::BytesRef;
use crate::store::directory::Directory;
use crate::store::flush_info::FlushInfo;
use crate::store::IOContext;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{Counter, CounterEnumBorrow};
use parking_lot::Mutex;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct TermVectorsConsumer<D, C, TVW>
where
    D: Directory,
    C: Codec,
    TVW: TermVectorsWriter,
{
    directory: Arc<Mutex<D>>,
    info: Rc<SegmentInfo<D>>,
    code: Arc<Mutex<C>>,
    writer: Option<TVW>,
    flush_term: BytesRef<Vec<u8>>,
    vector_slice_reader_pos: ByteSliceReader,
    vector_slice_reader_off: ByteSliceReader,
    has_vectors: bool,
    num_vector_fields: i32,
    last_doc_id: i32,
    per_fields: Vec<TermVectorsConsumerPerField>,
}
impl<D, C, TVW> TermVectorsConsumer<D, C, TVW>
where
    D: Directory,
    C: Codec,
    TVW: TermVectorsWriter,
{
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
    pub(crate) fn init_term_vectors_writer(&mut self, bytes_used: CounterEnumBorrow) -> Result<()> {
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
        todo!()
    }
}

impl<D, C, TVW> TermsHashBase for TermVectorsConsumer<D, C, TVW>
where
    D: Directory,
    C: Codec,
    TVW: TermVectorsWriter,
{
    fn abort(&mut self) {
        todo!()
    }

    type TermsHashPerFieldBase = TermVectorsConsumerPerField;

    fn add_field<O, P, T>(
        &mut self,
        _field_invert_state: Rc<FieldInvertState<O, P, T>>,
        _field_info: Rc<FieldInfo>,
    ) -> TermsHashPerField<Self::TermsHashPerFieldBase, O, P, T>
    where
        O: OffsetAttribute,
        P: PayloadAttribute,
        T: TermFrequencyAttribute,
    {
        todo!()
    }

    fn start_document(&mut self) -> Result<()> {
        todo!()
    }

    fn finish_document(&mut self, _doc_id: i32) -> Result<()> {
        todo!()
    }
}
