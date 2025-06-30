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

use crate::index::BytesRef;
use crate::util::error::lucene_error::{LuceneError, Result};

pub trait BytesRefIterator {
    /// The returned `BytesRef` may be re-used across calls to `next`. After
    /// this method returns `None`, do not call it again as the results are
    /// undefined.
    ///
    /// # Returns
    /// The next [`BytesRef`] in the iterator or `None` if the end of the
    /// iterator is reached.
    ///
    /// # Note
    /// In some scenarios, we need to return a reference to the `BytesRef` to
    /// avoid frequent copying operations.
    /// Like in [`TermsDict`](crate::codecs::lucene90::lucene90_doc_values_producer::TermsDict), this method can be used
    /// when reusing internal buffers to reduce allocations and improve
    /// performance.
    ///
    /// To simplify the interface and allow for both owned and borrowed variants
    /// in a unified way, it is recommended to use
    /// [`Cow<BytesRef>`](std::borrow::Cow). This enables returning either:
    ///
    /// - `Cow::Borrowed(&BytesRef)` when the data is internally reusable,
    ///   avoiding clone costs
    /// - `Cow::Owned(BytesRef)` when a fresh copy is required
    ///
    /// This approach provides flexibility to the implementor and clarity to the
    /// caller, while preserving performance by avoiding unnecessary
    /// allocations. # Errors
    /// Returns an `std::io::Error` if there is a low-level I/O error.
    fn next(&mut self) -> Result<Option<Cow<BytesRef<Vec<u8>>>>> {
        Err(LuceneError::need_implemented("this method need implement"))
    }
}

pub struct EmptyBytesRefIterator;

impl BytesRefIterator for EmptyBytesRefIterator {
    fn next(&mut self) -> Result<Option<Cow<BytesRef<Vec<u8>>>>> {
        Ok(None)
    }
}

impl EmptyBytesRefIterator {
    #[allow(unused)]
    pub const EMPTY: Self = EmptyBytesRefIterator;
}
