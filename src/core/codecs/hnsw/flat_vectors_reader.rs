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
use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
use crate::core::codecs::knn_vectors_reader::KnnVectorsReader;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::util::accountable::Accountable;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::random_vector_scorer::RandomVectorScorer;

/// Reads vectors from an index. When searching this reader, it iterates every vector in the index
/// and scores them
///
/// This class is useful when:
///
/// * the number of vectors is small
/// * when used along side some additional indexing structure that can be used to better search
///   the vectors (like HNSW).
pub trait FlatVectorsReader: KnnVectorsReader + Accountable {
  type FlatVectorsScorer: FlatVectorsScorer;

  /// @return the [`FlatVectorsScorer`] for this reader.
  fn get_flat_vector_scorer(&self) -> &Self::FlatVectorsScorer;

  fn search_float<B, K>(
    &self,
    _field: &str,
    _target: &[f32],
    _knn_collector: &mut K,
    _accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    // don't scan stored field data. If we didn't index it, produce no search results
    Ok(())
  }

  fn search_byte<B, K>(
    &self,
    _field: &str,
    _target: &[u8],
    _knn_collector: &mut K,
    _accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    // don't scan stored field data. If we didn't index it, produce no search results
    Ok(())
  }

  type RandomVectorScorer: RandomVectorScorer;
  /// Returns a [`RandomVectorScorer`] for the given field and target vector.
  ///
  /// # Arguments
  /// * `field` - the field to search
  /// * `target` - the target vector
  ///
  /// # Returns
  /// a [`RandomVectorScorer`] for the given field and target vector.
  ///
  /// # Errors
  /// if an I/O error occurs when reading from the index.
  fn get_random_vector_scorer_f32(
    &self,
    field: &str,
    target: &[f32],
  ) -> Result<Self::RandomVectorScorer>;

  /// Returns a [`RandomVectorScorer`] for the given field and target vector.
  ///
  /// # Arguments
  /// * `field` - the field to search
  /// * `target` - the target vector
  ///
  /// # Returns
  /// a [`RandomVectorScorer`] for the given field and target vector.
  ///
  /// # Errors
  /// if an I/O error occurs when reading from the index.
  fn get_random_vector_scorer_u8(
    &self,
    field: &str,
    target: &[u8],
  ) -> Result<Self::RandomVectorScorer>;

  /// Returns an instance optimized for merging. This instance may only be consumed in the thread
  /// that called `get_merge_instance`.
  ///
  /// The default implementation returns `self`
  fn get_merge_instance(&self) -> Result<Option<Self>>
  where
    Self: Sized,
  {
    Ok(None)
  }
}
