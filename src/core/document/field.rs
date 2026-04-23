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
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_attributes::bytes_term_attribute::BytesTermAttribute;
use crate::core::analysis::token_attributes::char_term_attribute::CharTermAttribute;
use crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::core::analysis::token_attributes::packed_token_and_binary::BinaryTokenStreamAttributeImpl;
use crate::core::analysis::token_stream::{
  AnalyzerTokenStreams, TokenStream, TokenStreamBase, TokenStreamEnum2,
};
use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
use crate::core::document::field_type::FieldType;
use crate::core::document::fields::TokenStreamEnum;
use crate::core::document::invertable_field::InvertableType;
use crate::core::index::BytesRef;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
use crate::impl_from_for_enum;
use std::borrow::Cow;
use std::fmt;
use std::fmt::{Debug, Display};
use std::sync::Arc;

/// Expert: directly creates a field for a document. Most users should use one
/// of the convenience SubStruct:
///
/// - [`TextField`](crate::core::document::text_field::TextField):
///   [`Reader`](std::io::Read) or `String` indexed for full-text search.
/// - [`StringField`](crate::core::document::string_field::StringField): `String`
///   indexed verbatim as a single token.
/// - [`IntField`](crate::core::document::int_field::IntField): `i32` indexed for
///   exact/range queries.
/// - [`LongField`](crate::core::document::long_field::LongField): `i64` indexed for
///   exact/range queries.
/// - [`FloatField`](crate::core::document::float_field::FloatField): `f32` indexed
///   for exact/range queries.
/// - [`DoubleField`](crate::core::document::double_field::DoubleField): `f64` indexed
///   for exact/range queries.
/// - [`SortedDocValuesField`](crate::core::document::sorted_doc_values_field::SortedDocValuesField): `&[u8]` indexed column-wise for sorting/faceting.
/// - [`SortedSetDocValuesField`](crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField): `SortedSet<&[u8]>` indexed column-wise for sorting/faceting.
/// - [`NumericDocValuesField`](crate::core::document::numeric_doc_values_field::NumericDocValuesField): `i64` indexed column-wise for sorting/faceting.
/// - [`SortedNumericDocValuesField`](crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField): `SortedSet<i64>` indexed column-wise for sorting/faceting.
/// - [`StoredField`](crate::core::document::stored_field::StoredField): Stored-only
///   value for retrieving in summary results.
///
/// A field is a section of a document. Each field has three parts: name, type,
/// and value. Values may be text (`String`, `Reader`, or a pre-analyzed
/// `TokenStream`), binary (`&[u8]`), or numeric (`Number`). Fields are
/// optionally stored in the index so they can be returned with document hits.
///
/// # Note
/// The field type is an `IndexableFieldType`. Modifying the state of the
/// [`IndexableFieldType`] will affect any field using it. It is strongly
/// recommended not to make changes after field instantiation.
pub struct Field {
  /// Field's type.
  indexable_field_type: FieldType,
  /// Field's name.
  pub(crate) name: String,
  /// Field's value.
  pub(crate) fields_data: FieldDataEnum,
}
#[cfg(test)]
impl Clone for Field {
  fn clone(&self) -> Self {
    Self {
      indexable_field_type: self.indexable_field_type.clone(),
      name: self.name.clone(),
      fields_data: self.fields_data.clone(),
    }
  }
}
impl Field {
  /// Expert: creates a field with no initial value. This is intended to be
  /// used by custom [`Field`] sub-classes with pre-configured
  /// [`IndexableFieldType`].
  ///
  /// # Parameters
  /// - `name`: Field name.
  /// - `field_type`: Field type.
  ///
  /// # Errors
  /// - Returns an error if either the `name` or `field_type` is `None`.
  pub fn new<T, FD>(name: T, fields_data: FD, indexable_field_type: FieldType) -> Self
  where
    T: Into<String>,
    FD: Into<FieldDataEnum>,
  {
    Self {
      name: name.into(),
      fields_data: fields_data.into(),
      indexable_field_type,

    }
  }
  /// Creates a field with a `Reader` value.
  ///
  /// # Parameters
  /// - `name`: Field name.
  /// - `reader`: Reader value.
  /// - `field_type`: Field type.
  ///
  /// # Errors
  /// - Returns an error if the field's type is `stored()`, or if
  ///   `tokenized()` is `false`.
  pub fn from_reader<T>(
    name: T,
    reader: ReaderEnum,
    indexable_field_type: FieldType,
  ) -> Result<Self>
  where
    T: Into<String>,
  {
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
      name: name.into(),
      fields_data: FieldDataEnum::Reader(reader),

    })
  }
  /// Creates a field with a `TokenStream` value.
  ///
  /// # Parameters
  /// - `name`: Field name.
  /// - `token_stream`: `TokenStream` value.
  /// - `field_type`: Field type.
  ///
  /// # Errors
  /// - Returns an error if the field's type is `stored()`, `tokenized()` is
  ///   `false`, or `indexed()` is `false`.
  pub fn from_token_stream<T>(
    name: T,
    token_stream: TokenStreamEnum,
    indexable_field_type: FieldType,
  ) -> Result<Self>
  where
    T: Into<String>,
  {
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
      name: name.into(),
      fields_data: FieldDataEnum::TokenStream(token_stream),

    })
  }
  /// Creates a field with a binary value.
  ///
  /// # Note
  /// The provided byte array is **not copied**, so ensure that it is not
  /// modified until you are done using this field.
  ///
  /// # Parameters
  /// - `name`: Field name.
  /// - `value`: Byte array pointing to binary content (**not copied**).
  /// - `field_type`: Field type.
  ///
  /// # Errors
  /// - Returns an error if the field's type is `indexed()`.
  pub fn from_binary<T>(name: T, value: Vec<u8>, indexable_field_type: FieldType) -> Result<Self>
  where
    T: Into<String>,
  {
    let len = value.len();
    Self::from_binary_range(name, value, 0, len, indexable_field_type)
  }
  /// Creates a field with a binary value.
  ///
  /// # Note
  /// The provided byte array is **not copied**, so ensure that it is not
  /// modified until you are done using this field.
  ///
  /// # Parameters
  /// - `name`: Field name.
  /// - `value`: Byte array pointing to binary content (**not copied**).
  /// - `offset`: Starting position in the byte array.
  /// - `length`: Valid length of the byte array.
  /// - `field_type`: Field type.
  ///
  /// # Errors
  /// - Returns an error if the field's type is `indexed()`.
  pub fn from_binary_range<T>(
    name: T,
    value: Vec<u8>,
    offset: usize,
    length: usize,
    indexable_field_type: FieldType,
  ) -> Result<Self>
  where
    T: Into<String>,
  {
    let value = BytesRef::from_slice(value, offset, length);
    Self::from_bytes_ref(name, value, indexable_field_type)
  }
  /// Creates a field with a binary value.
  ///
  /// # Note
  /// The provided `BytesRef` is **not copied**, so ensure that it is not
  /// modified until you are done using this field.
  ///
  /// # Parameters
  /// - `name`: Field name.
  /// - `bytes`: `BytesRef` pointing to binary content (**not copied**).
  /// - `field_type`: Field type.
  ///
  /// # Errors
  /// - Returns an error if the field's type is `indexed()`.
  pub fn from_bytes_ref<T>(
    name: T,
    bytes: BytesRef<Vec<u8>>,
    indexable_field_type: FieldType,
  ) -> Result<Self>
  where
    T: Into<String>,
  {
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
      return Err(LuceneError::illegal_argument(
        "it doesn't make sense to have a field that is neither indexed, nor doc-valued, nor stored",
      ));
    }
    Ok(Field {
      indexable_field_type,
      name: name.into(),
      fields_data: FieldDataEnum::Binary(bytes),

    })
  }
  /// Creates a field with a `String` value.
  ///
  /// # Parameters
  /// - `name`: Field name.
  /// - `value`: String value.
  /// - `field_type`: Field type.
  ///
  /// # Errors
  /// - Returns an error if the field's type is neither `indexed()` nor
  ///   `stored()`.
  /// - Returns an error if `indexed()` is `false` but `store_term_vectors()`
  ///   is `true`.
  pub fn from_string<T1, T2>(name: T1, value: T2, indexable_field_type: FieldType) -> Result<Self>
  where
    T1: Into<String>,
    T2: Into<String>,
  {
    if !indexable_field_type.stored() && *indexable_field_type.index_options() == IndexOptions::None
    {
      return Err(LuceneError::illegal_argument(
        "it doesn't make sense to have a field that is neither indexed nor stored",
      ));
    }
    Ok(Field {
      indexable_field_type,
      name: name.into(),
      fields_data: FieldDataEnum::String(value.into()),

    })
  }
  /// Returns the `TokenStream` for this field to be used when indexing, or
  /// `None` if not set. If `None`, the `Reader` value or `String` value
  /// is analyzed to produce the indexed tokens.
  pub fn token_stream_value(&self) -> Result<Option<TokenStreamEnum>> {
    // TODO: 这里要移除所有权
    todo!()
  }
  /// Expert: changes the value of this field. This can be used during
  /// indexing to re-use a single `Field` instance to improve indexing
  /// speed by reducing GC overhead from creating and reclaiming
  /// `Field` instances. Typically, a single `Document` instance is also
  /// re-used, which is especially beneficial for small documents.
  ///
  /// # Note
  /// Each `Field` instance should only be used once within a single
  /// `Document` instance. See [ImproveIndexingSpeed](http://wiki.apache.org/lucene-java/ImproveIndexingSpeed) for details.
  pub fn set_string_value<T>(&mut self, value: T) -> Result<()>
  where
    T: Into<String>,
  {
    match &self.fields_data {
      FieldDataEnum::String(_) => {},
      _ => {
        return Err(LuceneError::illegal_argument(format!(
          "cannot change value type from {:?} to String",
          self.fields_data
        )));
      },
    }
    self.fields_data = FieldDataEnum::String(value.into());
    Ok(())
  }
  /// Expert: changes the value of this field. See
  /// [`set_string_value`](Field::set_string_value).
  pub fn set_reader_value(&mut self, value: ReaderEnum) -> Result<()> {
    match &self.fields_data {
      FieldDataEnum::Reader(_) => {},
      _ => {
        return Err(LuceneError::illegal_argument(format!(
          "cannot change value type from {:?} to Reader",
          self.fields_data
        )));
      },
    }

    self.fields_data = FieldDataEnum::Reader(value);
    Ok(())
  }
  pub fn set_vec_value(&mut self, value: Vec<u8>) -> Result<()> {
    self.set_bytes_value(BytesRef::from_bytes(value))
  }
  /// Expert: changes the value of this field. See
  /// [`set_string_value`](Field::set_string_value).
  ///
  /// NOTE: the provided [`BytesRef`] is not copied, so be sure not to change
  /// it until you're done with this field.
  pub fn set_bytes_value(&mut self, value: BytesRef<Vec<u8>>) -> Result<()> {
    match &self.fields_data {
      FieldDataEnum::Binary(_) => {},
      _ => {
        return Err(LuceneError::illegal_argument(format!(
          "cannot change value type from {:?} to BytesRef",
          self.fields_data
        )));
      },
    }
    self.fields_data = FieldDataEnum::Binary(value);
    Ok(())
  }
  /// Expert: changes the value of this field. See
  /// [`set_string_value`](Field::set_string_value).
  pub fn set_byte_value(&mut self, value: u8) -> Result<()> {
    match &self.fields_data {
      FieldDataEnum::Number(Number::U8(_)) => {},
      _ => {
        return Err(LuceneError::illegal_argument(format!(
          "cannot change value type from {:?} to Byte",
          self.fields_data
        )));
      },
    }
    self.fields_data = value.into();
    Ok(())
  }
  /// Expert: changes the value of this field. See
  /// [`set_string_value`](Field::set_string_value).
  pub fn set_short_value(&mut self, value: i16) -> Result<()> {
    match &self.fields_data {
      FieldDataEnum::Number(Number::I16(_)) => {},
      _ => {
        return Err(LuceneError::illegal_argument(format!(
          "cannot change value type from {:?} to Short",
          self.fields_data
        )));
      },
    }
    self.fields_data = FieldDataEnum::Number(Number::I16(value));
    Ok(())
  }
  /// Expert: changes the value of this field. See
  /// [`set_string_value`](Field::set_string_value).
  pub fn set_int_value(&mut self, value: i32) -> Result<()> {
    match &self.fields_data {
      FieldDataEnum::Number(Number::I32(_)) => {},
      _ => {
        return Err(LuceneError::illegal_argument(format!(
          "cannot change value type from {:?} to Integer",
          self.fields_data
        )));
      },
    }

    self.fields_data = FieldDataEnum::Number(Number::I32(value));
    Ok(())
  }
  /// Expert: changes the value of this field. See
  /// [`set_string_value`](Field::set_string_value).
  pub fn set_long_value(&mut self, value: i64) -> Result<()> {
    match &self.fields_data {
      FieldDataEnum::Number(Number::I64(_)) => {},
      _ => {
        return Err(LuceneError::illegal_argument(format!(
          "cannot change value type from {:?} to Long",
          self.fields_data
        )));
      },
    }

    self.fields_data = value.into();
    Ok(())
  }
  /// Expert: changes the value of this field. See
  /// [`set_string_value`](Field::set_string_value).
  pub fn set_float_value(&mut self, value: f32) -> Result<()> {
    match &self.fields_data {
      FieldDataEnum::Number(Number::F32(_)) => {},
      _ => {
        return Err(LuceneError::illegal_argument(format!(
          "cannot change value type from {:?} to Float",
          self.fields_data
        )));
      },
    }

    self.fields_data = FieldDataEnum::Number(Number::F32(value));
    Ok(())
  }
  /// Expert: changes the value of this field. See
  /// [`set_string_value`](Field::set_string_value).
  pub fn set_double_value(&mut self, value: f64) -> Result<()> {
    match &self.fields_data {
      FieldDataEnum::Number(Number::F64(_)) => {},
      _ => {
        return Err(LuceneError::illegal_argument(format!(
          "cannot change value type from {:?} to Double",
          self.fields_data
        )));
      },
    }

    self.fields_data = FieldDataEnum::Number(Number::F64(value));
    Ok(())
  }
  /// Expert: sets the token stream to be used for indexing.
  pub fn set_token_stream(&mut self, token_stream: TokenStreamEnum) -> Result<()> {
    match &self.fields_data {
      FieldDataEnum::TokenStream(_) => {},
      _ => {
        return Err(LuceneError::illegal_argument(format!(
          "cannot change value type from {:?} to TokenStream",
          self.fields_data
        )));
      },
    }

    self.fields_data = FieldDataEnum::TokenStream(token_stream);
    Ok(())
  }
}
impl IndexableField for Field {
  fn name(&self) -> &str {
    self.name.as_str()
  }

  type FieldType = FieldType;

  /// Returns the [`FieldType`] for this field.
  fn field_type(&self) -> &Self::FieldType {
    &self.indexable_field_type
  }
  fn token_stream<'a>(
    &'a mut self,
    token_stream: Option<&'a mut AnalyzerTokenStreams>,
    reuse_token_stream: Option<&'a mut TokenStreamEnum2<BinaryTokenStream, StringTokenStream>>,
  ) -> Result<
    Option<
      TokenStreamEnum2<
        &'a mut AnalyzerTokenStreams,
        &'a mut TokenStreamEnum2<BinaryTokenStream, StringTokenStream>,
      >,
    >,
  > {
    if *self.field_type().index_options() == IndexOptions::None {
      return Ok(None);
    }
    if !self.field_type().tokenized() {
      if self.string_value()?.is_some() {
        // TODO IMPORTANT avoid copy here?
        let string_value = self
          .string_value()?
          .ok_or_else(|| {
            LuceneError::illegal_state("Expected string value to be present, but it was None")
          })?
          .into_owned();
        return match reuse_token_stream {
          Some(v) => {
            match v {
              TokenStreamEnum2::A(_) => {
                return Err(LuceneError::illegal_argument("should StringTokenStream here"));
              },
              TokenStreamEnum2::B(s) => s.set_value(string_value)
            }
            Ok(Some(TokenStreamEnum2::B(v)))
          }
          None => {
            Err(LuceneError::illegal_state("StringTokenStream should already inited in PerField"))
          }
        }
      }
      if self.binary_value()?.is_some() {
        // TODO IMPORTANT avoid copy here?
        let binary_value = self
          .binary_value()?
          .ok_or_else(|| {
            LuceneError::illegal_state("Expected binary value to be present after is_some() check")
          })?
          .into_owned();
        return match reuse_token_stream {
          Some(v) => {
            match v {
              TokenStreamEnum2::B(_) => {
                return Err(LuceneError::illegal_argument("should BinaryTokenStream here"));
              },
              TokenStreamEnum2::A(s) => s.set_value(binary_value)
            }
            Ok(Some(TokenStreamEnum2::B(v)))
          }
          None => {
            Err(LuceneError::illegal_state("BinaryTokenStream should already inited in PerField"))
          }
        }
      }
    }
    if let Some(token_stream) = token_stream {
      Ok(Some(TokenStreamEnum2::A(token_stream)))
    } else {
      Err(LuceneError::illegal_state(
        "not initialized Analyzer's token stream in IndexableField::init_token_stream()?",
      ))
    }
  }

  fn binary_value(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    if let FieldDataEnum::Binary(bytes) = &self.fields_data {
      Ok(Some(Cow::Borrowed(bytes)))
    } else {
      Ok(None)
    }
  }

  fn take_binary_value(&mut self) -> Result<Option<BytesRef<Vec<u8>>>> {
    if !matches!(self.fields_data, FieldDataEnum::Binary(_)) {
      return Ok(None);
    }
    if let FieldDataEnum::Binary(binary) =
      std::mem::replace(&mut self.fields_data, FieldDataEnum::Dummy(()))
    {
      Ok(Some(binary))
    } else {
      Ok(None)
    }
  }

  /// Returns the value of the field as a `String`, or `None` if not set.
  /// If `None`, the `Reader` value or binary value is used.
  ///
  /// Exactly one of `string_value()`, `reader_value()`, or `binary_value()`
  /// must be set.
  fn string_value(&self) -> Result<Option<Cow<'_, String>>> {
    match &self.fields_data {
      FieldDataEnum::String(s) => Ok(Some(Cow::Borrowed(s))),
      FieldDataEnum::Number(n) => Ok(Some(Cow::Owned(n.as_string()))),
      _ => Ok(None),
    }
  }

  fn take_string_value(&mut self) -> Result<Option<String>> {
    if !matches!(self.fields_data, FieldDataEnum::String(_))
      && !matches!(self.fields_data, FieldDataEnum::Number(_))
    {
      return Ok(None);
    }
    match std::mem::replace(&mut self.fields_data, FieldDataEnum::Dummy(())) {
      FieldDataEnum::String(s) => Ok(Some(s)),
      FieldDataEnum::Number(n) => Ok(Some(n.as_string())),
      _ => Ok(None),
    }
  }

  fn get_char_sequence_value(&self) -> Result<Option<Cow<'_, String>>> {
    if let FieldDataEnum::String(s) = &self.fields_data {
      Ok(Some(Cow::Borrowed(s)))
    } else {
      self.string_value()
    }
  }

  /// Returns the value of the field as a `Reader`, or `None` if not set.
  /// If `None`, the `String` value or binary value is used.
  ///
  /// Exactly one of `string_value()`, `reader_value()`, or `binary_value()`
  /// must be set.
  fn take_reader_value(&mut self) -> Result<Option<ReaderEnum>> {
    if !matches!(self.fields_data, FieldDataEnum::Reader(_)) {
      return Ok(None);
    }
    if let FieldDataEnum::Reader(reader) =
      std::mem::replace(&mut self.fields_data, FieldDataEnum::Dummy(()))
    {
      Ok(Some(reader))
    } else {
      Ok(None)
    }
  }

  fn numeric_value(&self) -> Result<Option<Number>> {
    if let FieldDataEnum::Number(n) = &self.fields_data {
      Ok(Some(*n))
    } else {
      Ok(None)
    }
  }

  fn stored_value(&self) -> Option<&FieldDataEnum> {
    if !self.indexable_field_type.stored() {
      return None;
    }

    Some(&self.fields_data)
  }

  fn take_stored_value(&mut self) -> Option<FieldDataEnum> {
    todo!()
  }

  fn invertable_type(&self) -> &InvertableType {
    &InvertableType::TokenStream
  }

  fn init_token_stream<A>(&mut self, analyzer: &A) -> Result<()>
  where
    A: Analyzer,
  {
    let field_type = self.field_type();
    if *field_type.index_options() == IndexOptions::None {
      return Ok(());
    }
    if field_type.tokenized() {
      // TODO: 这里只考虑String value, tokenStreamValue暂时不考虑
      if let Some(reader) = self.take_reader_value()? {
        analyzer.token_stream(self.name(), reader)?;
      }
      if let Some(v) = self.string_value()? {
        analyzer.token_stream(self.name(), v.as_ref())?;
      }
    }
    Ok(())
  }

  fn vector_value(&self) -> Result<&VectorValueEnum> {
    if let FieldDataEnum::VectorValue(v) = &self.fields_data {
      Ok(v)
    } else {
      Err(LuceneError::unsupported_operation(""))
    }
  }
}
impl FieldBase for Field {}
impl Display for Field {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}<{}:", self.indexable_field_type, self.name)?;

    write!(f, "{}", self.fields_data)?;

    write!(f, ">")
  }
}

pub trait FieldBase {
  fn set_bytes_value(&mut self, _value: BytesRef<Vec<u8>>) -> Result<()> {
    Err(LuceneError::not_implemented(
      "set_bytes_value not implement",
    ))
  }
  fn set_byte_value(&mut self, _value: u8) -> Result<()> {
    Err(LuceneError::not_implemented("set_byte_value not implement"))
  }
  fn set_short_value(&mut self, _value: i16) -> Result<()> {
    Err(LuceneError::not_implemented(
      "set_short_value not implement",
    ))
  }
  fn set_int_value(&mut self, _value: i32) -> Result<()> {
    Err(LuceneError::not_implemented("set_int_value not implement"))
  }
  fn set_long_value(&mut self, _value: i64) -> Result<()> {
    Err(LuceneError::not_implemented("set_long_value not implement"))
  }
  fn set_float_value(&mut self, _value: f32) -> Result<()> {
    Err(LuceneError::not_implemented(
      "set_float_value not implement",
    ))
  }
  fn set_double_value(&mut self, _value: f64) -> Result<()> {
    Err(LuceneError::not_implemented(
      "set_double_value not implement",
    ))
  }
  fn set_token_stream(&mut self, _token_stream: Arc<TokenStreamEnum>) -> Result<()> {
    Err(LuceneError::not_implemented(
      "set_token_stream not implement",
    ))
  }
  fn set_string_value<T>(&mut self, _value: T) -> Result<()>
  where
    T: Into<String>,
  {
    Err(LuceneError::not_implemented(
      "set_string_value not implement",
    ))
  }
  fn set_reader_value(&mut self, _value: Arc<ReaderEnum>) -> Result<()> {
    Err(LuceneError::not_implemented(
      "set_reader_value not implement",
    ))
  }
}
/// Specifies whether and how a field should be stored.
pub enum Store {
  /// Store the original field value in the index. This is useful for short
  /// texts like a document's title which should be displayed with the
  /// results. The value is stored in its original form, i.e. no analyzer
  /// is used before it is stored.
  Yes,

  /// Do not store the field value in the index.
  No,
}
impl From<Store> for bool {
  fn from(store: Store) -> bool {
    matches!(store, Store::Yes)
  }
}

#[derive(Debug, Clone)]
pub enum FieldDataEnum {
  Number(Number),
  Binary(BytesRef<Vec<u8>>),
  String(String),
  Reader(ReaderEnum),
  TokenStream(TokenStreamEnum),
  // used to std::mem::replace(FieldDataEnum)
  Dummy(()),
  VectorValue(VectorValueEnum),
}

impl From<i32> for FieldDataEnum {
  fn from(v: i32) -> Self {
    FieldDataEnum::Number(Number::I32(v))
  }
}

impl From<i64> for FieldDataEnum {
  fn from(v: i64) -> Self {
    FieldDataEnum::Number(Number::I64(v))
  }
}

impl From<u8> for FieldDataEnum {
  fn from(v: u8) -> Self {
    FieldDataEnum::Number(Number::U8(v))
  }
}

impl From<i16> for FieldDataEnum {
  fn from(v: i16) -> Self {
    FieldDataEnum::Number(Number::I16(v))
  }
}

impl From<f32> for FieldDataEnum {
  fn from(v: f32) -> Self {
    FieldDataEnum::Number(Number::F32(v))
  }
}

impl From<f64> for FieldDataEnum {
  fn from(v: f64) -> Self {
    FieldDataEnum::Number(Number::F64(v))
  }
}
impl_from_for_enum!(
    FieldDataEnum,
    BytesRef<Vec<u8>> => Binary,
    String => String,
    ReaderEnum => Reader,
    TokenStreamEnum => TokenStream,
     VectorValueEnum => VectorValue,
);

impl From<&str> for FieldDataEnum {
  fn from(s: &str) -> Self {
    FieldDataEnum::String(s.to_string())
  }
}

impl Display for FieldDataEnum {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      FieldDataEnum::Number(n) => write!(f, "{}", n),
      FieldDataEnum::Binary(b) => write!(f, "{}", b),
      FieldDataEnum::String(s) => write!(f, "{}", s),
      FieldDataEnum::Reader(r) => write!(f, "{:?}", r),
      FieldDataEnum::TokenStream(t) => write!(f, "{:?}", t),
      FieldDataEnum::Dummy(s) => write!(f, "{:?}", s),
      FieldDataEnum::VectorValue(v) => write!(f, "{:?}", v),
    }
  }
}
/// Creates a new TokenStream that returns a BytesRef as single token
pub struct BinaryTokenStream {
  used: bool,
  value: Option<BytesRef<Vec<u8>>>,
  token_stream_base: TokenStreamBase,
}

impl BinaryTokenStream {
  /// Creates a new TokenStream that returns a BytesRef as single token.
  pub(crate) fn new() -> Result<Self> {
    Ok(Self {
      used: false,
      value: None,
      token_stream_base: TokenStreamBase::new(BinaryTokenStreamAttributeImpl::new()?.into()),
    })
  }

  /// Sets the bytes value.
  pub(crate) fn set_value(&mut self, value: BytesRef<Vec<u8>>) {
    self.value = Some(value);
  }
}

impl Drop for BinaryTokenStream {
  fn drop(&mut self) {
    self.close().expect("should not fail");
  }
}

impl TokenStream for BinaryTokenStream {
  fn increment_token(&mut self) -> Result<bool> {
    if self.used {
      return Ok(false);
    }
    self.token_stream_base.att.clear_attributes();
    let value = self.value.take();
    BytesTermAttribute::set_bytes_ref(&mut self.token_stream_base.att, value)?;
    self.used = true;
    Ok(true)
  }

  fn end(&mut self) -> Result<()> {
    self.default_end()
  }

  fn reset(&mut self) -> Result<()> {
    self.used = false;
    Ok(())
  }

  fn close(&mut self) -> Result<()> {
    let _ = self.value.take();
    Ok(())
  }

  fn get_attribute_source(&self) -> &Attributes {
    &self.token_stream_base.att
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    &mut self.token_stream_base.att
  }
}

pub struct StringTokenStream {
  used: bool,
  value: Option<String>,
  token_stream_base: TokenStreamBase,
}
impl StringTokenStream {
  /// Creates a new TokenStream that returns a String as single token.
  pub(crate) fn new() -> Self {
    Self {
      used: false,
      value: None,
      token_stream_base: TokenStreamBase::new(Attributes::default()),
    }
  }
  pub(crate) fn set_value(&mut self, value: String) {
    self.value = Some(value);
  }
}

impl Drop for StringTokenStream {
  fn drop(&mut self) {
    self.close().expect("should not fail");
  }
}

impl TokenStream for StringTokenStream {
  fn increment_token(&mut self) -> Result<bool> {
    if self.used {
      return Ok(false);
    }
    self.token_stream_base.att.clear_attributes();
    let value = self
      .value
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_argument("set_value() not call?"))?;
    self.token_stream_base.att.append_str(Some(value));
    debug_assert!(value.len() <= i32::MAX as usize);
    self
      .token_stream_base
      .att
      .set_offset(0, value.len() as i32)?;
    self.used = true;
    Ok(true)
  }

  fn end(&mut self) -> Result<()> {
    self.default_end()?;
    let final_offset = self.value.as_ref().unwrap().len() as i32;
    self
      .token_stream_base
      .att
      .set_offset(final_offset, final_offset)
  }

  fn reset(&mut self) -> Result<()> {
    self.used = false;
    Ok(())
  }

  fn close(&mut self) -> Result<()> {
    let _ = self.value.take();
    Ok(())
  }

  fn get_attribute_source(&self) -> &Attributes {
    &self.token_stream_base.att
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    &mut self.token_stream_base.att
  }
}

#[cfg(test)]
mod tests {
  use crate::core::analysis::dummy::dummy_token_stream::DummyTokenStream;
  use crate::core::analysis::reader::ReaderEnum;
  use crate::core::analysis::reusable_string_reader::ReusableStringReader;

  use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
  use crate::core::document::document::Document;
  use crate::core::document::double_doc_values_field::DoubleDocValuesField;
  use crate::core::document::double_field::DoubleField;
  use crate::core::document::double_point::DoublePoint;
  use crate::core::document::field::{Field, FieldBase, FieldDataEnum, Store};
  use crate::core::document::field_type::FieldType;
  use crate::core::document::fields::TokenStreamEnum;
  use crate::core::document::float_doc_values_field::FloatDocValuesField;
  use crate::core::document::float_field::FloatField;
  use crate::core::document::float_point::FloatPoint;
  use crate::core::document::int_field::IntField;
  use crate::core::document::int_point::IntPoint;
  use crate::core::document::knn_byte_vector_field::KnnByteVectorField;
  use crate::core::document::knn_float_vector_field::KnnFloatVectorField;
  use crate::core::document::long_field::LongField;
  use crate::core::document::long_point::LongPoint;
  use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
  use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
  use crate::core::document::stored_field::StoredField;
  use crate::core::document::string_field::StringField;
  use crate::core::document::text_field::TextField;
  use crate::core::index::BytesRef;
  use crate::core::index::byte_vector_values::ByteVectorValues;
  use crate::core::index::composite_reader::get_context;
  use crate::core::index::float_vector_values::FloatVectorValues;
  use crate::core::index::index_options::IndexOptions;

  use crate::core::index::index_reader_context::IndexReaderContext;
  use crate::core::index::indexable_field::IndexableField;
  use crate::core::index::indexable_field_type::IndexableFieldType;
  use crate::core::index::knn_vector_values::KnnVectorValues;
  use crate::core::index::leaf_reader::LeafReader;
  use crate::core::index::stored_fields::StoredFields;
  use crate::core::index::term::Term;
  use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
  use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
  use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
  use crate::core::search::term_query::TermQuery;
  use crate::core::search::top_docs::TopDocsLike;
  use crate::core::util::TryIntoInt;
  use crate::core::util::error::lucene_error::{LuceneError, Result};
  use crate::core::util::number::Number;
  use crate::core::util::numeric_utils::NumericUtils;
  use crate::test::core::index::random_index_writer::RandomIndexWriter;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_bytes_ref_from_bytes, new_bytes_ref_from_string, new_directory_shared,
    new_searcher_with_reader, random,
  };
  use std::sync::Arc;

  #[allow(dead_code)] // for quick search
  struct TestField;

  #[test]
  fn test_double_point() -> Result<()> {
    let mut field = DoublePoint::new("foo", [5.0])?;
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    field.set_double_value(6.0)?;
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    match field.numeric_value() {
      Ok(Some(Number::F64(value))) => assert_eq!(value, 6.0),
      _ => unreachable!(),
    }
    assert_eq!(
      format!("{} <foo:6>", std::any::type_name::<DoublePoint>()),
      field.to_string()
    );
    Ok(())
  }
  #[test]
  fn test_double_point_2d() -> Result<()> {
    let mut field = DoublePoint::new("foo", [5.0, 4.0])?;
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    field.set_double_values(&[6.0, 7.0])?;
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    match field.numeric_value() {
      Err(err) => {
        assert!(
          err
            .to_string()
            .contains("cannot convert to a single numeric value")
        );
      },
      _ => unreachable!(),
    }

    assert_eq!(
      format!("{} <foo:6,7>", std::any::type_name::<DoublePoint>()),
      field.to_string()
    );
    Ok(())
  }

  #[test]
  fn test_double_doc_values_field() -> Result<()> {
    let mut field = DoubleDocValuesField::new("foo", 5.0);
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    field.set_double_value(6.0)?;
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    match field.numeric_value() {
      Ok(Some(Number::I64(bits))) => {
        let value = f64::from_bits(bits as u64);
        assert_eq!(value, 6.0);
      },
      _ => unreachable!(),
    }

    Ok(())
  }

  #[test]
  fn test_float_doc_values_field() -> Result<()> {
    let mut field = FloatDocValuesField::new("foo", 5.0);
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    field.set_float_value(6.0)?;
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    match field.numeric_value() {
      Ok(Some(Number::I64(bits))) => {
        let value = f32::from_bits(bits as u32);
        assert_eq!(value, 6.0);
      },
      _ => unreachable!(),
    }

    Ok(())
  }

  #[test]
  fn test_float_point() -> Result<()> {
    let mut field = FloatPoint::new("foo", [5.0])?;
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    field.set_float_value(6.0)?;
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    match field.numeric_value() {
      Ok(Some(Number::F32(value))) => assert_eq!(value, 6.0),
      _ => unreachable!(),
    }

    assert_eq!(
      format!("{} <foo:6>", std::any::type_name::<FloatPoint>()),
      field.to_string()
    );
    Ok(())
  }

  #[test]
  fn test_float_point_2d() -> Result<()> {
    let mut field = FloatPoint::new("foo", [5.0, 4.0])?;
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    field.set_float_values(&[6.0, 7.0])?;
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    match field.numeric_value() {
      Err(err) => {
        assert!(
          err
            .to_string()
            .contains("cannot convert to a single numeric value")
        );
      },
      _ => unreachable!(),
    }

    assert_eq!(
      format!("{} <foo:6,7>", std::any::type_name::<FloatPoint>()),
      field.to_string()
    );
    Ok(())
  }

  #[test]
  fn test_int_point() -> Result<()> {
    let mut field = IntPoint::new("foo", [5])?;
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    field.set_int_value(6)?;
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    match field.numeric_value() {
      Ok(Some(Number::I32(value))) => assert_eq!(value, 6),
      _ => unreachable!(),
    }

    assert_eq!(
      format!("{} <foo:6>", std::any::type_name::<IntPoint>()),
      field.to_string()
    );
    Ok(())
  }

  #[test]
  fn test_int_point_2d() -> Result<()> {
    let mut field = IntPoint::new("foo", [5, 4])?;
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    field.set_int_values(&[6, 7])?;
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    match field.numeric_value() {
      Err(err) => {
        assert!(
          err
            .to_string()
            .contains("cannot convert to a single numeric value")
        );
      },
      _ => unreachable!(),
    }

    assert_eq!(
      format!("{} <foo:6,7>", std::any::type_name::<IntPoint>()),
      field.to_string()
    );
    Ok(())
  }

  #[test]
  fn test_int_field() -> Result<()> {
    let fields = vec![
      IntField::new("foo", 12, Store::No)?,
      IntField::new("foo", 12, Store::Yes)?,
    ];

    for mut field in fields {
      assert!(matches!(
        try_set_byte_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_bytes_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_bytes_ref_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_double_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      field.set_int_value(6)?;
      assert!(matches!(
        try_set_long_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_float_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_long_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_reader_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_short_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_string_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_token_stream_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));

      match field.numeric_value() {
        Ok(Some(Number::I32(value))) => assert_eq!(value, 6),
        _ => unreachable!(),
      }

      assert_eq!(
        NumericUtils::sortable_bytes_to_int(&field.binary_value()?.unwrap().bytes, 0),
        6
      );

      assert_eq!(
        format!("{} <foo:6>", std::any::type_name::<IntField>()),
        field.to_string()
      );

      if field.field_type().stored() {
        match field.stored_value() {
          Some(FieldDataEnum::Number(v)) => assert_eq!(v.to_i32().unwrap(), 6),
          Some(_) => unreachable!(""),
          None => unreachable!(""),
        }
      } else {
        assert!(field.stored_value().is_none());
      }
    }

    Ok(())
  }

  #[test]
  fn test_long_field() -> Result<()> {
    let fields = vec![
      LongField::new("foo", 12, Store::No)?,
      LongField::new("foo", 12, Store::Yes)?,
    ];

    for mut field in fields {
      assert!(matches!(
        try_set_byte_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_bytes_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_bytes_ref_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_double_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_int_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      field.set_long_value(6)?;
      assert!(matches!(
        try_set_float_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_reader_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_short_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_string_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_token_stream_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));

      match field.numeric_value() {
        Ok(Some(Number::I64(value))) => assert_eq!(value, 6),
        _ => unreachable!(),
      }

      let decoded =
        NumericUtils::sortable_bytes_to_long(&field.binary_value()?.unwrap().as_ref().bytes, 0);
      assert_eq!(decoded, 6);

      assert_eq!(
        format!("{} <foo:6>", std::any::type_name::<LongField>()),
        field.to_string()
      );

      if field.field_type().stored() {
        match field.stored_value() {
          Some(FieldDataEnum::Number(v)) => assert_eq!(v.to_i64().unwrap(), 6),
          _ => unreachable!(),
        }
      } else {
        assert!(field.stored_value().is_none());
      }
    }

    Ok(())
  }

  #[test]
  fn test_float_field() -> Result<()> {
    let fields = vec![
      FloatField::new("foo", 12.6, Store::No)?,
      FloatField::new("foo", 12.6, Store::Yes)?,
    ];

    for mut field in fields {
      match field.numeric_value() {
        Ok(Some(Number::I64(bits))) => {
          let v = NumericUtils::sortable_int_to_float(bits as i32);
          assert!((v - 12.6).abs() < f32::EPSILON);
        },
        _ => unreachable!(),
      }
      assert!(
        (FloatPoint::decode_dimension(&field.binary_value()?.unwrap().as_ref().bytes, 0) - 12.6)
          .abs()
          < f32::EPSILON
      );
      assert_eq!(
        format!("{} <foo:12.6>", std::any::type_name::<FloatField>()),
        field.to_string()
      );

      assert!(matches!(
        try_set_byte_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_bytes_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_bytes_ref_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_double_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_int_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_long_value(&mut field),
        Err(LuceneError::IllegalArgument(_))
      ));
      field.set_float_value(-28.8)?;
      assert!(matches!(
        try_set_reader_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_short_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_string_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_token_stream_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));

      match field.numeric_value() {
        Ok(Some(Number::I64(bits))) => {
          let v = NumericUtils::sortable_int_to_float(bits.try_convert()?);
          assert!((v + 28.8).abs() < f32::EPSILON);
        },
        _ => unreachable!(),
      }
      assert!(
        (FloatPoint::decode_dimension(&field.binary_value()?.unwrap().as_ref().bytes, 0) + 28.8)
          .abs()
          < f32::EPSILON
      );
      assert_eq!(
        format!("{} <foo:-28.8>", std::any::type_name::<FloatField>()),
        field.to_string()
      );

      if field.field_type().stored() {
        match field.stored_value() {
          Some(FieldDataEnum::Number(v)) => {
            let v = v.to_f32().ok_or_else(|| {
              LuceneError::illegal_argument(format!("cannot convert to f32: {}", v))
            })?;
            assert!((v + 28.8).abs() < f32::EPSILON);
          },
          _ => unreachable!(),
        }
      } else {
        assert!(field.stored_value().is_none());
      }
    }

    Ok(())
  }

  #[test]
  fn test_double_field() -> Result<()> {
    let fields = vec![
      DoubleField::new("foo", 12.7, Store::No)?,
      DoubleField::new("foo", 12.7, Store::Yes)?,
    ];

    for mut field in fields {
      match field.numeric_value() {
        Ok(Some(Number::I64(bits))) => {
          let v = NumericUtils::sortable_long_to_double(bits);
          assert!((v - 12.7).abs() < f64::EPSILON);
        },
        _ => unreachable!(),
      }
      assert!(
        (DoublePoint::decode_dimension(&field.binary_value()?.unwrap().as_ref().bytes, 0) - 12.7)
          .abs()
          < f64::EPSILON
      );
      assert_eq!(
        format!("{} <foo:12.7>", std::any::type_name::<DoubleField>()),
        field.to_string()
      );

      assert!(matches!(
        try_set_byte_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_bytes_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_bytes_ref_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_int_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_long_value(&mut field),
        Err(LuceneError::IllegalArgument(_))
      ));
      assert!(matches!(
        try_set_float_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      field.set_double_value(-28.8)?;
      assert!(matches!(
        try_set_reader_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_short_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_string_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_token_stream_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));

      match field.numeric_value() {
        Ok(Some(Number::I64(bits))) => {
          let v = NumericUtils::sortable_long_to_double(bits);
          assert!((v + 28.8).abs() < f64::EPSILON);
        },
        _ => unreachable!(),
      }
      assert!(
        (DoublePoint::decode_dimension(&field.binary_value()?.unwrap().as_ref().bytes, 0) + 28.8)
          .abs()
          < f64::EPSILON
      );
      assert_eq!(
        format!("{} <foo:-28.8>", std::any::type_name::<DoubleField>()),
        field.to_string()
      );

      if field.field_type().stored() {
        match field.stored_value() {
          Some(FieldDataEnum::Number(v)) => {
            assert!((v.to_f64().unwrap() + 28.8).abs() < f64::EPSILON);
          },
          Some(_) => unreachable!(""),
          None => unreachable!(""),
        }
      } else {
        assert!(field.stored_value().is_none());
      }
    }

    Ok(())
  }

  #[test]
  fn test_numeric_doc_values_field() -> Result<()> {
    let mut field = NumericDocValuesField::new("foo", 5);
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    field.set_long_value(6)?;
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    match field.numeric_value() {
      Ok(Some(Number::I64(value))) => assert_eq!(value, 6),
      _ => unreachable!(),
    }

    Ok(())
  }

  #[test]
  fn test_long_point() -> Result<()> {
    let mut field = LongPoint::new("foo", [5])?;
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    field.set_long_value(6)?;
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    match field.numeric_value() {
      Ok(Some(Number::I64(value))) => assert_eq!(value, 6),
      _ => unreachable!(),
    }

    assert_eq!(
      format!("{} <foo:6>", std::any::type_name::<LongPoint>()),
      field.to_string()
    );

    Ok(())
  }

  #[test]
  fn test_long_point_2d() -> Result<()> {
    let mut field = LongPoint::new("foo", [5, 4])?;
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    field.set_long_values([6, 7])?;
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    match field.numeric_value() {
      Err(err) => {
        assert!(
          err
            .to_string()
            .contains("cannot convert to a single numeric value")
        );
      },
      _ => unreachable!(),
    }

    assert_eq!(
      format!("{} <foo:6,7>", std::any::type_name::<LongPoint>()),
      field.to_string()
    );
    Ok(())
  }

  #[test]
  fn test_sorted_bytes_doc_values_field() -> Result<()> {
    let mut random = random();
    let mut field =
      SortedDocValuesField::new("foo", new_bytes_ref_from_string(&mut random, "bar")?);
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    field.set_bytes_value("fubar".into())?;
    field.set_bytes_value(new_bytes_ref_from_string(&mut random, "baz")?)?;
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    let binary_value = field.binary_value()?;
    let v = binary_value.as_ref().unwrap().as_ref();
    assert_eq!(&new_bytes_ref_from_string(&mut random, "baz")?, v);
    Ok(())
  }
  #[test]
  fn test_binary_doc_values_field() -> Result<()> {
    let mut random = random();
    let mut field =
      BinaryDocValuesField::new("foo", new_bytes_ref_from_string(&mut random, "bar")?);
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    field.set_bytes_value("fubar".into())?;
    field.set_bytes_value(new_bytes_ref_from_string(&mut random, "baz")?)?;
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    let binary_value = field.binary_value()?;
    let v = binary_value.as_ref().unwrap().as_ref();
    assert_eq!(&new_bytes_ref_from_string(&mut random, "baz")?, v);
    Ok(())
  }
  #[test]
  fn test_string_field() -> Result<()> {
    let fields = vec![
      StringField::from_string("foo", "bar", Store::No)?,
      StringField::from_string("foo", "bar", Store::Yes)?,
    ];

    for mut field in fields {
      assert!(matches!(
        try_set_byte_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_bytes_value(&mut field),
        Err(LuceneError::IllegalArgument(_))
      ));
      assert!(matches!(
        try_set_bytes_ref_value(&mut field),
        Err(LuceneError::IllegalArgument(_))
      ));
      assert!(matches!(
        try_set_double_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_int_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_float_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_long_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_reader_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_short_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      field.set_string_value("baz")?;
      assert!(matches!(
        try_set_token_stream_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      let string_value = field.string_value()?;
      assert_eq!(string_value.as_ref().unwrap().as_ref(), "baz");

      if field.field_type().stored() {
        match field.stored_value() {
          Some(FieldDataEnum::String(v)) => assert_eq!(v, "baz"),
          _ => unreachable!(),
        }
      } else {
        assert!(field.stored_value().is_none());
      }
    }

    Ok(())
  }

  #[test]
  fn test_binary_string_field() -> Result<()> {
    let fields = vec![
      StringField::from_bytes_ref("foo", "bar".into(), Store::No)?,
      StringField::from_bytes_ref("foo", "bar".into(), Store::Yes)?,
    ];

    for mut field in fields {
      assert!(matches!(
        try_set_byte_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      field.set_bytes_value("baz".into())?;
      assert_eq!(
        field.binary_value()?.as_ref().unwrap().as_ref(),
        &BytesRef::from_string("baz")
      );
      field.set_bytes_value("baz".into())?;
      assert!(matches!(
        try_set_double_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_int_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_float_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_long_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_reader_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_short_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_string_value(&mut field),
        Err(LuceneError::IllegalArgument(_))
      ));
      assert!(matches!(
        try_set_token_stream_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));

      assert_eq!(
        field.binary_value()?.as_ref().unwrap().as_ref(),
        &BytesRef::from_string("baz")
      );

      if field.field_type().stored() {
        match field.stored_value() {
          Some(FieldDataEnum::Binary(v)) => assert_eq!(v, &BytesRef::from_string("baz")),
          _ => unreachable!(),
        }
      } else {
        assert!(field.stored_value().is_none());
      }
    }

    Ok(())
  }

  #[test]
  fn test_text_field_string() -> Result<()> {
    let fields = vec![
      TextField::from_string("foo", "bar", Store::No)?,
      TextField::from_string("foo", "bar", Store::Yes)?,
    ];

    for mut field in fields {
      assert!(matches!(
        try_set_byte_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_bytes_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_bytes_ref_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_double_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_int_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_float_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_long_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_reader_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_short_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      field.set_string_value("baz")?;
      assert!(matches!(
        try_set_token_stream_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));

      let string_value = field.string_value()?;
      let v = string_value.as_ref().unwrap();
      assert_eq!(v.as_ref(), "baz");
      if field.field_type().stored() {
        match field.stored_value() {
          Some(FieldDataEnum::String(v)) => assert_eq!(v.as_str(), "baz"),
          _ => unreachable!(),
        }
      } else {
        assert!(field.stored_value().is_none());
      }
    }

    Ok(())
  }

  #[test]
  fn test_text_field_reader() -> Result<()> {
    // TODO
    Ok(())
  }

  #[test]
  fn test_stored_field_bytes() -> Result<()> {
    let mut random = random();
    let fields = vec![
      StoredField::from_binary("foo", b"bar".to_vec())?,
      StoredField::from_binary_with_range("foo", b"bar".to_vec(), 0, 3)?,
      StoredField::from_bytes_ref("foo", new_bytes_ref_from_string(&mut random, "bar")?)?,
    ];

    for mut field in fields {
      assert!(matches!(
        try_set_byte_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      field.set_bytes_value("baz".into())?;
      field.set_bytes_value(new_bytes_ref_from_string(&mut random, "baz")?)?;
      assert!(matches!(
        try_set_double_value(&mut field),
        Err(LuceneError::IllegalArgument(_))
      ));
      assert!(matches!(
        try_set_int_value(&mut field),
        Err(LuceneError::IllegalArgument(_))
      ));
      assert!(matches!(
        try_set_float_value(&mut field),
        Err(LuceneError::IllegalArgument(_))
      ));
      assert!(matches!(
        try_set_long_value(&mut field),
        Err(LuceneError::IllegalArgument(_))
      ));
      assert!(matches!(
        try_set_reader_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_short_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));
      assert!(matches!(
        try_set_string_value(&mut field),
        Err(LuceneError::IllegalArgument(_))
      ));
      assert!(matches!(
        try_set_token_stream_value(&mut field),
        Err(LuceneError::NotImplemented(_))
      ));

      assert_eq!(
        field.binary_value()?.as_ref().unwrap().as_ref(),
        &new_bytes_ref_from_string(&mut random, "baz")?
      );
    }

    Ok(())
  }

  #[test]
  fn test_stored_field_string() -> Result<()> {
    let mut field = StoredField::from_string("foo", "bar")?;
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    field.set_string_value("baz")?;
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    let string_value = field.string_value()?;
    let v = string_value.as_ref().unwrap();
    assert_eq!(v.as_ref(), "baz");
    Ok(())
  }

  #[test]
  fn test_stored_field_int() -> Result<()> {
    let mut field = StoredField::from_i32("foo", 1)?;
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    field.set_int_value(5)?;
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    match field.numeric_value()? {
      Some(Number::I32(v)) => assert_eq!(v, 5),
      _ => unreachable!(),
    }

    Ok(())
  }

  #[test]
  fn test_stored_field_double() -> Result<()> {
    let mut field = StoredField::from_f64("foo", 1f64)?;
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    field.set_double_value(5.0)?;
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    match field.numeric_value()? {
      Some(Number::F64(v)) => assert!((v - 5.0).abs() < f64::EPSILON),
      _ => unreachable!(),
    }

    Ok(())
  }

  #[test]
  fn test_stored_field_float() -> Result<()> {
    let mut field = StoredField::from_f32("foo", 1.0)?;
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    field.set_float_value(5.0)?;
    assert!(matches!(
      try_set_long_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    match field.numeric_value()? {
      Some(Number::F32(v)) => assert!((v - 5.0).abs() < f32::EPSILON),
      _ => unreachable!(),
    }

    Ok(())
  }
  #[test]
  fn test_stored_field_long() -> Result<()> {
    let mut field = StoredField::from_i64("foo", 1)?;
    assert!(matches!(
      try_set_byte_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_bytes_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_bytes_ref_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_double_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_int_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_float_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    field.set_long_value(5)?;
    assert!(matches!(
      try_set_reader_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_short_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));
    assert!(matches!(
      try_set_string_value(&mut field),
      Err(LuceneError::IllegalArgument(_))
    ));
    assert!(matches!(
      try_set_token_stream_value(&mut field),
      Err(LuceneError::NotImplemented(_))
    ));

    match field.numeric_value()? {
      Some(Number::I64(v)) => assert_eq!(v, 5),
      _ => unreachable!(),
    }

    Ok(())
  }

  #[test]
  fn test_indexed_binary_field() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let writer = RandomIndexWriter::new(&mut random, dir);

    let mut doc = Document::new();
    let br = new_bytes_ref_from_bytes(&mut random, &[0u8; 5])?;
    let field = StringField::from_bytes_ref("binary", br.clone(), Store::Yes)?;
    assert_eq!(field.binary_value()?.as_ref().unwrap().as_ref(), &br);

    doc.add(field);
    writer.add_document(doc)?;

    let reader = writer.get_reader()?;
    let searcher = new_searcher_with_reader(reader)?;
    let query = TermQuery::new(Term::new("binary", br.clone()));
    let hits = searcher.search(query, 1)?;
    assert_eq!(hits.total_hits().value(), 1);
    let stored_doc = searcher
      .stored_fields()?
      .document(hits.score_docs()[0].doc)?;
    let stored_field = stored_doc.get_field("binary").unwrap();
    assert_eq!(stored_field.binary_value()?.as_ref().unwrap().as_ref(), &br);
    writer.close()?;
    Ok(())
  }

  #[test]
  fn test_knn_vector_field() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let writer = RandomIndexWriter::new(&mut random, dir);

    let mut doc = Document::new();
    let byte_vector = vec![0u8; 5];
    let byte_field = KnnByteVectorField::with_similarity_function(
      "binary",
      byte_vector.clone(),
      VectorSimilarityFunction::Euclidean,
    )?;
    assert!(byte_field.binary_value()?.is_none());
    match byte_field.vector_value()? {
      crate::core::codecs::knn_field_vectors_writer::VectorValueEnum::Byte(value) => {
        assert_eq!(value, &byte_vector)
      },
      _ => unreachable!("expected byte vector"),
    }

    let mismatched_float_field =
      KnnFloatVectorField::with_type("bogus", vec![1.0], byte_field.field_type().clone());
    assert!(matches!(
      mismatched_float_field,
      Err(LuceneError::IllegalArgument(_))
    ));

    let float_vector = vec![1.0f32, 2.0f32];
    let float_field = KnnFloatVectorField::new("float", float_vector.clone())?;
    assert!(float_field.binary_value()?.is_none());

    doc.add(byte_field);
    doc.add(float_field);
    writer.add_document(doc)?;

    let reader = writer.get_reader()?;
    let context = get_context(&reader)?;
    assert_eq!(1, context.leaves()?.len());
    let leaf = context.leaves()?[0].reader();

    let binary = leaf.get_byte_vector_values("binary")?.unwrap();
    assert_eq!(1, binary.size());
    let mut byte_iterator = binary.iterator()?;
    assert_ne!(NO_MORE_DOCS, byte_iterator.next_doc()?);
    assert_eq!(byte_vector.as_slice(), binary.vector_value(0)?.as_bytes()?);
    assert_eq!(NO_MORE_DOCS, byte_iterator.next_doc()?);
    assert!(binary.vector_value(1).is_err());

    let float_values = leaf.get_float_vector_values("float")?.unwrap();
    assert_eq!(1, float_values.size());
    let mut float_iterator = float_values.iterator()?;
    assert_ne!(NO_MORE_DOCS, float_iterator.next_doc()?);
    let stored_float_vector = float_values.vector_value(0)?;
    assert_eq!(float_vector.len(), stored_float_vector.len());
    assert_eq!(float_vector[0], stored_float_vector.as_floats()?[0]);
    assert_eq!(NO_MORE_DOCS, float_iterator.next_doc()?);
    assert!(float_values.vector_value(1).is_err());

    writer.close()?;
    Ok(())
  }

  fn try_set_byte_value<F>(f: &mut F) -> Result<()>
  where
    F: FieldBase,
  {
    f.set_byte_value(10)
  }
  fn try_set_bytes_value<F>(f: &mut F) -> Result<()>
  where
    F: FieldBase,
  {
    f.set_bytes_value(BytesRef::from_bytes(vec![5, 5]))
  }

  fn try_set_bytes_ref_value<F>(f: &mut F) -> Result<()>
  where
    F: FieldBase,
  {
    f.set_bytes_value(BytesRef::from_string("bogus"))
  }

  fn try_set_double_value<F>(f: &mut F) -> Result<()>
  where
    F: FieldBase,
  {
    f.set_double_value(f64::MAX)
  }

  fn try_set_int_value<F>(f: &mut F) -> Result<()>
  where
    F: FieldBase,
  {
    f.set_int_value(i32::MAX)
  }

  fn try_set_long_value<F>(f: &mut F) -> Result<()>
  where
    F: FieldBase,
  {
    f.set_long_value(i64::MAX)
  }

  fn try_set_float_value<F>(f: &mut F) -> Result<()>
  where
    F: FieldBase,
  {
    f.set_float_value(f32::MAX)
  }

  fn try_set_reader_value<F>(f: &mut F) -> Result<()>
  where
    F: FieldBase,
  {
    let mut reader = ReusableStringReader::new();
    reader.set_value("BOO!");
    let read = ReaderEnum::ReusedString(reader);
    f.set_reader_value(Arc::from(read))
  }

  fn try_set_short_value<F>(f: &mut F) -> Result<()>
  where
    F: FieldBase,
  {
    f.set_short_value(i16::MAX)
  }

  fn try_set_string_value<F>(f: &mut F) -> Result<()>
  where
    F: FieldBase,
  {
    f.set_string_value("BOO!")
  }

  fn try_set_token_stream_value<F>(f: &mut F) -> Result<()>
  where
    F: FieldBase,
  {
    let token_stream = TokenStreamEnum::Dummy(Arc::new(DummyTokenStream));
    f.set_token_stream(Arc::new(token_stream))
  }
  #[test]
  fn test_disabled_field() -> Result<()> {
    let ft = FieldType::new();
    let result = Field::from_string("foo", "", ft);
    assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
    Ok(())
  }
  #[test]
  fn test_tokenized_binary_field() -> Result<()> {
    let mut ft = FieldType::new();
    ft.set_tokenized(true)?;
    ft.set_index_options(IndexOptions::Docs)?;
    let result = Field::from_bytes_ref("foo", BytesRef::new(), ft);
    assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
    Ok(())
  }
  #[test]
  fn test_offsets_binary_field() -> Result<()> {
    let mut ft = FieldType::new();
    ft.set_tokenized(false)?;
    ft.set_index_options(IndexOptions::DocsAndFreqsAndPositionsAndOffsets)?;
    let result = Field::from_bytes_ref("foo", BytesRef::new(), ft);
    assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
    Ok(())
  }
  #[test]
  fn test_term_vectors_offsets_binary_field() -> Result<()> {
    let mut ft = FieldType::new();
    ft.set_tokenized(false)?;
    ft.set_store_term_vectors(true)?;
    ft.set_store_term_vector_offsets(true)?;
    ft.set_store_term_vector_offsets(true)?;
    let result = Field::from_bytes_ref("foo", BytesRef::new(), ft);
    assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
    Ok(())
  }
}
