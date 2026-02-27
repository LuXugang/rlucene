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
use crate::core::codecs::block_tree::lucene90_block_tree_terms_writer::Lucene90BlockTreeTermsWriter;
use crate::core::codecs::fields_producer::FieldsProducer;
use crate::core::codecs::lucene101::lucene101_postings_writer::Lucene101PostingsWriter;
use crate::core::codecs::norms_producer::NormsProducer;
use crate::core::codecs::push_postings_writer_base::PushPostingsWriterBase;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::fields::Fields;
use crate::core::index::mapped_multi_fields::MappedMultiFields;
use crate::core::index::merge_state::MergeState;
use crate::core::index::multi_fields::MultiFields;
use crate::core::index::reader_slice::ReaderSlice;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use std::rc::Rc;

/// Abstract API that consumes terms, doc, freq, prox, offset and payloads postings. Concrete
/// implementations of this actually do "something" with the postings (write it into the index in a
/// specific format).
pub trait FieldsConsumer {
    /// Write all fields, terms and postings. This is the "pull" API, allowing you to iterate more than
    /// once over the postings, somewhat analogous to using a DOM API to traverse an XML tree.
    ///
    /// # Notes
    ///
    /// - You must compute index statistics, including each Term’s `doc_freq` and `total_term_freq`, as
    ///   well as the summary `sum_total_term_freq`, `sum_total_doc_freq` and `doc_count`.
    /// - You must skip terms that have no docs and fields that have no terms, even though the
    ///   provided `Fields` API will expose them; this typically requires lazily writing the field or
    ///   term until you’ve actually seen the first term or document.
    /// - The provided `Fields` instance is limited: you cannot call any methods that return
    ///   statistics/counts; you cannot pass a non-null live docs when pulling docs/positions enums.
    fn write<F, N>(&mut self, fields: &mut F, norms: Option<&N>) -> Result<()>
    where
        F: Fields,
        N: NormsProducer;
    /// Merges the fields from the readers in `merge_state`.
    ///
    /// The default implementation skips and maps around deleted documents, and
    /// calls [`Self::write`] with the merged [`Fields`] and the provided
    /// [`NormsProducer`].
    ///
    /// Implementations may override this method to perform more sophisticated
    /// merging strategies (such as bulk byte copying, etc.).
    fn merge<D, N, CR>(&mut self, merge_state: &MergeState<D, CR>, norms: Option<&N>) -> Result<()>
    where
        D: Directory,
        N: NormsProducer,
        CR: CodecReader,
    {
        let mut fields = Vec::new();
        let mut slices = Vec::new();

        let mut doc_base = 0;

        for reader_index in 0..merge_state.fields_producers.len() {
            let f = &merge_state.fields_producers[reader_index];
            let max_doc = merge_state.max_docs[reader_index] as usize;

            if let Some(f) = f {
                f.check_integrity()?;
                slices.push(Rc::new(ReaderSlice::new(
                    doc_base,
                    max_doc as i32,
                    reader_index as i32,
                )));
                fields.push(f);
            }

            doc_base += max_doc;
        }

        let field = MultiFields::new(fields, slices);
        let mut merged_fields = MappedMultiFields::new(merge_state, &field);

        self.write(&mut merged_fields, norms)
    }

    fn close(&mut self) -> Result<()>;
}
pub type FieldsConsumerEnum<O> =
    Lucene90BlockTreeTermsWriter<O, PushPostingsWriterBase<Lucene101PostingsWriter<O>>>;
