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
use std::borrow::Cow;
use std::rc::Rc;

use crate::codecs::block_term_state::BlockTermStateEnum;
use crate::codecs::norms_producer::NormsProducer;
use crate::codecs::push_postings_writer_base::PushPostingsWriterBaseAbstract;
use crate::index::field_info::FieldInfo;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::terms_enum::TermsEnum;
use crate::index::BytesRef;
use crate::store::directory::Directory;
use crate::store::{DataOutput, IndexOutput};
use crate::util::error::lucene_error::Result;
use crate::util::fixed_bit_set::FixedBitSet;

/// Trait that plugs into term dictionaries, such as
/// [`Lucene90BlockTreeTermsWriter`](crate::codecs::lucene90::lucene90_block_trree_terms_writer::Lucene90BlockTreeTermsWriter),
/// and handles writing postings.
///
/// See also:
/// - [`PostingsReaderBase`](crate::codecs::postings_reader_base::PostingsReaderBase)
// TODO: find a better name; this defines the API that the
// terms dict impls use to talk to a postings impl.
// TermsDict + PostingsReader/WriterBase == FieldsProducer/Consumer
pub trait PostingsWriterBase {
    /// Called once after startup, before any terms have been added.
    /// Implementations typically write a header to the provided `termsOut`.
    fn init<D: Directory>(
        &mut self,
        terms_out: &mut impl IndexOutput,
        state: &SegmentWriteState<D>,
    ) -> Result<()>;

    type TermsEnum: TermsEnum;
    type Norms: NormsProducer;
    /// Write all postings for one term; use the provided [`TermsEnum`] to pull
    /// a [`PostingsEnum`](crate::index::postings_enum::PostingsEnum). This
    /// method should not re-position the `terms_enum`! It is already
    /// positioned on the term that should be written. This method must set the
    /// bit in the provided [`FixedBitSet`] for every docID written. If no
    /// docs were written, this method should return `None`, and the terms
    /// dict will skip the term.
    fn write_term(
        &mut self,
        term: &BytesRef<Vec<u8>>,
        terms_enum: &mut Self::TermsEnum,
        docs_seen: &mut FixedBitSet,
        norms: &mut Self::Norms,
        sub: &mut impl PushPostingsWriterBaseAbstract<Self::Norms>,
    ) -> Result<Option<BlockTermStateEnum>>;

    /// Encode metadata as `&[i64]` and `&[u8]`. `absolute` controls whether the
    /// current term is delta encoded according to the latest term. Usually
    /// elements in `longs` are file pointers, so each one always increases
    /// when a new term is consumed. `out` is used to write generic bytes,
    /// which are not monotonic.
    fn encode_term(
        &mut self,
        out: &mut impl DataOutput,
        field_info: &FieldInfo,
        state: Cow<BlockTermStateEnum>,
        absolute: bool,
    ) -> Result<()>;

    /// Sets the current field for writing.
    fn set_field(&mut self, field_info: Rc<FieldInfo>);
}
