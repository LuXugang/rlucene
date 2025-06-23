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
use crate::analysis::analyzer::Analyzer;
use crate::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::analysis::token_stream::TokenStream;
use crate::document::invertable_field::InvertableType;
use crate::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::index::doc_values_type::DocValuesType;
use crate::index::doc_values_writer::DocValuesWriter;
use crate::index::field_info::FieldInfo;
use crate::index::field_infos::build::Builder;
use crate::index::field_invert_state::FieldInvertState;
use crate::index::freq_prox_terms_writer::FreqProxTermsWriter;
use crate::index::freq_prox_terms_writer_per_field::FreqProxTermsWriterPerField;
use crate::index::index_options::IndexOptions;
use crate::index::index_writer::IndexWriter;
use crate::index::indexable_field::IndexableField;
use crate::index::indexable_field_type::IndexableFieldType;
use crate::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::index::norm_values_writer::NormValuesWriter;
use crate::index::point_values_writer::PointValuesWriter;
use crate::index::segment_info::SegmentInfo;
use crate::index::sorting_stored_fields_consumer::SortingStoredFieldsConsumer;
use crate::index::stored_fields_consumer::StoredFieldsConsumer;
use crate::index::term_vectors_consumer::TermVectorsConsumer;
use crate::index::vector_encoding::VectorEncoding;
use crate::index::vector_similarity_function::VectorSimilarityFunction;
use crate::index::BytesRef;
use crate::search::similarities::similarities::Similarity;
use crate::store::directory::Directory;
use crate::util::access::Access;
use crate::util::allocator_byte::STAllocatorByteEnum;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::int_block_pool::{ibp_util, AllocatorI32, AllocatorIntEnum};
use crate::util::{ByteBlockPoolBorrow, Counter, CounterEnum, CounterEnumBorrow, SliceCopyOps};
use parking_lot::Mutex;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;
use std::sync::Arc;

struct IndexingChain<D1, D2, A, S, O, P, T, DW, IF>
where
    D1: Directory,
    D2: Directory,
    A: Analyzer,
    S: Similarity,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    DW: DocValuesWriter,
    IF: IndexableField,
{
    bytes_used: CounterEnumBorrow,
    field_infos: Builder,
    terms_hash: FreqProxTermsWriter<D1, O, P, T>,
    doc_values_byte_pool: ByteBlockPoolBorrow,
    stored_fields_consumer: StoredFieldsConsumer<D1, D2, SortingStoredFieldsConsumer<D2>>,
    term_vectors_writer: TermVectorsConsumer<D1, O, P, T>,
    field_hash: Vec<Option<Rc<PerField<A, S, O, P, T, DW, IF>>>>,
    hash_mask: usize,
    total_field_count: usize,
    next_field_gen: i64,
    fields: Vec<Rc<PerField<A, S, O, P, T, DW, IF>>>,
    doc_fields: Vec<Rc<PerField<A, S, O, P, T, DW, IF>>>,
    byte_block_allocator: STAllocatorByteEnum,
    index_writer_config: Arc<LiveIndexWriterConfig>,
    index_created_version_major: i32,
    has_hit_aborting_exception: bool,
}
impl<D1, D2, A, S, O, P, T, DW, IF> IndexingChain<D1, D2, A, S, O, P, T, DW, IF>
where
    D1: Directory,
    D2: Directory,
    A: Analyzer,
    S: Similarity,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    DW: DocValuesWriter,
    IF: IndexableField,
{
    fn new(
        index_created_version_major: i32,
        segment_info: Rc<SegmentInfo<D2>>,
        directory: Arc<Mutex<D2>>,
        field_infos: Builder,
        index_writer_config: Arc<LiveIndexWriterConfig>,
    ) -> Self {
        // let bytes_used = Rc::new(RefCell::new(CounterEnum::new_counter(false)));
        // let byte_block_allocator =
        //     AllocatorByteEnum::DTA(DirectTrackingAllocatorByte::new(bytes_used.clone()));
        // let (stored_fields_consumer, term_vectors_writer) =
        //     if segment_info.get_index_sort().is_none() {
        //         (
        //             StoredFieldsConsumerEnum::UnSort(StoredFieldsConsumer::new(
        //                 Arc::clone(&directory),
        //                 Rc::clone(&segment_info),
        //             )),
        //             TermVectorsConsumer::new(
        //                 IntBlockAllocator::allocator_enum(bytes_used.clone()),
        //                 DirectTrackingAllocatorByte::allocator_enum(bytes_used.clone()),
        //                 Arc::clone(&directory),
        //                 Rc::clone(&segment_info),
        //             ),
        //         )
        //     } else {
        //         (
        //             StoredFieldsConsumerEnum::Sort(SortingStoredFieldsConsumer::new(
        //                 Arc::clone(&directory),
        //                 Rc::clone(&segment_info),
        //             )),
        //             SortingTermVectorsConsumer::new(
        //                 IntBlockAllocator::allocator_enum(bytes_used.clone()),
        //                 DirectTrackingAllocatorByte::allocator_enum(bytes_used.clone()),
        //                 Arc::clone(&directory),
        //                 Rc::clone(&segment_info),
        //             ),
        //         )
        //     };
        //
        // // postings writer
        // let terms_hash = FreqProxTermsWriter::new(
        //     IntBlockAllocator::allocator_enum(bytes_used.clone()),
        //     DirectTrackingAllocatorByte::allocator_enum(bytes_used.clone()),
        //     bytes_used.clone(),
        //     term_vectors_writer.clone(),
        // );
        //
        // let doc_values_byte_pool = Rc::new(RefCell::new(ByteBlockPool::new(
        //     DirectTrackingAllocatorByte::allocator_enum(bytes_used.clone()),
        // )));
        //
        // IndexingChain {
        //     bytes_used,
        //     field_infos,
        //     terms_hash,
        //     doc_values_byte_pool,
        //     stored_fields_consumer,
        //     // vector_values_consumer,
        //     term_vectors_writer,
        //     field_hash: vec![None; 2],
        //     hash_mask: 1,
        //     total_field_count: 0,
        //     next_field_gen: 0,
        //     fields: Vec::with_capacity(1),
        //     doc_fields: Vec::with_capacity(2),
        //     byte_block_allocator,
        //     index_writer_config,
        //     index_created_version_major,
        //     has_hit_aborting_exception: false,
        // }
        todo!()
    }
}

pub(crate) struct PerField<A, S, O, P, T, DW, IF>
where
    A: Analyzer,
    S: Similarity,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    DW: DocValuesWriter,
    IF: IndexableField,
{
    pub(crate) field_name: String,
    pub(crate) index_created_version_major: i32,
    pub(crate) schema: FieldSchema,
    pub(crate) reserved: bool,
    pub(crate) field_info: Option<Rc<FieldInfo>>,
    pub(crate) similarity: Arc<S>,
    pub(crate) invert_state: Option<FieldInvertState<O, P, T>>,
    pub(crate) terms_hash_per_field: Option<FreqProxTermsWriterPerField<O, P, T>>,
    pub(crate) doc_values_writer: Option<DW>,
    pub(crate) point_values_writer: Option<PointValuesWriter>,
    // pub(crate) knn_field_vectors_writer: Option<KnnFieldVectorsWriter>,
    pub(crate) field_gen: i64,
    pub(crate) next: Option<Box<PerField<A, S, O, P, T, DW, IF>>>,
    pub(crate) norms: Option<NormValuesWriter>,
    pub(crate) token_stream: Option<IF::TokenStream>,
    pub(crate) analyzer: Arc<A>,
    pub(crate) first: bool,
}
impl<A, S, O, P, T, DW, IF> PerField<A, S, O, P, T, DW, IF>
where
    A: Analyzer,
    S: Similarity,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    DW: DocValuesWriter,
    IF: IndexableField,
{
    pub(crate) fn new(
        field_name: impl Into<String>,
        index_created_version_major: i32,
        schema: FieldSchema,
        similarity: Arc<S>,
        analyzer: Arc<A>,
        reserved: bool,
    ) -> Self {
        PerField {
            field_name: field_name.into(),
            index_created_version_major,
            schema,
            reserved,
            field_info: None,
            similarity,
            invert_state: None,
            terms_hash_per_field: None,
            doc_values_writer: None,
            point_values_writer: None,
            field_gen: -1,
            next: None,
            norms: None,
            token_stream: None,
            analyzer,
            first: false,
        }
    }
    pub(crate) fn reset(&mut self, doc_id: i32) {
        self.first = true;
        self.schema.reset(doc_id);
    }

    pub(crate) fn set_field_info(&mut self, field_info: FieldInfo) {
        assert!(self.field_info.is_none());
        self.field_info = Some(Rc::new(field_info));
    }
    pub(crate) fn set_invert_state<D>(
        &mut self,
        terms_hash: &mut FreqProxTermsWriter<D, O, P, T>,
        term_vectors_writer: &mut TermVectorsConsumer<D, O, P, T>,
        bytes_used: CounterEnumBorrow,
    ) -> Result<()>
    where
        D: Directory,
    {
        let fi = Rc::clone(self.field_info.as_ref().unwrap());
        let state = FieldInvertState::new(
            self.index_created_version_major,
            fi.name.clone(),
            *fi.get_index_options(),
        );
        self.invert_state = Some(state);
        // self.terms_hash_per_field = Some(terms_hash.add_field(
        //     self.invert_state.as_ref().unwrap().clone(),
        //     self.field_info.as_ref().unwrap().clone(),
        // ));

        if !fi.omits_norms() {
            // Even if no documents actually succeed in setting a norm, we still write norms for this
            // segment
            debug_assert!(self.norms.is_none());
            self.norms = Some(NormValuesWriter::new(fi.clone(), bytes_used)?);
        }

        if fi.has_term_vectors() {
            term_vectors_writer.set_has_vectors();
        }
        Ok(())
    }
    pub(crate) fn finish<D>(
        &mut self,
        doc_id: i32,
        term_vectors_consumer: &mut TermVectorsConsumer<D, O, P, T>,
    ) -> Result<()>
    where
        D: Directory,
    {
        if !self.field_info.as_ref().unwrap().omits_norms() {
            let norm_value = {
                let state = self.invert_state.as_ref().unwrap();
                if state.length == 0 {
                    // the field exists in this document, but it did not have
                    // any indexed tokens, so we assign a default value of zero
                    // to the norm
                    0
                } else {
                    let nv = self.similarity.compute_norm(state)?;
                    if nv == 0 {
                        return Err(LuceneError::illegal_state(format!(
                            "Similarity {} returned 0 for non-empty field",
                            self.similarity
                        )));
                    }
                    nv
                }
            };
            self.norms.as_mut().unwrap().add_value(doc_id, norm_value)?;
        }
        self.terms_hash_per_field
            .as_mut()
            .unwrap()
            .finish(term_vectors_consumer);
        Ok(())
    }
    /// Inverts one field for one document; first is true if this is the first time we are seeing
    /// this field name in this document.
    pub(crate) fn invert<F>(&mut self, doc_id: i32, field: &F, first: bool) -> Result<()>
    where
        F: IndexableField,
    {
        debug_assert!(
            *field.field_type().index_options() >= IndexOptions::Docs,
            "field must be indexed with at least Docs"
        );

        if first {
            match &mut self.invert_state {
                Some(invert_state) => {
                    // First time we're seeing this field (indexed) in this document
                    invert_state.reset()
                },
                None => {
                    return Err(LuceneError::illegal_state("invert_state not initialized"));
                },
            }
        }

        match field.invertable_type() {
            InvertableType::BINARY => {
                self.invert_term(doc_id, field, first)?;
            },
            InvertableType::TokenStream => {
                // self.invert_token_stream(doc_id, field, first)?;
            },
        }

        Ok(())
    }
    fn invert_token_stream(&mut self, doc_id: i32, field: &IF, first: bool) -> Result<()> {
        let analyzed = field.field_type().tokenized();
        /*
         * To assist people in tracking down problems in analysis components, we wish to write the field name to the infostream
         * when we fail. We expect some caller to eventually deal with the real exception, so we don't want any 'catch' clauses,
         * but rather a finally that takes note of the problem.
         */

        // obtain and reuse TokenStream
        let mut stream = field.token_stream(&*self.analyzer, self.token_stream.take())?;

        let mut succeeded = false;
        // ensure end()/close() semantics
        let result = (|| {
            stream.reset()?;
            {
                let state = self.invert_state.as_mut().unwrap();
                // state.set_attribute_source(Some(&mut stream));
            }
            let terms_hash_per_field = self.terms_hash_per_field.as_mut().unwrap();
            terms_hash_per_field.start(field, first)?;

            while stream.increment_token()? {
                // If we hit an exception in stream.next below
                // (which is fairly common, e.g. if analyzer
                // chokes on a given document), then it's
                // non-aborting and (above) this one document
                // will be marked as deleted, but still
                // consume a docID
                let invert_state = self.invert_state.as_mut().unwrap();
                // TODO
                let pos_incr = 0;
                // let pos_incr = state.pos_incr_attribute.get_position_increment();
                invert_state.position += pos_incr;
                if invert_state.position < invert_state.last_position {
                    if pos_incr == 0 {
                        return Err(LuceneError::illegal_argument(format!(
                            "first position increment must be > 0 (got 0) for field '{}'",
                            field.name()
                        )));
                    } else if pos_incr < 0 {
                        // position increment must be > 0
                        return Err(LuceneError::illegal_argument(format!(
                            "position increment must be > 0 (got {}) for field '{}'",
                            pos_incr,
                            field.name()
                        )));
                    } else {
                        return Err(LuceneError::illegal_argument(format!(
                            "position overflowed Integer.MAX_VALUE (got posIncr={} last_position={} position={}) for field '{}'",
                            pos_incr, invert_state.last_position, invert_state.position, field.name()
                        )));
                    }
                } else if invert_state.position > IndexWriter::MAX_POSITION {
                    return Err(LuceneError::illegal_argument(format!(
                        "position {} too large for field {}",
                        invert_state.position,
                        field.name()
                    )));
                }
                if pos_incr == 0 {
                    invert_state.num_overlap += 1;
                }
                invert_state.last_position = invert_state.position;

                let start_offset = invert_state.offset
                    + invert_state
                        .offset_attribute
                        .as_ref()
                        .unwrap()
                        .start_offset();
                let end_offset = invert_state.offset
                    + invert_state.offset_attribute.as_ref().unwrap().end_offset();
                if start_offset < invert_state.last_start_offset || end_offset < start_offset {
                    return Err(LuceneError::illegal_argument(format!(
                        "startOffset must be non-negative, and endOffset must be >= startOffset, and offsets must not go backwards offsets: start={} end={} last_start={} for field {}",
                        start_offset,
                        end_offset,
                        invert_state.last_start_offset,
                        field.name()
                    )));
                }
                invert_state.last_start_offset = start_offset;

                // update length
                let tf = invert_state
                    .term_freq_attribute
                    .as_ref()
                    .unwrap()
                    .get_term_frequency();
                invert_state.length = invert_state.length.checked_add(tf).ok_or_else(|| {
                    LuceneError::number_overflow(format!(
                        "too many tokens for field {}",
                        field.name()
                    ))
                })?;
                // If we hit an exception in here, we abort
                // all buffered documents since the last
                // flush, on the likelihood that the
                // internal state of the terms hash is now
                // corrupt and should not be flushed to a
                // new segment:
                if let Err(e) =
                    // TODO
                    terms_hash_per_field.add_with_bytes_ref(&BytesRef::new(), doc_id)
                // terms_hash_per_field.add_with_bytes_ref(invert_state.term_attribute.as_ref().unwrap().get_bytes_ref(), doc_id)
                {
                    let mut prefix = [0u8; 30];
                    // TODO
                    let big_term: BytesRef<Vec<u8>> = BytesRef::new();
                    // let big_term =invert_state.term_attribute.as_ref().unwrap().get_bytes_ref();
                    prefix.copy_from(&big_term.bytes[big_term.offset..big_term.offset + 30], 0);
                    return Err(LuceneError::illegal_argument(format!(
                        "Document contains at least one immense term in field=\"{}\" (whose UTF8 encoding is longer than the max length {}), all of which were skipped. Please correct the analyzer to not produce such terms. The prefix of the first immense term is: '{:?}...', original message: {}",
                        self.field_info.as_ref().unwrap().name,
                        IndexWriter::MAX_TERM_LENGTH,
                        prefix,
                        e
                    )));
                }
            }
            // trigger streams to perform end-of-stream operations
            stream.end()?;
            {
                // when we come back around to the field...
                let invert_state = self.invert_state.as_mut().unwrap();
                // TODO
                invert_state.position += 0;
                // invert_state.position += invert_state.pos_incr_attribute.as_ref().unwrap().get_position_increment();
                invert_state.offset += invert_state.offset_attribute.as_ref().unwrap().end_offset();
            }

            succeeded = true;
            Ok(())
        })();

        // if !succeeded && self.info_stream.is_enabled("DW") {
        //     self.info_stream.message(
        //         "DW",
        //         &format!("exception in invert_token_stream for {}", field.name()),
        //     );
        // }

        result?;
        self.token_stream = Some(stream);

        if analyzed {
            let invert_state = self.invert_state.as_mut().unwrap();
            invert_state.position += self
                .analyzer
                .get_position_increment_gap(&self.field_info.as_ref().unwrap().name);
            invert_state.offset += self
                .analyzer
                .get_offset_gap(&self.field_info.as_ref().unwrap().name);
        }
        Ok(())
    }

    fn invert_term<F>(&mut self, doc_id: i32, field: &F, first: bool) -> Result<()>
    where
        F: IndexableField,
    {
        let binary_value = field
            .binary_value()?
            .ok_or_else(|| LuceneError::illegal_argument(format!(
                "Field {} returns TERM for invertable_type() and null for binary_value(), which is illegal",
                field.name()
            )))?;

        let field_type = field.field_type();
        if field_type.tokenized()
            || *field_type.index_options() > IndexOptions::DocsAndFreqs
            || field_type.store_term_vector_positions()
            || field_type.store_term_vector_offsets()
            || field_type.store_term_vector_payloads()
        {
            return Err(LuceneError::illegal_argument(format!(
                "Fields that are tokenized or index proximity data must produce a non-null TokenStream, but {} did not",
                field.name()
            )));
        }
        let state = self.invert_state.as_mut().unwrap();
        // TODO
        // state.set_attribute_source();
        state.position += 1;
        state.length += 1;
        let terms_hash_per_field = self.terms_hash_per_field.as_mut().unwrap();
        terms_hash_per_field.start(field, first)?;
        match state.length.checked_add(1) {
            Some(new_length) => {
                state.length = new_length;
            },
            None => {
                return Err(LuceneError::number_overflow(
                    "Field length overflowed".to_string(),
                ));
            },
        }

        if let Err(e) = terms_hash_per_field.add_with_bytes_ref(&binary_value, doc_id) {
            let mut prefix = [0u8; 30];
            prefix.copy_from(
                &binary_value.bytes[binary_value.offset..binary_value.offset + 30],
                0,
            );
            let msg = format!(
                "Document contains at least one immense term in field=\"{}\" (whose length is longer than the max length {}), all of which were skipped. The prefix of the first immense term is: '{:?}...'",
                self.field_info.as_ref().unwrap().name,
                IndexWriter::MAX_TERM_LENGTH,
                prefix
            );
            // if self.info_stream.is_enabled("IW") {
            //     self.info_stream.message("IW", &format!("ERROR: {}", msg));
            // }
            return Err(LuceneError::illegal_state(format!("{} {}", msg, e)));
        }
        Ok(())
    }
}

impl<A, S, O, P, T, DW, IF> PartialEq for PerField<A, S, O, P, T, DW, IF>
where
    A: Analyzer,
    S: Similarity,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    DW: DocValuesWriter,
    IF: IndexableField,
{
    fn eq(&self, other: &Self) -> bool {
        self.field_name == other.field_name
    }
}
impl<A, S, O, P, T, DW, IF> Eq for PerField<A, S, O, P, T, DW, IF>
where
    A: Analyzer,
    S: Similarity,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    DW: DocValuesWriter,
    IF: IndexableField,
{
}

impl<A, S, O, P, T, DW, IF> PartialOrd for PerField<A, S, O, P, T, DW, IF>
where
    A: Analyzer,
    S: Similarity,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    DW: DocValuesWriter,
    IF: IndexableField,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.field_name.cmp(&other.field_name))
    }
}
impl<A, S, O, P, T, DW, IF> Ord for PerField<A, S, O, P, T, DW, IF>
where
    A: Analyzer,
    S: Similarity,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    DW: DocValuesWriter,
    IF: IndexableField,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.field_name.cmp(&other.field_name)
    }
}

pub(crate) struct IntBlockAllocator<C>
where
    C: Access<CounterEnum>,
{
    block_size: usize,
    pub(crate) byte_used: C,
}
impl<C> IntBlockAllocator<C>
where
    C: Access<CounterEnum>,
{
    fn new(byte_used: C) -> Self {
        IntBlockAllocator {
            block_size: ibp_util::INT_BLOCK_SIZE as usize,
            byte_used,
        }
    }
    fn allocator_enum(byte_used: C) -> AllocatorIntEnum<C> {
        AllocatorIntEnum::IBA(IntBlockAllocator::new(byte_used))
    }
}
impl<C> AllocatorI32 for IntBlockAllocator<C>
where
    C: Access<CounterEnum>,
{
    fn recycle_int_blocks(&mut self, _blocks: &[Vec<i32>], _offset: usize, length: usize) {
        self.byte_used.access_mut(|byte_used| {
            let delta = length as i64 * (self.block_size as i64 * BitUtil::INT_BYTES as i64);
            byte_used.add_and_get(-delta);
        });
    }

    fn get_byte_block(&mut self) -> Vec<i32> {
        let b = vec![0; ibp_util::INT_BLOCK_SIZE as usize];
        self.byte_used.access_mut(|byte_used| {
            byte_used.add_and_get(ibp_util::INT_BLOCK_SIZE as i64 * BitUtil::INT_BYTES as i64);
        });
        b
    }

    fn get_block_size(&self) -> usize {
        self.block_size
    }
}

/// A schema of the field in the current document. With every new document this schema is reset.
/// As the document’s fields are processed, we update the schema with any options encountered in
/// this document. Once processing for the document is complete, we compare the built schema of
/// the current document with the corresponding `FieldInfo` (constructed from the first document
/// in the segment where this field appeared). If there is any inconsistency, we return an error.
/// This ensures that a field’s data structures remain consistent across all documents.
pub(crate) struct FieldSchema {
    name: String,
    doc_id: i32,
    attributes: HashMap<String, String>,
    omit_norms: bool,
    store_term_vector: bool,
    index_options: IndexOptions,
    doc_values_type: DocValuesType,
    doc_values_skip_index: DocValuesSkipIndexType,
    point_dimension_count: i32,
    point_index_dimension_count: i32,
    point_num_bytes: i32,
    vector_dimension: i32,
    vector_encoding: VectorEncoding,
    vector_similarity_function: VectorSimilarityFunction,
}
impl FieldSchema {
    const ERR_MSG: &'static str =
        "Inconsistency of field data structures across documents for field ";
    pub(crate) fn new(name: &str) -> Self {
        FieldSchema {
            name: name.to_string(),
            doc_id: 0,
            attributes: HashMap::new(),
            omit_norms: false,
            store_term_vector: false,
            index_options: IndexOptions::None,
            doc_values_type: DocValuesType::None,
            doc_values_skip_index: DocValuesSkipIndexType::None,
            point_dimension_count: 0,
            point_index_dimension_count: 0,
            point_num_bytes: 0,
            vector_dimension: 0,
            vector_encoding: VectorEncoding::FLOAT32(4),
            vector_similarity_function: VectorSimilarityFunction::Euclidean,
        }
    }
    pub(crate) fn assert_same<T>(&self, label: &str, expected: &T, given: &T) -> Result<()>
    where
        T: PartialEq + Display,
    {
        if expected != given {
            return Err(LuceneError::illegal_argument(format!(
                "{}[{}] of doc [{}]. {}: expected '{}', but it has '{}'.",
                Self::ERR_MSG,
                self.name,
                self.doc_id,
                label,
                expected,
                given
            )));
        }
        Ok(())
    }
    pub(crate) fn update_attributes(&mut self, attrs: HashMap<String, String>) {
        self.attributes.extend(attrs);
    }

    pub(crate) fn set_index_options(
        &mut self,
        new_index_options: IndexOptions,
        new_omit_norms: bool,
        new_store_term_vector: bool,
    ) -> Result<()> {
        if self.index_options == IndexOptions::None {
            self.index_options = new_index_options;
            self.omit_norms = new_omit_norms;
            self.store_term_vector = new_store_term_vector;
        } else {
            self.assert_same("index options", &self.index_options, &new_index_options)?;
            self.assert_same("omit norms", &self.omit_norms, &new_omit_norms)?;
            self.assert_same(
                "store term vector",
                &self.store_term_vector,
                &new_store_term_vector,
            )?;
        }
        Ok(())
    }
    pub(crate) fn set_doc_values(
        &mut self,
        new_doc_values_type: DocValuesType,
        new_doc_values_skip_index: DocValuesSkipIndexType,
    ) -> Result<()> {
        if self.doc_values_type == DocValuesType::None {
            self.doc_values_type = new_doc_values_type;
            self.doc_values_skip_index = new_doc_values_skip_index;
        } else {
            self.assert_same(
                "doc values type",
                &self.doc_values_type,
                &new_doc_values_type,
            )?;
            self.assert_same(
                "doc values skip index type",
                &self.doc_values_skip_index,
                &new_doc_values_skip_index,
            )?;
        }
        Ok(())
    }

    pub(crate) fn set_points(
        &mut self,
        dimension_count: i32,
        index_dimension_count: i32,
        num_bytes: i32,
    ) -> Result<()> {
        if self.point_index_dimension_count == 0 {
            self.point_dimension_count = dimension_count;
            self.point_index_dimension_count = index_dimension_count;
            self.point_num_bytes = num_bytes;
        } else {
            self.assert_same(
                "point dimension",
                &self.point_dimension_count,
                &dimension_count,
            )?;
            self.assert_same(
                "point index dimension",
                &self.point_index_dimension_count,
                &index_dimension_count,
            )?;
            self.assert_same("point num bytes", &self.point_num_bytes, &num_bytes)?;
        }
        Ok(())
    }

    pub(crate) fn set_vectors(
        &mut self,
        encoding: VectorEncoding,
        similarity_function: VectorSimilarityFunction,
        dimension: i32,
    ) -> Result<()> {
        if self.vector_dimension == 0 {
            self.vector_encoding = encoding;
            self.vector_similarity_function = similarity_function;
            self.vector_dimension = dimension;
        } else {
            self.assert_same("vector encoding", &self.vector_encoding, &encoding)?;
            self.assert_same(
                "vector similarity function",
                &self.vector_similarity_function,
                &similarity_function,
            )?;
            self.assert_same("vector dimension", &self.vector_dimension, &dimension)?;
        }
        Ok(())
    }
    pub(crate) fn reset(&mut self, doc: i32) {
        self.doc_id = doc;
        self.omit_norms = false;
        self.store_term_vector = false;
        self.index_options = IndexOptions::None;
        self.doc_values_type = DocValuesType::None;
        self.doc_values_skip_index = DocValuesSkipIndexType::None;
        self.point_dimension_count = 0;
        self.point_index_dimension_count = 0;
        self.point_num_bytes = 0;
        self.vector_dimension = 0;
        self.vector_encoding = VectorEncoding::FLOAT32(4);
        self.vector_similarity_function = VectorSimilarityFunction::Euclidean;
    }

    pub(crate) fn assert_same_schema(&self, fi: &FieldInfo) -> Result<()> {
        self.assert_same("index options", fi.get_index_options(), &self.index_options)?;
        self.assert_same("omit norms", &fi.omits_norms(), &self.omit_norms)?;
        self.assert_same(
            "store term vector",
            &fi.has_term_vectors(),
            &self.store_term_vector,
        )?;
        self.assert_same(
            "doc values type",
            fi.get_doc_values_type(),
            &self.doc_values_type,
        )?;
        self.assert_same(
            "doc values skip index type",
            fi.doc_values_skip_index_type(),
            &self.doc_values_skip_index,
        )?;
        self.assert_same(
            "vector similarity function",
            fi.get_vector_similarity_function(),
            &self.vector_similarity_function,
        )?;
        self.assert_same(
            "vector encoding",
            fi.get_vector_encoding(),
            &self.vector_encoding,
        )?;
        self.assert_same(
            "vector dimension",
            &fi.get_vector_dimension(),
            &self.vector_dimension,
        )?;
        self.assert_same(
            "point dimension",
            &fi.get_point_dimension_count(),
            &self.point_dimension_count,
        )?;
        self.assert_same(
            "point index dimension",
            &fi.get_point_index_dimension_count(),
            &self.point_index_dimension_count,
        )?;
        self.assert_same(
            "point num bytes",
            &fi.get_point_num_bytes(),
            &self.point_num_bytes,
        )?;
        Ok(())
    }
}
