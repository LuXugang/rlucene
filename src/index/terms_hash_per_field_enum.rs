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
use crate::index::byte_slice_reader::ByteSliceReader;
use crate::index::freq_prox_terms_writer_per_field::FreqProxTermsWriterPerField;
use crate::index::term_vectors_consumer_per_field::TermVectorsConsumerPerField;
#[cfg(test)]
use crate::index::terms_hash_per_field::tests::TermsHashPerFieldMock;
use crate::index::terms_hash_per_field::TermsHashPerFieldBase;
use crate::index::BytesRef;
use crate::util::error::lucene_error::Result;
use crate::util::ByteBlockPoolBorrow;
#[allow(unused)]
pub(crate) enum TermsHashPerFieldEnum {
    TermVectorsConsumer(TermVectorsConsumerPerField),
    FreqProxTermsWriter(FreqProxTermsWriterPerField),
    #[cfg(test)]
    Mock(TermsHashPerFieldMock),
}

#[allow(unused)]
impl TermsHashPerFieldEnum {
    pub(crate) fn reset(&mut self) {
        match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(inner) => inner.parent_per_field.reset(),
            TermsHashPerFieldEnum::FreqProxTermsWriter(inner) => inner.parent_per_field.reset(),
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(inner) => inner.parent_per_field.reset(),
        }
    }
    pub fn init_reader(&self, reader: &mut ByteSliceReader, term_id: i32, stream: i32) {
        match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(inner) => {
                inner.parent_per_field.init_reader(reader, term_id, stream)
            }
            TermsHashPerFieldEnum::FreqProxTermsWriter(inner) => {
                inner.parent_per_field.init_reader(reader, term_id, stream)
            }
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(inner) => {
                inner.parent_per_field.init_reader(reader, term_id, stream)
            }
        }
    }
    fn reinit_hash(&mut self) {
        match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(inner) => {
                inner.parent_per_field.reinit_hash()
            }
            TermsHashPerFieldEnum::FreqProxTermsWriter(inner) => {
                inner.parent_per_field.reinit_hash()
            }
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(inner) => inner.parent_per_field.reinit_hash(),
        }
    }

    pub(crate) fn write_byte(&mut self, stream: i32, b: u8) -> Result<()> {
        match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(inner) => {
                inner.parent_per_field.write_byte(stream, b)
            }
            TermsHashPerFieldEnum::FreqProxTermsWriter(inner) => {
                inner.parent_per_field.write_byte(stream, b)
            }
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(inner) => inner.parent_per_field.write_byte(stream, b),
        }
    }

    pub(crate) fn write_bytes(
        &mut self,
        stream: i32,
        b: &[u8],
        offset: i32,
        len: i32,
    ) -> Result<()> {
        match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(inner) => {
                inner.parent_per_field.write_bytes(stream, b, offset, len)
            }
            TermsHashPerFieldEnum::FreqProxTermsWriter(inner) => {
                inner.parent_per_field.write_bytes(stream, b, offset, len)
            }
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(inner) => {
                inner.parent_per_field.write_bytes(stream, b, offset, len)
            }
        }
    }

    pub(crate) fn write_vint(&mut self, stream: i32, i: i32) -> Result<()> {
        match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(inner) => {
                inner.parent_per_field.write_vint(stream, i)
            }
            TermsHashPerFieldEnum::FreqProxTermsWriter(inner) => {
                inner.parent_per_field.write_vint(stream, i)
            }
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(inner) => inner.parent_per_field.write_vint(stream, i),
        }
    }
    pub(crate) fn get_byte_block_pool(&self) -> ByteBlockPoolBorrow {
        match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(t) => t.parent_per_field.byte_pool.clone(),
            TermsHashPerFieldEnum::FreqProxTermsWriter(t) => t.parent_per_field.byte_pool.clone(),
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(t) => t.parent_per_field.byte_pool.clone(),
        }
    }
    fn add_with_text_start(&mut self, text_start: i32, doc_id: i32) -> Result<()> {
        let parent = match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(inner) => &mut inner.parent_per_field,
            TermsHashPerFieldEnum::FreqProxTermsWriter(inner) => &mut inner.parent_per_field,
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(inner) => &mut inner.parent_per_field,
        };
        let term_id = parent.bytes_hash.add_by_pool_offset(text_start)?;
        if term_id >= 0 {
            self.init_stream_slices(term_id, doc_id)?;
        } else {
            self.position_stream_slice(term_id, doc_id)?;
        }
        Ok(())
    }
    /// Called once per inverted token. This is the primary entry point (for first TermsHash); postings
    /// use this API.
    pub(crate) fn add_with_bytes_ref(
        &mut self,
        term_bytes: &BytesRef<Vec<u8>>,
        doc_id: i32,
    ) -> Result<()> {
        let mut term_id;
        {
            let parent = match self {
                TermsHashPerFieldEnum::TermVectorsConsumer(inner) => &mut inner.parent_per_field,
                TermsHashPerFieldEnum::FreqProxTermsWriter(inner) => &mut inner.parent_per_field,
                #[cfg(test)]
                TermsHashPerFieldEnum::Mock(inner) => &mut inner.parent_per_field,
            };
            debug_assert!(parent.assert_doc_id(doc_id));
            // We are first in the chain so we must "intern" the
            // term text into textStart address
            // Get the text & hash of this term.
            term_id = parent.bytes_hash.add(term_bytes)?;
        }
        if term_id >= 0 {
            self.init_stream_slices(term_id, doc_id)?;
        } else {
            term_id = self.position_stream_slice(term_id, doc_id)?;
        }
        let parent = match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(inner) => &mut inner.parent_per_field,
            TermsHashPerFieldEnum::FreqProxTermsWriter(inner) => &mut inner.parent_per_field,
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(inner) => &mut inner.parent_per_field,
        };
        if parent.do_next_call {
            debug_assert!(parent.next_per_field.is_some());
            if let Some(ref next_per_field) = parent.next_per_field {
                let mut next_per_field = next_per_field.borrow_mut();
                let postings_array_wrapper = parent.postings_array_wrapper.borrow_mut();
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
}

impl TermsHashPerFieldBase for TermsHashPerFieldEnum {
    fn init_stream_slices(&mut self, term_id: i32, doc_id: i32) -> Result<()> {
        match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(t) => t.init_stream_slices(term_id, doc_id),
            TermsHashPerFieldEnum::FreqProxTermsWriter(t) => t.init_stream_slices(term_id, doc_id),
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(t) => t.init_stream_slices(term_id, doc_id),
        }
    }

    fn position_stream_slice(&mut self, term_id: i32, doc_id: i32) -> Result<i32> {
        match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(t) => {
                t.position_stream_slice(term_id, doc_id)
            }
            TermsHashPerFieldEnum::FreqProxTermsWriter(t) => {
                t.position_stream_slice(term_id, doc_id)
            }
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(t) => t.position_stream_slice(term_id, doc_id),
        }
    }

    fn start(&mut self, field: &Fields, first: bool) -> Result<bool> {
        match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(t) => t.start(field, first),
            TermsHashPerFieldEnum::FreqProxTermsWriter(t) => t.start(field, first),
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(t) => t.start(field, first),
        }
    }

    fn new_term(&mut self, term_id: i32, doc_id: i32) -> Result<()> {
        match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(t) => t.new_term(term_id, doc_id),
            TermsHashPerFieldEnum::FreqProxTermsWriter(t) => t.new_term(term_id, doc_id),
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(t) => t.new_term(term_id, doc_id),
        }
    }

    fn add_term(&mut self, term_id: i32, doc_id: i32) -> Result<()> {
        match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(t) => t.add_term(term_id, doc_id),
            TermsHashPerFieldEnum::FreqProxTermsWriter(t) => t.add_term(term_id, doc_id),
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(t) => t.add_term(term_id, doc_id),
        }
    }

    fn finish(&mut self) {
        match self {
            TermsHashPerFieldEnum::TermVectorsConsumer(t) => t.finish(),
            TermsHashPerFieldEnum::FreqProxTermsWriter(t) => t.finish(),
            #[cfg(test)]
            TermsHashPerFieldEnum::Mock(t) => t.finish(),
        }
    }
}
