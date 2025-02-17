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
// TODO: how to handle versioning here...?

use crate::analysis::analyzer::Analyzer;
use crate::analysis::token_stream::TokenStream;
use crate::document::invertable_field::InvertableType;
use crate::document::stored_value::StoredValue;
use crate::index::indexable_field_type::IndexableFieldType;
use crate::index::BytesRef;
use crate::util::error::lucene_error::LuceneError;
use crate::util::number::Number;
use std::io::Read;
use std::sync::Arc;

/// Represents a single field for indexing. IndexWriter consumes `Iterable<IndexableField>` as a
/// document.
///
/// @lucene.experimental
pub trait IndexableField {
    /// Field name
    fn name(&self) -> &str;

    /// {@link IndexableFieldType} describing the properties of this field.
    type FieldType: IndexableFieldType;
    fn field_type(&self) -> Result<&Self::FieldType, LuceneError> {
        Err(LuceneError::not_implemented(
            "field_type is not implemented",
        ))
    }
    /// Creates the TokenStream used for indexing this field. If appropriate, implementations should
    /// use the given Analyzer to create the TokenStreams.
    ///
    /// * `analyzer` - Analyzer that should be used to create the TokenStreams from
    /// * `reuse` - TokenStream for a previous instance of this field **name**. This allows custom
    ///   field types (like StringField and NumericField) that do not use the analyzer to still have
    ///   good performance. Note: the passed-in type may be inappropriate, for example if you mix up
    ///   different types of Fields for the same field name. So it's the responsibility of the
    ///   implementation to check.
    ///
    /// # Returns
    /// TokenStream value for indexing the document. Should always return a non-null value if
    /// the field is to be indexed.
    type TokenStreamType: TokenStream;
    fn token_stream(
        &self,
        _analyzer: Option<&impl Analyzer>,
        _reuse: Option<&impl TokenStream>,
    ) -> Result<Self::TokenStreamType, LuceneError> {
        Err(LuceneError::not_implemented(
            "token_stream is not implemented",
        ))
    }
    /// Non-null if this field has a binary value.
    fn binary_value(&self) -> Result<Option<Arc<BytesRef>>, LuceneError> {
        Err(LuceneError::not_implemented(
            "binary_value is not implemented",
        ))
    }

    /// Non-null if this field has a string value.
    fn string_value(&self) -> Result<Option<Arc<String>>, LuceneError> {
        Err(LuceneError::not_implemented(
            "string_value is not implemented",
        ))
    }

    /// Non-null if this field has a string value.
    fn get_char_sequence_value(&self) -> Result<Option<Arc<String>>, LuceneError> {
        self.string_value()
    }

    /// Non-null if this field has a Reader value.
    type ReadType: Read;
    fn reader_value(&self) -> Result<Option<Self::ReadType>, LuceneError> {
        Err(LuceneError::not_implemented(
            "reader_value is not implemented",
        ))
    }

    /// Non-null if this field has a numeric value.
    fn numeric_value(&self) -> Result<Option<Number>, LuceneError> {
        Err(LuceneError::not_implemented(
            "numeric_value is not implemented",
        ))
    }

    /// Stored value. This method is called to populate stored fields and must return a non-null value
    /// if the field stored.
    fn stored_value(&self) -> Result<Option<StoredValue>, LuceneError> {
        Err(LuceneError::not_implemented(
            "stored_value is not implemented",
        ))
    }

    /// Describes how this field should be inverted. This must return a non-null value if the field
    /// indexes terms and postings.
    fn invertable_type(&self) -> Result<&InvertableType, LuceneError> {
        Err(LuceneError::not_implemented(
            "invertable_type is not implemented",
        ))
    }
}

#[cfg(test)]
mod tests {
    // TODO:waiting for implementation after IndexWriter
    #[allow(dead_code)]
    struct TestIndexableField;
}
