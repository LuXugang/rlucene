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
use std::borrow::Cow;
use std::rc::Rc;

use crate::codecs::block_term_state::BlockTermStateEnum;
use crate::codecs::norms_producer::NormsProducer;
use crate::index::field_info::FieldInfo;
use crate::index::postings_enum::PostingsEnum;
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

    /// Write all postings for one term; use the provided [`TermsEnum`] to pull
    /// a [`PostingsEnum`](crate::index::postings_enum::PostingsEnum). This
    /// method should not re-position the `terms_enum`! It is already
    /// positioned on the term that should be written. This method must set the
    /// bit in the provided [`FixedBitSet`] for every docID written. If no
    /// docs were written, this method should return `None`, and the terms
    /// dict will skip the term.
    fn write_term<N: NormsProducer, PE: PostingsEnum>(
        &mut self,
        _term: &BytesRef<Vec<u8>>,
        _terms_enum: &mut impl TermsEnum<PostingsEnum = PE>,
        _docs_seen: &mut FixedBitSet,
        _norms: &mut N,
        _postings_enum: Option<PE>,
    ) -> Result<(Option<PE>, Option<BlockTermStateEnum>)> {
        unimplemented!()
    }

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
