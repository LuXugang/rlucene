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
use crate::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::index::doc_values_type::DocValuesType;
use crate::index::field_info::FieldInfo;
use crate::index::index_options::IndexOptions;
use crate::index::vector_encoding::VectorEncoding;
use crate::index::vector_similarity_function::VectorSimilarityFunction;
use crate::util::access::Access;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::int_block_pool::{ibp_util, AllocatorI32};
use crate::util::{Counter, CounterEnum};
use std::collections::HashMap;
use std::fmt::Display;

#[allow(unused)]
pub(crate) struct IndexingChain;

pub struct IntBlockAllocator<C>
where
    C: Access<CounterEnum>,
{
    block_size: usize,
    pub(crate) byte_used: C,
}
impl<C> IntBlockAllocator<C>
where
    C: Access<CounterEnum>,
{
    pub fn new(byte_used: C) -> Self {
        IntBlockAllocator {
            block_size: ibp_util::INT_BLOCK_SIZE as usize,
            byte_used,
        }
    }
}
impl<C> AllocatorI32 for IntBlockAllocator<C>
where
    C: Access<CounterEnum>,
{
    fn recycle_int_blocks(&mut self, _blocks: &[Vec<i32>], _offset: usize, length: usize) {
        self.byte_used.access_mut(|byte_used| {
            let delta = length as i64 * (self.block_size as i64 * BitUtil::INT_BYTES as i64);
            byte_used.add_and_get(-delta);
        });
    }

    fn get_byte_block(&mut self) -> Vec<i32> {
        let b = vec![0; ibp_util::INT_BLOCK_SIZE as usize];
        self.byte_used.access_mut(|byte_used| {
            byte_used.add_and_get(ibp_util::INT_BLOCK_SIZE as i64 * BitUtil::INT_BYTES as i64);
        });
        b
    }

    fn get_block_size(&self) -> usize {
        self.block_size
    }
}

/// A schema of the field in the current document. With every new document this schema is reset.
/// As the document’s fields are processed, we update the schema with any options encountered in
/// this document. Once processing for the document is complete, we compare the built schema of
/// the current document with the corresponding `FieldInfo` (constructed from the first document
/// in the segment where this field appeared). If there is any inconsistency, we return an error.
/// This ensures that a field’s data structures remain consistent across all documents.
pub(crate) struct FieldSchema {
    name: String,
    doc_id: i32,
    attributes: HashMap<String, String>,
    omit_norms: bool,
    store_term_vector: bool,
    index_options: IndexOptions,
    doc_values_type: DocValuesType,
    doc_values_skip_index: DocValuesSkipIndexType,
    point_dimension_count: i32,
    point_index_dimension_count: i32,
    point_num_bytes: i32,
    vector_dimension: i32,
    vector_encoding: VectorEncoding,
    vector_similarity_function: VectorSimilarityFunction,
}
impl FieldSchema {
    const ERR_MSG: &'static str =
        "Inconsistency of field data structures across documents for field ";
    pub(crate) fn new(name: &str) -> Self {
        FieldSchema {
            name: name.to_string(),
            doc_id: 0,
            attributes: HashMap::new(),
            omit_norms: false,
            store_term_vector: false,
            index_options: IndexOptions::None,
            doc_values_type: DocValuesType::None,
            doc_values_skip_index: DocValuesSkipIndexType::None,
            point_dimension_count: 0,
            point_index_dimension_count: 0,
            point_num_bytes: 0,
            vector_dimension: 0,
            vector_encoding: VectorEncoding::FLOAT32(4),
            vector_similarity_function: VectorSimilarityFunction::Euclidean,
        }
    }
    pub(crate) fn assert_same<T>(&self, label: &str, expected: &T, given: &T) -> Result<()>
    where
        T: PartialEq + Display,
    {
        if expected != given {
            return Err(LuceneError::illegal_argument(format!(
                "{}[{}] of doc [{}]. {}: expected '{}', but it has '{}'.",
                Self::ERR_MSG,
                self.name,
                self.doc_id,
                label,
                expected,
                given
            )));
        }
        Ok(())
    }
    pub(crate) fn update_attributes(&mut self, attrs: HashMap<String, String>) {
        self.attributes.extend(attrs);
    }

    pub(crate) fn set_index_options(
        &mut self,
        new_index_options: IndexOptions,
        new_omit_norms: bool,
        new_store_term_vector: bool,
    ) -> Result<()> {
        if self.index_options == IndexOptions::None {
            self.index_options = new_index_options;
            self.omit_norms = new_omit_norms;
            self.store_term_vector = new_store_term_vector;
        } else {
            self.assert_same("index options", &self.index_options, &new_index_options)?;
            self.assert_same("omit norms", &self.omit_norms, &new_omit_norms)?;
            self.assert_same(
                "store term vector",
                &self.store_term_vector,
                &new_store_term_vector,
            )?;
        }
        Ok(())
    }
    pub(crate) fn set_doc_values(
        &mut self,
        new_doc_values_type: DocValuesType,
        new_doc_values_skip_index: DocValuesSkipIndexType,
    ) -> Result<()> {
        if self.doc_values_type == DocValuesType::None {
            self.doc_values_type = new_doc_values_type;
            self.doc_values_skip_index = new_doc_values_skip_index;
        } else {
            self.assert_same(
                "doc values type",
                &self.doc_values_type,
                &new_doc_values_type,
            )?;
            self.assert_same(
                "doc values skip index type",
                &self.doc_values_skip_index,
                &new_doc_values_skip_index,
            )?;
        }
        Ok(())
    }

    pub(crate) fn set_points(
        &mut self,
        dimension_count: i32,
        index_dimension_count: i32,
        num_bytes: i32,
    ) -> Result<()> {
        if self.point_index_dimension_count == 0 {
            self.point_dimension_count = dimension_count;
            self.point_index_dimension_count = index_dimension_count;
            self.point_num_bytes = num_bytes;
        } else {
            self.assert_same(
                "point dimension",
                &self.point_dimension_count,
                &dimension_count,
            )?;
            self.assert_same(
                "point index dimension",
                &self.point_index_dimension_count,
                &index_dimension_count,
            )?;
            self.assert_same("point num bytes", &self.point_num_bytes, &num_bytes)?;
        }
        Ok(())
    }

    pub(crate) fn set_vectors(
        &mut self,
        encoding: VectorEncoding,
        similarity_function: VectorSimilarityFunction,
        dimension: i32,
    ) -> Result<()> {
        if self.vector_dimension == 0 {
            self.vector_encoding = encoding;
            self.vector_similarity_function = similarity_function;
            self.vector_dimension = dimension;
        } else {
            self.assert_same("vector encoding", &self.vector_encoding, &encoding)?;
            self.assert_same(
                "vector similarity function",
                &self.vector_similarity_function,
                &similarity_function,
            )?;
            self.assert_same("vector dimension", &self.vector_dimension, &dimension)?;
        }
        Ok(())
    }
    pub(crate) fn reset(&mut self, doc: i32) {
        self.doc_id = doc;
        self.omit_norms = false;
        self.store_term_vector = false;
        self.index_options = IndexOptions::None;
        self.doc_values_type = DocValuesType::None;
        self.doc_values_skip_index = DocValuesSkipIndexType::None;
        self.point_dimension_count = 0;
        self.point_index_dimension_count = 0;
        self.point_num_bytes = 0;
        self.vector_dimension = 0;
        self.vector_encoding = VectorEncoding::FLOAT32(4);
        self.vector_similarity_function = VectorSimilarityFunction::Euclidean;
    }

    pub(crate) fn assert_same_schema(&self, fi: &FieldInfo) -> Result<()> {
        self.assert_same("index options", fi.get_index_options(), &self.index_options)?;
        self.assert_same("omit norms", &fi.omits_norms(), &self.omit_norms)?;
        self.assert_same(
            "store term vector",
            &fi.has_term_vectors(),
            &self.store_term_vector,
        )?;
        self.assert_same(
            "doc values type",
            fi.get_doc_values_type(),
            &self.doc_values_type,
        )?;
        self.assert_same(
            "doc values skip index type",
            fi.doc_values_skip_index_type(),
            &self.doc_values_skip_index,
        )?;
        self.assert_same(
            "vector similarity function",
            fi.get_vector_similarity_function(),
            &self.vector_similarity_function,
        )?;
        self.assert_same(
            "vector encoding",
            fi.get_vector_encoding(),
            &self.vector_encoding,
        )?;
        self.assert_same(
            "vector dimension",
            &fi.get_vector_dimension(),
            &self.vector_dimension,
        )?;
        self.assert_same(
            "point dimension",
            &fi.get_point_dimension_count(),
            &self.point_dimension_count,
        )?;
        self.assert_same(
            "point index dimension",
            &fi.get_point_index_dimension_count(),
            &self.point_index_dimension_count,
        )?;
        self.assert_same(
            "point num bytes",
            &fi.get_point_num_bytes(),
            &self.point_num_bytes,
        )?;
        Ok(())
    }
}
