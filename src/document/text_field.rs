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
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use crate::analysis::analyzer::Analyzer;
use crate::document::field::{Field, FieldBase, Store};
use crate::document::field_type::FieldType;
use crate::document::fields::{ReaderEnum, TokenStreamEnum};
use crate::document::invertable_field::InvertableType;
use crate::document::stored_value::StoredValue;
use crate::index::indexable_field::IndexableField;
use crate::index::BytesRef;
use crate::util::error::lucene_error::Result;
use crate::util::number::Number;

pub mod text {
    use std::sync::Arc;

    use once_cell::sync::Lazy;

    use crate::document::field_type::FieldType;
    use crate::index::index_options::IndexOptions;

    /// Indexed, tokenized, not stored.
    pub(crate) static TYPE_NOT_STORED: Lazy<Arc<FieldType>> = Lazy::new(|| {
        let mut ft = FieldType::new();
        ft.set_index_options(IndexOptions::DocsAndFreqsAndPositions)
            .expect("set_index_options should never fail in this context");
        ft.set_tokenized(true)
            .expect("set_tokenized(true) should never fail in this context");
        ft.freeze();
        Arc::new(ft)
    });
    /// Indexed, tokenized, stored.
    pub(crate) static TYPE_STORED: Lazy<Arc<FieldType>> = Lazy::new(|| {
        let mut ft = FieldType::new();
        ft.set_index_options(IndexOptions::DocsAndFreqsAndPositions)
            .expect("set_index_options should never fail in this context");
        ft.set_tokenized(true)
            .expect("set_tokenized(true) should never fail in this context");
        ft.set_stored(true)
            .expect("set_stored(true) should never fail in this context");
        ft.freeze();
        Arc::new(ft)
    });
}

/// A field that is indexed and tokenized, without term vectors.
/// For example, this would be used on a `body` field that contains the bulk of
/// a document's text.
pub struct TextField {
    parent_field: Field,
    stored_value: Option<StoredValue>,
}

#[allow(unused)]
impl TextField {
    /// Creates a new un-stored `TextField` with a `ReaderEnum` value.
    ///
    /// # Parameters
    /// - `name`: Field name.
    /// - `reader`: `ReaderEnum` value.
    pub fn with_reader(name: &str, reader: ReaderEnum) -> Result<Self> {
        let name_arc = Arc::new(name.to_string());
        let parent_field = Field::with_reader(name, reader, Arc::clone(&text::TYPE_NOT_STORED))?;
        Ok(Self {
            parent_field,
            stored_value: None,
        })
    }
    /// Creates a new `TextField` with a string value.
    ///
    /// # Parameters
    /// - `name`: Field name.
    /// - `value`: String value.
    /// - `store`: `Store::Yes` if the content should also be stored.
    pub fn with_string(name: &str, value: &str, store: Store) -> Result<Self> {
        let store = store.into();
        let value_str = Rc::new(value.to_string());
        let field_type = if store {
            Arc::clone(&text::TYPE_STORED)
        } else {
            Arc::clone(&text::TYPE_NOT_STORED)
        };
        let parent_field = Field::with_string(name, value_str.clone(), field_type.clone())?;
        let stored_value = if store {
            Some(StoredValue::new_string(value_str))
        } else {
            None
        };
        Ok(Self {
            parent_field,
            stored_value,
        })
    }
    /// Creates a new un-stored `TextField` with a `TokenStreamEnum` value.
    ///
    /// # Parameters
    /// - `name`: Field name.
    /// - `stream`: `TokenStream` value.
    pub fn with_token_stream(name: &str, stream: TokenStreamEnum) -> Result<Self> {
        let parent_field =
            Field::with_token_stream(name, stream, Arc::clone(&text::TYPE_NOT_STORED))?;
        Ok(Self {
            parent_field,
            stored_value: None,
        })
    }
}
impl FieldBase for TextField {
    fn set_string_value(&mut self, value: &str) -> Result<()> {
        let value_str = Rc::new(value.to_string());
        self.parent_field.set_string_value(value_str.clone())?;
        if let Some(ref mut sv) = self.stored_value {
            sv.set_string_value(value_str)?;
        }
        Ok(())
    }
}
impl IndexableField for TextField {
    fn name(&self) -> &str {
        self.parent_field.name()
    }

    type FieldType = FieldType;

    fn field_type(&self) -> Arc<Self::FieldType> {
        self.parent_field.field_type()
    }

    type TokenStream = <Field as IndexableField>::TokenStream;

    fn token_stream<A>(
        &self,
        analyzer: &A,
        reuse: Option<Self::TokenStream>,
    ) -> Result<Self::TokenStream>
    where
        A: Analyzer,
    {
        self.parent_field.token_stream(analyzer, reuse)
    }

    fn binary_value(&self) -> Result<Option<Rc<BytesRef<Vec<u8>>>>> {
        self.parent_field.binary_value()
    }

    fn string_value(&self) -> Result<Option<Rc<String>>> {
        self.parent_field.string_value()
    }

    fn get_char_sequence_value(&self) -> Result<Option<Rc<String>>> {
        self.parent_field.get_char_sequence_value()
    }

    fn reader_value(&self) -> Result<Option<ReaderEnum>> {
        self.parent_field.reader_value()
    }

    fn numeric_value(&self) -> Result<Option<Number>> {
        self.parent_field.numeric_value()
    }

    fn stored_value(&self) -> Result<Option<StoredValue>> {
        Ok(self.stored_value.clone())
    }

    fn invertable_type(&self) -> &InvertableType {
        todo!()
    }
}

impl fmt::Display for TextField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TextField(name: {})", self.parent_field.name())
    }
}
