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
// External luceneutil-style profiling adapter. Production HNSW/scoring is unchanged.
use rlucene::core::{
  index::{
    field_info::FieldInfo, index_reader::Identity, index_reader_context::IndexReaderContext,
    leaf_reader::LeafReader, leaf_reader_context::LeafReaderContext,
  },
  search::{
    abstract_knn_vector_query::{AbstractKnnVectorQuery, AbstractKnnVectorQueryBase},
    index_searcher::IndexSearcher,
    knn::{
      knn_collector_manager::KnnCollectorManager, top_knn_collector_manager::TopKnnCollectorManager,
    },
    knn_float_vector_query::KnnFloatVectorQuery,
    query::{Query, QueryBase, QueryWeight},
    query_visitor::QueryVisitor,
    score_doc::ScoreDoc,
    score_mode::ScoreMode,
    similarities_impl::similarities::Similarity,
    top_docs::TopDocs,
  },
  util::{HasIdentity, accountable::Accountable, bits::Bits, error::lucene_error::Result},
};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct Profiled {
  inner: KnnFloatVectorQuery,
  base: AbstractKnnVectorQueryBase,
  search_k: usize,
  count: AtomicU64,
}
impl Profiled {
  pub fn new(target: Vec<f32>, k: usize, fanout: usize) -> Result<Self> {
    Ok(Self {
      inner: KnnFloatVectorQuery::new("knn", target, k + fanout)?,
      base: AbstractKnnVectorQueryBase::new("knn", k, None)?,
      search_k: k + fanout,
      count: AtomicU64::new(0),
    })
  }
  pub fn visited(&self) -> u64 {
    self.count.load(Ordering::Relaxed)
  }
}
impl HasIdentity for Profiled {
  fn identity(&self) -> &Identity {
    self.inner.identity()
  }
}
impl Accountable for Profiled {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(std::mem::size_of::<Self>() as i64)
  }
}
impl QueryBase for Profiled {
  type Weight<IRC, SIM>
    = QueryWeight<IRC, SIM>
  where
    SIM: Similarity,
    IRC: IndexReaderContext + 'static;
  fn to_string(&self, f: &str) -> Result<String> {
    self.inner.to_string(f)
  }
  fn create_weight<IRC, SIM>(
    self,
    s: &IndexSearcher<IRC, SIM>,
    m: &ScoreMode,
    b: f32,
  ) -> Result<Self::Weight<IRC, SIM>>
  where
    SIM: Similarity,
    IRC: IndexReaderContext + 'static,
    IndexSearcher<IRC, SIM>: Sync,
  {
    self.inner.create_weight(s, m, b)
  }
  fn visit<V: QueryVisitor>(&self, v: &mut V) -> Result<()> {
    self.inner.visit(v)
  }
  fn rewrite<IRC, SIM>(&self, s: &IndexSearcher<IRC, SIM>) -> Result<Option<Query>>
  where
    SIM: Similarity,
    IRC: IndexReaderContext + 'static,
    IndexSearcher<IRC, SIM>: Sync,
  {
    AbstractKnnVectorQuery::rewrite(self, s).map(Some)
  }
}
impl AbstractKnnVectorQuery for Profiled {
  fn base(&self) -> &AbstractKnnVectorQueryBase {
    &self.base
  }
  type KnnCollectorManager = TopKnnCollectorManager;
  fn get_knn_collector_manager<IRC, SIM>(
    &self,
    _k: usize,
    s: &IndexSearcher<IRC, SIM>,
  ) -> Result<Self::KnnCollectorManager>
  where
    SIM: Similarity,
    IRC: IndexReaderContext + 'static,
    IndexSearcher<IRC, SIM>: Sync,
  {
    self.inner.get_knn_collector_manager(self.search_k, s)
  }
  fn approximate_search<LR: LeafReader, B: Bits, K: KnnCollectorManager>(
    &self,
    c: &LeafReaderContext<LR>,
    a: Option<B>,
    l: usize,
    k: &K,
  ) -> Result<TopDocs<ScoreDoc>> {
    let docs = self.inner.approximate_search(c, a, l, k)?;
    self
      .count
      .fetch_add(docs.total_hits.value as u64, Ordering::Relaxed);
    Ok(docs)
  }
  type VectorScorer<LR>
    = <KnnFloatVectorQuery as AbstractKnnVectorQuery>::VectorScorer<LR>
  where
    LR: LeafReader;
  fn create_vector_scorer<LR: LeafReader>(
    &self,
    c: &LeafReaderContext<LR>,
    f: &FieldInfo,
  ) -> Result<Option<Self::VectorScorer<LR>>> {
    self.inner.create_vector_scorer(c, f)
  }
}
pub fn query<IRC, SIM>(
  s: &IndexSearcher<IRC, SIM>,
  target: Vec<f32>,
  k: usize,
  fanout: usize,
) -> Result<(TopDocs<ScoreDoc>, u64)>
where
  SIM: Similarity,
  IRC: IndexReaderContext + 'static,
  IndexSearcher<IRC, SIM>: Sync,
{
  let query = Profiled::new(target, k, fanout)?;
  // Query is a closed production enum. Invoke the existing public abstract KNN
  // rewrite with this external adapter, then perform the ordinary full search.
  // No filter/timeout is enabled in this single-segment performance workload.
  let rewritten = AbstractKnnVectorQuery::rewrite(&query, s)?;
  let docs = s.search(rewritten, k)?;
  Ok((docs, query.visited()))
}
