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
use crate::util::error::lucene_error::Result;
use crate::util::hnsw::random_vector_scorer::RandomVectorScorer;

/// A supplier that creates  [`RandomVectorScorer`] from an ordinal.
pub trait RandomVectorScorerSupplier {
    type Scorer: RandomVectorScorer;
    /// This creates a [`RandomVectorScorer`] for scoring random nodes in
    /// batches against the given ordinal.
    ///
    /// # Arguments
    ///
    /// * `ord` - The ordinal of the node to compare.
    ///
    /// # Returns
    ///
    /// A new [`RandomVectorScorer`].
    fn scorer(&self, ord: i32) -> Result<Self::Scorer>;

    /// Make a copy of the supplier, which will copy the underlying
    /// `vectorValues` so the copy is safe to be used in other threads.
    fn copy(&self) -> Result<Self>
    where
        Self: Sized;
}
