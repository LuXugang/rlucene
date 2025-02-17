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
use std::io::Cursor;
use std::sync::Arc;

pub struct Field {
    indexable_field_type: Arc<FieldType>,
    name: Arc<String>,
    pub(crate) fields_data: Option<FieldDataEnum>,
}
impl Field {
    pub fn new(name: Arc<String>, indexable_field_type: Arc<FieldType>) -> Self {
        Field {
            indexable_field_type,
            name,
            fields_data: None,
        }
    }
    pub fn with_reader(
        name: Arc<String>,
        reader: ReaderEnum,
        indexable_field_type: Arc<FieldType>,
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
        })
    }
    pub fn with_token_stream(
        name: Arc<String>,
        token_stream: TokenStreamEnum,
        indexable_field_type: Arc<FieldType>,
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
        })
    }
    pub fn with_binary(
        name: Arc<String>,
        value: Vec<u8>,
        indexable_field_type: Arc<FieldType>,
    ) -> Result<Self, LuceneError> {
        let len = value.len() as i32;
        Self::with_binary_range(name, value, 0, len, indexable_field_type)
    }
    pub fn with_binary_range(
        name: Arc<String>,
        value: Vec<u8>,
        offset: i32,
        length: i32,
        indexable_field_type: Arc<FieldType>,
    ) -> Result<Self, LuceneError> {
        let value = Arc::new(BytesRef::from_vec(value, offset, length));
        Self::with_bytes_ref(name, value, indexable_field_type)
    }

    pub fn with_bytes_ref(
        name: Arc<String>,
        bytes: Arc<BytesRef>,
        indexable_field_type: Arc<FieldType>,
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
        if indexable_field_type.index_options() != &IndexOptions::None
            && indexable_field_type.tokenized()
        {
            return Err(LuceneError::illegal_argument(
                "cannot set a BytesRef value on a tokenized field",
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
        })
    }
    pub fn with_string(
        name: Arc<String>,
        value: Arc<String>,
        indexable_field_type: Arc<FieldType>,
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
        })
    }
    pub fn token_stream_value(&self) -> Result<Option<TokenStreamEnum>, LuceneError> {
        if let Some(token_stream) = &self.fields_data {
            match token_stream {
                FieldDataEnum::TokenStream(token_stream) => Ok(Option::from(token_stream.clone())),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }
    pub fn set_string_value(&mut self, value: Arc<String>) -> Result<(), LuceneError> {
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
    pub fn set_reader_value(&mut self, value: ReaderEnum) -> Result<(), LuceneError> {
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
    pub fn set_vec_value(&mut self, value: Vec<u8>) -> Result<(), LuceneError> {
        self.set_bytes_value(Arc::new(BytesRef::from_bytes(value)))
    }
    pub fn set_bytes_value(&mut self, value: Arc<BytesRef>) -> Result<(), LuceneError> {
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
        match &self.fields_data {
            Some(FieldDataEnum::Number(Number::U8(_))) => {}
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
        match &self.fields_data {
            Some(FieldDataEnum::Number(Number::I16(_))) => {}
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
        match &self.fields_data {
            Some(FieldDataEnum::Number(Number::I32(_))) => {}
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
        match &self.fields_data {
            Some(FieldDataEnum::Number(Number::I64(_))) => {}
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
        match &self.fields_data {
            Some(FieldDataEnum::Number(Number::F32(_))) => {}
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
        match &self.fields_data {
            Some(FieldDataEnum::Number(Number::F64(_))) => {}
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
    pub fn set_token_stream(&mut self, token_stream: TokenStreamEnum) -> Result<(), LuceneError> {
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
impl IndexableField for Field {
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
        if let Some(FieldDataEnum::Binary(ref bytes)) = &self.fields_data {
            Ok(Some(bytes.clone()))
        } else {
            Ok(None)
        }
    }

    fn string_value(&self) -> Result<Option<Arc<String>>, LuceneError> {
        if let Some(FieldDataEnum::String(ref s)) = &self.fields_data {
            Ok(Some(s.clone()))
        } else if let Some(FieldDataEnum::Number(val)) = &self.fields_data {
            Ok(Some(Arc::from(val.as_string())))
        } else {
            Ok(None)
        }
    }

    fn get_char_sequence_value(&self) -> Result<Option<Arc<String>>, LuceneError> {
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
        if let Some(FieldDataEnum::Number(ref n)) = &self.fields_data {
            Ok(Some(*n))
        } else {
            Ok(None)
        }
    }

    fn stored_value(&self) -> Result<Option<StoredValue>, LuceneError> {
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
        Ok(&InvertableType::TokenStream)
    }
}
impl Display for Field {
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
    fn set_token_stream(&mut self, _token_stream: Arc<TokenStreamEnum>) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented(
            "set_token_stream is not implemented",
        ))
    }
    fn set_string_value(&mut self, _value: String) -> Result<(), LuceneError> {
        Err(LuceneError::not_implemented(
            "set_string_value is not implemented",
        ))
    }
    fn set_reader_value(&mut self, _value: Arc<ReaderEnum>) -> Result<(), LuceneError> {
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
pub enum FieldDataEnum {
    Number(Number),
    Binary(Arc<BytesRef>),
    String(Arc<String>),
    Reader(ReaderEnum),
    TokenStream(TokenStreamEnum),
}

#[derive(Debug, Clone)]
pub enum ReaderEnum {
    CursorStr(Arc<Cursor<String>>),
}

#[derive(Debug, Clone)]
pub enum TokenStreamEnum {
    Dummy(Arc<DummyTokenStream>),
}

#[cfg(test)]
mod tests {
    use crate::analysis::dummy::dummy_token_stream::DummyTokenStream;
    use crate::document::double_point::DoublePoint;
    

    use crate::document::field::{Field, FieldBase, ReaderEnum, TokenStreamEnum};

    use crate::index::indexable_field::IndexableField;
    use crate::index::BytesRef;
    use crate::test::util::test_error::TestError;

    use crate::util::error::lucene_error::LuceneError;
    use crate::util::number::Number;

    use crate::document::field_type::FieldType;
    use crate::index::index_options::IndexOptions;
    use std::sync::Arc;

    #[allow(dead_code)] // for quick search
    struct TestField;

    #[test]
    fn test_double_point() -> Result<(), TestError> {
        let mut field = DoublePoint::new("foo", &[5.0])?;
        let mut result = try_set_byte_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_bytes_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_bytes_ref_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        field.set_double_value(6.0)?;
        result = try_set_int_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_long_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_float_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_reader_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_short_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_string_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_token_stream_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        match field.numeric_value() {
            Ok(Some(Number::F64(value))) => assert_eq!(value, 6.0),
            _ => unreachable!(),
        }
        assert_eq!("DoublePoint <foo:6>", field.to_string());
        Ok(())
    }
    #[test]
    fn test_double_point_2d() -> Result<(), TestError> {
        let mut field = DoublePoint::new("foo", &[5.0, 4.0])?;
        let mut result = try_set_byte_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_bytes_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_bytes_ref_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_double_value(&mut field);
        assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
        field.set_double_values(&[6.0, 7.0])?;
        result = try_set_int_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_long_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_float_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_reader_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_short_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_string_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));
        result = try_set_token_stream_value(&mut field);
        assert!(matches!(result, Err(LuceneError::NotImplemented(_))));

        let result = field.numeric_value();
        assert!(result.is_err() || matches!(result, Ok(Some(_)) if false));

        if let Err(err) = result {
            assert!(err
                .to_string()
                .contains("cannot convert to a single numeric value"));
        }

        assert_eq!(field.to_string(), "DoublePoint <foo:6,7>");

        Ok(())
    }
    #[test]
    fn test_double_doc_values_field() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }
    #[test]
    fn test_float_doc_values_field() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_float_point() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_float_point_2d() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_int_point() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_int_point_2d() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_int_field() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_long_field() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_float_field() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_double_field() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_numeric_doc_values_field() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_long_point() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_long_point_2d() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_sorted_bytes_doc_values_field() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_binary_doc_values_field() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_string_field() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_binary_string_field() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_text_field_string() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_text_field_reader() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_stored_field_bytes() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_stored_field_string() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_stored_field_int() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_stored_field_double() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_stored_field_float() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_stored_field_long() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_indexed_binary_field() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    #[test]
    fn test_knn_vector_field() -> Result<(), LuceneError> {
        // TODO
        Ok(())
    }

    fn try_set_byte_value<F: FieldBase>(f: &mut F) -> Result<(), LuceneError> {
        f.set_byte_value(10)
    }
    fn try_set_bytes_value<F: FieldBase>(f: &mut F) -> Result<(), LuceneError> {
        f.set_bytes_value(Arc::new(BytesRef::from_bytes(vec![5, 5])))
    }

    fn try_set_bytes_ref_value<F: FieldBase>(f: &mut F) -> Result<(), LuceneError> {
        f.set_bytes_value(Arc::new(BytesRef::from_string("bogus")))
    }

    fn try_set_double_value<F: FieldBase>(f: &mut F) -> Result<(), LuceneError> {
        f.set_double_value(f64::MAX)
    }

    fn try_set_int_value<F: FieldBase>(f: &mut F) -> Result<(), LuceneError> {
        f.set_int_value(i32::MAX)
    }

    fn try_set_long_value<F: FieldBase>(f: &mut F) -> Result<(), LuceneError> {
        f.set_long_value(i64::MAX)
    }

    fn try_set_float_value<F: FieldBase>(f: &mut F) -> Result<(), LuceneError> {
        f.set_float_value(f32::MAX)
    }

    fn try_set_reader_value<F: FieldBase>(f: &mut F) -> Result<(), LuceneError> {
        let cursor = Arc::new(std::io::Cursor::new("BOO!".to_string()));
        let read = ReaderEnum::CursorStr(cursor);
        f.set_reader_value(Arc::from(read))
    }

    fn try_set_short_value<F: FieldBase>(f: &mut F) -> Result<(), LuceneError> {
        f.set_short_value(i16::MAX)
    }

    fn try_set_string_value<F: FieldBase>(f: &mut F) -> Result<(), LuceneError> {
        f.set_string_value("BOO!".to_string())
    }

    fn try_set_token_stream_value<F: FieldBase>(f: &mut F) -> Result<(), LuceneError> {
        let token_stream = TokenStreamEnum::Dummy(Arc::new(DummyTokenStream));
        f.set_token_stream(Arc::new(token_stream))
    }
    #[test]
    fn test_disabled_field() -> Result<(), LuceneError> {
        let ft = FieldType::new();
        let result = Field::with_string(
            Arc::new("foo".to_string()),
            Arc::new("".to_string()),
            Arc::new(ft),
        );
        assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
        Ok(())
    }
    #[test]
    fn test_tokenized_binary_field() -> Result<(), LuceneError> {
        let mut ft = FieldType::new();
        ft.set_tokenized(true)?;
        ft.set_index_options(IndexOptions::DOCS)?;
        let result = Field::with_bytes_ref(
            Arc::new("foo".to_string()),
            Arc::new(BytesRef::new()),
            Arc::new(ft),
        );
        assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
        Ok(())
    }
    #[test]
    fn test_offsets_binary_field() -> Result<(), LuceneError> {
        let mut ft = FieldType::new();
        ft.set_tokenized(false)?;
        ft.set_index_options(IndexOptions::DocsAndFreqsAndPositionsAndOffsets)?;
        let result = Field::with_bytes_ref(
            Arc::new("foo".to_string()),
            Arc::new(BytesRef::new()),
            Arc::new(ft),
        );
        assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
        Ok(())
    }
    #[test]
    fn test_term_vectors_offsets_binary_field() -> Result<(), LuceneError> {
        let mut ft = FieldType::new();
        ft.set_tokenized(false)?;
        ft.set_store_term_vectors(true)?;
        ft.set_store_term_vector_offsets(true)?;
        ft.set_store_term_vector_offsets(true)?;
        let result = Field::with_bytes_ref(
            Arc::new("foo".to_string()),
            Arc::new(BytesRef::new()),
            Arc::new(ft),
        );
        assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
        Ok(())
    }
}
