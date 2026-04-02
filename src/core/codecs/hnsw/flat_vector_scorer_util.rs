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
use crate::core::internal::vectorization::vectorization_provider::{
  DEFAULT_VECTORIZATION_PROVIDER, VectorizationProvider,
};
use std::sync::LazyLock;

#[allow(dead_code)] // for quick search
pub struct FlatVectorScorerUtil;

pub static LUCENE99_FLAT_VECTORS_SCORER: LazyLock<DefaultFlatVectorScorer> =
  LazyLock::new(|| DEFAULT_VECTORIZATION_PROVIDER.get_lucene99_flat_vectors_scorer());
