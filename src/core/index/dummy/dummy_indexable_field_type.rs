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
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;

pub struct DummyIndexableFieldType;
impl IndexableFieldType for DummyIndexableFieldType {
  fn stored(&self) -> bool {
    dummy_unreachable!()
  }

  fn tokenized(&self) -> bool {
    dummy_unreachable!()
  }

  fn store_term_vectors(&self) -> bool {
    dummy_unreachable!()
  }

  fn store_term_vector_offsets(&self) -> bool {
    dummy_unreachable!()
  }

  fn store_term_vector_positions(&self) -> bool {
    dummy_unreachable!()
  }

  fn store_term_vector_payloads(&self) -> bool {
    dummy_unreachable!()
  }

  fn omit_norms(&self) -> bool {
    dummy_unreachable!()
  }

  fn index_options(&self) -> &IndexOptions {
    dummy_unreachable!()
  }

  fn doc_values_type(&self) -> &DocValuesType {
    dummy_unreachable!()
  }

  fn doc_values_skip_index_type(&self) -> &DocValuesSkipIndexType {
    dummy_unreachable!()
  }

  fn point_dimension_count(&self) -> usize {
    dummy_unreachable!()
  }

  fn point_index_dimension_count(&self) -> usize {
    dummy_unreachable!()
  }

  fn point_num_bytes(&self) -> usize {
    dummy_unreachable!()
  }

  fn vector_dimension(&self) -> i32 {
    dummy_unreachable!()
  }

  fn vector_encoding(&self) -> &VectorEncoding {
    dummy_unreachable!()
  }

  fn vector_similarity_function(&self) -> &VectorSimilarityFunction {
    dummy_unreachable!()
  }

  fn get_attributes(&self) -> Option<&HashMap<String, String>> {
    dummy_unreachable!()
  }
}
