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
use crate::index::dummy::dummy_fields::DummyFields;
use crate::index::fields::Fields;
use crate::util::error::lucene_error::Result;
/// API for reading term vectors.
///
/// **NOTE**: This class is not thread-safe and should only be consumed in the thread where it
/// was acquired.
pub trait TermVectors {
    /// Optional method: Give a hint to this [`TermVectors`] instance that the given document will
    /// be read in the near future. This typically delegates to [`IndexInput::prefetch`](crate::store::index_input::IndexInput::prefetch) and is
    /// useful to parallelize I/O across multiple documents.
    ///
    /// NOTE: This API is expected to be called on a small enough set of doc IDs that they could all
    /// fit in the page cache. If you plan on retrieving a very large number of documents, it may be a
    /// good idea to perform calls to [`Self::prefetch`] and [`Self::get`] in batches instead of
    /// prefetching all documents up-front.
    fn prefetch(&mut self, _doc_id: i32) -> Result<()> {
        Ok(())
    }
    /// The associated `Fields` type.
    type Fields: Fields;
    /// Returns term vectors for this document, or `None` if term vectors were not indexed.
    ///
    /// The returned [`Fields`] instance acts like a single-document inverted index (the `doc_id` will be
    /// `0`). If offsets are available they are in an [`OffsetAttribute`](crate::analysis::token_attributes::offset_attribute::OffsetAttribute) available from the [`PostingsEnum`](crate::index::postings_enum::PostingsEnum).
    fn get(&mut self, doc: i32) -> Result<Option<Self::Fields>>;
    /// Returns term vectors for this document, or `None` if term vectors were not indexed.
    ///
    /// The returned [`Fields`] instance acts like a single-document inverted index (the `doc_id` will be
    /// `0`). If offsets are available they are in an [`OffsetAttribute`](crate::analysis::token_attributes::offset_attribute::OffsetAttribute) available from the [`PostingsEnum`](crate::index::postings_enum::PostingsEnum).
    fn get_field_terms(
        &mut self,
        doc: i32,
        field: &str,
    ) -> Result<Option<<Self::Fields as Fields>::Terms>> {
        match self.get(doc)? {
            Some(fields) => fields.terms(field),
            None => Ok(None),
        }
    }
}
/// Instance that never returns term vectors
pub struct EmptyTermVectors;

impl TermVectors for EmptyTermVectors {
    type Fields = DummyFields;

    fn get(&mut self, _doc: i32) -> Result<Option<Self::Fields>> {
        Ok(None)
    }
}
