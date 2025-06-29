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
use std::fmt::{Display, Formatter};
use std::io::Cursor;
use std::rc::Rc;
use std::sync::Arc;

use crate::analysis::analyzer::Analyzer;
use crate::analysis::dummy::dummy_token_stream::DummyTokenStream;
use crate::document::field::Field;
use crate::document::field_type::FieldType;
use crate::document::invertable_field::InvertableType;
use crate::document::stored_field::StoredField;
use crate::document::stored_value::StoredValue;
use crate::document::string_field::StringField;
use crate::document::text_field::TextField;
use crate::index::indexable_field::IndexableField;
use crate::index::BytesRef;
use crate::util::error::lucene_error::Result;
use crate::util::number::Number;

pub enum Fields {
    Field(Field),
    Text(TextField),
    String(StringField),
    Stored(StoredField),
}
impl Display for Fields {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Fields::Field(f1) => write!(f, "{}", f1),
            Fields::Text(f1) => write!(f, "{}", f1),
            Fields::String(f1) => write!(f, "{}", f1),
            Fields::Stored(f1) => write!(f, "{}", f1),
        }
    }
}

impl IndexableField for Fields {
    fn name(&self) -> &str {
        match self {
            Fields::Field(f) => f.name(),
            Fields::Text(f) => f.name(),
            Fields::String(f) => f.name(),
            Fields::Stored(f) => f.name(),
        }
    }

    type FieldType = FieldType;

    fn field_type(&self) -> Arc<Self::FieldType> {
        match self {
            Fields::Field(f) => f.field_type(),
            Fields::Text(f) => f.field_type(),
            Fields::String(f) => f.field_type(),
            Fields::Stored(f) => f.field_type(),
        }
    }

    type TokenStream = DummyTokenStream;

    fn token_stream<A>(
        &self,
        analyzer: &A,
        reuse: Option<Self::TokenStream>,
    ) -> Result<Self::TokenStream>
    where
        A: Analyzer,
    {
        match self {
            Fields::Field(f) => f.token_stream(analyzer, reuse),
            Fields::Text(f) => f.token_stream(analyzer, reuse),
            Fields::String(f) => f.token_stream(analyzer, reuse),
            Fields::Stored(f) => f.token_stream(analyzer, reuse),
        }
    }

    fn binary_value(&self) -> Result<Option<Rc<BytesRef<Vec<u8>>>>> {
        match self {
            Fields::Field(f) => f.binary_value(),
            Fields::Text(f) => f.binary_value(),
            Fields::String(f) => f.binary_value(),
            Fields::Stored(f) => f.binary_value(),
        }
    }

    fn string_value(&self) -> Result<Option<Rc<String>>> {
        match self {
            Fields::Field(f) => f.string_value(),
            Fields::Text(f) => f.string_value(),
            Fields::String(f) => f.string_value(),
            Fields::Stored(f) => f.string_value(),
        }
    }

    fn get_char_sequence_value(&self) -> Result<Option<Rc<String>>> {
        match self {
            Fields::Field(f) => f.get_char_sequence_value(),
            Fields::Text(f) => f.get_char_sequence_value(),
            Fields::String(f) => f.get_char_sequence_value(),
            Fields::Stored(f) => f.get_char_sequence_value(),
        }
    }

    fn reader_value(&self) -> Result<Option<ReaderEnum>> {
        match self {
            Fields::Field(f) => f.reader_value(),
            Fields::Text(f) => f.reader_value(),
            Fields::String(f) => f.reader_value(),
            Fields::Stored(f) => f.reader_value(),
        }
    }

    fn numeric_value(&self) -> Result<Option<Number>> {
        match self {
            Fields::Field(f) => f.numeric_value(),
            Fields::Text(f) => f.numeric_value(),
            Fields::String(f) => f.numeric_value(),
            Fields::Stored(f) => f.numeric_value(),
        }
    }

    fn stored_value(&self) -> Result<Option<StoredValue>> {
        match self {
            Fields::Field(f) => f.stored_value(),
            Fields::Text(f) => f.stored_value(),
            Fields::String(f) => f.stored_value(),
            Fields::Stored(f) => f.stored_value(),
        }
    }

    fn invertable_type(&self) -> &InvertableType {
        match self {
            Fields::Field(f) => f.invertable_type(),
            Fields::Text(f) => f.invertable_type(),
            Fields::String(f) => f.invertable_type(),
            Fields::Stored(f) => f.invertable_type(),
        }
    }
}

impl From<Field> for Fields {
    fn from(f: Field) -> Self {
        Fields::Field(f)
    }
}

impl From<TextField> for Fields {
    fn from(t: TextField) -> Self {
        Fields::Text(t)
    }
}

impl From<StringField> for Fields {
    fn from(s: StringField) -> Self {
        Fields::String(s)
    }
}

impl From<StoredField> for Fields {
    fn from(s: StoredField) -> Self {
        Fields::Stored(s)
    }
}

#[derive(Debug, Clone)]
pub enum ReaderEnum {
    CursorStr(Arc<Cursor<String>>),
}

#[derive(Debug, Clone)]
pub enum TokenStreamEnum {
    Dummy(Arc<DummyTokenStream>),
}
