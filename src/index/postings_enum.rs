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
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::Result;
use std::borrow::Cow;

/// Iterates through the postings.
/// NOTE: you must first call [`next_doc`](DocIdSetIterator::next_doc) before
/// using any of the per-doc methods.
pub trait PostingsEnum: DocIdSetIterator {
    /// Returns term frequency in the current document, or 1 if the field was
    /// indexed with [`DOCS`](crate::index::index_options::IndexOptions::Docs)
    /// only.  Do not call this before
    /// [`nextDoc`](DocIdSetIterator::next_doc) is first called, nor after
    /// [`nextDoc`](DocIdSetIterator::next_doc) returns
    /// [`DocIdSetIterator#
    /// NO_MORE_DOCS`](crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS)
    ///
    /// NOTE: if this enum was obtained with `NONE`, the result of this method
    /// is undefined.
    fn freq(&mut self) -> Result<i32>;

    /// Returns the next position, or -1 if positions were not indexed.
    /// Calling this more than [`freq()`](Self::freq) times is undefined.
    fn next_position(&mut self) -> Result<i32>;

    /// Returns start offset for the current position, or -1 if offsets were not
    /// indexed.
    fn start_offset(&self) -> Result<i32>;

    /// Returns end offset for the current position, or -1 if offsets were not
    /// indexed.
    fn end_offset(&self) -> Result<i32>;

    /// Returns the payload at this position, or None if no payload was indexed.
    /// Do not modify the returned bytes.
    fn get_payload(&self) -> Result<Option<Cow<BytesRef<Vec<u8>>>>>;
}

pub mod postings_enum_util {
    pub const NONE: i16 = 0;
    pub const FREQS: i16 = 1 << 3;
    pub const POSITIONS: i16 = FREQS | 1 << 4;
    pub const OFFSETS: i16 = POSITIONS | 1 << 5;
    pub const PAYLOADS: i16 = POSITIONS | 1 << 6;
    pub const ALL: i16 = OFFSETS | PAYLOADS;

    /// Returns true if the given feature is requested in the flags.
    pub fn feature_requested(flags: i32, feature: i16) -> bool {
        (flags & (feature as i32)) == (feature as i32)
    }
}
