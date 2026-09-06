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
use crate::core::index::field_info::FieldInfo;
use crate::core::index::float_vector_values::{FloatVectorValues, check_field};
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::knn_vector_values::KnnVectorValues;
use crate::core::index::leaf_reader::{LRFloatVectorValues, LeafReader};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::query_timeout::QueryTimeout;
use crate::core::search::abstract_knn_vector_query::{
  AbstractKnnVectorQuery, AbstractKnnVectorQueryBase, AbstractKnnVectorQueryDefaults, NO_RESULTS,
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
use crate::core::util::bit_set::BitSet;
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::bits::Bits;
use crate::core::util::core_helper::CoreHelper;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::vector_util::VectorUtil;
use std::hash::{Hash, Hasher};

/// Uses [`KnnVectorsReader::search_f32`](crate::core::codecs::knn_vectors_reader::KnnVectorsReader::search_f32) to perform nearest neighbour search.
///
/// This query also allows for performing a kNN search subject to a filter. In this case, it first
/// executes the filter for each leaf, then chooses a strategy dynamically:
///
/// - If the filter cost is less than `k`, just execute an exact search
/// - Otherwise run a kNN search subject to the filter
/// - If the kNN search visits too many vectors without completing, stop and run an exact search
#[derive(Clone, Debug)]
pub struct KnnFloatVectorQuery {
  base: AbstractKnnVectorQueryBase,
  target: Vec<f32>,
  hook: KnnFloatVectorQueryHook,
  id: Identity,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum KnnFloatVectorQueryHook {
  Default,
  #[cfg(test)]
  Throwing,
}

impl KnnFloatVectorQuery {
  /// Find the `k` nearest documents to the target vector according to the vectors in the
  /// given field. `target` vector.
  ///
  /// # Arguments
  ///
  /// * `field` - a field that has been indexed as a [`KnnFloatVectorField`](crate::core::document::knn_float_vector_field::KnnFloatVectorField).
  /// * `target` - the target of the search
  /// * `k` - the number of documents to find
  ///
  /// # Errors
  ///
  /// Returns an error if `k` is less than `1`.
  pub fn new<T>(field: T, target: Vec<f32>, k: usize) -> Result<Self>
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
  /// * `field` - a field that has been indexed as a [`KnnFloatVectorField`](crate::core::document::knn_float_vector_field::KnnFloatVectorField).
  /// * `target` - the target of the search
  /// * `k` - the number of documents to find
  /// * `filter` - a filter applied before the vector search
  ///
  /// # Errors
  ///
  /// Returns an error if `k` is less than `1`.
  pub fn with_filter<T>(field: T, target: Vec<f32>, k: usize, filter: Option<Query>) -> Result<Self>
  where
    T: Into<String>,
  {
    let field = field.into();
    VectorUtil::check_finite(target.as_ref())?;
    Ok(Self {
      base: AbstractKnnVectorQueryBase::new(field, k, filter)?,
      target,
      hook: KnnFloatVectorQueryHook::Default,
      id: Identity::new(),
    })
  }

  #[cfg(test)]
  pub(crate) fn throwing_with_filter<T>(
    field: T,
    target: Vec<f32>,
    k: usize,
    filter: Option<Query>,
  ) -> Result<Self>
  where
    T: Into<String>,
  {
    let mut query = Self::with_filter(field, target, k, filter)?;
    query.hook = KnnFloatVectorQueryHook::Throwing;
    Ok(query)
  }

  /// Returns the target query vector of the search. Each vector element is a float.
  pub fn get_target_copy(&self) -> Vec<f32> {
    self.target.clone()
  }
}

impl PartialEq for KnnFloatVectorQuery {
  fn eq(&self, other: &Self) -> bool {
    CoreHelper::array_equals_f32(&self.target, &other.target)
      && self.base == other.base
      && self.hook == other.hook
  }
}

impl Eq for KnnFloatVectorQuery {}

impl Hash for KnnFloatVectorQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.base.hash(state);
    self.hook.hash(state);
    for &v in &self.target {
      (BitUtil::float_to_int_bits(v) as u32).hash(state);
    }
  }
}

impl HasIdentity for KnnFloatVectorQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for KnnFloatVectorQuery {
  fn to_string(&self, _field: &str) -> Result<String> {
    let mut buffer = String::new();
    let type_name = std::any::type_name::<Self>();
    buffer.push_str(
      type_name
        .rsplit_once("::")
        .map_or(type_name, |(_, name)| name),
    );
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

  fn rewrite<IRC>(&self, searcher: &IndexSearcher<IRC>) -> Result<Option<Query>>
  where
    IRC: IndexReaderContext,
    IndexSearcher<IRC>: Sync,
    Self: Sized,
  {
    AbstractKnnVectorQuery::rewrite(self, searcher).map(Some)
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

impl AbstractKnnVectorQuery for KnnFloatVectorQuery {
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

    let float_vector_values = reader.get_float_vector_values(&self.base.field)?;
    let float_vector_values = match float_vector_values {
      Some(v) => v,
      None => {
        check_field(reader, &self.base.field)?;
        return Ok(NO_RESULTS.clone());
      },
    };

    if std::cmp::min(knn_collector.k(), float_vector_values.size()) == 0 {
      return Ok(NO_RESULTS.clone());
    }

    reader.search_nearest_vectors_f32(
      &self.base.field,
      self.target.clone(),
      &mut knn_collector,
      accept_docs,
    )?;
    knn_collector.top_docs()
  }

  type VectorScorer<LR>
    = <LRFloatVectorValues<LR> as FloatVectorValues>::VectorScorer
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
    let vector_values = match reader.get_float_vector_values(&self.base.field)? {
      Some(vector_values) => vector_values,
      None => {
        check_field(reader, &self.base.field)?;
        return Ok(None);
      },
    };
    vector_values.scorer(self.target.clone())
  }

  fn exact_search<LR, T, Q>(
    &self,
    context: &LeafReaderContext<LR>,
    accept_iterator: BitSetIterator<T>,
    query_timeout: Option<&Q>,
  ) -> Result<TopDocs<ScoreDoc>>
  where
    LR: LeafReader,
    T: BitSet,
    Q: QueryTimeout,
  {
    match self.hook {
      KnnFloatVectorQueryHook::Default => {
        AbstractKnnVectorQueryDefaults::exact_search(self, context, accept_iterator, query_timeout)
      },
      #[cfg(test)]
      KnnFloatVectorQueryHook::Throwing => Err(LuceneError::unsupported_operation(
        "exact search is not supported",
      )),
    }
  }
}

impl crate::core::util::accountable::Accountable for KnnFloatVectorQuery {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}
