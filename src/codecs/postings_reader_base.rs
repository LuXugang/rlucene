/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/
use std::fmt::Display;
use std::rc::Rc;

use crate::codecs::block_term_state::BlockTermStateEnum;
use crate::index::field_info::FieldInfo;
use crate::index::impacts_enum::ImpactsEnum;
use crate::index::postings_enum::PostingsEnum;
use crate::index::segment_read_state::SegmentReadState;
use crate::store::directory::Directory;
use crate::store::{DataInput, IndexInput};
use crate::util::error::lucene_error::Result;

/// The core terms dictionaries (BlockTermsReader, BlockTreeTermsReader)
/// interact with a single instance of this class to manage creation of
/// [`PostingsEnum`] and
/// [`ImpactsEnum`] instances. It
/// provides an IndexInput (`termsIn`) where this class may read any previously
/// stored data that it had written in its corresponding
/// [`PostingsWriterBase`](crate::codecs::postings_writer_base::PostingsWriterBase) at indexing time.
// TODO: maybe move under blocktree?  but it's used by other terms dicts (e.g.
// Block) TODO: find a better name; this defines the API that the
// terms dict impls use to talk to a postings impl.
// TermsDict + PostingsReader/WriterBase == PostingsConsumer/Producer
pub trait PostingsReaderBase: Display {
    /// Performs any initialization, such as reading and verifying the header
    /// from the provided terms dictionary [`IndexInput`].
    fn init<D>(&self, terms_in: &mut impl IndexInput, state: &SegmentReadState<D>) -> Result<()>
    where
        D: Directory;

    /// Return a newly created empty `TermState`.
    // TODO: 这里是不是应该返回关联类型
    fn new_term_state(&self) -> Result<BlockTermStateEnum>;

    /// Actually decode metadata for next term
    ///
    /// See also:
    /// - [`PostingsWriterBase::encodeTerm`](crate::codecs::postings_writer_base::PostingsWriterBase::encode_term)
    fn decode_term(
        &self,
        input: &mut impl DataInput,
        field_info: &Rc<FieldInfo>,
        state: &mut BlockTermStateEnum,
        absolute: bool,
    ) -> Result<()>;

    /// Must fully consume `state`, since after this call that `TermState` may
    /// be reused.
    type PostingsEnum: PostingsEnum;
    fn postings(
        &self,
        field_info: &FieldInfo,
        state: &BlockTermStateEnum,
        reuse: Option<Self::PostingsEnum>,
        flags: i32,
    ) -> Result<Option<Self::PostingsEnum>>;

    type ImpactsEnum: ImpactsEnum;
    /// Return an [`ImpactsEnum`] that computes impacts
    /// with `scorer`.
    ///
    /// See also:
    /// - [`postings`](Self::postings)
    fn impacts(
        &self,
        field_info: &FieldInfo,
        state: &BlockTermStateEnum,
        flags: i32,
    ) -> Result<Self::ImpactsEnum>;

    /// Checks consistency of this reader.
    ///
    /// Note that this may be costly in terms of I/O, e.g. may involve computing
    /// a checksum value against large data files.
    fn check_integrity(&self) -> Result<()>;
}
