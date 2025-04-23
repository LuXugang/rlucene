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
use std::rc::Rc;

use crate::codecs::block_term_state::BlockTermStateEnum;
use crate::index::field_info::FieldInfo;
use crate::index::impacts_enum::ImpactsEnum;
use crate::index::postings_enum::PostingsEnums;
use crate::index::segment_read_state::SegmentReadState;
use crate::store::directory::Directory;
use crate::store::{DataInput, IndexInput};
use crate::util::error::lucene_error::Result;

/// The core terms dictionaries (BlockTermsReader, BlockTreeTermsReader)
/// interact with a single instance of this class to manage creation of
/// [`PostingsEnum`](crate::index::postings_enum::PostingsEnum) and
/// [`ImpactsEnum`] instances. It
/// provides an IndexInput (`termsIn`) where this class may read any previously
/// stored data that it had written in its corresponding
/// [`PostingsWriterBase`](crate::codecs::postings_writer_base::PostingsWriterBase) at indexing time.
// TODO: maybe move under blocktree?  but it's used by other terms dicts (e.g.
// Block) TODO: find a better name; this defines the API that the
// terms dict impls use to talk to a postings impl.
// TermsDict + PostingsReader/WriterBase == PostingsConsumer/Producer
pub trait PostingsReaderBase<I>
where
    I: IndexInput,
{
    /// Performs any initialization, such as reading and verifying the header
    /// from the provided terms dictionary [`IndexInput`].
    fn init<D>(
        &mut self,
        terms_in: &mut impl IndexInput,
        state: &SegmentReadState<D>,
    ) -> Result<()>
    where
        D: Directory;

    /// Return a newly created empty `TermState`.
    fn new_term_state(&mut self) -> Result<BlockTermStateEnum>;

    /// Actually decode metadata for next term
    ///
    /// See also:
    /// - [`PostingsWriterBase::encodeTerm`](crate::codecs::postings_writer_base::PostingsWriterBase::encode_term)
    fn decode_term(
        &mut self,
        input: &mut impl DataInput,
        field_info: &Rc<FieldInfo>,
        state: &mut BlockTermStateEnum,
        absolute: bool,
    ) -> Result<()>;

    /// Must fully consume `state`, since after this call that `TermState` may
    /// be reused.
    fn postings(
        &mut self,
        field_info: &FieldInfo,
        state: &BlockTermStateEnum,
        reuse: Option<&mut PostingsEnums<I>>,
        flags: i32,
    ) -> Result<Option<PostingsEnums<I>>>;

    type ImpactsEnum: ImpactsEnum;
    /// Return an [`ImpactsEnum`](TermsEnum::ImpactsEnum) that computes impacts
    /// with `scorer`.
    ///
    /// See also:
    /// - [`postings`](Self::postings)
    fn impacts(
        &mut self,
        field_info: &FieldInfo,
        state: &BlockTermStateEnum,
        flags: i32,
    ) -> Result<Self::ImpactsEnum>;

    /// Checks consistency of this reader.
    ///
    /// Note that this may be costly in terms of I/O, e.g. may involve computing
    /// a checksum value against large data files.
    fn check_integrity(&mut self) -> Result<()>;
}
