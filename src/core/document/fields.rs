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
use crate::core::analysis::analyzer::Analyzer;
use crate::core::analysis::dummy::dummy_token_stream::DummyTokenStream;
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_stream::{InnerTokenStreams, TokenStreamEnum2};
use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
use crate::core::document::binary_point::BinaryPoint;
use crate::core::document::double_doc_values_field::DoubleDocValuesField;
use crate::core::document::double_field::DoubleField;
use crate::core::document::double_point::DoublePoint;
use crate::core::document::field::{Field, FieldDataEnum};
use crate::core::document::field_type::FieldType;
use crate::core::document::float_doc_values_field::FloatDocValuesField;
use crate::core::document::float_field::FloatField;
use crate::core::document::float_point::FloatPoint;
use crate::core::document::int_field::IntField;
use crate::core::document::int_point::IntPoint;
use crate::core::document::int_range::IntRange;
use crate::core::document::invertable_field::InvertableType;
use crate::core::document::keyword_field::KeywordField;
use crate::core::document::knn_byte_vector_field::KnnByteVectorField;
use crate::core::document::knn_float_vector_field::KnnFloatVectorField;
use crate::core::document::long_field::LongField;
use crate::core::document::long_point::LongPoint;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::document::stored_field::StoredField;
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::TextField;
use crate::core::index::BytesRef;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexing_chain::ReservedField;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::number::Number;
use crate::impl_from_for_enum;
#[cfg(test)]
use crate::test::core::index::test_doc_values_indexing::FieldImpl;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub enum Fields {
  Binary(BinaryPoint),
  BinaryDocValues(BinaryDocValuesField),
  DoubleDocValues(DoubleDocValuesField),
  DoubleField(DoubleField),
  DoublePoint(DoublePoint),
  Field(Field),
  #[cfg(test)]
  FieldImpl(FieldImpl),
  FloatDocValues(FloatDocValuesField),
  FloatField(FloatField),
  FloatPoint(FloatPoint),
  Int(IntRange),
  IntField(IntField),
  IntPoint(IntPoint),
  Keyword(KeywordField),
  KnnByteVector(KnnByteVectorField),
  KnnFloatVector(KnnFloatVectorField),
  LongField(LongField),
  LongPoint(LongPoint),
  NumericDocValues(NumericDocValuesField),
  Reverse(ReservedField<NumericDocValuesField>),
  SortedDocValues(SortedDocValuesField),
  SortedNumericDocValues(SortedNumericDocValuesField),
  SortedSetDocValues(SortedSetDocValuesField),
  Stored(StoredField),
  String(StringField),
  Text(TextField),
}

impl_from_for_enum!(
    Fields,
    BinaryPoint => Binary,
    BinaryDocValuesField => BinaryDocValues,
    DoubleDocValuesField => DoubleDocValues,
    DoubleField => DoubleField,
    DoublePoint => DoublePoint,
    Field => Field,
    FloatDocValuesField => FloatDocValues,
    FloatField => FloatField,
    FloatPoint => FloatPoint,
    IntRange => Int,
    IntField => IntField,
    IntPoint => IntPoint,
    KeywordField => Keyword,
    KnnByteVectorField => KnnByteVector,
    KnnFloatVectorField => KnnFloatVector,
    LongField => LongField,
    LongPoint => LongPoint,
    NumericDocValuesField => NumericDocValues,
    ReservedField<NumericDocValuesField> => Reverse,
    SortedDocValuesField => SortedDocValues,
    SortedNumericDocValuesField => SortedNumericDocValues,
    SortedSetDocValuesField => SortedSetDocValues,
    StoredField => Stored,
    StringField => String,
    TextField => Text,
);
#[cfg(test)]
impl_from_for_enum!(Fields, FieldImpl => FieldImpl);

impl Display for Fields {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Fields::Binary(f1) => f1.fmt(f),
      Fields::BinaryDocValues(f1) => f1.fmt(f),
      Fields::DoubleDocValues(f1) => f1.fmt(f),
      Fields::DoubleField(f1) => f1.fmt(f),
      Fields::DoublePoint(f1) => f1.fmt(f),
      Fields::Field(f1) => f1.fmt(f),
      #[cfg(test)]
      Fields::FieldImpl(f1) => f1.fmt(f),
      Fields::FloatDocValues(f1) => f1.fmt(f),
      Fields::FloatField(f1) => f1.fmt(f),
      Fields::FloatPoint(f1) => f1.fmt(f),
      Fields::Int(f1) => f1.fmt(f),
      Fields::IntField(f1) => f1.fmt(f),
      Fields::IntPoint(f1) => f1.fmt(f),
      Fields::Keyword(f1) => f1.fmt(f),
      Fields::KnnByteVector(f1) => f1.fmt(f),
      Fields::KnnFloatVector(f1) => f1.fmt(f),
      Fields::LongField(f1) => f1.fmt(f),
      Fields::LongPoint(f1) => f1.fmt(f),
      Fields::NumericDocValues(f1) => f1.fmt(f),
      Fields::Reverse(f1) => f1.fmt(f),
      Fields::SortedDocValues(f1) => f1.fmt(f),
      Fields::SortedNumericDocValues(f1) => f1.fmt(f),
      Fields::SortedSetDocValues(f1) => f1.fmt(f),
      Fields::Stored(f1) => f1.fmt(f),
      Fields::String(f1) => f1.fmt(f),
      Fields::Text(f1) => f1.fmt(f),
    }
  }
}

impl IndexableField for Fields {
  fn name(&self) -> &str {
    match self {
      Fields::Binary(f) => f.name(),
      Fields::BinaryDocValues(f) => f.name(),
      Fields::DoubleDocValues(f) => f.name(),
      Fields::DoubleField(f) => f.name(),
      Fields::DoublePoint(f) => f.name(),
      Fields::Field(f) => f.name(),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.name(),
      Fields::FloatDocValues(f) => f.name(),
      Fields::FloatField(f) => f.name(),
      Fields::FloatPoint(f) => f.name(),
      Fields::Int(f) => f.name(),
      Fields::IntField(f) => f.name(),
      Fields::IntPoint(f) => f.name(),
      Fields::Keyword(f) => f.name(),
      Fields::KnnByteVector(f) => f.name(),
      Fields::KnnFloatVector(f) => f.name(),
      Fields::LongField(f) => f.name(),
      Fields::LongPoint(f) => f.name(),
      Fields::NumericDocValues(f) => f.name(),
      Fields::Reverse(f) => f.name(),
      Fields::SortedDocValues(f) => f.name(),
      Fields::SortedNumericDocValues(f) => f.name(),
      Fields::SortedSetDocValues(f) => f.name(),
      Fields::Stored(f) => f.name(),
      Fields::String(f) => f.name(),
      Fields::Text(f) => f.name(),
    }
  }

  type FieldType = FieldType;

  fn field_type(&self) -> &Self::FieldType {
    match self {
      Fields::Binary(f) => f.field_type(),
      Fields::BinaryDocValues(f) => f.field_type(),
      Fields::DoubleDocValues(f) => f.field_type(),
      Fields::DoubleField(f) => f.field_type(),
      Fields::DoublePoint(f) => f.field_type(),
      Fields::Field(f) => f.field_type(),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.field_type(),
      Fields::FloatDocValues(f) => f.field_type(),
      Fields::FloatField(f) => f.field_type(),
      Fields::FloatPoint(f) => f.field_type(),
      Fields::Int(f) => f.field_type(),
      Fields::IntField(f) => f.field_type(),
      Fields::IntPoint(f) => f.field_type(),
      Fields::Keyword(f) => f.field_type(),
      Fields::KnnByteVector(f) => f.field_type(),
      Fields::KnnFloatVector(f) => f.field_type(),
      Fields::LongField(f) => f.field_type(),
      Fields::LongPoint(f) => f.field_type(),
      Fields::NumericDocValues(f) => f.field_type(),
      Fields::Reverse(f) => f.field_type(),
      Fields::SortedDocValues(f) => f.field_type(),
      Fields::SortedNumericDocValues(f) => f.field_type(),
      Fields::SortedSetDocValues(f) => f.field_type(),
      Fields::Stored(f) => f.field_type(),
      Fields::String(f) => f.field_type(),
      Fields::Text(f) => f.field_type(),
    }
  }

  type TokenStream = <Field as IndexableField>::TokenStream;

  fn token_stream<'a>(
    &'a mut self,
    token_stream: Option<&'a mut InnerTokenStreams>,
  ) -> Result<Option<TokenStreamEnum2<&'a mut InnerTokenStreams, &'a mut Self::TokenStream>>> {
    match self {
      Fields::Binary(f) => f.token_stream(token_stream),
      Fields::BinaryDocValues(f) => f.token_stream(token_stream),
      Fields::DoubleDocValues(f) => f.token_stream(token_stream),
      Fields::DoubleField(f) => f.token_stream(token_stream),
      Fields::DoublePoint(f) => f.token_stream(token_stream),
      Fields::Field(f) => f.token_stream(token_stream),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.token_stream(token_stream),
      Fields::FloatDocValues(f) => f.token_stream(token_stream),
      Fields::FloatField(f) => f.token_stream(token_stream),
      Fields::FloatPoint(f) => f.token_stream(token_stream),
      Fields::Int(f) => f.token_stream(token_stream),
      Fields::IntField(f) => f.token_stream(token_stream),
      Fields::IntPoint(f) => f.token_stream(token_stream),
      Fields::Keyword(f) => f.token_stream(token_stream),
      Fields::KnnByteVector(f) => f.token_stream(token_stream),
      Fields::KnnFloatVector(f) => f.token_stream(token_stream),
      Fields::LongField(f) => f.token_stream(token_stream),
      Fields::LongPoint(f) => f.token_stream(token_stream),
      Fields::NumericDocValues(f) => f.token_stream(token_stream),
      Fields::Reverse(f) => f.token_stream(token_stream),
      Fields::SortedDocValues(f) => f.token_stream(token_stream),
      Fields::SortedNumericDocValues(f) => f.token_stream(token_stream),
      Fields::SortedSetDocValues(f) => f.token_stream(token_stream),
      Fields::Stored(f) => f.token_stream(token_stream),
      Fields::String(f) => f.token_stream(token_stream),
      Fields::Text(f) => f.token_stream(token_stream),
    }
  }

  fn binary_value(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    match self {
      Fields::Binary(f) => f.binary_value(),
      Fields::BinaryDocValues(f) => f.binary_value(),
      Fields::DoubleDocValues(f) => f.binary_value(),
      Fields::DoubleField(f) => f.binary_value(),
      Fields::DoublePoint(f) => f.binary_value(),
      Fields::Field(f) => f.binary_value(),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.binary_value(),
      Fields::FloatDocValues(f) => f.binary_value(),
      Fields::FloatField(f) => f.binary_value(),
      Fields::FloatPoint(f) => f.binary_value(),
      Fields::Int(f) => f.binary_value(),
      Fields::IntField(f) => f.binary_value(),
      Fields::IntPoint(f) => f.binary_value(),
      Fields::Keyword(f) => f.binary_value(),
      Fields::KnnByteVector(f) => f.binary_value(),
      Fields::KnnFloatVector(f) => f.binary_value(),
      Fields::LongField(f) => f.binary_value(),
      Fields::LongPoint(f) => f.binary_value(),
      Fields::NumericDocValues(f) => f.binary_value(),
      Fields::Reverse(f) => f.binary_value(),
      Fields::SortedDocValues(f) => f.binary_value(),
      Fields::SortedNumericDocValues(f) => f.binary_value(),
      Fields::SortedSetDocValues(f) => f.binary_value(),
      Fields::Stored(f) => f.binary_value(),
      Fields::String(f) => f.binary_value(),
      Fields::Text(f) => f.binary_value(),
    }
  }

  fn take_binary_value(&mut self) -> Result<Option<BytesRef<Vec<u8>>>> {
    match self {
      Fields::Binary(f) => f.take_binary_value(),
      Fields::BinaryDocValues(f) => f.take_binary_value(),
      Fields::DoubleDocValues(f) => f.take_binary_value(),
      Fields::DoubleField(f) => f.take_binary_value(),
      Fields::DoublePoint(f) => f.take_binary_value(),
      Fields::Field(f) => f.take_binary_value(),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.take_binary_value(),
      Fields::FloatDocValues(f) => f.take_binary_value(),
      Fields::FloatField(f) => f.take_binary_value(),
      Fields::FloatPoint(f) => f.take_binary_value(),
      Fields::Int(f) => f.take_binary_value(),
      Fields::IntField(f) => f.take_binary_value(),
      Fields::IntPoint(f) => f.take_binary_value(),
      Fields::Keyword(f) => f.take_binary_value(),
      Fields::KnnByteVector(f) => f.take_binary_value(),
      Fields::KnnFloatVector(f) => f.take_binary_value(),
      Fields::LongField(f) => f.take_binary_value(),
      Fields::LongPoint(f) => f.take_binary_value(),
      Fields::NumericDocValues(f) => f.take_binary_value(),
      Fields::Reverse(f) => f.take_binary_value(),
      Fields::SortedDocValues(f) => f.take_binary_value(),
      Fields::SortedNumericDocValues(f) => f.take_binary_value(),
      Fields::SortedSetDocValues(f) => f.take_binary_value(),
      Fields::Stored(f) => f.take_binary_value(),
      Fields::String(f) => f.take_binary_value(),
      Fields::Text(f) => f.take_binary_value(),
    }
  }

  fn string_value(&self) -> Result<Option<Cow<'_, String>>> {
    match self {
      Fields::Binary(f) => f.string_value(),
      Fields::BinaryDocValues(f) => f.string_value(),
      Fields::DoubleDocValues(f) => f.string_value(),
      Fields::DoubleField(f) => f.string_value(),
      Fields::DoublePoint(f) => f.string_value(),
      Fields::Field(f) => f.string_value(),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.string_value(),
      Fields::FloatDocValues(f) => f.string_value(),
      Fields::FloatField(f) => f.string_value(),
      Fields::FloatPoint(f) => f.string_value(),
      Fields::Int(f) => f.string_value(),
      Fields::IntField(f) => f.string_value(),
      Fields::IntPoint(f) => f.string_value(),
      Fields::Keyword(f) => f.string_value(),
      Fields::KnnByteVector(f) => f.string_value(),
      Fields::KnnFloatVector(f) => f.string_value(),
      Fields::LongField(f) => f.string_value(),
      Fields::LongPoint(f) => f.string_value(),
      Fields::NumericDocValues(f) => f.string_value(),
      Fields::Reverse(f) => f.string_value(),
      Fields::SortedDocValues(f) => f.string_value(),
      Fields::SortedNumericDocValues(f) => f.string_value(),
      Fields::SortedSetDocValues(f) => f.string_value(),
      Fields::Stored(f) => f.string_value(),
      Fields::String(f) => f.string_value(),
      Fields::Text(f) => f.string_value(),
    }
  }

  fn take_string_value(&mut self) -> Result<Option<String>> {
    match self {
      Fields::Binary(f) => f.take_string_value(),
      Fields::BinaryDocValues(f) => f.take_string_value(),
      Fields::DoubleDocValues(f) => f.take_string_value(),
      Fields::DoubleField(f) => f.take_string_value(),
      Fields::DoublePoint(f) => f.take_string_value(),
      Fields::Field(f) => f.take_string_value(),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.take_string_value(),
      Fields::FloatDocValues(f) => f.take_string_value(),
      Fields::FloatField(f) => f.take_string_value(),
      Fields::FloatPoint(f) => f.take_string_value(),
      Fields::Int(f) => f.take_string_value(),
      Fields::IntField(f) => f.take_string_value(),
      Fields::IntPoint(f) => f.take_string_value(),
      Fields::Keyword(f) => f.take_string_value(),
      Fields::KnnByteVector(f) => f.take_string_value(),
      Fields::KnnFloatVector(f) => f.take_string_value(),
      Fields::LongField(f) => f.take_string_value(),
      Fields::LongPoint(f) => f.take_string_value(),
      Fields::NumericDocValues(f) => f.take_string_value(),
      Fields::Reverse(f) => f.take_string_value(),
      Fields::SortedDocValues(f) => f.take_string_value(),
      Fields::SortedNumericDocValues(f) => f.take_string_value(),
      Fields::SortedSetDocValues(f) => f.take_string_value(),
      Fields::Stored(f) => f.take_string_value(),
      Fields::String(f) => f.take_string_value(),
      Fields::Text(f) => f.take_string_value(),
    }
  }

  fn get_char_sequence_value(&self) -> Result<Option<Cow<'_, String>>> {
    match self {
      Fields::Binary(f) => f.get_char_sequence_value(),
      Fields::BinaryDocValues(f) => f.get_char_sequence_value(),
      Fields::DoubleDocValues(f) => f.get_char_sequence_value(),
      Fields::DoubleField(f) => f.get_char_sequence_value(),
      Fields::DoublePoint(f) => f.get_char_sequence_value(),
      Fields::Field(f) => f.get_char_sequence_value(),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.get_char_sequence_value(),
      Fields::FloatDocValues(f) => f.get_char_sequence_value(),
      Fields::FloatField(f) => f.get_char_sequence_value(),
      Fields::FloatPoint(f) => f.get_char_sequence_value(),
      Fields::Int(f) => f.get_char_sequence_value(),
      Fields::IntField(f) => f.get_char_sequence_value(),
      Fields::IntPoint(f) => f.get_char_sequence_value(),
      Fields::Keyword(f) => f.get_char_sequence_value(),
      Fields::KnnByteVector(f) => f.get_char_sequence_value(),
      Fields::KnnFloatVector(f) => f.get_char_sequence_value(),
      Fields::LongField(f) => f.get_char_sequence_value(),
      Fields::LongPoint(f) => f.get_char_sequence_value(),
      Fields::NumericDocValues(f) => f.get_char_sequence_value(),
      Fields::Reverse(f) => f.get_char_sequence_value(),
      Fields::SortedDocValues(f) => f.get_char_sequence_value(),
      Fields::SortedNumericDocValues(f) => f.get_char_sequence_value(),
      Fields::SortedSetDocValues(f) => f.get_char_sequence_value(),
      Fields::Stored(f) => f.get_char_sequence_value(),
      Fields::String(f) => f.get_char_sequence_value(),
      Fields::Text(f) => f.get_char_sequence_value(),
    }
  }

  fn take_reader_value(&mut self) -> Result<Option<ReaderEnum>> {
    match self {
      Fields::Binary(f) => f.take_reader_value(),
      Fields::BinaryDocValues(f) => f.take_reader_value(),
      Fields::DoubleDocValues(f) => f.take_reader_value(),
      Fields::DoubleField(f) => f.take_reader_value(),
      Fields::DoublePoint(f) => f.take_reader_value(),
      Fields::Field(f) => f.take_reader_value(),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.take_reader_value(),
      Fields::FloatDocValues(f) => f.take_reader_value(),
      Fields::FloatField(f) => f.take_reader_value(),
      Fields::FloatPoint(f) => f.take_reader_value(),
      Fields::Int(f) => f.take_reader_value(),
      Fields::IntField(f) => f.take_reader_value(),
      Fields::IntPoint(f) => f.take_reader_value(),
      Fields::Keyword(f) => f.take_reader_value(),
      Fields::KnnByteVector(f) => f.take_reader_value(),
      Fields::KnnFloatVector(f) => f.take_reader_value(),
      Fields::LongField(f) => f.take_reader_value(),
      Fields::LongPoint(f) => f.take_reader_value(),
      Fields::NumericDocValues(f) => f.take_reader_value(),
      Fields::Reverse(f) => f.take_reader_value(),
      Fields::SortedDocValues(f) => f.take_reader_value(),
      Fields::SortedNumericDocValues(f) => f.take_reader_value(),
      Fields::SortedSetDocValues(f) => f.take_reader_value(),
      Fields::Stored(f) => f.take_reader_value(),
      Fields::String(f) => f.take_reader_value(),
      Fields::Text(f) => f.take_reader_value(),
    }
  }

  fn numeric_value(&self) -> Result<Option<Number>> {
    match self {
      Fields::Binary(f) => f.numeric_value(),
      Fields::BinaryDocValues(f) => f.numeric_value(),
      Fields::DoubleDocValues(f) => f.numeric_value(),
      Fields::DoubleField(f) => f.numeric_value(),
      Fields::DoublePoint(f) => f.numeric_value(),
      Fields::Field(f) => f.numeric_value(),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.numeric_value(),
      Fields::FloatDocValues(f) => f.numeric_value(),
      Fields::FloatField(f) => f.numeric_value(),
      Fields::FloatPoint(f) => f.numeric_value(),
      Fields::Int(f) => f.numeric_value(),
      Fields::IntField(f) => f.numeric_value(),
      Fields::IntPoint(f) => f.numeric_value(),
      Fields::Keyword(f) => f.numeric_value(),
      Fields::KnnByteVector(f) => f.numeric_value(),
      Fields::KnnFloatVector(f) => f.numeric_value(),
      Fields::LongField(f) => f.numeric_value(),
      Fields::LongPoint(f) => f.numeric_value(),
      Fields::NumericDocValues(f) => f.numeric_value(),
      Fields::Reverse(f) => f.numeric_value(),
      Fields::SortedDocValues(f) => f.numeric_value(),
      Fields::SortedNumericDocValues(f) => f.numeric_value(),
      Fields::SortedSetDocValues(f) => f.numeric_value(),
      Fields::Stored(f) => f.numeric_value(),
      Fields::String(f) => f.numeric_value(),
      Fields::Text(f) => f.numeric_value(),
    }
  }

  fn stored_value(&self) -> Option<&FieldDataEnum> {
    match self {
      Fields::Binary(f) => f.stored_value(),
      Fields::BinaryDocValues(f) => f.stored_value(),
      Fields::DoubleDocValues(f) => f.stored_value(),
      Fields::DoubleField(f) => f.stored_value(),
      Fields::DoublePoint(f) => f.stored_value(),
      Fields::Field(f) => f.stored_value(),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.stored_value(),
      Fields::FloatDocValues(f) => f.stored_value(),
      Fields::FloatField(f) => f.stored_value(),
      Fields::FloatPoint(f) => f.stored_value(),
      Fields::Int(f) => f.stored_value(),
      Fields::IntField(f) => f.stored_value(),
      Fields::IntPoint(f) => f.stored_value(),
      Fields::Keyword(f) => f.stored_value(),
      Fields::KnnByteVector(f) => f.stored_value(),
      Fields::KnnFloatVector(f) => f.stored_value(),
      Fields::LongField(f) => f.stored_value(),
      Fields::LongPoint(f) => f.stored_value(),
      Fields::NumericDocValues(f) => f.stored_value(),
      Fields::Reverse(f) => f.stored_value(),
      Fields::SortedDocValues(f) => f.stored_value(),
      Fields::SortedNumericDocValues(f) => f.stored_value(),
      Fields::SortedSetDocValues(f) => f.stored_value(),
      Fields::Stored(f) => f.stored_value(),
      Fields::String(f) => f.stored_value(),
      Fields::Text(f) => f.stored_value(),
    }
  }

  fn take_stored_value(&mut self) -> Option<FieldDataEnum> {
    match self {
      Fields::Binary(f) => f.take_stored_value(),
      Fields::BinaryDocValues(f) => f.take_stored_value(),
      Fields::DoubleDocValues(f) => f.take_stored_value(),
      Fields::DoubleField(f) => f.take_stored_value(),
      Fields::DoublePoint(f) => f.take_stored_value(),
      Fields::Field(f) => f.take_stored_value(),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.take_stored_value(),
      Fields::FloatDocValues(f) => f.take_stored_value(),
      Fields::FloatField(f) => f.take_stored_value(),
      Fields::FloatPoint(f) => f.take_stored_value(),
      Fields::Int(f) => f.take_stored_value(),
      Fields::IntField(f) => f.take_stored_value(),
      Fields::IntPoint(f) => f.take_stored_value(),
      Fields::Keyword(f) => f.take_stored_value(),
      Fields::KnnByteVector(f) => f.take_stored_value(),
      Fields::KnnFloatVector(f) => f.take_stored_value(),
      Fields::LongField(f) => f.take_stored_value(),
      Fields::LongPoint(f) => f.take_stored_value(),
      Fields::NumericDocValues(f) => f.take_stored_value(),
      Fields::Reverse(f) => f.take_stored_value(),
      Fields::SortedDocValues(f) => f.take_stored_value(),
      Fields::SortedNumericDocValues(f) => f.take_stored_value(),
      Fields::SortedSetDocValues(f) => f.take_stored_value(),
      Fields::Stored(f) => f.take_stored_value(),
      Fields::String(f) => f.take_stored_value(),
      Fields::Text(f) => f.take_stored_value(),
    }
  }

  fn invertable_type(&self) -> &InvertableType {
    match self {
      Fields::Binary(f) => f.invertable_type(),
      Fields::BinaryDocValues(f) => f.invertable_type(),
      Fields::DoubleDocValues(f) => f.invertable_type(),
      Fields::DoubleField(f) => f.invertable_type(),
      Fields::DoublePoint(f) => f.invertable_type(),
      Fields::Field(f) => f.invertable_type(),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.invertable_type(),
      Fields::FloatDocValues(f) => f.invertable_type(),
      Fields::FloatField(f) => f.invertable_type(),
      Fields::FloatPoint(f) => f.invertable_type(),
      Fields::Int(f) => f.invertable_type(),
      Fields::IntField(f) => f.invertable_type(),
      Fields::IntPoint(f) => f.invertable_type(),
      Fields::Keyword(f) => f.invertable_type(),
      Fields::KnnByteVector(f) => f.invertable_type(),
      Fields::KnnFloatVector(f) => f.invertable_type(),
      Fields::LongField(f) => f.invertable_type(),
      Fields::LongPoint(f) => f.invertable_type(),
      Fields::NumericDocValues(f) => f.invertable_type(),
      Fields::Reverse(f) => f.invertable_type(),
      Fields::SortedDocValues(f) => f.invertable_type(),
      Fields::SortedNumericDocValues(f) => f.invertable_type(),
      Fields::SortedSetDocValues(f) => f.invertable_type(),
      Fields::Stored(f) => f.invertable_type(),
      Fields::String(f) => f.invertable_type(),
      Fields::Text(f) => f.invertable_type(),
    }
  }

  fn is_reserved(&self) -> bool {
    match self {
      Fields::Binary(f) => f.is_reserved(),
      Fields::BinaryDocValues(f) => f.is_reserved(),
      Fields::DoubleDocValues(f) => f.is_reserved(),
      Fields::DoubleField(f) => f.is_reserved(),
      Fields::DoublePoint(f) => f.is_reserved(),
      Fields::Field(f) => f.is_reserved(),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.is_reserved(),
      Fields::FloatDocValues(f) => f.is_reserved(),
      Fields::FloatField(f) => f.is_reserved(),
      Fields::FloatPoint(f) => f.is_reserved(),
      Fields::Int(f) => f.is_reserved(),
      Fields::IntField(f) => f.is_reserved(),
      Fields::IntPoint(f) => f.is_reserved(),
      Fields::Keyword(f) => f.is_reserved(),
      Fields::KnnByteVector(f) => f.is_reserved(),
      Fields::KnnFloatVector(f) => f.is_reserved(),
      Fields::LongField(f) => f.is_reserved(),
      Fields::LongPoint(f) => f.is_reserved(),
      Fields::NumericDocValues(f) => f.is_reserved(),
      Fields::Reverse(f) => f.is_reserved(),
      Fields::SortedDocValues(f) => f.is_reserved(),
      Fields::SortedNumericDocValues(f) => f.is_reserved(),
      Fields::SortedSetDocValues(f) => f.is_reserved(),
      Fields::Stored(f) => f.is_reserved(),
      Fields::String(f) => f.is_reserved(),
      Fields::Text(f) => f.is_reserved(),
    }
  }

  fn init_token_stream<A>(&mut self, analyzer: &A) -> Result<()>
  where
    A: Analyzer,
  {
    match self {
      Fields::Binary(f) => f.init_token_stream(analyzer),
      Fields::BinaryDocValues(f) => f.init_token_stream(analyzer),
      Fields::DoubleDocValues(f) => f.init_token_stream(analyzer),
      Fields::DoubleField(f) => f.init_token_stream(analyzer),
      Fields::DoublePoint(f) => f.init_token_stream(analyzer),
      Fields::Field(f) => f.init_token_stream(analyzer),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.init_token_stream(analyzer),
      Fields::FloatDocValues(f) => f.init_token_stream(analyzer),
      Fields::FloatField(f) => f.init_token_stream(analyzer),
      Fields::FloatPoint(f) => f.init_token_stream(analyzer),
      Fields::Int(f) => f.init_token_stream(analyzer),
      Fields::IntField(f) => f.init_token_stream(analyzer),
      Fields::IntPoint(f) => f.init_token_stream(analyzer),
      Fields::Keyword(f) => f.init_token_stream(analyzer),
      Fields::KnnByteVector(f) => f.init_token_stream(analyzer),
      Fields::KnnFloatVector(f) => f.init_token_stream(analyzer),
      Fields::LongField(f) => f.init_token_stream(analyzer),
      Fields::LongPoint(f) => f.init_token_stream(analyzer),
      Fields::NumericDocValues(f) => f.init_token_stream(analyzer),
      Fields::Reverse(f) => f.init_token_stream(analyzer),
      Fields::SortedDocValues(f) => f.init_token_stream(analyzer),
      Fields::SortedNumericDocValues(f) => f.init_token_stream(analyzer),
      Fields::SortedSetDocValues(f) => f.init_token_stream(analyzer),
      Fields::Stored(f) => f.init_token_stream(analyzer),
      Fields::String(f) => f.init_token_stream(analyzer),
      Fields::Text(f) => f.init_token_stream(analyzer),
    }
  }

  fn vector_value(&self) -> Result<&VectorValueEnum> {
    match self {
      Fields::Binary(f) => f.vector_value(),
      Fields::BinaryDocValues(f) => f.vector_value(),
      Fields::DoubleDocValues(f) => f.vector_value(),
      Fields::DoubleField(f) => f.vector_value(),
      Fields::DoublePoint(f) => f.vector_value(),
      Fields::Field(f) => f.vector_value(),
      #[cfg(test)]
      Fields::FieldImpl(f) => f.vector_value(),
      Fields::FloatDocValues(f) => f.vector_value(),
      Fields::FloatField(f) => f.vector_value(),
      Fields::FloatPoint(f) => f.vector_value(),
      Fields::Int(f) => f.vector_value(),
      Fields::IntField(f) => f.vector_value(),
      Fields::IntPoint(f) => f.vector_value(),
      Fields::Keyword(f) => f.vector_value(),
      Fields::KnnByteVector(f) => f.vector_value(),
      Fields::KnnFloatVector(f) => f.vector_value(),
      Fields::LongField(f) => f.vector_value(),
      Fields::LongPoint(f) => f.vector_value(),
      Fields::NumericDocValues(f) => f.vector_value(),
      Fields::Reverse(f) => f.vector_value(),
      Fields::SortedDocValues(f) => f.vector_value(),
      Fields::SortedNumericDocValues(f) => f.vector_value(),
      Fields::SortedSetDocValues(f) => f.vector_value(),
      Fields::Stored(f) => f.vector_value(),
      Fields::String(f) => f.vector_value(),
      Fields::Text(f) => f.vector_value(),
    }
  }
}

#[cfg(test)]
impl Clone for Fields {
  fn clone(&self) -> Self {
    match self {
      Fields::Binary(f) => f.clone().into(),
      Fields::BinaryDocValues(f) => f.clone().into(),
      Fields::DoubleDocValues(f) => f.clone().into(),
      Fields::DoubleField(f) => f.clone().into(),
      Fields::DoublePoint(f) => f.clone().into(),
      Fields::Field(f) => f.clone().into(),
      Fields::FieldImpl(f) => f.clone().into(),
      Fields::FloatDocValues(f) => f.clone().into(),
      Fields::FloatField(f) => f.clone().into(),
      Fields::FloatPoint(f) => f.clone().into(),
      Fields::Int(f) => f.clone().into(),
      Fields::IntField(f) => f.clone().into(),
      Fields::IntPoint(f) => f.clone().into(),
      Fields::Keyword(f) => f.clone().into(),
      Fields::KnnByteVector(f) => f.clone().into(),
      Fields::KnnFloatVector(f) => f.clone().into(),
      Fields::LongField(f) => f.clone().into(),
      Fields::LongPoint(f) => f.clone().into(),
      Fields::NumericDocValues(f) => f.clone().into(),
      Fields::Reverse(f) => f.clone().into(),
      Fields::SortedDocValues(f) => f.clone().into(),
      Fields::SortedNumericDocValues(f) => f.clone().into(),
      Fields::SortedSetDocValues(f) => f.clone().into(),
      Fields::Stored(f) => f.clone().into(),
      Fields::String(f) => f.clone().into(),
      Fields::Text(f) => f.clone().into(),
    }
  }
}

#[derive(Debug, Clone)]
pub enum TokenStreamEnum {
  Dummy(Arc<DummyTokenStream>),
}
