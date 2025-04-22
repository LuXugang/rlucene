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
use crate::codecs::block_term_state::BlockTermStateEnum;
use crate::codecs::norms_producer::NormsProducer;
use crate::codecs::postings_writer_base::PostingsWriterBase;
use crate::index::field_info::FieldInfo;
use crate::index::index_options::IndexOptions;
use crate::index::postings_enum::{postings_enum_util, PostingsEnum};
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::terms_enum::TermsEnum;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::directory::Directory;
use crate::store::{DataOutput, IndexOutput};
use crate::util::access::AccessVec;
use crate::util::bit_set::BitSet;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fixed_bit_set::FixedBitSet;
use std::borrow::Cow;
use std::marker::PhantomData;
use std::rc::Rc;

/// Extension of [`PostingsWriterBase`], adding a push API for writing each element of the
/// postings. This API is somewhat analogous to an XML SAX API, while [`PostingsWriterBase`] is
/// more like an XML DOM API.
///
/// @see [`PostingsWriterBase`]
// TODO: find a better name; this defines the API that the
// terms dict impls use to talk to a postings impl.
/// TermsDict + PostingsReader/WriterBase == PostingsConsumer/Producer
pub struct PushPostingsWriterBase<T: TermsEnum<AV>, N: NormsProducer, AV: AccessVec<u8>> {
    /// Reused in `write_term`
    postings_enum: Option<T::PostingsEnum>,
    enum_flags: i32,

    /// `FieldInfo` of current field being written.
    pub(crate) field_info: Rc<FieldInfo>,

    /// `IndexOptions` of current field being written.
    pub(crate) index_options: IndexOptions,

    /// True if the current field writes freqs.
    pub(crate) write_freqs: bool,

    /// True if the current field writes positions.
    pub(crate) write_positions: bool,

    /// True if the current field writes payloads.
    pub(crate) write_payloads: bool,

    /// True if the current field writes offsets.
    pub(crate) write_offsets: bool,
    phantom2: PhantomData<N>,
    phantom1: PhantomData<AV>,
}

impl<T, N, AV> PushPostingsWriterBase<T, N, AV>
where
    T: TermsEnum<AV>,
    N: NormsProducer,
    AV: AccessVec<u8>,
{
    #[allow(clippy::too_many_arguments)]
    /// # Parameters
    /// - `field_info`: It is just a placeholder value; it should be initialized as None, but I don't want to add extra wrapping around it.
    ///   It would be set in [`set_field`](Self::set_field) before used
    pub fn new(field_info: FieldInfo) -> Self {
        PushPostingsWriterBase {
            postings_enum: None,
            enum_flags: 0,
            field_info: Rc::new(field_info),
            index_options: Default::default(),
            write_freqs: false,
            write_positions: false,
            write_payloads: false,
            write_offsets: false,
            phantom2: PhantomData,
            phantom1: PhantomData,
        }
    }
}
impl<T, N, AV> PostingsWriterBase<T, N, AV> for PushPostingsWriterBase<T, N, AV>
where
    T: TermsEnum<AV>,
    N: NormsProducer,
    AV: AccessVec<u8>,
{
    fn init<D: Directory>(
        &mut self,
        _terms_out: &mut impl IndexOutput,
        _state: &SegmentWriteState<D>,
    ) -> Result<()> {
        Err(LuceneError::unsupported_operation(
            "this method need to be implemented",
        ))
    }

    fn write_term(
        &mut self,
        _term: &BytesRef<Vec<u8>>,
        terms_enum: &mut T,
        docs_seen: &mut FixedBitSet,
        norms: &mut N,
        sub: &mut impl PushPostingsWriterBaseAbstract<N>,
    ) -> Result<Option<BlockTermStateEnum>> {
        let norm_values = if self.field_info.has_norms() {
            Some(norms.get_norms(&self.field_info)?)
        } else {
            None
        };

        sub.start_term(norm_values)?;

        self.postings_enum =
            Some(terms_enum.postings_with_flags(self.postings_enum.take(), self.enum_flags)?);

        let mut doc_freq = 0;
        let mut total_term_freq = 0i64;
        let postings_enum = self.postings_enum.as_mut().unwrap();
        loop {
            let doc_id = postings_enum.next_doc()?;
            if doc_id == NO_MORE_DOCS {
                break;
            }
            doc_freq += 1;
            docs_seen.set(doc_id);

            let freq = if self.write_freqs {
                let f = postings_enum.freq()?;
                total_term_freq += f as i64;
                f
            } else {
                -1
            };

            sub.start_doc(doc_id, freq)?;

            if self.write_positions {
                for _ in 0..freq {
                    let pos = postings_enum.next_position()?;
                    let payload = if self.write_payloads {
                        postings_enum.get_payload()?
                    } else {
                        None
                    };
                    let (start_offset, end_offset) = if self.write_offsets {
                        (postings_enum.start_offset()?, postings_enum.end_offset()?)
                    } else {
                        (-1, -1)
                    };
                    sub.add_position(pos, payload, start_offset, end_offset)?;
                }
            }

            sub.finish_doc()?;
        }

        if doc_freq == 0 {
            return Ok(None);
        }

        let mut upper = sub.new_term_state()?;
        let state = upper.get_block_term_state();
        state.doc_freq = doc_freq;
        state.total_term_freq = if self.write_freqs {
            total_term_freq
        } else {
            -1
        };
        sub.finish_term(&mut upper)?;
        Ok(Some(upper))
    }

    fn encode_term(
        &mut self,
        _out: &mut impl DataOutput,
        _field_info: &FieldInfo,
        _state: Cow<BlockTermStateEnum>,
        _absolute: bool,
    ) -> Result<()> {
        Err(LuceneError::unsupported_operation(
            "this method need to be implemented",
        ))
    }
    /// Sets the current field for writing, and returns the fixed length of `&[i64]` metadata
    /// (which is fixed per field), called when the writing switches to another field.
    fn set_field(&mut self, field_info: Rc<FieldInfo>) {
        self.field_info = field_info.clone();
        self.index_options = *self.field_info.get_index_options();

        self.write_freqs = self.index_options >= IndexOptions::DocsAndFreqs;
        self.write_positions = self.index_options >= IndexOptions::DocsAndFreqsAndPositions;
        self.write_offsets = self.index_options >= IndexOptions::DocsAndFreqsAndPositionsAndOffsets;
        self.write_payloads = self.field_info.has_payloads();

        self.enum_flags = if !self.write_freqs {
            0
        } else if !self.write_positions {
            postings_enum_util::FREQS as i32
        } else if !self.write_offsets {
            if self.write_payloads {
                postings_enum_util::PAYLOADS as i32
            } else {
                postings_enum_util::POSITIONS as i32
            }
        } else if self.write_payloads {
            (postings_enum_util::PAYLOADS | postings_enum_util::OFFSETS) as i32
        } else {
            postings_enum_util::OFFSETS as i32
        };
    }
}
pub trait PushPostingsWriterBaseAbstract<N: NormsProducer> {
    /// Return a newly created empty TermState
    fn new_term_state(&mut self) -> Result<BlockTermStateEnum>;

    /// Start a new term.
    /// A matching call to [`finish_term`](Self::finish_term) will be done only if the term has at least one document.
    fn start_term(&mut self, norms: Option<N::NumericDocValues>) -> Result<()>;

    /// Finishes the current term. The provided [`BlockTermState`] contains
    /// the term's summary statistics and will hold metadata from PBF when returned.
    fn finish_term(&mut self, state: &mut BlockTermStateEnum) -> Result<()>;

    /// Adds a new doc in this term. `freq` will be -1 when term
    /// frequencies are omitted for the field.
    fn start_doc(&mut self, doc_id: i32, freq: i32) -> Result<()>;

    /// Add a new position and payload, and start/end offset.
    /// A null payload means no payload; a non-null payload with zero length
    /// also means no payload. Caller may reuse the [`BytesRef`] for the payload
    /// between calls (method must fully consume the payload).
    /// `start_offset` and `end_offset` will be -1 when offsets are not indexed.
    fn add_position(
        &mut self,
        position: i32,
        payload: Option<&BytesRef<Vec<u8>>>,
        start_offset: i32,
        end_offset: i32,
    ) -> Result<()>;

    /// Called when we are done adding positions and payloads for each doc.
    fn finish_doc(&mut self) -> Result<()>;
}
