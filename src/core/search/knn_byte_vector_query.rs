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
use crate::core::index::field_info::FieldInfo;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::knn_vector_values::KnnVectorValues;
use crate::core::index::leaf_reader::{LRByteVectorValues, LeafReader};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::abstract_knn_vector_query::{
  AbstractKnnVectorQuery, AbstractKnnVectorQueryBase, NO_RESULTS,
};
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::knn::knn_collector_manager::KnnCollectorManager;
use crate::core::search::knn::top_knn_collector_manager::TopKnnCollectorManager;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::top_docs::TopDocs;
use crate::core::util::HasIdentity;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::hash::{Hash, Hasher};

/// Uses `KnnVectorsReader::search` to perform nearest neighbour search.
///
/// This query also allows for performing a kNN search subject to a filter. In this case, it first
/// executes the filter for each leaf, then chooses a strategy dynamically:
///
/// - If the filter cost is less than `k`, just execute an exact search
/// - Otherwise run a kNN search subject to the filter
/// - If the kNN search visits too many vectors without completing, stop and run an exact search
#[derive(Clone, Debug)]
pub struct KnnByteVectorQuery {
  base: AbstractKnnVectorQueryBase,
  target: Vec<u8>,
  id: Identity,
}

impl KnnByteVectorQuery {
  /// Find the `k` nearest documents to the target vector according to the vectors in the
  /// given field. `target` vector.
  ///
  /// # Arguments
  ///
  /// * `field` - a field that has been indexed as a `KnnByteVectorField`.
  /// * `target` - the target of the search
  /// * `k` - the number of documents to find
  ///
  /// # Errors
  ///
  /// Returns an error if `k` is less than `1`.
  pub fn new<T>(field: T, target: Vec<u8>, k: usize) -> Result<Self>
  where
    T: Into<String>,
  {
    Self::with_filter(field, target, k, None)
  }

  /// Find the `k` nearest documents to the target vector according to the vectors in the
  /// given field. `target` vector.
  ///
  /// # Arguments
  ///
  /// * `field` - a field that has been indexed as a `KnnByteVectorField`.
  /// * `target` - the target of the search
  /// * `k` - the number of documents to find
  /// * `filter` - a filter applied before the vector search
  ///
  /// # Errors
  ///
  /// Returns an error if `k` is less than `1`.
  pub fn with_filter<T>(field: T, target: Vec<u8>, k: usize, filter: Option<Query>) -> Result<Self>
  where
    T: Into<String>,
  {
    let field = field.into();
    Ok(Self {
      base: AbstractKnnVectorQueryBase::new(field, k, filter)?,
      target,
      id: Identity::new(),
    })
  }
  /// Returns the target query vector of the search. Each vector element is a float.
  pub fn get_target_copy(&self) -> Vec<u8> {
    self.target.clone()
  }
}
impl PartialEq for KnnByteVectorQuery {
  fn eq(&self, other: &Self) -> bool {
    self.target == other.target && self.base == other.base
  }
}
impl Eq for KnnByteVectorQuery {}
impl Hash for KnnByteVectorQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.base.hash(state);
    self.target.hash(state);
  }
}

impl HasIdentity for KnnByteVectorQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for KnnByteVectorQuery {
  fn to_string(&self, _field: &str) -> Result<String> {
    let mut buffer = String::new();
    buffer.push_str(std::any::type_name::<Self>().rsplit("::").next().unwrap());
    buffer.push(':');
    buffer.push_str(&self.base.field);
    buffer.push('[');
    buffer.push_str(&self.target[0].to_string());
    buffer.push_str(",...]");
    buffer.push('[');
    buffer.push_str(&self.base.k.to_string());
    buffer.push(']');
    if let Some(filter) = &self.base.filter {
      buffer.push('[');
      buffer.push_str(&filter.to_string("")?);
      buffer.push(']');
    }
    Ok(buffer)
  }

  fn create_weight<IRC>(
    self,
    _searcher: &IndexSearcher<IRC>,
    _score_mode: &ScoreMode,
    _boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Err(LuceneError::unsupported_operation(""))
  }

  fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    AbstractKnnVectorQuery::rewrite(self, searcher)
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
    todo!()
  }
}
impl AbstractKnnVectorQuery for KnnByteVectorQuery {
  fn base(&self) -> &AbstractKnnVectorQueryBase {
    &self.base
  }

  type KnnCollectorManager = TopKnnCollectorManager;

  fn get_knn_collector_manager<IRC>(
    &self,
    k: usize,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::KnnCollectorManager>
  where
    IRC: IndexReaderContext,
  {
    self.default_get_knn_collector_manager(k, searcher)
  }

  fn approximate_search<LR, B, K>(
    &self,
    context: &LeafReaderContext<LR>,
    accept_docs: Option<B>,
    visited_limit: usize,
    knn_collector_manager: &K,
  ) -> Result<TopDocs<ScoreDoc>>
  where
    LR: LeafReader,
    B: Bits,
    K: KnnCollectorManager,
  {
    let mut knn_collector = knn_collector_manager.new_collector(visited_limit, context)?;
    let reader = context.reader();

    let byte_vector_values = reader.get_byte_vector_values(&self.base.field)?;
    let byte_vector_values = match byte_vector_values {
      Some(v) => v,
      None => {
        check_field(reader, &self.base.field)?;
        return Ok(NO_RESULTS.clone());
      },
    };

    if std::cmp::min(knn_collector.k(), byte_vector_values.size()) == 0 {
      return Ok(NO_RESULTS.clone());
    }

    reader.search_nearest_vectors_u8(
      &self.base.field,
      self.target.clone(),
      &mut knn_collector,
      accept_docs,
    )?;
    knn_collector.top_docs()
  }

  type VectorScorer<LR>
    = <LRByteVectorValues<LR> as ByteVectorValues>::VectorScorer
  where
    LR: LeafReader;

  fn create_vector_scorer<LR>(
    &self,
    context: &LeafReaderContext<LR>,
    _fi: &FieldInfo,
  ) -> Result<Option<Self::VectorScorer<LR>>>
  where
    LR: LeafReader,
  {
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
}

impl crate::core::util::accountable::Accountable for KnnByteVectorQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}
