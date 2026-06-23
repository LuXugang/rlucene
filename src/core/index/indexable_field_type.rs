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
use std::collections::HashMap;

use crate::core::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;

/// Describes the properties of a field.
///
/// # Experimental
pub trait IndexableFieldType {
  /// Returns true if the field's value should be stored.
  fn stored(&self) -> bool;

  /// Returns true if this field's value should be analyzed by the Analyzer.
  ///
  /// This has no effect if `index_options()` returns IndexOptions::None.
  fn tokenized(&self) -> bool;

  /// Returns true if this field's indexed form should also be stored into
  /// term vectors.
  ///
  /// This builds a miniature inverted-index for this field which can be
  /// accessed in a document-oriented way.
  ///
  /// This option is illegal if `index_options()` returns IndexOptions::None.
  fn store_term_vectors(&self) -> bool;

  /// Returns true if this field's token character offsets should also be
  /// stored into term vectors.
  ///
  /// This option is illegal if term vectors are not enabled for the field
  /// (`store_term_vectors()` is false).
  fn store_term_vector_offsets(&self) -> bool;

  /// Returns true if this field's token positions should also be stored into
  /// term vectors.
  ///
  /// This option is illegal if term vectors are not enabled for the field
  /// (`store_term_vectors()` is false).
  fn store_term_vector_positions(&self) -> bool;

  /// Returns true if this field's token payloads should also be stored into
  /// term vectors.
  ///
  /// This option is illegal if term vector positions are not enabled for the
  /// field (`store_term_vectors()` is false).
  fn store_term_vector_payloads(&self) -> bool;

  /// Returns true if normalization values should be omitted for the field.
  ///
  /// Omitting norms saves memory, but at the expense of scoring quality
  /// (length normalization will be disabled), and if you omit norms, you
  /// cannot use index-time boosts.
  fn omit_norms(&self) -> bool;

  /// Returns the IndexOptions, describing what should be recorded into the
  /// inverted index.
  fn index_options(&self) -> &IndexOptions;

  /// Returns the DocValuesType: how the field's value will be indexed into
  /// docValues.
  fn doc_values_type(&self) -> &DocValuesType;

  /// Returns the DocValuesSkipIndexType, indicating whether a skip index for
  /// doc values should be created on this field.
  fn doc_values_skip_index_type(&self) -> &DocValuesSkipIndexType;

  /// Returns the number of point dimensions if positive, indicating the field
  /// is indexed as a point.
  fn point_dimension_count(&self) -> usize;

  /// Returns the number of dimensions used for the index key.
  fn point_index_dimension_count(&self) -> usize;

  /// Returns the number of bytes in each dimension's values.
  fn point_num_bytes(&self) -> usize;

  /// Returns the number of dimensions of the field's vector value.
  fn vector_dimension(&self) -> i32;

  /// Returns the VectorEncoding of the field's vector value.
  fn vector_encoding(&self) -> &VectorEncoding;

  /// Returns the VectorSimilarityFunction of the field's vector value.
  fn vector_similarity_function(&self) -> &VectorSimilarityFunction;

  /// Returns the attributes for the field type.
  fn get_attributes(&self) -> Option<&HashMap<String, String>>;
}

impl<T> IndexableFieldType for &T
where
  T: IndexableFieldType,
{
  fn stored(&self) -> bool {
    (**self).stored()
  }

  fn tokenized(&self) -> bool {
    (**self).tokenized()
  }

  fn store_term_vectors(&self) -> bool {
    (**self).store_term_vectors()
  }

  fn store_term_vector_offsets(&self) -> bool {
    (**self).store_term_vector_offsets()
  }

  fn store_term_vector_positions(&self) -> bool {
    (**self).store_term_vector_positions()
  }

  fn store_term_vector_payloads(&self) -> bool {
    (**self).store_term_vector_payloads()
  }

  fn omit_norms(&self) -> bool {
    (**self).omit_norms()
  }

  fn index_options(&self) -> &IndexOptions {
    (**self).index_options()
  }

  fn doc_values_type(&self) -> &DocValuesType {
    (**self).doc_values_type()
  }

  fn doc_values_skip_index_type(&self) -> &DocValuesSkipIndexType {
    (**self).doc_values_skip_index_type()
  }

  fn point_dimension_count(&self) -> usize {
    (**self).point_dimension_count()
  }

  fn point_index_dimension_count(&self) -> usize {
    (**self).point_index_dimension_count()
  }

  fn point_num_bytes(&self) -> usize {
    (**self).point_num_bytes()
  }

  fn vector_dimension(&self) -> i32 {
    (**self).vector_dimension()
  }

  fn vector_encoding(&self) -> &VectorEncoding {
    (**self).vector_encoding()
  }

  fn vector_similarity_function(&self) -> &VectorSimilarityFunction {
    (**self).vector_similarity_function()
  }

  fn get_attributes(&self) -> Option<&HashMap<String, String>> {
    (**self).get_attributes()
  }
}

pub enum IndexableFieldTypeEnum<'a> {
  FieldType(&'a crate::core::document::field_type::FieldType),
  Custom(&'a dyn IndexableFieldType),
}

impl<'a> From<&'a crate::core::document::field_type::FieldType> for IndexableFieldTypeEnum<'a> {
  fn from(field_type: &'a crate::core::document::field_type::FieldType) -> Self {
    Self::FieldType(field_type)
  }
}

impl IndexableFieldType for IndexableFieldTypeEnum<'_> {
  fn stored(&self) -> bool {
    match self {
      Self::FieldType(field_type) => field_type.stored(),
      Self::Custom(field_type) => field_type.stored(),
    }
  }

  fn tokenized(&self) -> bool {
    match self {
      Self::FieldType(field_type) => field_type.tokenized(),
      Self::Custom(field_type) => field_type.tokenized(),
    }
  }

  fn store_term_vectors(&self) -> bool {
    match self {
      Self::FieldType(field_type) => field_type.store_term_vectors(),
      Self::Custom(field_type) => field_type.store_term_vectors(),
    }
  }

  fn store_term_vector_offsets(&self) -> bool {
    match self {
      Self::FieldType(field_type) => field_type.store_term_vector_offsets(),
      Self::Custom(field_type) => field_type.store_term_vector_offsets(),
    }
  }

  fn store_term_vector_positions(&self) -> bool {
    match self {
      Self::FieldType(field_type) => field_type.store_term_vector_positions(),
      Self::Custom(field_type) => field_type.store_term_vector_positions(),
    }
  }

  fn store_term_vector_payloads(&self) -> bool {
    match self {
      Self::FieldType(field_type) => field_type.store_term_vector_payloads(),
      Self::Custom(field_type) => field_type.store_term_vector_payloads(),
    }
  }

  fn omit_norms(&self) -> bool {
    match self {
      Self::FieldType(field_type) => field_type.omit_norms(),
      Self::Custom(field_type) => field_type.omit_norms(),
    }
  }

  fn index_options(&self) -> &IndexOptions {
    match self {
      Self::FieldType(field_type) => field_type.index_options(),
      Self::Custom(field_type) => field_type.index_options(),
    }
  }

  fn doc_values_type(&self) -> &DocValuesType {
    match self {
      Self::FieldType(field_type) => field_type.doc_values_type(),
      Self::Custom(field_type) => field_type.doc_values_type(),
    }
  }

  fn doc_values_skip_index_type(&self) -> &DocValuesSkipIndexType {
    match self {
      Self::FieldType(field_type) => field_type.doc_values_skip_index_type(),
      Self::Custom(field_type) => field_type.doc_values_skip_index_type(),
    }
  }

  fn point_dimension_count(&self) -> usize {
    match self {
      Self::FieldType(field_type) => field_type.point_dimension_count(),
      Self::Custom(field_type) => field_type.point_dimension_count(),
    }
  }

  fn point_index_dimension_count(&self) -> usize {
    match self {
      Self::FieldType(field_type) => field_type.point_index_dimension_count(),
      Self::Custom(field_type) => field_type.point_index_dimension_count(),
    }
  }

  fn point_num_bytes(&self) -> usize {
    match self {
      Self::FieldType(field_type) => field_type.point_num_bytes(),
      Self::Custom(field_type) => field_type.point_num_bytes(),
    }
  }

  fn vector_dimension(&self) -> i32 {
    match self {
      Self::FieldType(field_type) => field_type.vector_dimension(),
      Self::Custom(field_type) => field_type.vector_dimension(),
    }
  }

  fn vector_encoding(&self) -> &VectorEncoding {
    match self {
      Self::FieldType(field_type) => field_type.vector_encoding(),
      Self::Custom(field_type) => field_type.vector_encoding(),
    }
  }

  fn vector_similarity_function(&self) -> &VectorSimilarityFunction {
    match self {
      Self::FieldType(field_type) => field_type.vector_similarity_function(),
      Self::Custom(field_type) => field_type.vector_similarity_function(),
    }
  }

  fn get_attributes(&self) -> Option<&HashMap<String, String>> {
    match self {
      Self::FieldType(field_type) => field_type.get_attributes(),
      Self::Custom(field_type) => field_type.get_attributes(),
    }
  }
}
