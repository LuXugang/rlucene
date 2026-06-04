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
use crate::core::index::composite_reader::{CompositeReader, get_context};
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::query_timeout::QueryTimeoutEnum;
use crate::core::index::reader_util::ReaderUtil;
use crate::core::index::term::Term;
use crate::core::index::terms::{Terms, get_terms};
use crate::core::search::QueryCache;
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::explanation::Explanation;
use crate::core::search::field_doc::FieldDoc;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::lru_query_cache::{LRUQueryCache, MinSegmentSizePredicate};
use crate::core::search::query::{IntoQuery, Query, QueryBase, QueryWeight};
use crate::core::search::query_cache::QueryCacheEnum;
use crate::core::search::query_caching_policy::{QueryCachingPolicyArc, QueryCachingPolicyEnum};
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::similarities_impl::bm25_similarity::BM25Similarity;
use crate::core::search::similarities_impl::similarities::{IntoSimilarityArc, SimilarityEnum};
use crate::core::search::sort::Sort;
use crate::core::search::term_statistics::TermStatistics;
use crate::core::search::time_limiting_bulk_scorer::TimeLimitingBulkScorer;
use crate::core::search::top_docs::{TopDocs, TopDocsLike};
use crate::core::search::top_field_collector::populate_scores;
use crate::core::search::top_field_collector_manager::TopFieldCollectorManager;
use crate::core::search::top_field_docs::TopFieldDocs;
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::search::total_hit_count_collector_manager::TotalHitCountCollectorManager;
use crate::core::search::usage_tracking_query_caching_policy::UsageTrackingQueryCachingPolicy;
use crate::core::search::weight::Weight;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{HasIdentity, TryIntoInt};
#[cfg(test)]
use crate::test::core::search::scorer_index_searcher::ScorerIndexSearcherSearchLeafHelper;
use parking_lot::Mutex;
#[cfg(test)]
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use sysinfo::System;

const DEFAULT_MAX_CLAUSE_COUNT: usize = 1024;
#[cfg(not(test))]
static MAX_CLAUSE_COUNT: AtomicUsize = AtomicUsize::new(DEFAULT_MAX_CLAUSE_COUNT);
#[cfg(test)]
thread_local! {
  static MAX_CLAUSE_COUNT: Cell<usize> = const { Cell::new(DEFAULT_MAX_CLAUSE_COUNT) };
}
const TOTAL_HITS_THRESHOLD: usize = 1000;
/// Thresholds for index slice allocation logic.
/// To change the default, extend IndexSearcher and use custom values
const MAX_DOCS_PER_SLICE: i32 = 250000;
const MAX_SEGMENTS_PER_SLICE: usize = 5;

pub static MAX_CACHED_QUERIES: i32 = 1000;
pub static MAX_RAM_BYTES_USED: LazyLock<i64> = LazyLock::new(|| {
  let mut sys = System::new();
  sys.refresh_memory();
  let total_mem_bytes = sys.total_memory() * 1024;
  let five_percent = total_mem_bytes / 20;
  debug_assert!(five_percent <= i64::MAX as u64);
  std::cmp::min(32 * (1 << 20), five_percent as i64)
});
pub struct IndexSearcher<IRC>
where
  IRC: IndexReaderContext + 'static,
{
  pub reader_context: IRC,
  similarity: Arc<SimilarityEnum>,
  inner: Mutex<Inner>,
  query_timeout: Option<Arc<QueryTimeoutEnum>>,
  query_caching_policy: Arc<QueryCachingPolicyEnum>,
  query_cache: Option<QueryCacheEnum<IRC>>,
  // partialResult may be set on one of the threads of the executor. It may be correct to not make
  // this variable volatile since joining these threads should ensure a happens-before relationship
  // that guarantees that writes become visible on the main thread, but making the variable volatile
  // shouldn't hurt either.
  partial_result: AtomicBool,
  #[cfg(test)]
  pub(crate) disable_rewrite: bool,
  #[cfg(test)]
  pub(crate) count_invocations: AtomicUsize,
  #[cfg(test)]
  pub(crate) use_scorer_search: bool,
}
pub(crate) struct Inner {
  leaf_slices: Option<Arc<Vec<LeafSlice>>>,
}
pub type DefaultIndexSearcher<IRC> = IndexSearcher<IRC>;

impl<IRC> DefaultIndexSearcher<IRC>
where
  IRC: IndexReaderContext,
{
  // TODO IMPORTANT 这里没有加入Executor的rust版本 所以暂时不添加这个参数
  pub fn new(context: IRC) -> Result<Self> {
    debug_assert!(
      context.base().is_top_level,
      "IndexSearcher's ReaderContext must be topLevel for reader {}",
      context.reader()
    );

    let reader = context.reader();
    let leaf_contexts = context.leaves()?;

    let leaf_slices = if leaf_contexts.is_empty() {
      Some(Arc::new(Vec::new()))
    } else {
      let partitions = leaf_contexts
        .iter()
        .map(LeafReaderContextPartition::create_for_entire_segment)
        .collect::<Result<Vec<_>>>()?;

      let slice = LeafSlice {
        partitions,
        max_docs: reader.max_doc()?,
      };
      Some(Arc::new(vec![slice]))
    };
    let leaves_to_cache = MinSegmentSizePredicate::new(10000);
    let lru_query_cache = Arc::new(LRUQueryCache::with_skip_cache_factor(
      MAX_CACHED_QUERIES,
      *MAX_RAM_BYTES_USED,
      10f32,
      leaves_to_cache,
    )?);
    let inner = Mutex::new(Inner { leaf_slices });
    Ok(Self {
      reader_context: context,
      similarity: Arc::new(get_default_similarity()),
      inner,
      query_timeout: None,
      query_caching_policy: Arc::new(UsageTrackingQueryCachingPolicy::new()?.into()),
      query_cache: Some(lru_query_cache.into()),
      partial_result: AtomicBool::new(false),
      #[cfg(test)]
      disable_rewrite: false,
      #[cfg(test)]
      count_invocations: AtomicUsize::new(0),
      #[cfg(test)]
      use_scorer_search: false,
    })
  }
}
impl<LR> DefaultIndexSearcher<LeafReaderContext<LR>>
where
  LR: LeafReader + Clone,
{
  pub fn from_lr(leaf_reader: LR) -> Result<Self>
  where
    LR: LeafReader,
  {
    let context = crate::core::index::leaf_reader::get_context(leaf_reader)?;
    Self::new(context)
  }
}

pub fn get_default_similarity() -> SimilarityEnum {
  BM25Similarity::new()
    .expect("Cannot create BM25Similarity")
    .into()
}
impl<CR> IndexSearcher<CompositeReaderContext<CR>>
where
  CR: CompositeReader,
{
  pub fn from_cr(context: CR) -> Result<Self> {
    let reader = get_context(context)?;
    Self::new(reader)
  }
}

impl<IRC> IndexSearcher<IRC>
where
  IRC: IndexReaderContext,
{
  pub fn stored_fields(&self) -> Result<<IRC::IndexReader as IndexReader>::StoredFields> {
    self.reader_context.reader().stored_fields()
  }

  pub fn set_similarity<T>(&mut self, similarity: T)
  where
    T: IntoSimilarityArc,
  {
    self.similarity = similarity.into_similarity_arc();
  }

  pub fn get_slices(&self) -> Result<Arc<Vec<LeafSlice>>> {
    let mut inner = self.inner.lock();
    if inner.leaf_slices.is_none() {
      self.compute_and_cache_slices(&mut inner)?;
    }
    Ok(inner.leaf_slices.as_ref().unwrap().clone())
  }

  fn compute_and_cache_slices(&self, inner: &mut Inner) -> Result<()> {
    if inner.leaf_slices.is_none() {
      let res = slices(self.reader_context.leaves()?)?;
      // Enforce that there aren't multiple leaf partitions within the same leaf slice pointing to the
      // same leaf context. It is a requirement that [`Collector::get_leaf_collector(LeafReaderContext)`]
      // gets called once per leaf context.
      //
      // Also, it does not make sense to partition a segment to then search those partitions as part of
      // the same slice, because the goal of partitioning is parallel searching which happens at the
      // slice level.
      for leaf_slice in &res {
        if leaf_slice.partitions.len() <= 1 {
          continue;
        }
        enforce_distinct_leaves(leaf_slice)?;
      }

      inner.leaf_slices = Some(Arc::new(res));
    }
    Ok(())
  }

  pub fn search_after_score(
    &self,
    after: Option<ScoreDoc>,
    query: impl IntoQuery,
    num_hits: usize,
  ) -> Result<TopDocs<ScoreDoc>> {
    let limit = std::cmp::max(1, self.reader_context.reader().max_doc()?).try_convert()?;

    if let Some(ref a) = after
      && a.doc >= limit.try_convert()?
    {
      return Err(LuceneError::illegal_argument(format!(
        "after.doc exceeds the number of documents in the reader: after.doc={} limit={}",
        a.doc, limit
      )));
    }

    let capped_num_hits = std::cmp::min(num_hits, limit);
    let manager =
      TopScoreDocCollectorManager::with_after(capped_num_hits, after, TOTAL_HITS_THRESHOLD)?;

    self.search_with_collector_manager(query, &manager)
  }
  /// Get the configured `QueryTimeout` for all searches that run through this `IndexSearcher`,
  /// or `None` if not set.
  pub fn get_timeout<T>(&self) -> Option<Arc<QueryTimeoutEnum>> {
    self.query_timeout.clone()
  }
  /// Set a `QueryTimeout` for all searches that run through this `IndexSearcher`.
  pub fn set_timeout<T>(&mut self, query_timeout: T)
  where
    T: Into<QueryTimeoutEnum>,
  {
    self.query_timeout = Some(Arc::new(query_timeout.into()))
  }
  pub fn search(&self, query: impl IntoQuery, n: usize) -> Result<TopDocs<ScoreDoc>> {
    self.search_after_score(None, query, n)
  }
  /// Search implementation with arbitrary sorting, plus control over whether hit scores and max
  /// score should be computed.
  /// Finds the top `n` hits for `query`, sorting the hits by the criteria in `sort`.
  /// If `do_doc_scores` is `true`, the score of each hit will be computed and returned.
  /// If `do_max_score` is `true`, the maximum score over all collected hits will be computed.
  ///
  /// # Errors
  /// Returns a [`LuceneError::TooManyClauses`] if a query would exceed
  /// [`get_max_clause_count()`] clauses.
  pub fn search_with_sort_score<T>(
    &self,
    query: impl IntoQuery,
    n: usize,
    sort: T,
    do_doc_scores: bool,
  ) -> Result<TopFieldDocs>
  where
    T: Into<Arc<Sort>>,
  {
    self.search_after_field_with_score(None, query, n, sort, do_doc_scores)
  }
  /// Search implementation with arbitrary sorting.
  ///
  /// * `query` — The query to search for
  /// * `n` — Return only the top `n` results
  /// * `sort` — The `Sort` object
  ///
  /// # Returns
  /// The top docs, sorted according to the supplied `Sort` instance.
  ///
  /// # Errors
  /// Returns an error if a low-level I/O error occurs.
  pub fn search_with_sort<T>(
    &self,
    query: impl IntoQuery,
    n: usize,
    sort: T,
  ) -> Result<TopFieldDocs>
  where
    T: Into<Arc<Sort>>,
  {
    self.search_after_field_with_score(None, query, n, sort, false)
  }

  pub fn get_top_reader_context(&self) -> &IRC {
    &self.reader_context
  }
  pub fn get_similarity(&self) -> Arc<SimilarityEnum> {
    self.similarity.clone()
  }

  /// Count how many documents match the given query.
  /// May be faster than counting number of hits by collecting all matches,
  /// as the number of hits is retrieved from the index statistics when possible.
  pub fn count(&self, query: impl IntoQuery) -> Result<i32> {
    #[cfg(test)]
    self.count_invocations.fetch_add(1, Ordering::Relaxed);

    let query = query.into_query();
    let mut query = self.rewrite(ConstantScoreQuery::new(query))?;
    if let Query::ConstantScore(csq) = query {
      query = csq.into_inner()
    }

    if let Query::Boolean(boolean_query) = &query {
      let has_deletions = self.reader_context.reader().has_deletions()?;
      if !has_deletions && boolean_query.is_two_clause_pure_disjunction_with_terms() {
        let [query0, query1, query2] =
          boolean_query.rewrite_two_clause_disjunction_with_terms_for_count(self)?;
        let count_term1 = self.count(query0)?;
        let count_term2 = self.count(query1)?;

        if count_term1 == 0 || count_term2 == 0 {
          return Ok(count_term1.max(count_term2));
        } else if (count_term1.min(count_term2) as f64) / (count_term1.max(count_term2) as f64)
          < 0.1
        {
          return Ok(count_term1 + count_term2 - self.count(query2)?);
        }
      }
    }
    let v = TotalHitCountCollectorManager::new(self.get_slices()?.as_slice());
    self.search_with_collector_manager(ConstantScoreQuery::new(query), &v)
  }

  pub fn search_after_field_with_score<Q, T>(
    &self,
    after: Option<FieldDoc>,
    query: Q,
    num_hits: usize,
    sort: T,
    do_doc_scores: bool,
  ) -> Result<TopFieldDocs>
  where
    Q: IntoQuery,
    T: Into<Arc<Sort>>,
  {
    self.do_search_after_field(after, query, num_hits, sort, do_doc_scores)
  }
  pub fn search_after<Q, T>(
    &self,
    after: Option<FieldDoc>,
    query: Q,
    num_hits: usize,
    sort: T,
  ) -> Result<TopFieldDocs>
  where
    Q: IntoQuery,
    T: Into<Arc<Sort>>,
  {
    self.do_search_after_field(after, query, num_hits, sort, false)
  }

  fn do_search_after_field<Q, T>(
    &self,
    after: Option<FieldDoc>,
    query: Q,
    num_hits: usize,
    sort: T,
    do_doc_scores: bool,
  ) -> Result<TopFieldDocs>
  where
    Q: IntoQuery,
    T: Into<Arc<Sort>>,
  {
    let limit: usize = std::cmp::max(1, self.reader_context.reader().max_doc()?).try_convert()?;

    if let Some(ref a) = after
      && a.base.doc >= limit.try_convert()?
    {
      return Err(LuceneError::illegal_argument(format!(
        "after.doc exceeds the number of documents in the reader: after.doc={} limit={}",
        a.base.doc, limit
      )));
    }

    let capped_num_hits = std::cmp::min(num_hits, limit);
    // TODO IMPORTANT
    // let rewritten_sort = sort.rewrite(self)?;
    let manager =
      TopFieldCollectorManager::with_after(sort, capped_num_hits, after, TOTAL_HITS_THRESHOLD)?;
    let query = query.into_query();
    let mut top_field_docs = self.search_with_collector_manager(query.clone(), &manager)?;

    if do_doc_scores {
      populate_scores(top_field_docs.score_docs_mut(), self, query.clone())?;
    }

    Ok(top_field_docs)
  }

  pub fn search_with_collector_manager<CM>(
    &self,
    query: impl IntoQuery,
    collector_manager: &CM,
  ) -> Result<CM::T>
  where
    CM: CollectorManager,
  {
    let mut query = query.into_query();
    let first_collector = collector_manager.new_collector()?;
    let needs_scores = first_collector.score_mode().needs_scores();
    query = self.rewrite_with_needs_scores(query, needs_scores)?;
    let score_mode = first_collector.score_mode();
    let weight = self.create_weight(query, score_mode, 1.0)?;
    self.search_with_first_collector(weight.as_ref(), collector_manager, first_collector)
  }
  pub fn search_with_collector<C>(&self, query: impl IntoQuery, collector: &mut C) -> Result<()>
  where
    C: Collector,
  {
    let query = query.into_query();
    let needs_scores = collector.score_mode().needs_scores();
    let query = self.rewrite_with_needs_scores(query, needs_scores)?;
    let weight = self.create_weight(query, collector.score_mode(), 1.0)?;
    collector.set_weight(Some(&weight))?;
    let leaves = self.get_leaf_contexts()?;
    for ctx in leaves {
      self.search_leaf(ctx.ord, 0, NO_MORE_DOCS, &weight, collector)?;
    }
    Ok(())
  }
  /// Returns true if any search hit the timeout.
  pub fn timeout(&self) -> bool {
    self.partial_result.load(Ordering::Relaxed)
  }
  fn search_with_first_collector<W, CM>(
    &self,
    weight: &W,
    collector_manager: &CM,
    first_collector: CM::C,
  ) -> Result<CM::T>
  where
    CM: CollectorManager,
    W: Weight<IRC> + ?Sized,
  {
    let leaf_slices = self.get_slices()?;
    if leaf_slices.is_empty() {
      debug_assert!(self.reader_context.leaves()?.is_empty());
      collector_manager.reduce(vec![first_collector])
    } else {
      let mut collectors = Vec::with_capacity(leaf_slices.len());
      let score_mode = first_collector.score_mode();
      collectors.push(Some(first_collector));
      for _ in 1..leaf_slices.len() {
        let collector = collector_manager.new_collector()?;
        if score_mode != collector.score_mode() {
          return Err(LuceneError::illegal_state(
            "CollectorManager does not always produce collectors with the same score mode",
          ));
        }
        collectors.push(Some(collector));
      }
      let mut list_tasks = Vec::with_capacity(leaf_slices.len());
      // TODO IMPORTANT： 多线程查询 不支持
      for i in 0..leaf_slices.len() {
        let leaves = leaf_slices[i].partitions.as_slice();
        let mut collector = collectors[i].take().unwrap();
        self.search_partitions(leaves, weight, &mut collector)?;
        list_tasks.push(collector)
      }
      collector_manager.reduce(list_tasks)
    }
  }

  pub(crate) fn search_partitions<W, C>(
    &self,
    partitions: &[LeafReaderContextPartition],
    weight: &W,
    collector: &mut C,
  ) -> Result<()>
  where
    C: Collector,
    W: Weight<IRC> + ?Sized,
  {
    collector.set_weight(Some(weight))?;

    for partition in partitions {
      self.search_leaf(
        partition.ctx,
        partition.min_doc_id,
        partition.max_doc_id,
        weight,
        collector,
      )?;
    }

    Ok(())
  }
  pub(crate) fn search_leaf<W, C>(
    &self,
    ctx_ord: usize,
    min_doc_id: i32,
    max_doc_id: i32,
    weight: &W,
    collector: &mut C,
  ) -> Result<()>
  where
    C: Collector,
    W: Weight<IRC> + ?Sized,
  {
    #[cfg(test)]
    {
      if self.use_scorer_search {
        let v = ScorerIndexSearcherSearchLeafHelper;
        return v.search_leaf(self, ctx_ord, min_doc_id, max_doc_id, weight, collector);
      }
    }
    let ctx = &self.reader_context.leaves()?[ctx_ord];
    let mut leaf_collector = match collector.get_leaf_collector(ctx, Some(weight)) {
      Ok(leaf_collector) => leaf_collector,
      Err(LuceneError::CollectionTerminated(_)) => {
        // there is no doc of interest in this reader context
        // continue with the following leaf
        return Ok(());
      },
      Err(e) => return Err(e),
    };

    if let Some(mut scorer_supplier) = weight.scorer_supplier(ctx, self)? {
      scorer_supplier.set_top_level_scoring_clause()?;
      let mut scorer = match scorer_supplier.bulk_scorer(ctx, self)? {
        Some(scorer) => scorer,
        None => return Err(LuceneError::illegal_state("BulkScorer is None")),
      };
      let bits = ctx.reader().get_live_docs()?;
      let live_docs = bits.as_ref().map(|b| b as &dyn Bits);
      let result: Result<()> = (|| {
        let _ = match self.query_timeout {
          None => scorer.score(&mut leaf_collector, live_docs, min_doc_id, max_doc_id)?,
          Some(ref qt) => {
            let mut scorer = TimeLimitingBulkScorer::new(scorer, qt);
            scorer.score(&mut leaf_collector, live_docs, min_doc_id, max_doc_id)?
          },
        };
        Ok(())
      })();

      match result {
        Ok(_) => {},
        Err(LuceneError::CollectionTerminated(_)) => {
          // collection was terminated prematurely
          // continue with the following leaf
        },
        Err(LuceneError::TimeExceeded(_)) => {
          self.partial_result.store(true, Ordering::Relaxed);
        },
        Err(e) => return Err(e),
      }
    }
    // Note: this is called if collection ran successfully, including the above special cases of
    // CollectionTerminatedException and TimeExceededException, but no other exception.
    leaf_collector.finish()?;
    Ok(())
  }
  pub fn rewrite<Q>(&self, query: Q) -> Result<Query>
  where
    Q: IntoQuery,
  {
    let mut query = query.into_query();
    #[cfg(test)]
    if self.disable_rewrite {
      return Ok(query);
    }
    let mut query_id = query.identity().clone();
    loop {
      query = query.rewrite(self)?;
      if query.identity() == &query_id {
        break;
      }
      query_id = query.identity().clone();
    }
    // query.visit(self.get_num_clauses_check_visitor());
    Ok(query)
  }

  pub(crate) fn rewrite_with_needs_scores(
    &self,
    original: Query,
    needs_scores: bool,
  ) -> Result<Query> {
    if needs_scores {
      self.rewrite(original)
    } else {
      // Take advantage of the few extra rewrite rules of ConstantScoreQuery.
      let v = ConstantScoreQuery::new(original);
      self.rewrite(v)
    }
  }
  /// Returns an Explanation that describes how `doc` scored against `query`.
  ///
  /// This is intended to be used in developing Similarity implementations, and, for good
  /// performance, should not be displayed with every hit. Computing an explanation is as expensive
  /// as executing the query over the entire index.
  pub fn explain<T>(&self, query: T, doc: i32) -> Result<Explanation>
  where
    T: IntoQuery,
  {
    let query = self.rewrite(query.into_query())?;
    let weight = self.create_weight(query, ScoreMode::Complete, 1.0)?;
    self.explain_from_weight(&weight, doc)
  }
  /// Expert: low-level implementation method Returns an Explanation that describes how `doc`
  /// scored against `weight`.
  ///
  /// This is intended to be used in developing Similarity implementations, and, for good
  /// performance, should not be displayed with every hit. Computing an explanation is as expensive
  /// as executing the query over the entire index.
  ///
  /// Applications should call [`IndexSearcher::explain`].
  ///
  /// # Errors
  ///
  /// Returns an error if a query would exceed `IndexSearcher::get_max_clause_count` clauses.
  pub fn explain_from_weight(&self, weight: &QueryWeight<IRC>, doc: i32) -> Result<Explanation> {
    let leaf_contexts = self.reader_context.leaves()?;
    let n = ReaderUtil::sub_index_with_leaves(doc, leaf_contexts);
    let ctx = &leaf_contexts[n];
    let de_based_doc = doc as usize - ctx.doc_base;

    let live_docs = ctx.reader().get_live_docs()?;
    if let Some(live_docs) = live_docs
      && !live_docs.get(de_based_doc)?
    {
      return Ok(Explanation::no_match_no_details(format!(
        "Document {} is deleted",
        doc
      )));
    }

    weight.explain(ctx, de_based_doc as i32, self)
  }

  #[allow(clippy::type_complexity)]
  pub(crate) fn create_weight<T>(
    &self,
    query: T,
    score_mode: ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    T: QueryBase,
  {
    let mut weight = query.create_weight(self, &score_mode, boost)?;
    if !score_mode.needs_scores()
      && let Some(query_cache) = self.query_cache.as_ref()
    {
      weight = query_cache.do_cache(weight, self.query_caching_policy.clone());
    }

    Ok(weight)
  }

  /// Returns [`TermStatistics`] for a term.
  ///
  /// This method can be overridden, for example, to return a term's statistics across
  /// a distributed collection.
  ///
  /// # Arguments
  ///
  /// * `doc_freq` — The document frequency of the term. It must be greater or equal to 1.
  /// * `total_term_freq` — The total term frequency.
  ///
  /// # Returns
  ///
  /// A [`TermStatistics`] (never `None`).
  ///
  /// **Lucene Experimental**
  pub fn term_statistics<T>(
    &self,
    term: T,
    doc_freq: i32,
    total_term_freq: i64,
  ) -> Result<TermStatistics>
  where
    T: Into<Arc<Term>>,
  {
    TermStatistics::new(term, doc_freq as i64, total_term_freq)
  }
  /// Returns [`CollectionStatistics`] for a field, or `None` if the field does not exist
  /// (has no indexed terms).
  ///
  ///
  /// This method can be overridden, for example, to return a field's statistics across
  /// a distributed collection.
  pub fn collection_statistics(&self, field: &str) -> Result<Option<CollectionStatistics>> {
    let mut doc_count: i64 = 0;
    let mut sum_total_term_freq: i64 = 0;
    let mut sum_doc_freq: i64 = 0;

    for leaf in self.reader_context.leaves()? {
      let reader = leaf.reader();
      let terms = get_terms(reader, field)?;
      doc_count += terms.get_doc_count()? as i64;
      sum_total_term_freq += terms.get_sum_total_term_freq()?;
      sum_doc_freq += terms.get_sum_doc_freq()?;
    }

    if doc_count == 0 {
      return Ok(None);
    }

    let stats = CollectionStatistics::new(
      field,
      self.reader_context.reader().max_doc()? as i64,
      doc_count,
      sum_total_term_freq,
      sum_doc_freq,
    )?;

    Ok(Some(stats))
  }
  pub fn get_leaf_contexts(&self) -> Result<&[LeafReaderContext<IRC::LeafReader>]> {
    self.reader_context.leaves()
  }
  pub fn get_index_reader(&self) -> &IRC::IndexReader {
    self.reader_context.reader()
  }

  pub fn set_query_cache(&mut self, query_cache: Option<QueryCacheEnum<IRC>>) {
    self.query_cache = query_cache;
  }
  pub fn get_query_cache(&self) -> Option<&QueryCacheEnum<IRC>> {
    self.query_cache.as_ref()
  }

  pub fn get_query_caching_policy(&self) -> Arc<QueryCachingPolicyEnum> {
    self.query_caching_policy.clone()
  }

  pub fn set_query_caching_policy<T>(&mut self, query_caching_policy: T)
  where
    T: QueryCachingPolicyArc,
  {
    self.query_caching_policy = query_caching_policy.into_query_cache_policy_arc();
  }
}

/// Returns the maximum number of clauses permitted, `1024` by default.
///
/// Attempts to add more than the permitted number of clauses cause a [`TooManyClauses`] error to be thrown.
///
/// Tests can override this value with `set_max_clause_count`.
pub fn get_max_clause_count() -> usize {
  #[cfg(test)]
  {
    MAX_CLAUSE_COUNT.with(Cell::get)
  }
  #[cfg(not(test))]
  {
    MAX_CLAUSE_COUNT.load(Ordering::Relaxed)
  }
}
/// Set the maximum number of clauses permitted per Query. Default value is 1024.
pub fn set_max_clause_count(value: usize) -> Result<()> {
  if value < 1 {
    return Err(LuceneError::illegal_argument("maxClauseCount must be >= 1"));
  }
  #[cfg(test)]
  {
    MAX_CLAUSE_COUNT.with(|max_clause_count| max_clause_count.set(value));
  }
  #[cfg(not(test))]
  {
    MAX_CLAUSE_COUNT.store(value, Ordering::Relaxed);
  }
  Ok(())
}

pub fn do_slices<LR>(
  leaves: &[LeafReaderContext<LR>],
  max_docs_per_slice: i32,
  max_segments_per_slice: usize,
  allow_segment_partitions: bool,
) -> Result<Vec<LeafSlice>>
where
  LR: LeafReader,
{
  let mut ctx_map: HashMap<usize, usize> = HashMap::with_capacity(leaves.len());
  let mut sorted_leaves: Vec<(usize, i32)> = Vec::with_capacity(leaves.len());

  for (idx, ctx) in leaves.iter().enumerate() {
    let ord = ctx.ord;
    let max_doc = ctx.reader().max_doc()?;
    ctx_map.insert(ord, idx);
    sorted_leaves.push((ord, max_doc));
  }
  sorted_leaves.sort_by_key(|leaf| std::cmp::Reverse(leaf.1));

  if allow_segment_partitions {
    let mut grouped_leaf_partitions: Vec<Vec<LeafReaderContextPartition>> = Vec::new();
    let mut current_slice_num_docs = 0;
    let mut group: Option<Vec<LeafReaderContextPartition>> = None;

    for (ord, _) in sorted_leaves {
      let ctx_idx = ctx_map[&ord];
      let ctx_max_doc = leaves[ctx_idx].reader().max_doc()?;
      if ctx_max_doc > max_docs_per_slice {
        debug_assert!(group.is_none());
        // if the segment does not fit in a single slice, we split it into maximum 5 partitions of equal size
        let num_slices = std::cmp::min(
          5,
          (ctx_max_doc + max_docs_per_slice - 1) / max_docs_per_slice,
        );
        let num_docs = ctx_max_doc / num_slices;
        let mut max_doc_id = num_docs;
        let mut min_doc_id = 0;

        for _ in 0..(num_slices - 1) {
          grouped_leaf_partitions.push(vec![LeafReaderContextPartition::create_from_and_to(
            &leaves[ctx_idx],
            min_doc_id,
            max_doc_id,
          )?]);
          min_doc_id = max_doc_id;
          max_doc_id += num_docs;
        }
        // the last slice gets all the remaining docs
        grouped_leaf_partitions.push(vec![LeafReaderContextPartition::create_from_and_to(
          &leaves[ctx_idx],
          min_doc_id,
          ctx_max_doc,
        )?]);
      } else {
        if group.is_none() {
          group = Some(Vec::new());
        }
        let group_ref = group.as_mut().unwrap();
        group_ref.push(LeafReaderContextPartition::create_for_entire_segment(
          &leaves[ctx_idx],
        )?);
        current_slice_num_docs += ctx_max_doc;
        // We only split a segment when it does not fit entirely in a slice. We don't partition
        // the
        // segment that makes the current slice (which holds multiple segments) go over
        // maxDocsPerSlice. This means that a slice either contains multiple entire segments, or a
        // single partition of a segment.
        if group_ref.len() >= max_segments_per_slice || current_slice_num_docs > max_docs_per_slice
        {
          grouped_leaf_partitions.push(group.take().unwrap());
          current_slice_num_docs = 0;
        }
      }
    }

    if let Some(g) = group.take() {
      grouped_leaf_partitions.push(g);
    }

    return Ok(
      grouped_leaf_partitions
        .into_iter()
        .map(LeafSlice::new)
        .collect(),
    );
  }

  let mut grouped_leaves: Vec<Vec<usize>> = Vec::new();
  let mut doc_sum: i64 = 0;
  let mut group: Option<Vec<usize>> = None;

  for (ord, _) in sorted_leaves {
    let ctx_idx = ctx_map[&ord];
    let ctx_max_doc = leaves[ctx_idx].reader().max_doc()?;

    if ctx_max_doc > max_docs_per_slice {
      debug_assert!(group.is_none());
      grouped_leaves.push(vec![ord]);
    } else {
      if group.is_none() {
        group = Some(Vec::new());
      }
      let group_ref = group.as_mut().unwrap();
      group_ref.push(ord);
      doc_sum += ctx_max_doc as i64;

      if group_ref.len() >= max_segments_per_slice || doc_sum > max_docs_per_slice as i64 {
        grouped_leaves.push(group.take().unwrap());
        doc_sum = 0;
      }
    }
  }

  if let Some(g) = group.take() {
    grouped_leaves.push(g);
  }

  let mut slices = Vec::new();

  for ords in grouped_leaves {
    let mut partitions = Vec::new();
    for ord in ords {
      let ctx_idx = ctx_map[&ord];
      let partition = LeafReaderContextPartition::create_for_entire_segment(&leaves[ctx_idx])?;
      partitions.push(partition);
    }
    slices.push(LeafSlice::new(partitions));
  }
  Ok(slices)
}
/// Expert: Creates an array of [`LeafSlice`] each holding a subset of the given leaves.
/// Each [`LeafSlice`] is executed in a single thread.
///
/// By default, segments with more than `MAX_DOCS_PER_SLICE` will get their own thread.
///
///
/// It is possible to leverage intra-segment concurrency by splitting segments into multiple
/// partitions. Such behaviour is not enabled by default as there is still a performance penalty
/// for queries that require segment-level computation ahead of time, such as points/range queries.
///
/// This is an implementation limitation that we expect to improve in future releases,
/// see [the corresponding GitHub issue](https://github.com/apache/lucene/issues/13745).
pub fn slices<LR>(leaves: &[LeafReaderContext<LR>]) -> Result<Vec<LeafSlice>>
where
  LR: LeafReader,
{
  do_slices(leaves, MAX_DOCS_PER_SLICE, MAX_SEGMENTS_PER_SLICE, false)
}

fn enforce_distinct_leaves(leaf_slice: &LeafSlice) -> Result<()> {
  let mut distinct_leaves = HashSet::new();

  for partition in &leaf_slice.partitions {
    if !distinct_leaves.insert(partition.ctx) {
      return Err(LuceneError::illegal_state(
        "The same slice targets multiple leaf partitions of the same leaf reader context. \
                A physical segment should rather get partitioned to be searched concurrently from \
                as many slices as the number of leaf partitions it is split into.",
      ));
    }
  }

  Ok(())
}
/// Thrown when an attempt is made to add more than [`get_max_clause_count()`] clauses.
///
/// This typically happens if a `PrefixQuery`, `FuzzyQuery`, `WildcardQuery`,
/// or `TermRangeQuery` is expanded to many terms during search.
pub struct TooManyClauses;
pub fn new() -> LuceneError {
  with_msg(format!(
    "maxClauseCount is set to {}",
    get_max_clause_count()
  ))
}
pub fn with_msg(msg: String) -> LuceneError {
  LuceneError::too_many_clauses(msg)
}
pub struct TooManyNestedClauses;
pub fn new_nested() -> LuceneError {
  LuceneError::too_many_nested_clauses(format!(
    "Query contains too many nested clauses; maxClauseCount is set to {}",
    get_max_clause_count()
  ))
}

/// Holds information about a specific leaf context and the corresponding range of doc ids to
/// search within. Used to optionally search across partitions of the same segment concurrently.
///
/// A partition instance can be created via [`LeafReaderContextPartition::create_for_entire_segment`],
/// in which case it will target the entire provided [`LeafReaderContext`].
/// A true partition of a segment can be created via
/// [`LeafReaderContextPartition::create_from_and_to`] providing the minimum doc id (inclusive) to
/// search as well as the max doc id (exclusive).
pub struct LeafReaderContextPartition {
  pub min_doc_id: i32,
  pub max_doc_id: i32,
  pub ctx: usize,
  pub doc_base: usize,
  pub ctx_max_doc: i32,
  // we keep track of maxDocs separately because we use NO_MORE_DOCS as upper bound when targeting
  // the entire segment. We use this only in tests.
  max_docs: i32,
}
impl LeafReaderContextPartition {
  pub fn new<LR>(
    leaf_reader_context: &LeafReaderContext<LR>,
    min_doc_id: i32,
    max_doc_id: i32,
    max_docs: i32,
  ) -> Result<Self>
  where
    LR: LeafReader,
  {
    let ctx_max_doc = leaf_reader_context.reader().max_doc()?;
    if min_doc_id >= max_doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "minDocId is greater than or equal to maxDocId: [{}] >= [{}]",
        min_doc_id, max_doc_id
      )));
    }
    if min_doc_id < 0 {
      return Err(LuceneError::illegal_argument(format!(
        "minDocId is lower than 0: [{}]",
        min_doc_id
      )));
    }
    if min_doc_id >= ctx_max_doc {
      return Err(LuceneError::illegal_argument(format!(
        "minDocId is greater than maxDoc: [{}] >= [{}]",
        min_doc_id, ctx_max_doc
      )));
    }

    Ok(Self {
      min_doc_id,
      max_doc_id,
      ctx_max_doc,
      ctx: leaf_reader_context.ord,
      doc_base: leaf_reader_context.doc_base,
      max_docs,
    })
  }
  /// Creates a partition of the provided leaf context that targets the entire segment
  pub fn create_for_entire_segment<LR>(ctx: &LeafReaderContext<LR>) -> Result<Self>
  where
    LR: LeafReader,
  {
    Self::new(ctx, 0, NO_MORE_DOCS, ctx.reader().max_doc()?)
  }

  /// Creates a partition of the provided leaf context that targets a subset of the entire segment,
  /// starting from and including the min doc id provided, until and not including the provided max doc id
  pub fn create_from_and_to<LR>(
    ctx: &LeafReaderContext<LR>,
    min_doc_id: i32,
    max_doc_id: i32,
  ) -> Result<Self>
  where
    LR: LeafReader,
  {
    debug_assert!(max_doc_id != NO_MORE_DOCS);
    Self::new(ctx, min_doc_id, max_doc_id, max_doc_id - min_doc_id)
  }
}
/// A class holding a subset of the [`IndexSearcher`]’s leaf contexts to be executed within a
/// single thread. A leaf slice holds references to one or more [`LeafReaderContextPartition`]
/// instances. Each partition targets a specific doc id range of a [`LeafReaderContext`].
pub struct LeafSlice {
  /// The leaves that make up this slice.
  pub partitions: Vec<LeafReaderContextPartition>,

  max_docs: i32,
}

impl LeafSlice {
  pub fn new(mut partitions: Vec<LeafReaderContextPartition>) -> Self {
    partitions.sort_by(|a, b| {
      let doc_base_cmp = a.doc_base.cmp(&b.doc_base);
      if doc_base_cmp == std::cmp::Ordering::Equal {
        a.min_doc_id.cmp(&b.min_doc_id)
      } else {
        doc_base_cmp
      }
    });
    let max_docs = partitions.iter().map(|p| p.max_docs).sum();

    Self {
      partitions,
      max_docs,
    }
  }
  /// Returns the total number of docs that a slice targets,
  /// by summing the number of docs that each of its leaf context partitions targets.
  pub fn max_docs(&self) -> i32 {
    self.max_docs
  }
}
