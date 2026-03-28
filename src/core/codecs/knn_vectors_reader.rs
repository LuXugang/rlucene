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
use crate::core::index::byte_vector_values::ByteVectorValues;
use crate::core::index::dummy::dummy_byte_vector_values::DummyByteVectorValues;
use crate::core::index::dummy::dummy_float_vector_values::DummyFloatVectorValues;
use crate::core::index::float_vector_values::FloatVectorValues;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
/// Reads vectors from an index.
pub trait KnnVectorsReader {
  /// Checks consistency of this reader.
  ///
  /// Note that this may be costly in terms of I/O, e.g. may involve computing a checksum value
  /// against large data files.
  fn check_integrity(&self) -> Result<()>;

  type FloatVectorValues: FloatVectorValues;
  /// Returns the [`FloatVectorValues`] for the given `field`. The behavior is undefined if
  /// the given field doesn't have KNN vectors enabled on its [`FieldInfo`]. The return value is
  /// never `None`.
  fn get_float_vector_values(&self, field: &str) -> Result<Self::FloatVectorValues>;

  type ByteVectorValues: ByteVectorValues;
  /// Returns the [`ByteVectorValues`] for the given `field`. The behavior is undefined if
  /// the given field doesn't have KNN vectors enabled on its [`FieldInfo`]. The return value is
  /// never `None`.
  fn get_byte_vector_values(&self, field: &str) -> Result<Self::ByteVectorValues>;

  /// Return the k nearest neighbor documents as determined by comparison of their vector values for
  /// this field, to the given vector, by the field's similarity function. The score of each document
  /// is derived from the vector similarity in a way that ensures scores are positive and that a
  /// larger score corresponds to a higher ranking.
  ///
  /// The search is allowed to be approximate, meaning the results are not guaranteed to be the
  /// true k closest neighbors. For large values of k (for example when k is close to the total
  /// number of documents), the search may also retrieve fewer than k documents.
  ///
  /// The returned [`TopDocs`] will contain a [`ScoreDoc`] for each nearest neighbor, in
  /// order of their similarity to the query vector (decreasing scores). The [`TotalHits`]
  /// contains the number of documents visited during the search. If the search stopped early because
  /// it hit `visitedLimit`, it is indicated through the relation
  /// `TotalHits.Relation.GREATER_THAN_OR_EQUAL_TO`.
  ///
  /// The behavior is undefined if the given field doesn't have KNN vectors enabled on its [`FieldInfo`].
  /// The return value is never `None`.
  ///
  /// # Arguments
  /// * `field` - the vector field to search
  /// * `target` - the vector-valued query
  /// * `knn_collector` - a KnnResults collector and relevant settings for gathering vector results
  /// * `accept_docs` - [`Bits`] that represents the allowed documents to match, or `None`
  ///   if they are all allowed to match.
  fn search_f32<B, K>(
    &self,
    field: &str,
    target: Vec<f32>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector;

  /// Return the k nearest neighbor documents as determined by comparison of their vector values for
  /// this field, to the given vector, by the field's similarity function. The score of each document
  /// is derived from the vector similarity in a way that ensures scores are positive and that a
  /// larger score corresponds to a higher ranking.
  ///
  /// The search is allowed to be approximate, meaning the results are not guaranteed to be the
  /// true k closest neighbors. For large values of k (for example when k is close to the total
  /// number of documents), the search may also retrieve fewer than k documents.
  ///
  /// The returned [`TopDocs`] will contain a [`ScoreDoc`] for each nearest neighbor, in
  /// order of their similarity to the query vector (decreasing scores). The [`TotalHits`]
  /// contains the number of documents visited during the search. If the search stopped early because
  /// it hit `visitedLimit`, it is indicated through the relation
  /// `TotalHits.Relation.GREATER_THAN_OR_EQUAL_TO`.
  ///
  /// The behavior is undefined if the given field doesn't have KNN vectors enabled on its [`FieldInfo`].
  /// The return value is never `None`.
  ///
  /// # Arguments
  /// * `field` - the vector field to search
  /// * `target` - the vector-valued query
  /// * `knn_collector` - a KnnResults collector and relevant settings for gathering vector results
  /// * `accept_docs` - [`Bits`] that represents the allowed documents to match, or `None`
  ///   if they are all allowed to match.
  fn search_u8<B, K>(
    &self,
    field: &str,
    target: Vec<u8>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector;

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

  /// Optional: reset or close merge resources used in the reader
  ///
  /// The default implementation is empty
  fn finish_merge(&mut self) -> Result<()> {
    Ok(())
  }
}
pub enum KnnVectorsReaderEnum {}
impl KnnVectorsReader for KnnVectorsReaderEnum {
  fn check_integrity(&self) -> Result<()> {
    todo!()
  }

  type FloatVectorValues = DummyFloatVectorValues;

  fn get_float_vector_values(&self, _field: &str) -> Result<Self::FloatVectorValues> {
    todo!()
  }

  type ByteVectorValues = DummyByteVectorValues;

  fn get_byte_vector_values(&self, _field: &str) -> Result<Self::ByteVectorValues> {
    todo!()
  }

  fn search_f32<B, K>(
    &self,
    _field: &str,
    _target: Vec<f32>,
    _knn_collector: &mut K,
    _accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    todo!()
  }

  fn search_u8<B, K>(
    &self,
    _field: &str,
    _target: Vec<u8>,
    _knn_collector: &mut K,
    _accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    todo!()
  }

  fn get_merge_instance(&self) -> Result<Option<Self>>
  where
    Self: Sized,
  {
    todo!()
  }

  fn finish_merge(&mut self) -> Result<()> {
    todo!()
  }
}
