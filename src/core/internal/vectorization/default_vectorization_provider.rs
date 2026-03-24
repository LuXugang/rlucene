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
use crate::core::internal::vectorization::default_vector_util_support::DefaultVectorUtilSupport;
use crate::core::internal::vectorization::posting_decoding_util::PostingDecodingUtil;
use crate::core::internal::vectorization::vectorization_provider::VectorizationProvider;
use crate::core::store::IndexInput;

#[derive(Default)]
pub struct DefaultVectorizationProvider;
impl VectorizationProvider for DefaultVectorizationProvider {
  type VectorUtilSupport = DefaultVectorUtilSupport;

  fn get_vector_util_support(&self) -> Self::VectorUtilSupport {
    DefaultVectorUtilSupport
  }

  type FlatVectorsScorer = DefaultFlatVectorScorer;

  fn get_lucene99_flat_vectors_scorer(&self) -> Self::FlatVectorsScorer {
    DefaultFlatVectorScorer
  }

  fn new_posting_decoding_util<I: IndexInput>(&self, input: I) -> PostingDecodingUtil<I> {
    PostingDecodingUtil::new(input)
  }
}
