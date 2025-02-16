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
use crate::document::dummy::dummy_filed::DummyField;
use crate::document::field_type::FieldType;
use crate::document::invertable_field::InvertableType;
use crate::document::stored_value::StoredValue;
use crate::index::doc_values_type::DocValuesType;
use crate::index::index_options::IndexOptions;
use crate::index::indexable_field::IndexableField;
use crate::index::indexable_field_type::IndexableFieldType;
use crate::index::BytesRef;
use crate::util::dummy::dummy_read::DummyRead;
use crate::util::error::lucene_error::LuceneError;
use crate::util::number::Number;
use std::fmt;
use std::fmt::{Debug, Display};
use std::io::Read;
use std::sync::Arc;

pub struct Field<R, T, F>
where
    R: Read + Debug,
    T: TokenStream + Debug,
    F: FieldBase + IndexableField,
{
    indexable_field_type: FieldType,
    name: String,
    fields_data: Option<FieldDataEnum<R, T>>,
    delegate: Option<F>,
}
impl<F> Field<DummyRead, DummyTokenStream, F>
where
    F: FieldBase + IndexableField,
{
    pub fn new_with_delegate(
        name: String,
        indexable_field_type: FieldType,
        delegate: Option<F>,
    ) -> Self {
        Field {
            indexable_field_type,
            name,
            fields_data: None,
            delegate,
        }
    }
    pub fn with_binary_delegate(
        name: String,
        value: Vec<u8>,
        indexable_field_type: FieldType,
        delegate: Option<F>,
    ) -> Result<Self, LuceneError> {
        let len = value.len() as i32;
        Self::with_binary_range_delegate(name, value, 0, len, indexable_field_type, delegate)
    }
    pub fn with_binary_range_delegate(
        name: String,
        value: Vec<u8>,
        offset: i32,
        length: i32,
        indexable_field_type: FieldType,
        delegate: Option<F>,
    ) -> Result<Self, LuceneError> {
        let value = Arc::new(BytesRef::from_vec(value, offset, length));
        Self::with_bytes_ref_delegate(name, value, indexable_field_type, delegate)
    }
    pub fn with_bytes_ref_delegate(
        name: String,
        bytes: Arc<BytesRef>,
        indexable_field_type: FieldType,
        delegate: Option<F>,
    ) -> Result<Self, LuceneError> {
        if indexable_field_type
            .index_options()
            .cmp(&IndexOptions::DocsAndFreqsAndPositionsAndOffsets)
            != std::cmp::Ordering::Less
            || indexable_field_type.store_term_vector_offsets()
        {
            return Err(LuceneError::illegal_argument(
                "It doesn't make sense to index offsets on binary fields",
            ));
        }
        if indexable_field_type.index_options() == &IndexOptions::None
            && indexable_field_type.point_dimension_count() == 0
            && indexable_field_type.doc_values_type() == &DocValuesType::None
            && !indexable_field_type.stored()
        {
            return Err(LuceneError::illegal_argument("it doesn't make sense to have a field that is neither indexed, nor doc-valued, nor stored"));
        }
        Ok(Field {
            indexable_field_type,
            name,
            fields_data: Some(FieldDataEnum::Binary(bytes)),
            delegate,
        })
    }
    pub fn with_string_delegate(
        name: String,
        value: String,
        indexable_field_type: FieldType,
        delegate: Option<F>,
    ) -> Result<Self, LuceneError> {
        if !indexable_field_type.stored()
            && indexable_field_type.index_options() == &IndexOptions::None
        {
            return Err(LuceneError::illegal_argument(
                "it doesn't make sense to have a field that is neither indexed nor stored",
            ));
        }
        Ok(Field {
            indexable_field_type,
            name,
            fields_data: Some(FieldDataEnum::String(value)),
            delegate,
        })
    }
}

impl Field<DummyRead, DummyTokenStream, DummyField> {
    pub fn new(name: String, indexable_field_type: FieldType) -> Self {
        Self::new_with_delegate(name, indexable_field_type, None)
    }

    pub fn with_binary(
        name: String,
        value: Vec<u8>,
        indexable_field_type: FieldType,
    ) -> Result<Self, LuceneError> {
        let len = value.len() as i32;
        Self::with_binary_range(name, value, 0, len, indexable_field_type)
    }
    pub fn with_binary_range(
        name: String,
        value: Vec<u8>,
        offset: i32,
        length: i32,
        indexable_field_type: FieldType,
    ) -> Result<Self, LuceneError> {
        let value = Arc::new(BytesRef::from_vec(value, offset, length));
        Self::with_bytes_ref(name, value, indexable_field_type)
    }
    pub fn with_bytes_ref(
        name: String,
        bytes: Arc<BytesRef>,
        indexable_field_type: FieldType,
    ) -> Result<Self, LuceneError> {
        Self::with_bytes_ref_delegate(name, bytes, indexable_field_type, None)
    }

    pub fn with_string(
        name: String,
        value: String,
        indexable_field_type: FieldType,
    ) -> Result<Self, LuceneError> {
        Self::with_string_delegate(name, value, indexable_field_type, None)
    }
    pub fn set_string_value(&mut self, value: String) -> Result<(), LuceneError> {
        if let Some(delegate) = &mut self.delegate {
            match delegate.set_string_value(value.clone()) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }

        match &self.fields_data {
            Some(FieldDataEnum::String(_)) => {}
            _ => {
                return Err(LuceneError::illegal_argument(format!(
                    "cannot change value type from {:?} to String",
                    self.fields_data
                )));
            }
        }

        self.fields_data = Some(FieldDataEnum::String(value));
        Ok(())
    }

    pub fn set_vec_value(&mut self, value: Vec<u8>) -> Result<(), LuceneError> {
        self.set_bytes_value(Arc::new(BytesRef::from_bytes(value)))
    }
    pub fn set_bytes_value(&mut self, value: Arc<BytesRef>) -> Result<(), LuceneError> {
        if self.delegate.is_some() {
            match self
                .delegate
                .as_mut()
                .unwrap()
                .set_bytes_value(value.clone())
            {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }
        match &self.fields_data {
            Some(FieldDataEnum::Binary(_)) => {}
            _ => {
                return Err(LuceneError::illegal_argument(format!(
                    "cannot change value type from {:?} to BytesRef",
                    self.fields_data
                )));
            }
        }
        self.fields_data = Some(FieldDataEnum::Binary(value));
        Ok(())
    }
    pub fn set_byte_value(&mut self, value: u8) -> Result<(), LuceneError> {
        if let Some(delegate) = &mut self.delegate {
            match delegate.set_byte_value(value) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }
        match &self.fields_data {
            Some(FieldDataEnum::Number(_)) => {}
            _ => {
                return Err(LuceneError::illegal_argument(format!(
                    "cannot change value type from {:?} to Byte",
                    self.fields_data
                )));
            }
        }
        self.fields_data = Some(FieldDataEnum::Number(Number::U8(value)));
        Ok(())
    }
    pub fn set_short_value(&mut self, value: i16) -> Result<(), LuceneError> {
        if let Some(delegate) = &mut self.delegate {
            match delegate.set_short_value(value) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }
        match &self.fields_data {
            Some(FieldDataEnum::Number(_)) => {}
            _ => {
                return Err(LuceneError::illegal_argument(format!(
                    "cannot change value type from {:?} to Short",
                    self.fields_data
                )));
            }
        }
        self.fields_data = Some(FieldDataEnum::Number(Number::I16(value)));
        Ok(())
    }
    pub fn set_int_value(&mut self, value: i32) -> Result<(), LuceneError> {
        if let Some(delegate) = &mut self.delegate {
            match delegate.set_int_value(value) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }

        match &self.fields_data {
            Some(FieldDataEnum::Number(_)) => {}
            _ => {
                return Err(LuceneError::illegal_argument(format!(
                    "cannot change value type from {:?} to Integer",
                    self.fields_data
                )));
            }
        }

        self.fields_data = Some(FieldDataEnum::Number(Number::I32(value)));
        Ok(())
    }

    pub fn set_long_value(&mut self, value: i64) -> Result<(), LuceneError> {
        if let Some(delegate) = &mut self.delegate {
            match delegate.set_long_value(value) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }

        match &self.fields_data {
            Some(FieldDataEnum::Number(_)) => {}
            _ => {
                return Err(LuceneError::illegal_argument(format!(
                    "cannot change value type from {:?} to Long",
                    self.fields_data
                )));
            }
        }

        self.fields_data = Some(FieldDataEnum::Number(Number::I64(value)));
        Ok(())
    }

    pub fn set_float_value(&mut self, value: f32) -> Result<(), LuceneError> {
        if let Some(delegate) = &mut self.delegate {
            match delegate.set_float_value(value) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }

        match &self.fields_data {
            Some(FieldDataEnum::Number(_)) => {}
            _ => {
                return Err(LuceneError::illegal_argument(format!(
                    "cannot change value type from {:?} to Float",
                    self.fields_data
                )));
            }
        }

        self.fields_data = Some(FieldDataEnum::Number(Number::F32(value)));
        Ok(())
    }

    pub fn set_double_value(&mut self, value: f64) -> Result<(), LuceneError> {
        if let Some(delegate) = &mut self.delegate {
            match delegate.set_double_value(value) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }

        match &self.fields_data {
            Some(FieldDataEnum::Number(_)) => {}
            _ => {
                return Err(LuceneError::illegal_argument(format!(
                    "cannot change value type from {:?} to Double",
                    self.fields_data
                )));
            }
        }

        self.fields_data = Some(FieldDataEnum::Number(Number::F64(value)));
        Ok(())
    }
}
impl<R, F> Field<R, DummyTokenStream, F>
where
    R: Read + Debug,
    F: FieldBase + IndexableField,
{
    pub fn with_reader_delegate(
        name: String,
        reader: Arc<R>,
        indexable_field_type: FieldType,
        delegate: Option<F>,
    ) -> Result<Self, LuceneError> {
        if indexable_field_type.stored() {
            return Err(LuceneError::illegal_argument(
                "fields with a Reader value cannot be stored",
            ));
        }
        if !indexable_field_type.tokenized() {
            return Err(LuceneError::illegal_argument(
                "non-tokenized fields must use String values",
            ));
        }
        Ok(Field {
            indexable_field_type,
            name,
            fields_data: Some(FieldDataEnum::Reader(reader)),
            delegate,
        })
    }
}
impl<R> Field<R, DummyTokenStream, DummyField>
where
    R: Read + Debug,
{
    pub fn with_reader(
        name: String,
        reader: Arc<R>,
        indexable_field_type: FieldType,
    ) -> Result<Self, LuceneError> {
        Self::with_reader_delegate(name, reader, indexable_field_type, None)
    }
    pub fn set_reader_value(&mut self, value: Arc<R>) -> Result<(), LuceneError> {
        if let Some(delegate) = &mut self.delegate {
            match delegate.set_reader_value(value.clone()) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }

        match &self.fields_data {
            Some(FieldDataEnum::Reader(_)) => {}
            _ => {
                return Err(LuceneError::illegal_argument(format!(
                    "cannot change value type from {:?} to Reader",
                    self.fields_data
                )));
            }
        }

        self.fields_data = Some(FieldDataEnum::Reader(value));
        Ok(())
    }
}

impl<T, F> Field<DummyRead, T, F>
where
    T: TokenStream + Debug,
    F: FieldBase + IndexableField,
{
    pub fn with_token_stream_delegate(
        name: String,
        token_stream: Arc<T>,
        indexable_field_type: FieldType,
        delegate: Option<F>,
    ) -> Result<Self, LuceneError> {
        if !indexable_field_type.tokenized()
            || indexable_field_type.index_options() == &IndexOptions::None
        {
            return Err(LuceneError::illegal_argument(
                "TokenStream fields must be indexed and tokenized",
            ));
        }
        if indexable_field_type.stored() {
            return Err(LuceneError::illegal_argument(
                "TokenStream fields cannot be stored",
            ));
        }
        Ok(Field {
            indexable_field_type,
            name,
            fields_data: Some(FieldDataEnum::TokenStream(token_stream)),
            delegate,
        })
    }
    pub fn set_token_stream(&mut self, token_stream: Arc<T>) -> Result<(), LuceneError> {
        if let Some(delegate) = &mut self.delegate {
            match delegate.set_token_stream(token_stream.clone()) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }

        match &self.fields_data {
            Some(FieldDataEnum::TokenStream(_)) => {}
            _ => {
                return Err(LuceneError::illegal_argument(format!(
                    "cannot change value type from {:?} to TokenStream",
                    self.fields_data
                )));
            }
        }

        self.fields_data = Some(FieldDataEnum::TokenStream(token_stream));
        Ok(())
    }
}
impl<T> Field<DummyRead, T, DummyField>
where
    T: TokenStream + Debug,
{
    pub fn with_token_stream(
        name: String,
        token_stream: Arc<T>,
        indexable_field_type: FieldType,
    ) -> Result<Self, LuceneError> {
        Self::with_token_stream_delegate(name, token_stream, indexable_field_type, None)
    }
    pub fn token_stream_value(&self) -> Result<Option<Arc<T>>, LuceneError> {
        if let Some(FieldDataEnum::TokenStream(ref token_stream)) = &self.fields_data {
            Ok(Some(token_stream.clone()))
        } else {
            Ok(None)
        }
    }
}

impl<R, T, F> IndexableField for Field<R, T, F>
where
    R: Read + Debug,
    T: TokenStream + Debug,
    F: FieldBase + IndexableField,
{
    fn name(&self) -> Result<&str, LuceneError> {
        Ok(&self.name)
    }

    type FieldType = FieldType;

    fn field_type(&self) -> Result<&Self::FieldType, LuceneError> {
        Ok(&self.indexable_field_type)
    }

    type TokenStreamType = DummyTokenStream;

    fn token_stream(
        &self,
        _analyzer: Option<&impl Analyzer>,
        _reuse: Option<&impl TokenStream>,
    ) -> Result<Self::TokenStreamType, LuceneError> {
        todo!()
    }

    fn binary_value(&mut self) -> Result<Option<Arc<BytesRef>>, LuceneError> {
        if let Some(delegate) = &mut self.delegate {
            match delegate.binary_value() {
                Ok(r) => return Ok(r),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }
        if let Some(FieldDataEnum::Binary(ref bytes)) = &self.fields_data {
            Ok(Some(bytes.clone()))
        } else {
            Ok(None)
        }
    }

    fn string_value(&self) -> Result<Option<String>, LuceneError> {
        if let Some(delegate) = &self.delegate {
            match delegate.string_value() {
                Ok(r) => return Ok(r),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }
        if let Some(FieldDataEnum::String(ref s)) = &self.fields_data {
            Ok(Some(s.clone()))
        } else if let Some(FieldDataEnum::Number(val)) = &self.fields_data {
            Ok(Some(val.as_string()))
        } else {
            Ok(None)
        }
    }

    fn get_char_sequence_value(&self) -> Result<Option<String>, LuceneError> {
        if let Some(delegate) = &self.delegate {
            match delegate.get_char_sequence_value() {
                Ok(r) => return Ok(r),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }
        if let Some(FieldDataEnum::String(ref s)) = &self.fields_data {
            Ok(Some(s.clone()))
        } else {
            self.string_value()
        }
    }

    type ReadType = DummyRead;

    fn reader_value(&self) -> Result<Option<Self::ReadType>, LuceneError> {
        todo!()
    }

    fn numeric_value(&self) -> Result<Option<Number>, LuceneError> {
        if let Some(delegate) = &self.delegate {
            match delegate.numeric_value() {
                Ok(r) => return Ok(r),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }
        if let Some(FieldDataEnum::Number(ref n)) = &self.fields_data {
            Ok(Some(*n))
        } else {
            Ok(None)
        }
    }

    fn stored_value(&self) -> Result<Option<StoredValue>, LuceneError> {
        if let Some(delegate) = &self.delegate {
            match delegate.stored_value() {
                Ok(r) => return Ok(r),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }
        if !self.indexable_field_type.stored() {
            return Ok(None);
        }

        if self.fields_data.is_none() {
            return Err(LuceneError::illegal_argument("fieldsData is unset"));
        }
        match &self.fields_data {
            Some(FieldDataEnum::Number(val)) => match val {
                Number::U8(_) | Number::I16(_) => {
                    Err(LuceneError::illegal_state("Cannot store value of type"))
                }
                Number::I32(val) => Ok(Some(StoredValue::Integer(*val))),
                Number::I64(val) => Ok(Some(StoredValue::Long(*val))),
                Number::F32(val) => Ok(Some(StoredValue::Float(*val))),
                Number::F64(val) => Ok(Some(StoredValue::Double(*val))),
            },
            Some(FieldDataEnum::Binary(val)) => Ok(Some(StoredValue::Binary(val.clone()))),
            Some(FieldDataEnum::String(val)) => Ok(Some(StoredValue::String(val.clone()))),
            _ => Err(LuceneError::illegal_state("Cannot store value of type ")),
        }
    }

    fn invertable_type(&self) -> Result<&InvertableType, LuceneError> {
        if let Some(delegate) = &self.delegate {
            match delegate.invertable_type() {
                Ok(r) => return Ok(r),
                Err(e) => {
                    if !matches!(e, LuceneError::NotImplemented(_)) {
                        return Err(e);
                    }
                }
            }
        }
        Ok(&InvertableType::TokenStream)
    }
}
impl<R, T, F> Display for Field<R, T, F>
where
    R: Read + Debug,
    T: TokenStream + Debug,
    F: FieldBase + IndexableField,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}<{}:", self.indexable_field_type, self.name)?;

        if let Some(ref fields_data) = self.fields_data {
            write!(f, "{:?}", fields_data)?;
        }

        write!(f, ">")
    }
}

pub trait FieldBase {
    fn set_bytes_value(&mut self, _value: Arc<BytesRef>) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented(
            "set_bytes_value is not implemented",
        ))
    }
    fn set_byte_value(&mut self, _value: u8) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented(
            "set_byte_value is not implemented",
        ))
    }
    fn set_short_value(&mut self, _value: i16) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented(
            "set_short_value is not implemented",
        ))
    }
    fn set_int_value(&mut self, _value: i32) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented(
            "set_int_value is not implemented",
        ))
    }
    fn set_long_value(&mut self, _value: i64) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented(
            "set_long_value is not implemented",
        ))
    }
    fn set_float_value(&mut self, _value: f32) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented(
            "set_float_value is not implemented",
        ))
    }
    fn set_double_value(&mut self, _value: f64) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented(
            "set_double_value is not implemented",
        ))
    }
    fn set_token_stream<T: TokenStream>(
        &mut self,
        _token_stream: Arc<T>,
    ) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented(
            "set_token_stream is not implemented",
        ))
    }
    fn set_string_value(&mut self, _value: String) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented(
            "set_string_value is not implemented",
        ))
    }
    fn set_reader_value<R: Read>(&mut self, _value: Arc<R>) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented(
            "set_reader_value is not implemented",
        ))
    }
}
/// Specifies whether and how a field should be stored.
pub enum Store {
    /// Store the original field value in the index. This is useful for short texts like a document's
    /// title which should be displayed with the results. The value is stored in its original form,
    /// i.e. no analyzer is used before it is stored.
    Yes,

    /// Do not store the field value in the index.
    No,
}

#[derive(Debug)]
pub enum FieldDataEnum<R, T>
where
    R: Read + Debug,
    T: TokenStream + Debug,
{
    Number(Number),
    Binary(Arc<BytesRef>),
    String(String),
    Reader(Arc<R>),
    TokenStream(Arc<T>),
}
