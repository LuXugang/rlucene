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
use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
use crate::core::document::field::{Field, FieldBase, FieldDataEnum};
use crate::core::document::field_type::FieldType;
use crate::core::document::invertable_field::InvertableType;
use crate::core::index::BytesRef;
use crate::core::index::indexable_field::{
  IndexableField, IndexingTokenStream, ReusedIndexingTokenStream,
};
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::number::Number;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};

/// A field that contains a single byte numeric vector (or none) for each document. Vectors are dense
/// that is, every dimension of a vector contains an explicit value, stored packed into an array
/// (as a byte slice) whose length is the vector dimension. Values can be retrieved using
/// `ByteVectorValues`, which is a forward-only docID-based iterator and also offers random-access by
/// dense ordinal (not docId). [`VectorSimilarityFunction`] may be used to compare vectors at
/// query time (for example as part of result ranking). A [`KnnByteVectorField`] may be associated with a
/// search similarity function defining the metric used for nearest-neighbor search among vectors of
/// that field.
pub struct KnnByteVectorField {
  parent_field: Field,
}

impl KnnByteVectorField {
  /// A field that contains a single byte numeric vector (or none) for each document. Vectors are dense
  /// that is, every dimension of a vector contains an explicit value, stored packed into an array
  /// (as a byte slice) whose length is the vector dimension. Values can be retrieved using
  /// [`ByteVectorValues`], which is a forward-only docID-based iterator and also offers random-access by
  /// dense ordinal (not docId). [`VectorSimilarityFunction`] may be used to compare vectors at
  /// query time (for example as part of result ranking). A [`KnnByteVectorField`] may be associated with a
  /// search similarity function defining the metric used for nearest-neighbor search among vectors of
  /// that field.
  fn create_type(v: &[u8], similarity_function: VectorSimilarityFunction) -> Result<FieldType> {
    if v.is_empty() {
      return Err(LuceneError::illegal_argument(
        "cannot index an empty vector",
      ));
    }

    let dimension = v.len() as i32;

    let mut field_type = FieldType::new();

    field_type.set_vector_attributes(dimension, VectorEncoding::BYTE(1), similarity_function)?;

    field_type.freeze();

    Ok(field_type)
  }

  /// A convenience method for creating a vector field type.
  ///
  /// # Arguments
  ///
  /// * `dimension` - dimension of vectors
  /// * `similarity_function` - a function defining vector proximity.
  ///
  /// # Errors
  ///
  /// returns [`LuceneError::IllegalArgument`] if has dimension > 1024.
  pub fn create_field_type(
    dimension: i32,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<FieldType> {
    let mut field_type = FieldType::new();

    field_type.set_vector_attributes(dimension, VectorEncoding::BYTE(1), similarity_function)?;

    field_type.freeze();

    Ok(field_type)
  }

  /// Creates a numeric vector field. Fields are single-valued: each document has either one value or
  /// no value. Vectors of a single field share the same dimension and similarity function. Note that
  /// some vector similarities (like `VectorSimilarityFunction::DOT_PRODUCT`) require values to
  /// be constant-length.
  ///
  /// # Arguments
  ///
  /// * `name` - field name
  /// * `vector` - value
  /// * `similarity_function` - a function defining vector proximity.
  ///
  /// # Errors
  ///
  /// returns [`LuceneError::IllegalArgument`] if the vector is empty or has
  /// dimension > 1024.
  pub fn with_similarity_function(
    name: &str,
    vector: Vec<u8>,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<Self> {
    let field_type = Self::create_type(vector.as_ref(), similarity_function)?;
    let v: VectorValueEnum = vector.into();
    let field = Field::new(name, v, field_type);

    Ok(Self {
      parent_field: field,
    })
  }

  /// Creates a numeric vector field with the default EUCLIDEAN_HNSW (L2) similarity. Fields are
  /// single-valued: each document has either one value or no value. Vectors of a single field share
  /// the same dimension and similarity function.
  ///
  /// # Arguments
  ///
  /// * `name` - field name
  /// * `vector` - value
  ///
  /// # Errors
  ///
  /// returns [`LuceneError::IllegalArgument`] if the vector is empty or has
  /// dimension > 1024.
  pub fn new(name: &str, vector: Vec<u8>) -> Result<Self> {
    Self::with_similarity_function(name, vector, VectorSimilarityFunction::Euclidean)
  }

  /// Creates a numeric vector field. Fields are single-valued: each document has either one value or
  /// no value. Vectors of a single field share the same dimension and similarity function.
  ///
  /// # Arguments
  ///
  /// * `name` - field name
  /// * `vector` - value
  /// * `field_type` - field type
  ///
  /// # Errors
  ///
  /// returns [`LuceneError::IllegalArgument`] if the vector is empty or has
  /// dimension > 1024.
  pub fn with_type(name: &str, vector: Vec<u8>, field_type: FieldType) -> Result<Self> {
    if *field_type.vector_encoding() != VectorEncoding::BYTE(1) {
      return Err(LuceneError::illegal_argument(format!(
        "Attempt to create a vector for field {} using bytes but the field encoding is {:?}",
        name,
        field_type.vector_encoding()
      )));
    }

    if vector.len() as i32 != field_type.vector_dimension() {
      return Err(LuceneError::illegal_argument(
        "The number of vector dimensions does not match the field type",
      ));
    }

    let v: VectorValueEnum = vector.into();
    let field = Field::new(name, v, field_type);

    Ok(Self {
      parent_field: field,
    })
  }

  /// Return the vector value of this field
  pub fn vector_value(&self) -> Result<&VectorValueEnum> {
    match self.parent_field.fields_data {
      FieldDataEnum::VectorValue(ref v) => Ok(v),
      _ => Err(LuceneError::illegal_state(
        "field value is not a byte vector",
      )),
    }
  }

  /// Set the vector value of this field
  ///
  /// # Arguments
  ///
  /// * `value` - value to set; its length must match the field type.
  ///
  /// # Errors
  ///
  /// returns [`LuceneError::IllegalArgument`] if value is invalid
  pub fn set_vector_value(&mut self, value: Vec<u8>) -> Result<()> {
    if value.len() != self.parent_field.field_type().vector_dimension() as usize {
      return Err(LuceneError::illegal_argument(format!(
        "value length {} must match field dimension {}",
        value.len(),
        self.parent_field.field_type().vector_dimension()
      )));
    }
    let v: VectorValueEnum = value.into();
    self.parent_field.fields_data = v.into();
    Ok(())
  }
}

impl FieldBase for KnnByteVectorField {}

impl Display for KnnByteVectorField {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    self.parent_field.fmt(f)
  }
}

impl IndexableField for KnnByteVectorField {
  fn name(&self) -> &str {
    self.parent_field.name()
  }

  type FieldType<'a>
    = &'a FieldType
  where
    Self: 'a;

  fn field_type(&self) -> Self::FieldType<'_> {
    self.parent_field.field_type()
  }
  fn token_stream<'a, A>(
    &'a mut self,
    analyzer: &'a A,
    reuse_token_stream: &'a mut Option<ReusedIndexingTokenStream>,
  ) -> Result<IndexingTokenStream<'a>>
  where
    A: Analyzer,
  {
    self.parent_field.token_stream(analyzer, reuse_token_stream)
  }

  fn binary_value(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    self.parent_field.binary_value()
  }

  fn take_binary_value(&mut self) -> Result<Option<BytesRef<Vec<u8>>>> {
    self.binary_value().map(|v| v.map(|c| c.into_owned()))
  }

  fn string_value(&self) -> Result<Option<Cow<'_, String>>> {
    self.parent_field.string_value()
  }

  fn take_string_value(&mut self) -> Result<Option<String>> {
    self.parent_field.take_string_value()
  }

  fn take_reader_value(&mut self) -> Result<Option<ReaderEnum>> {
    self.parent_field.take_reader_value()
  }

  fn numeric_value(&self) -> Result<Option<Number>> {
    self.parent_field.numeric_value()
  }

  fn stored_value(&self) -> Option<FieldDataEnum> {
    self.parent_field.stored_value()
  }

  fn invertable_type(&self) -> &InvertableType {
    self.parent_field.invertable_type()
  }
  fn vector_value(&self) -> Result<&VectorValueEnum> {
    self.parent_field.vector_value()
  }
}
#[cfg(test)]
impl Clone for KnnByteVectorField {
  fn clone(&self) -> Self {
    Self {
      parent_field: self.parent_field.clone(),
    }
  }
}
