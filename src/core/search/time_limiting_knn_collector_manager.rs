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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::query_timeout::QueryTimeout;
use crate::core::search::knn::knn_collector_manager::{KnnCollectorEnum, KnnCollectorManager};
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::top_docs::TopDocs;
use crate::core::search::total_hits::Relation::GreaterThanOrEqualTo;
use crate::core::search::total_hits::TotalHits;
use crate::core::util::error::lucene_error::Result;

/// A [`KnnCollectorManager`] that collects results with a timeout.
pub struct TimeLimitingKnnCollectorManager<K, Q>
where
  K: KnnCollectorManager,
  Q: QueryTimeout,
{
  delegate: K,
  query_timeout: Option<Q>,
}
impl<K, Q> TimeLimitingKnnCollectorManager<K, Q>
where
  K: KnnCollectorManager,
  Q: QueryTimeout,
{
  pub fn new(delegate: K, query_timeout: Option<Q>) -> Self {
    Self {
      delegate,
      query_timeout,
    }
  }
}
impl<K, Q> KnnCollectorManager for TimeLimitingKnnCollectorManager<K, Q>
where
  K: KnnCollectorManager,
  Q: QueryTimeout,
{
  type KnnCollector<'a>
    = KnnCollectorEnum<K::KnnCollector<'a>, TimeLimitingKnnCollector<'a, K::KnnCollector<'a>, Q>>
  where
    Self: 'a;

  fn new_collector<LR>(
    &self,
    visited_limit: usize,
    context: LeafReaderContext<LR>,
  ) -> Result<Self::KnnCollector<'_>>
  where
    LR: LeafReader,
  {
    let collector = self.delegate.new_collector(visited_limit, context)?;
    match self.query_timeout {
      Some(ref timeout) => Ok(KnnCollectorEnum::B(TimeLimitingKnnCollector::new(
        collector, timeout,
      ))),
      None => Ok(KnnCollectorEnum::A(collector)),
    }
  }
}

pub struct TimeLimitingKnnCollector<'a, K, Q>
where
  K: KnnCollector,
  Q: QueryTimeout,
{
  collector: K,
  query_timeout: &'a Q,
}
impl<'a, K, Q> TimeLimitingKnnCollector<'a, K, Q>
where
  K: KnnCollector,
  Q: QueryTimeout,
{
  pub fn new(delegate: K, query_timeout: &'a Q) -> Self {
    Self {
      collector: delegate,
      query_timeout,
    }
  }
}

impl<'a, K, Q> KnnCollector for TimeLimitingKnnCollector<'a, K, Q>
where
  K: KnnCollector,
  Q: QueryTimeout,
{
  fn early_terminated(&self) -> bool {
    self.query_timeout.should_exit() || self.collector.early_terminated()
  }

  fn inc_visited_count(&mut self, count: usize) {
    self.collector.inc_visited_count(count)
  }

  fn visited_count(&self) -> usize {
    self.collector.visited_count()
  }

  fn visit_limit(&self) -> usize {
    self.collector.visit_limit()
  }

  fn k(&self) -> usize {
    self.collector.k()
  }

  fn collect(&mut self, doc_id: usize, similarity: f32) -> Result<bool> {
    self.collector.collect(doc_id, similarity)
  }

  fn min_competitive_similarity(&self) -> Result<f32> {
    self.collector.min_competitive_similarity()
  }

  type ScoreDocLike = <K as KnnCollector>::ScoreDocLike;

  fn top_docs(&mut self) -> Result<TopDocs<Self::ScoreDocLike>> {
    let docs = self.collector.top_docs()?;
    // Mark results as partial if timeout is met
    let relation = if self.query_timeout.should_exit() {
      GreaterThanOrEqualTo
    } else {
      docs.total_hits.relation
    };
    let total_hits = TotalHits::new(docs.total_hits.value(), relation);
    Ok(TopDocs::new(total_hits, docs.score_docs))
  }
}
