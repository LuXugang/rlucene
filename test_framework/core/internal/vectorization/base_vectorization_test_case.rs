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
use crate::test_framework::core::internal::vectorization::reference_vector_util_support::ReferenceVectorUtilSupport;

/// Shared base for comparing the production provider with independent reference calculations.
///
/// The scalar reference is test-only. Floating-point reference calculations use
/// f64 intermediates, and the concrete tests retain their absolute tolerance.
pub trait BaseVectorizationTestCase {
  const LUCENE_PROVIDER: DefaultVectorizationProvider = DefaultVectorizationProvider;
  const REFERENCE_SUPPORT: ReferenceVectorUtilSupport = ReferenceVectorUtilSupport;
}
