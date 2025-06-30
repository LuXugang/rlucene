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
use strum_macros::{Display, EnumCount, FromRepr};

/// Controls how much information is stored in the postings lists.
///
/// # Experimental
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, FromRepr, Hash, EnumCount, Display,
)]
#[repr(u8)]
pub enum IndexOptions {
    /// Not indexed
    None,
    /// Only documents are indexed: term frequencies and positions are omitted.
    /// Phrase and other positional queries on the field will throw an
    /// exception, and scoring will behave as if any term in the document
    /// appears only once.
    Docs,
    /// Only documents and term frequencies are indexed: positions are omitted.
    /// This enables normal scoring, but Phrase and other positional queries
    /// will throw an Error.
    DocsAndFreqs,
    /// Indexes documents, frequencies, and positions.
    /// This is the typical default for full-text search: full scoring is
    /// enabled, and positional queries are supported.
    DocsAndFreqsAndPositions,
    /// Indexes documents, frequencies, positions, and offsets.
    /// Character offsets are encoded alongside the positions.
    DocsAndFreqsAndPositionsAndOffsets,
}
/// Use Default for padding
impl Default for IndexOptions {
    fn default() -> Self {
        IndexOptions::None
    }
}
