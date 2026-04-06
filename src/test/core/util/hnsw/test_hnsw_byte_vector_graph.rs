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
use crate::core::codecs::hnsw::default_flat_vector_scorer::DefaultFlatVectorScorer;
use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
use crate::core::document::field::Field;
use crate::core::index::byte_vector_values::ByteVectorValues;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::index::knn_vector_values::KnnVectorValuesType;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::query::Query;
use crate::test::core::util::hnsw::hnsw_graph_test_case::{CircularByteVectorValues, HnswGraphTestCase};
use crate::test::core::util::hnsw::mock_byte_vector_values::MockByteVectorValues;

pub struct TestHnswByteVectorGraph;

impl HnswGraphTestCase<u8> for TestHnswByteVectorGraph {
    fn similarity_function(&self) -> VectorSimilarityFunction {
        todo!()
    }

    fn get_vector_encoding(&self) -> VectorEncoding {
        todo!()
    }

    fn knn_query(&self, field: &str, vector: u8, k: usize) -> Query {
        todo!()
    }

    fn random_vector(&self, dim: usize) -> u8 {
        todo!()
    }

    type KnnVectorValues = MockByteVectorValues;

    fn vector_values(&self, size: usize, dimension: usize) -> Self::KnnVectorValues {
        todo!()
    }

    fn vector_values_from_values(&self, values: Vec<Vec<f32>>) -> Self::KnnVectorValues {
        todo!()
    }

    fn vector_values_from_reader<LR>(&self, reader: &LR, field_name: &str) -> crate::core::util::error::lucene_error::Result<Self::KnnVectorValues>
    where
        LR: LeafReader
    {
        todo!()
    }

    fn vector_values_with_pregenerated(&self, size: usize, dimension: usize, pregenerated_vector_values: Self::KnnVectorValues, pregenerated_offset: usize) -> Self::KnnVectorValues {
        todo!()
    }

    fn knn_vector_field(&self, name: &str, vector: u8, similarity_function: VectorSimilarityFunction) -> crate::core::util::error::lucene_error::Result<Field> {
        todo!()
    }

    type CircularKnnVectorValues = CircularByteVectorValues;

    fn circular_vector_values(&self, n_doc: usize) -> Self::CircularKnnVectorValues {
        todo!()
    }


    fn get_target_vector(&self) -> u8 {
        todo!()
    }

}