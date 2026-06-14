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
use crate::core::index::field_invert_state::FieldInvertState;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::term_statistics::TermStatistics;
use crate::core::util::error::lucene_error::Result;
/// Provides the ability to use a different [`Similarity`] for different fields.
///
/// Implementations should implement [`Self::get`] to return an appropriate [`Similarity`] for the field.
/// (for example, using field-specific parameter values) for the field.
pub trait PerFieldSimilarityWrapper: Similarity {
  fn compute_norm(&self, state: &FieldInvertState) -> Result<i64> {
    self.get(state.get_name()).compute_norm(state)
  }

  fn scorer(
    &self,
    boost: f32,
    collection_stats: &CollectionStatistics,
    term_stats: &[TermStatistics],
  ) -> Result<Self::SimScorer> {
    self
      .get(collection_stats.get_field())
      .scorer(boost, collection_stats, term_stats)
  }
  type Similarity: Similarity<SimScorer = Self::SimScorer>;
  /// Returns a Similarity for scoring a field.
  fn get(&self, name: &str) -> Self::Similarity;
}
