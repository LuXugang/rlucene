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
use crate::analysis::analyzer::Analyzer;
use crate::analysis::dummy::dummy_token_stream::DummyTokenStream;
use crate::analysis::token_stream::TokenStream;
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
use std::fmt::{Display, Formatter};
use std::io::Cursor;
use std::sync::Arc;

pub enum Fields {
    Field(Field),
    TextField(TextField),
    StringField(StringField),
    StoredField(StoredField),
}
impl Display for Fields {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Fields::Field(f1) => write!(f, "{}", f1),
            Fields::TextField(f1) => write!(f, "{}", f1),
            Fields::StringField(f1) => write!(f, "{}", f1),
            Fields::StoredField(f1) => write!(f, "{}", f1),
        }
    }
}

impl IndexableField for Fields {
    fn name(&self) -> &str {
        match self {
            Fields::Field(f) => f.name(),
            Fields::TextField(f) => f.name(),
            Fields::StringField(f) => f.name(),
            Fields::StoredField(f) => f.name(),
        }
    }

    type FieldType = FieldType;

    fn field_type(&self) -> &Self::FieldType {
        match self {
            Fields::Field(f) => f.field_type(),
            Fields::TextField(f) => f.field_type(),
            Fields::StringField(f) => f.field_type(),
            Fields::StoredField(f) => f.field_type(),
        }
    }

    fn token_stream(
        &self,
        _analyzer: Option<&impl Analyzer>,
        _reuse: Option<&impl TokenStream>,
    ) -> Result<TokenStreamEnum> {
        match self {
            Fields::Field(f) => f.token_stream(_analyzer, _reuse),
            Fields::TextField(f) => f.token_stream(_analyzer, _reuse),
            Fields::StringField(f) => f.token_stream(_analyzer, _reuse),
            Fields::StoredField(f) => f.token_stream(_analyzer, _reuse),
        }
    }

    fn binary_value(&self) -> Result<Option<Arc<BytesRef>>> {
        match self {
            Fields::Field(f) => f.binary_value(),
            Fields::TextField(f) => f.binary_value(),
            Fields::StringField(f) => f.binary_value(),
            Fields::StoredField(f) => f.binary_value(),
        }
    }

    fn string_value(&self) -> Result<Option<Arc<String>>> {
        match self {
            Fields::Field(f) => f.string_value(),
            Fields::TextField(f) => f.string_value(),
            Fields::StringField(f) => f.string_value(),
            Fields::StoredField(f) => f.string_value(),
        }
    }

    fn get_char_sequence_value(&self) -> Result<Option<Arc<String>>> {
        match self {
            Fields::Field(f) => f.get_char_sequence_value(),
            Fields::TextField(f) => f.get_char_sequence_value(),
            Fields::StringField(f) => f.get_char_sequence_value(),
            Fields::StoredField(f) => f.get_char_sequence_value(),
        }
    }

    fn reader_value(&self) -> Result<Option<ReaderEnum>> {
        match self {
            Fields::Field(f) => f.reader_value(),
            Fields::TextField(f) => f.reader_value(),
            Fields::StringField(f) => f.reader_value(),
            Fields::StoredField(f) => f.reader_value(),
        }
    }

    fn numeric_value(&self) -> Result<Option<Number>> {
        match self {
            Fields::Field(f) => f.numeric_value(),
            Fields::TextField(f) => f.numeric_value(),
            Fields::StringField(f) => f.numeric_value(),
            Fields::StoredField(f) => f.numeric_value(),
        }
    }

    fn stored_value(&self) -> Result<Option<StoredValue>> {
        match self {
            Fields::Field(f) => f.stored_value(),
            Fields::TextField(f) => f.stored_value(),
            Fields::StringField(f) => f.stored_value(),
            Fields::StoredField(f) => f.stored_value(),
        }
    }

    fn invertable_type(&self) -> Result<&InvertableType> {
        match self {
            Fields::Field(f) => f.invertable_type(),
            Fields::TextField(f) => f.invertable_type(),
            Fields::StringField(f) => f.invertable_type(),
            Fields::StoredField(f) => f.invertable_type(),
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
        Fields::TextField(t)
    }
}

impl From<StringField> for Fields {
    fn from(s: StringField) -> Self {
        Fields::StringField(s)
    }
}

impl From<StoredField> for Fields {
    fn from(s: StoredField) -> Self {
        Fields::StoredField(s)
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
