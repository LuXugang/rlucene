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
use std::fmt;
use std::hash::{Hash, Hasher};

use crate::core::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::point_values::{MAX_DIMENSIONS, MAX_INDEX_DIMENSIONS, MAX_NUM_BYTES};
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Describes the properties of a field.
#[derive(Clone, Debug)]
pub struct FieldType {
    stored: bool,
    tokenized: bool,
    store_term_vectors: bool,
    store_term_vector_offsets: bool,
    store_term_vector_positions: bool,
    store_term_vector_payloads: bool,
    omit_norms: bool,
    index_options: IndexOptions,
    frozen: bool,
    doc_values_type: DocValuesType,
    doc_values_skip_index: DocValuesSkipIndexType,
    dimension_count: usize,
    index_dimension_count: usize,
    dimension_num_bytes: usize,
    vector_dimension: i32,
    vector_encoding: VectorEncoding,
    vector_similarity_function: VectorSimilarityFunction,
    attributes: Option<HashMap<String, String>>,
}

impl Default for FieldType {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldType {
    /// Creates a new FieldType with default properties.
    pub fn new() -> Self {
        Self {
            stored: false,
            tokenized: true,
            store_term_vectors: false,
            store_term_vector_offsets: false,
            store_term_vector_positions: false,
            store_term_vector_payloads: false,
            omit_norms: false,
            index_options: IndexOptions::None,
            frozen: false,
            doc_values_type: DocValuesType::None,
            doc_values_skip_index: DocValuesSkipIndexType::None,
            dimension_count: 0,
            index_dimension_count: 0,
            dimension_num_bytes: 0,
            vector_dimension: 0,
            vector_encoding: VectorEncoding::FLOAT32(4),
            vector_similarity_function: VectorSimilarityFunction::Euclidean,
            attributes: None,
        }
    }

    /// Creates a new mutable FieldType with all of the properties from
    /// `ref_field`.
    pub fn from_ref(ref_field: &impl IndexableFieldType) -> Result<Self> {
        // Copy attributes if available; otherwise use an empty map.
        let attributes = ref_field.get_attributes().cloned();

        Ok(Self {
            stored: ref_field.stored(),
            tokenized: ref_field.tokenized(),
            store_term_vectors: ref_field.store_term_vectors(),
            store_term_vector_offsets: ref_field.store_term_vector_offsets(),
            store_term_vector_positions: ref_field.store_term_vector_positions(),
            store_term_vector_payloads: ref_field.store_term_vector_payloads(),
            omit_norms: ref_field.omit_norms(),
            index_options: *ref_field.index_options(),
            frozen: false,
            doc_values_type: *ref_field.doc_values_type(),
            doc_values_skip_index: *ref_field.doc_values_skip_index_type(),
            dimension_count: ref_field.point_dimension_count(),
            index_dimension_count: ref_field.point_index_dimension_count(),
            dimension_num_bytes: ref_field.point_num_bytes(),
            vector_dimension: ref_field.vector_dimension(),
            vector_encoding: *ref_field.vector_encoding(),
            vector_similarity_function: *ref_field.vector_similarity_function(),
            attributes,
        })
    }
    /// Throws an error if this FieldType is frozen.
    /// SubStruct should call this within setters for additional state.
    pub fn check_if_frozen(&self) -> Result<()> {
        if self.frozen {
            return Err(LuceneError::illegal_state(
                "this FieldType is already frozen and cannot be changed",
            ));
        }
        Ok(())
    }
    /// Prevents future changes.
    /// It is recommended that this is called once the FieldType's properties
    /// have been set, to prevent unintentional state changes.
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    /// Set to true to store this field.
    ///
    /// # Error
    ///
    /// Error if this FieldType is frozen against future modifications.
    pub fn set_stored(&mut self, value: bool) -> Result<()> {
        self.check_if_frozen()?;
        self.stored = value;
        Ok(())
    }

    /// Set to true to tokenize this field's contents via the configured
    /// Analyzer.
    ///
    /// # Error
    ///
    /// Error if this FieldType is frozen against future modifications.
    pub fn set_tokenized(&mut self, value: bool) -> Result<()> {
        self.check_if_frozen()?;
        self.tokenized = value;
        Ok(())
    }

    /// Set to true if this field should store term vectors.
    ///
    /// # Error
    ///
    /// Error if this FieldType is frozen against future modifications.
    pub fn set_store_term_vectors(&mut self, value: bool) -> Result<()> {
        self.check_if_frozen()?;
        self.store_term_vectors = value;
        Ok(())
    }
    /// Set to true to also store token character offsets into the term vector
    /// for this field.
    ///
    /// # Error
    ///
    /// Error if this FieldType is frozen against future modifications.
    pub fn set_store_term_vector_offsets(&mut self, value: bool) -> Result<()> {
        self.check_if_frozen()?;
        self.store_term_vector_offsets = value;
        Ok(())
    }

    /// Set to true to also store token positions into the term vector for this
    /// field.
    ///
    /// # Error
    ///
    /// Error if this FieldType is frozen against future modifications.
    pub fn set_store_term_vector_positions(&mut self, value: bool) -> Result<()> {
        self.check_if_frozen()?;
        self.store_term_vector_positions = value;
        Ok(())
    }
    /// Set to true to also store token payloads into the term vector for this
    /// field.
    ///
    /// # Error
    ///
    /// Error if this FieldType is frozen against future modifications.
    pub fn set_store_term_vector_payloads(&mut self, value: bool) -> Result<()> {
        self.check_if_frozen()?;
        self.store_term_vector_payloads = value;
        Ok(())
    }
    /// Set to true to omit normalization values for the field.
    ///
    /// # Error
    ///
    /// Error if this FieldType is frozen against future modifications.
    pub fn set_omit_norms(&mut self, value: bool) -> Result<()> {
        self.check_if_frozen()?;
        self.omit_norms = value;
        Ok(())
    }
    /// Sets the indexing options for the field.
    ///
    /// # Error
    ///
    /// Error if this FieldType is frozen against future modifications or if the
    /// provided value is invalid.
    pub fn set_index_options(&mut self, value: IndexOptions) -> Result<()> {
        self.check_if_frozen()?;
        self.index_options = value;
        Ok(())
    }

    /// Enables points indexing.
    pub fn set_dimensions(
        &mut self,
        dimension_count: usize,
        dimension_num_bytes: usize,
    ) -> Result<()> {
        self.set_dimensions_all(dimension_count, dimension_count, dimension_num_bytes)
    }

    /// Enables points indexing with selectable dimension indexing.
    pub fn set_dimensions_all(
        &mut self,
        dimension_count: usize,
        index_dimension_count: usize,
        dimension_num_bytes: usize,
    ) -> Result<()> {
        self.check_if_frozen()?;

        if dimension_count > MAX_DIMENSIONS {
            return Err(LuceneError::illegal_argument(format!(
                "dimensionCount must be <= {}; got {}",
                MAX_DIMENSIONS, dimension_count
            )));
        }

        if index_dimension_count > dimension_count {
            return Err(LuceneError::illegal_argument(format!(
                "indexDimensionCount must be <= dimensionCount: {dimension_count}; got {index_dimension_count}"
            )));
        }
        if index_dimension_count > MAX_INDEX_DIMENSIONS {
            return Err(LuceneError::illegal_argument(format!(
                "indexDimensionCount must be <= {}; got {}",
                MAX_INDEX_DIMENSIONS, index_dimension_count
            )));
        }

        if dimension_num_bytes > MAX_NUM_BYTES {
            return Err(LuceneError::illegal_argument(format!(
                "dimensionNumBytes must be <= {}; got {}",
                MAX_NUM_BYTES, dimension_num_bytes
            )));
        }
        if dimension_count == 0 {
            if index_dimension_count != 0 {
                return Err(LuceneError::illegal_argument(format!(
                    "when dimensionCount is 0, indexDimensionCount must be 0; got {index_dimension_count}"
                )));
            }
            if dimension_num_bytes != 0 {
                return Err(LuceneError::illegal_argument(format!(
                    "when dimensionCount is 0, dimensionNumBytes must be 0; got {dimension_num_bytes}"
                )));
            }
        } else if index_dimension_count == 0 {
            return Err(LuceneError::illegal_argument(format!(
                "when dimensionCount is > 0, indexDimensionCount must be > 0; got {index_dimension_count}"
            )));
        } else if dimension_num_bytes == 0 {
            return Err(LuceneError::illegal_argument(format!(
                "when dimensionNumBytes is 0, dimensionCount must be 0; got {dimension_count}"
            )));
        }

        self.dimension_count = dimension_count;
        self.index_dimension_count = index_dimension_count;
        self.dimension_num_bytes = dimension_num_bytes;
        Ok(())
    }

    /// Enables vector indexing, with the specified number of dimensions and
    /// distance function.
    pub fn set_vector_attributes(
        &mut self,
        num_dimensions: i32,
        encoding: VectorEncoding,
        similarity: VectorSimilarityFunction,
    ) -> Result<()> {
        self.check_if_frozen()?;
        if num_dimensions <= 0 {
            return Err(LuceneError::illegal_argument(format!(
                "vector numDimensions must be > 0; got {num_dimensions}"
            )));
        }
        self.vector_dimension = num_dimensions;
        self.vector_similarity_function = similarity;
        self.vector_encoding = encoding;
        Ok(())
    }

    /// Puts an attribute value.
    ///
    /// This is a key-value mapping for the field that the codec can use to
    /// store additional metadata.
    ///
    /// If a value already exists for the field, it will be replaced with the
    /// new value.
    pub fn put_attribute<T1, T2>(&mut self, key: T1, value: T2) -> Result<Option<String>>
    where
        T1: Into<String>,
        T2: Into<String>,
    {
        let key = key.into();
        let value = value.into();
        self.check_if_frozen()?;
        match self.attributes {
            Some(ref mut attrs) => Ok(attrs.insert(key, value)),
            None => {
                let mut attrs = HashMap::new();
                attrs.insert(key, value);
                self.attributes = Some(attrs);
                Ok(None)
            },
        }
    }

    /// Sets the field's DocValuesType.
    ///
    /// # Error
    ///
    /// Error if this FieldType is frozen against future modifications or if the
    /// provided type is invalid.
    pub fn set_doc_values_type(&mut self, doc_values_type: DocValuesType) -> Result<()> {
        self.check_if_frozen()?;
        self.doc_values_type = doc_values_type;
        Ok(())
    }
    /// Sets whether to enable a skip index for doc values on this field.
    ///
    /// This is typically useful on fields that are part of the index sort, or
    /// that correlate with fields that are part of the index sort,
    /// so that values can be expected to be clustered in the doc ID space.
    pub fn set_doc_values_skip_index_type(
        &mut self,
        skip_index: DocValuesSkipIndexType,
    ) -> Result<()> {
        self.check_if_frozen()?;
        self.doc_values_skip_index = skip_index;
        Ok(())
    }
}

impl IndexableFieldType for FieldType {
    /// Returns true if the field's value should be stored.
    fn stored(&self) -> bool {
        self.stored
    }

    /// Returns true if this field's value should be analyzed by the Analyzer.
    fn tokenized(&self) -> bool {
        self.tokenized
    }

    /// Returns true if this field's indexed form should also be stored into
    /// term vectors.
    fn store_term_vectors(&self) -> bool {
        self.store_term_vectors
    }

    /// Returns true if this field's token character offsets should also be
    /// stored into term vectors.
    fn store_term_vector_offsets(&self) -> bool {
        self.store_term_vector_offsets
    }

    /// Returns true if this field's token positions should also be stored into
    /// term vectors.
    fn store_term_vector_positions(&self) -> bool {
        self.store_term_vector_positions
    }

    /// Returns true if this field's token payloads should also be stored into
    /// term vectors.
    fn store_term_vector_payloads(&self) -> bool {
        self.store_term_vector_payloads
    }

    /// Returns true if normalization values should be omitted for the field.
    fn omit_norms(&self) -> bool {
        self.omit_norms
    }

    /// Returns the IndexOptions, describing what should be recorded into the
    /// inverted index.
    fn index_options(&self) -> &IndexOptions {
        &self.index_options
    }

    /// Returns the DocValuesType (the default is DocValuesType::None, meaning
    /// no docValues).
    fn doc_values_type(&self) -> &DocValuesType {
        &self.doc_values_type
    }

    /// Returns the DocValuesSkipIndexType.
    fn doc_values_skip_index_type(&self) -> &DocValuesSkipIndexType {
        &self.doc_values_skip_index
    }

    /// Returns the number of point dimensions.
    fn point_dimension_count(&self) -> usize {
        self.dimension_count
    }

    /// Returns the number of dimensions used for the index key.
    fn point_index_dimension_count(&self) -> usize {
        self.index_dimension_count
    }

    /// Returns the number of bytes in each dimension's values.
    fn point_num_bytes(&self) -> usize {
        self.dimension_num_bytes
    }

    /// Returns the number of dimensions of the field's vector value.
    fn vector_dimension(&self) -> i32 {
        self.vector_dimension
    }

    /// Returns the VectorEncoding of the field's vector value.
    fn vector_encoding(&self) -> &VectorEncoding {
        &self.vector_encoding
    }
    /// Returns the VectorSimilarityFunction of the field's vector value.
    fn vector_similarity_function(&self) -> &VectorSimilarityFunction {
        &self.vector_similarity_function
    }

    /// Returns the attributes for the field type.
    fn get_attributes(&self) -> Option<&HashMap<String, String>> {
        self.attributes.as_ref()
    }
}
impl PartialEq for FieldType {
    fn eq(&self, other: &Self) -> bool {
        self.dimension_count == other.dimension_count
            && self.index_dimension_count == other.index_dimension_count
            && self.dimension_num_bytes == other.dimension_num_bytes
            && self.doc_values_type == other.doc_values_type
            && self.doc_values_skip_index == other.doc_values_skip_index
            && self.index_options == other.index_options
            && self.omit_norms == other.omit_norms
            && self.store_term_vector_offsets == other.store_term_vector_offsets
            && self.store_term_vector_payloads == other.store_term_vector_payloads
            && self.store_term_vector_positions == other.store_term_vector_positions
            && self.store_term_vectors == other.store_term_vectors
            && self.stored == other.stored
            && self.tokenized == other.tokenized
    }
}

impl Eq for FieldType {}

impl Hash for FieldType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dimension_count.hash(state);
        self.index_dimension_count.hash(state);
        self.dimension_num_bytes.hash(state);
        self.doc_values_type.hash(state);
        self.doc_values_skip_index.hash(state);
        self.index_options.hash(state);
        self.omit_norms.hash(state);
        self.store_term_vector_offsets.hash(state);
        self.store_term_vector_payloads.hash(state);
        self.store_term_vector_positions.hash(state);
        self.store_term_vectors.hash(state);
        self.stored.hash(state);
        self.tokenized.hash(state);
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result = String::new();

        if self.stored() {
            result.push_str("stored");
        }

        if self.index_options != IndexOptions::None {
            if !result.is_empty() {
                result.push(',');
            }
            result.push_str("indexed");
            if self.tokenized() {
                result.push_str(",tokenized");
            }
            if self.store_term_vectors() {
                result.push_str(",termVector");
            }
            if self.store_term_vector_offsets() {
                result.push_str(",termVectorOffsets");
            }
            if self.store_term_vector_positions() {
                result.push_str(",termVectorPosition");
            }
            if self.store_term_vector_payloads() {
                result.push_str(",termVectorPayloads");
            }
            if self.omit_norms() {
                result.push_str(",omitNorms");
            }
            if self.index_options != IndexOptions::DocsAndFreqsAndPositions {
                result.push_str(",indexOptions=");
                result.push_str(&format!("{:?}", self.index_options));
            }
        }

        if self.dimension_count != 0 {
            if !result.is_empty() {
                result.push(',');
            }
            result.push_str("pointDimensionCount=");
            result.push_str(&self.dimension_count.to_string());
            result.push_str(",pointIndexDimensionCount=");
            result.push_str(&self.index_dimension_count.to_string());
            result.push_str(",pointNumBytes=");
            result.push_str(&self.dimension_num_bytes.to_string());
        }

        if self.doc_values_type != DocValuesType::None {
            if !result.is_empty() {
                result.push(',');
            }
            result.push_str("docValuesType=");
            result.push_str(&format!("{:?}", self.doc_values_type));
        }

        write!(f, "{result}")
    }
}
#[cfg(test)]
mod tests {
    use std::hash::{DefaultHasher, Hash, Hasher};

    use rand::Rng;

    use crate::core::document::field_type::FieldType;
    use crate::core::index::doc_values_type::DocValuesType;
    use crate::core::index::index_options::IndexOptions;
    use crate::core::index::point_values::{MAX_DIMENSIONS, MAX_INDEX_DIMENSIONS, MAX_NUM_BYTES};
    use crate::core::index::vector_encoding::VectorEncoding;
    use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::util::lucene_test_case::lucene_test_case_util::random;

    #[allow(dead_code)] // for quick search
    struct TestFieldType;
    #[test]
    fn test_equals() -> Result<()> {
        let ft = FieldType::new();
        assert_eq!(ft, ft);
        assert_ne!(Some(ft.clone()), None);

        let ft2 = FieldType::new();
        assert_eq!(ft, ft2);
        let mut hasher1 = DefaultHasher::new();
        ft.hash(&mut hasher1);
        let mut hasher2 = DefaultHasher::new();
        ft2.hash(&mut hasher2);
        assert_eq!(hasher1.finish(), hasher2.finish());

        let mut ft3 = FieldType::new();
        ft3.set_index_options(IndexOptions::DocsAndFreqs)?;
        assert_ne!(ft3, ft);

        let mut ft4 = FieldType::new();
        ft4.set_doc_values_type(DocValuesType::Binary)?;
        assert_ne!(ft4, ft);

        let mut ft5 = FieldType::new();
        ft5.set_index_options(IndexOptions::DocsAndFreqsAndPositions)?;
        assert_ne!(ft5, ft);

        let mut ft6 = FieldType::new();
        ft6.set_stored(true)?;
        assert_ne!(ft6, ft);

        let mut ft7 = FieldType::new();
        ft7.set_omit_norms(true)?;
        assert_ne!(ft7, ft);

        let mut ft10 = FieldType::new();
        ft10.set_store_term_vectors(true)?;
        assert_ne!(ft10, ft);

        let mut ft11 = FieldType::new();
        ft11.set_dimensions(1, 4)?;
        assert_ne!(ft11, ft);
        Ok(())
    }

    #[test]
    fn test_points_to_string() -> Result<()> {
        let mut ft = FieldType::new();
        ft.set_dimensions(1, MAX_NUM_BYTES)?;
        let expected = format!(
            "pointDimensionCount=1,pointIndexDimensionCount=1,pointNumBytes={}",
            MAX_NUM_BYTES
        );
        let s = ft.to_string();
        assert_eq!(s, expected);
        Ok(())
    }

    #[test]
    fn test_attribute_map_frozen() -> Result<()> {
        // FieldType#put_attribute no need to Implement, so as this test
        Ok(())
    }

    #[test]
    fn test_attribute_map_not_frozen() -> Result<()> {
        // FieldType#put_attribute no need to Implement, so as this test
        Ok(())
    }

    fn random_value_bool<R: Rng + ?Sized>(random: &mut R) -> bool {
        random.random_bool(0.5)
    }

    // Generates a random FieldType.
    fn random_field_type<R: Rng + ?Sized>(random: &mut R) -> Result<FieldType> {
        let mut ft = FieldType::new();
        let max_idx_dims = MAX_INDEX_DIMENSIONS;
        let max_dims = MAX_DIMENSIONS;
        let max_bytes = MAX_NUM_BYTES;
        let dim = random.random_range(1..=max_dims);
        let idx_dim = random.random_range(1..=max_idx_dims.min(dim));
        let num_bytes = random.random_range(1..=max_bytes);
        ft.set_dimensions_all(dim, idx_dim, num_bytes)?;
        ft.set_stored(random_value_bool(random))?;
        ft.set_tokenized(random_value_bool(random))?;
        ft.set_store_term_vectors(random_value_bool(random))?;
        ft.set_store_term_vector_offsets(random_value_bool(random))?;
        ft.set_store_term_vector_positions(random_value_bool(random))?;
        ft.set_store_term_vector_payloads(random_value_bool(random))?;
        ft.set_omit_norms(random_value_bool(random))?;
        let options = if random_value_bool(random) {
            IndexOptions::DocsAndFreqs
        } else {
            IndexOptions::DocsAndFreqsAndPositions
        };
        ft.set_index_options(options)?;
        let dv = if random_value_bool(random) {
            DocValuesType::Binary
        } else {
            DocValuesType::None
        };
        ft.set_doc_values_type(dv)?;

        if random_value_bool(random) {
            let vec_dim = random.random_range(1..=4);
            ft.set_vector_attributes(
                vec_dim,
                VectorEncoding::FLOAT32(4),
                VectorSimilarityFunction::Euclidean,
            )?;
        }
        // ft.put_attribute("random".to_string(), "value".to_string())?;
        Ok(ft)
    }

    #[test]
    fn test_copy_constructor() -> Result<()> {
        let mut random = random();
        let iters = 10;
        for _ in 0..iters {
            let ft = random_field_type(&mut random)?;
            let ft2 = FieldType::from_ref(&ft)?;
            assert_eq!(ft, ft2);
        }
        Ok(())
    }
}
