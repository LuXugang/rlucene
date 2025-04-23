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
use std::sync::Arc;

use parking_lot::Mutex;

use crate::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::index::doc_values_type::DocValuesType;
use crate::index::index_options::IndexOptions;
use crate::index::indexable_field_type::IndexableFieldType;
use crate::index::vector_encoding::VectorEncoding;
use crate::index::vector_similarity_function::VectorSimilarityFunction;

pub struct DummyIndexableFieldType;
impl IndexableFieldType for DummyIndexableFieldType {
    fn stored(&self) -> bool {
        todo!()
    }

    fn tokenized(&self) -> bool {
        todo!()
    }

    fn store_term_vectors(&self) -> bool {
        todo!()
    }

    fn store_term_vector_offsets(&self) -> bool {
        todo!()
    }

    fn store_term_vector_positions(&self) -> bool {
        todo!()
    }

    fn store_term_vector_payloads(&self) -> bool {
        todo!()
    }

    fn omit_norms(&self) -> bool {
        todo!()
    }

    fn index_options(&self) -> &IndexOptions {
        todo!()
    }

    fn doc_values_type(&self) -> &DocValuesType {
        todo!()
    }

    fn doc_values_skip_index_type(&self) -> &DocValuesSkipIndexType {
        todo!()
    }

    fn point_dimension_count(&self) -> i32 {
        todo!()
    }

    fn point_index_dimension_count(&self) -> i32 {
        todo!()
    }

    fn point_num_bytes(&self) -> i32 {
        todo!()
    }

    fn vector_dimension(&self) -> i32 {
        todo!()
    }

    fn vector_encoding(&self) -> &VectorEncoding {
        todo!()
    }

    fn vector_similarity_function(&self) -> &VectorSimilarityFunction {
        todo!()
    }

    fn get_attributes(&self) -> Arc<Mutex<HashMap<String, String>>> {
        todo!()
    }
}
