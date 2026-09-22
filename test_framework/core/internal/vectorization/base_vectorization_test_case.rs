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
use crate::core::internal::vectorization::default_vectorization_provider::DefaultVectorizationProvider;
use crate::core::internal::vectorization::panama_vector_util_support::PanamaVectorUtilSupport;

/// Shared base for tests comparing the default and explicit vector implementations.
///
/// Unlike Java's runtime provider lookup, both implementations are selected
/// explicitly, so absence of a JVM vector module cannot skip these tests.
/// The complete Panama provider is not assembled yet; only its mathematical
/// support is selected here, without changing the production provider.
pub trait BaseVectorizationTestCase {
  const LUCENE_PROVIDER: DefaultVectorizationProvider = DefaultVectorizationProvider;
  const PANAMA_SUPPORT: PanamaVectorUtilSupport = PanamaVectorUtilSupport;
}
