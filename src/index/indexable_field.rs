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
// TODO: how to handle versioning here...?

use crate::analysis::analyzer::Analyzer;
use crate::analysis::token_stream::TokenStream;
use crate::document::fields::ReaderEnum;
use crate::document::invertable_field::InvertableType;
use crate::document::stored_value::StoredValue;
use crate::index::indexable_field_type::IndexableFieldType;
use crate::index::BytesRef;
use crate::util::error::lucene_error::Result;
use crate::util::number::Number;
use std::fmt::Display;
use std::rc::Rc;
use std::sync::Arc;

/// Represents a single field for indexing. IndexWriter consumes
/// `Iterable<IndexableField>` as a document.
///
/// @lucene.experimental
pub trait IndexableField: Display {
    /// Field name
    fn name(&self) -> &str;

    /// {@link IndexableFieldType} describing the properties of this field.
    type FieldType: IndexableFieldType;
    fn field_type(&self) -> Arc<Self::FieldType>;
    /// Creates the TokenStream used for indexing this field. If appropriate,
    /// implementations should use the given Analyzer to create the
    /// TokenStreams.
    ///
    /// * `analyzer` - Analyzer that should be used to create the TokenStreams
    ///   from
    /// * `reuse` - TokenStream for a previous instance of this field **name**.
    ///   This allows custom field types (like StringField and NumericField)
    ///   that do not use the analyzer to still have good performance. Note: the
    ///   passed-in type may be inappropriate, for example if you mix up
    ///   different types of Fields for the same field name. So it's the
    ///   responsibility of the implementation to check.
    ///
    /// # Returns
    /// TokenStream value for indexing the document. Should always return a
    /// non-null value if the field is to be indexed.
    type TokenStream: TokenStream;
    fn token_stream<A>(
        &self,
        analyzer: &A,
        reuse: Option<Self::TokenStream>,
    ) -> Result<Self::TokenStream>
    where
        A: Analyzer;
    /// Non-null if this field has a binary value.
    fn binary_value(&self) -> Result<Option<Rc<BytesRef<Vec<u8>>>>>;

    /// Non-null if this field has a string value.
    fn string_value(&self) -> Result<Option<Rc<String>>>;
    /// Non-null if this field has a string value.
    fn get_char_sequence_value(&self) -> Result<Option<Rc<String>>> {
        self.string_value()
    }

    /// Non-null if this field has a Reader value.
    fn reader_value(&self) -> Result<Option<ReaderEnum>>;

    /// Non-null if this field has a numeric value.
    fn numeric_value(&self) -> Result<Option<Number>>;

    /// Stored value. This method is called to populate stored fields and must
    /// return a non-null value if the field stored.
    fn stored_value(&self) -> Result<Option<StoredValue>>;

    /// Describes how this field should be inverted. This must return a non-null
    /// value if the field indexes terms and postings.
    fn invertable_type(&self) -> &InvertableType;

    fn is_reserved(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    // TODO : IndexWriter not implemented
    #[allow(dead_code)]
    struct TestIndexableField;
}
