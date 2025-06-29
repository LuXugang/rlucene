/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/
use std::collections::HashMap;

use crate::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::index::doc_values_type::DocValuesType;
use crate::index::index_options::IndexOptions;
use crate::index::indexable_field_type::IndexableFieldType;
use crate::index::vector_encoding::VectorEncoding;
use crate::index::vector_similarity_function::VectorSimilarityFunction;

pub struct DummyIndexableFieldType;
impl IndexableFieldType for DummyIndexableFieldType {
    fn stored(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn tokenized(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn store_term_vectors(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn store_term_vector_offsets(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn store_term_vector_positions(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn store_term_vector_payloads(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn omit_norms(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn index_options(&self) -> &IndexOptions {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn doc_values_type(&self) -> &DocValuesType {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn doc_values_skip_index_type(&self) -> &DocValuesSkipIndexType {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn point_dimension_count(&self) -> i32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn point_index_dimension_count(&self) -> i32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn point_num_bytes(&self) -> i32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn vector_dimension(&self) -> i32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn vector_encoding(&self) -> &VectorEncoding {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn vector_similarity_function(&self) -> &VectorSimilarityFunction {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_attributes(&self) -> Option<&HashMap<String, String>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
