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
use crate::core::index::byte_vector_values::{ByteVectorValues, check_field};
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::{LRByteVectorValues, LeafReader};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::abstract_vector_similarity_query::{
  AbstractVectorSimilarityQuery, AbstractVectorSimilarityQueryBase,
};
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::knn::knn_collector_manager::KnnCollectorManager;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::top_docs::TopDocs;
use crate::core::util::HasIdentity;
use crate::core::util::bits::Bits;
#[cfg(test)]
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::hash::{Hash, Hasher};

/// Search for all approximate byte vectors above a similarity threshold.
#[derive(Clone, Debug)]
pub struct ByteVectorSimilarityQuery {
  base: AbstractVectorSimilarityQueryBase,
  target: Vec<u8>,
  id: Identity,
  #[cfg(test)]
  pub(crate) has_vector_scorer: bool,
}

impl ByteVectorSimilarityQuery {
  /// Searches for all approximate byte vectors above a similarity threshold.
  ///
  /// If a filter is applied, the search traverses as many nodes as the filter
  /// cost, and falls back to exact search if the approximate results are
  /// incomplete.
  ///
  /// # Arguments
  ///
  /// * `field` - a field that has been indexed as a [`KnnByteVectorField`](crate::core::document::knn_byte_vector_field::KnnByteVectorField).
  /// * `target` - the target vector of the search.
  /// * `traversal_similarity` - lower similarity score for graph traversal.
  /// * `result_similarity` - higher similarity score for result collection.
  /// * `filter` - a filter applied before the vector search.
  ///
  /// # Errors
  ///
  /// Returns an error if `traversal_similarity` is greater than
  /// `result_similarity`.
  pub fn with_traversal_similarity_and_filter<T>(
    field: T,
    target: Vec<u8>,
    traversal_similarity: f32,
    result_similarity: f32,
    filter: Option<Query>,
  ) -> Result<Self>
  where
    T: Into<String>,
  {
    let field = field.into();
    Ok(Self {
      base: AbstractVectorSimilarityQueryBase::new(
        field,
        traversal_similarity,
        result_similarity,
        filter,
      )?,
      target,
      id: Identity::new(),
      #[cfg(test)]
      has_vector_scorer: true,
    })
  }

  /// Searches for all approximate byte vectors above a similarity threshold.
  ///
  /// # Arguments
  ///
  /// * `field` - a field that has been indexed as a [`KnnByteVectorField`](crate::core::document::knn_byte_vector_field::KnnByteVectorField).
  /// * `target` - the target vector of the search.
  /// * `traversal_similarity` - lower similarity score for graph traversal.
  /// * `result_similarity` - higher similarity score for result collection.
  pub fn with_traversal_similarity<T>(
    field: T,
    target: Vec<u8>,
    traversal_similarity: f32,
    result_similarity: f32,
  ) -> Result<Self>
  where
    T: Into<String>,
  {
    Self::with_traversal_similarity_and_filter(
      field,
      target,
      traversal_similarity,
      result_similarity,
      None,
    )
  }

  /// Searches for all approximate byte vectors above a similarity threshold.
  ///
  /// If a filter is applied, the search traverses as many nodes as the filter
  /// cost, and falls back to exact search if the approximate results are
  /// incomplete.
  ///
  /// # Arguments
  ///
  /// * `field` - a field that has been indexed as a [`KnnByteVectorField`](crate::core::document::knn_byte_vector_field::KnnByteVectorField).
  /// * `target` - the target vector of the search.
  /// * `result_similarity` - similarity score for result collection.
  /// * `filter` - a filter applied before the vector search.
  pub fn with_filter<T>(
    field: T,
    target: Vec<u8>,
    result_similarity: f32,
    filter: Option<Query>,
  ) -> Result<Self>
  where
    T: Into<String>,
  {
    Self::with_traversal_similarity_and_filter(
      field,
      target,
      result_similarity,
      result_similarity,
      filter,
    )
  }

  /// Searches for all approximate byte vectors above a similarity threshold.
  ///
  /// # Arguments
  ///
  /// * `field` - a field that has been indexed as a [`KnnByteVectorField`](crate::core::document::knn_byte_vector_field::KnnByteVectorField).
  /// * `target` - the target vector of the search.
  /// * `result_similarity` - similarity score for result collection.
  pub fn new<T>(field: T, target: Vec<u8>, result_similarity: f32) -> Result<Self>
  where
    T: Into<String>,
  {
    Self::with_filter(field, target, result_similarity, None)
  }

  /// Returns a copy of the target query vector.
  pub fn get_target_copy(&self) -> Vec<u8> {
    self.target.clone()
  }
}

impl PartialEq for ByteVectorSimilarityQuery {
  fn eq(&self, other: &Self) -> bool {
    self.target == other.target && self.base == other.base
  }
}

impl Eq for ByteVectorSimilarityQuery {}

impl Hash for ByteVectorSimilarityQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.base.hash(state);
    self.target.hash(state);
  }
}

impl HasIdentity for ByteVectorSimilarityQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for ByteVectorSimilarityQuery {
  fn to_string(&self, _field: &str) -> Result<String> {
    let target = self
      .target
      .first()
      .map(|value| value.to_string())
      .unwrap_or_default();
    Ok(format!(
      "{}[field={} target=[{}...] traversal_similarity={} result_similarity={} filter={}]",
      std::any::type_name::<Self>().rsplit("::").next().unwrap(),
      self.base.field,
      target,
      self.base.traversal_similarity,
      self.base.result_similarity,
      match &self.base.filter {
        Some(filter) => filter.to_string("")?,
        None => "None".to_string(),
      }
    ))
  }

  fn create_weight<IRC>(
    self,
    searcher: &IndexSearcher<IRC>,
    _score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: crate::core::index::index_reader_context::IndexReaderContext,
    IndexSearcher<IRC>: Sync,
    Self: Sized,
  {
    AbstractVectorSimilarityQuery::create_weight(self, searcher, boost)
  }

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: crate::core::index::index_reader_context::IndexReaderContext,
    Self: Sized,
  {
    Ok(self.into())
  }

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    let query = self.into();
    if visitor.accept_field(&self.base.field) {
      visitor.visit_leaf(query)?;
    }
    Ok(())
  }
}

impl AbstractVectorSimilarityQuery for ByteVectorSimilarityQuery {
  fn base(&self) -> &AbstractVectorSimilarityQueryBase {
    &self.base
  }

  type VectorScorer<LR>
    = <LRByteVectorValues<LR> as ByteVectorValues>::VectorScorer
  where
    LR: LeafReader;

  fn create_vector_scorer<LR>(
    &self,
    context: &LeafReaderContext<LR>,
  ) -> Result<Option<Self::VectorScorer<LR>>>
  where
    LR: LeafReader,
  {
    #[cfg(test)]
    {
      if !self.has_vector_scorer {
        return Err(LuceneError::unsupported_operation(""));
      }
    }
    let reader = context.reader();
    let vector_values = match reader.get_byte_vector_values(&self.base.field)? {
      Some(vector_values) => vector_values,
      None => {
        check_field(reader, &self.base.field)?;
        return Ok(None);
      },
    };
    vector_values.scorer(self.target.clone())
  }

  fn approximate_search<LR, B, K>(
    &self,
    context: &LeafReaderContext<LR>,
    accept_docs: Option<B>,
    visit_limit: usize,
    knn_collector_manager: &K,
  ) -> Result<TopDocs<ScoreDoc>>
  where
    LR: LeafReader,
    B: Bits,
    K: KnnCollectorManager,
  {
    let mut collector = knn_collector_manager.new_collector(visit_limit, context)?;
    context.reader().search_nearest_vectors_u8(
      &self.base.field,
      self.target.clone(),
      &mut collector,
      accept_docs,
    )?;
    collector.top_docs()
  }
}

impl crate::core::util::accountable::Accountable for ByteVectorSimilarityQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}
