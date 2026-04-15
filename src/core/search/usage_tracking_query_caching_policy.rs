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
use crate::core::search::query::Query;
use crate::core::search::query_caching_policy::QueryCachingPolicy;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::frequency_tracking_ring_buffer::FrequencyTrackingRingBuffer;
use parking_lot::Mutex;
use std::hash::{DefaultHasher, Hash, Hasher};

/// the hash code that we use as a sentinel in the ring buffer.
const SENTINEL: i32 = i32::MAX;
pub struct UsageTrackingQueryCachingPolicy {
  recently_used_filters: Mutex<FrequencyTrackingRingBuffer>,
}
impl UsageTrackingQueryCachingPolicy {
  pub fn new() -> Result<Self> {
    Self::with_history_size(256)
  }
  pub fn with_history_size(history_size: usize) -> Result<Self> {
    Ok(Self {
      recently_used_filters: Mutex::new(FrequencyTrackingRingBuffer::new(history_size, SENTINEL)?),
    })
  }
  pub(crate) fn min_frequency_to_cache(&self, query: &Query) -> i32 {
    if is_costly(query) {
      2
    } else {
      // default: cache after the filter has been seen 5 times
      let mut min_frequency = 5;
      if matches!(query, Query::Boolean(_) | Query::DisjunctionMax(_)) {
        // Say you keep reusing a boolean query that looks like "A OR B" and
        // never use the A and B queries out of that context. 5 times after it
        // has been used, we would cache both A, B and A OR B, which is
        // wasteful. So instead we cache compound queries a bit earlier so that
        // we would only cache "A OR B" in that case.
        min_frequency -= 1;
      }
      min_frequency
    }
  }
  fn should_never_cache(&self, query: &Query) -> bool {
    match query {
      Query::Term(_) => {
        // We do not bother caching term queries since they are already plenty fast.
        true
      },
      Query::FieldExists(_) => {
        // We do not bother caching FieldExistsQuery queries since they are already plenty fast.
        true
      },
      Query::MatchAllDocs(_) => {
        // MatchAllDocsQuery has an iterator that is faster than what a bit set could do.
        true
      },
      Query::MatchNoDocs(_) => {
        // For the below queries, it's cheap to notice they cannot match any docs so
        // we do not bother caching them.
        true
      },
      Query::Boolean(bq) => bq.clauses().is_empty(),
      Query::DisjunctionMax(dmq) => dmq.get_disjuncts().is_empty(),
      _ => false,
    }
  }
  pub(crate) fn frequency(&self, query: &Query) -> i32 {
    debug_assert!(!matches!(query, Query::Boost(_)));
    debug_assert!(!matches!(query, Query::ConstantScore(_)));

    let mut hasher = DefaultHasher::new();
    query.hash(&mut hasher);
    let hash_code = hasher.finish();
    let hash_code = (hash_code & 0x7FFF_FFFF) as i32;
    let recently_used_filters = self.recently_used_filters.lock();
    recently_used_filters.frequency(hash_code)
  }
}
impl QueryCachingPolicy for UsageTrackingQueryCachingPolicy {
  fn on_use(&self, query: &Query) {
    debug_assert!(
      !matches!(query, Query::Boost(_)),
      "BoostQuery should not be passed to on_use()"
    );
    debug_assert!(
      !matches!(query, Query::ConstantScore(_)),
      "ConstantScoreQuery should not be passed to on_use()"
    );
    if self.should_never_cache(query) {
      return;
    }

    let mut hasher = DefaultHasher::new();
    query.hash(&mut hasher);
    let hash_code = hasher.finish();
    let hash_code = (hash_code & 0x7FFF_FFFF) as i32;

    // we only track hash codes to avoid holding references to possible
    // large queries; this may cause rare false positives, but at worse
    // this just means we cache a query that was not in fact used enough:
    let mut recently_used_filters = self.recently_used_filters.lock();
    recently_used_filters.add(hash_code);
  }

  fn should_cache(&self, query: &Query) -> Result<bool> {
    if self.should_never_cache(query) {
      return Ok(false);
    }
    let frequency = self.frequency(query);
    let min_frequency = self.min_frequency_to_cache(query);
    Ok(frequency >= min_frequency)
  }
}
pub(crate) fn is_costly(query: &Query) -> bool {
  pub trait QueryCost {
    fn is_costly(&self) -> bool;
  }
  impl QueryCost for Query {
    fn is_costly(&self) -> bool {
      match self {
        // MultiTermQuery
        Query::Prefix(_) => true,
        Query::TermRange(_) => true,
        Query::Automaton(_) => true,
        Query::Wildcard(_) => true,
        Query::Regexp(_) => true,

        Query::MultiTermQueryConstantScoreBlendedWrapper(_) => true,
        Query::MultiTermQueryConstantScoreWrapper(_) => true,
        Query::TermInSet(_) => true,

        Query::PointRange(_) => true,

        _ => {
          #[cfg(debug_assertions)]
          {
            debug_assert!(!self.is_multi_term_query());
          }
          false
        },
      }
    }
  }

  query.is_costly()
}
#[cfg(test)]
pub(crate) mod tests {
  use crate::core::document::document::Document;
  use crate::core::document::int_point::IntPoint;
  use crate::core::index::index_reader::Identity;
  use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
  use crate::core::index::leaf_reader_context::{LeafReaderContext, TopParentMeta};
  use crate::core::index::term::Term;
  use crate::core::search::boolean_clause::Occur;
  use crate::core::search::boolean_query::Builder;
  use crate::core::search::constant_score_scorer::ConstantScoreScorer;
  use crate::core::search::constant_score_weight::ConstantScoreWeight;
  use crate::core::search::doc_id_set_iterator::AllDISI;
  use crate::core::search::explanation::Explanation;
  use crate::core::search::field_exists_query::FieldExistsQuery;
  use crate::core::search::index_searcher::IndexSearcher;
  use crate::core::search::lru_query_cache::LRUQueryCache;
  use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
  use crate::core::search::matches_utils::MatchWithNoTerms;
  use crate::core::search::prefix_query::PrefixQuery;
  use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
  use crate::core::search::query_cache::QueryCacheEnum;
  use crate::core::search::query_caching_policy::QueryCachingPolicy;
  use crate::core::search::query_visitor::QueryVisitor;
  use crate::core::search::score_mode::ScoreMode;
  use crate::core::search::segment_cacheable::SegmentCacheable;
  use crate::core::search::term_query::TermQuery;
  use crate::core::search::usage_tracking_query_caching_policy::{
    UsageTrackingQueryCachingPolicy, is_costly,
  };
  use crate::core::search::weight::{DefaultScorerSupplier, Weight};
  use crate::core::util::HasIdentity;
  use crate::core::util::error::lucene_error::Result;
  use crate::core::util::predicate::Predicate;
  use crate::test::core::index::random_index_writer::RandomIndexWriter;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_searcher_with_reader, random,
  };
  use std::fmt::{Debug, Formatter};
  use std::hash::{Hash, Hasher};
  use std::sync::Arc;

  #[allow(dead_code)] // for quick search
  struct TestUsageTrackingFilterCachingPolicy;
  #[test]
  fn test_costly_filter() -> Result<()> {
    let prefix: Query = PrefixQuery::new(Term::from_text("field", "prefix"))?.into();
    assert!(is_costly(&prefix));

    let point: Query = IntPoint::new_range_query("intField", 1, 1000)?.into();
    assert!(is_costly(&point));

    let term: Query = TermQuery::new(Term::from_text("field", "value")).into();
    assert!(!is_costly(&term));

    Ok(())
  }
  #[test]
  fn test_never_cache_match_all() -> Result<()> {
    let q: Query = MatchAllDocsQuery::new().into();
    let policy = UsageTrackingQueryCachingPolicy::new()?;
    for _ in 0..1000 {
      policy.on_use(&q);
    }
    assert!(!policy.should_cache(&q)?);
    Ok(())
  }
  #[test]
  fn test_never_cache_term_filter() -> Result<()> {
    let q: Query = TermQuery::new(Term::from_text("foo", "bar")).into();
    let policy = UsageTrackingQueryCachingPolicy::new()?;
    for _ in 0..1000 {
      policy.on_use(&q);
    }
    assert!(!policy.should_cache(&q)?);
    Ok(())
  }

  #[test]
  fn test_never_cache_doc_values_field_exists_filter() -> Result<()> {
    let q: Query = FieldExistsQuery::new("foo").into();
    let policy = UsageTrackingQueryCachingPolicy::new()?;
    for _ in 0..1000 {
      policy.on_use(&q);
    }
    assert!(!policy.should_cache(&q)?);
    Ok(())
  }

  // TODO IMPORTANT 测试未通过
  fn test_boolean_queries() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let w = RandomIndexWriter::new(&mut random, dir.clone());

    w.add_document(Document::new())?;
    let reader = w.get_reader()?;
    w.close()?;

    let mut searcher = new_searcher_with_reader(reader)?;
    let policy = UsageTrackingQueryCachingPolicy::new()?;
    let cache = Arc::new(LRUQueryCache::with_skip_cache_factor(
      10,
      i64::MAX,
      f32::INFINITY,
      PredicateImpl,
    )?);
    let cache = QueryCacheEnum::LruImpl(cache);
    searcher.set_query_cache(Some(cache));
    searcher.set_query_caching_policy(policy);

    let q1: Query = DummyQuery1::new(1).into();
    let q2: Query = DummyQuery1::new(2).into();

    let mut builder = Builder::new();
    builder.add(q1.clone(), Occur::Should)?;
    builder.add(q2.clone(), Occur::Should)?;
    let bq = builder.build();

    for _ in 0..3 {
      searcher.count(bq.clone())?;
    }
    let cache = match searcher.get_query_cache().unwrap() {
      QueryCacheEnum::LruImpl(v) => v,
      _ => unreachable!("expected LRUQueryCache"),
    };
    assert_eq!(0, cache.get_cache_size());

    searcher.count(bq.clone())?;
    assert_eq!(1, cache.get_cache_size());

    for _ in 0..10 {
      searcher.count(bq.clone())?;
    }
    assert_eq!(1, cache.get_cache_size());

    searcher.count(q1)?;
    assert_eq!(2, cache.get_cache_size());

    Ok(())
  }

  pub struct PredicateImpl;
  impl Predicate<TopParentMeta> for PredicateImpl {
    fn test(&self, _context: &TopParentMeta) -> Result<bool> {
      Ok(true)
    }
  }

  #[derive(Clone)]
  pub struct DummyQuery1 {
    id: i32,
    identity: Identity,
  }
  impl DummyQuery1 {
    fn new(id: i32) -> Self {
      Self {
        id,
        identity: Identity::new(),
      }
    }
  }
  impl PartialEq for DummyQuery1 {
    fn eq(&self, other: &Self) -> bool {
      self.id == other.id
    }
  }
  impl Eq for DummyQuery1 {}
  impl Hash for DummyQuery1 {
    fn hash<H>(&self, state: &mut H)
    where
      H: Hasher,
    {
      self.id.hash(state);
    }
  }

  impl Debug for DummyQuery1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
      write!(f, "dummy")
    }
  }

  impl HasIdentity for DummyQuery1 {
    fn identity(&self) -> &Identity {
      &self.identity
    }
  }

  impl QueryBase for DummyQuery1 {
    fn as_string(&self, _field: &str) -> Result<String> {
      Ok("dummy".to_string())
    }

    fn create_weight<IRC>(
      self,
      _searcher: &IndexSearcher<IRC>,
      score_mode: &ScoreMode,
      boost: f32,
    ) -> Result<QueryWeight<IRC>>
    where
      IRC: IndexReaderContext,
      Self: Sized,
    {
      let base = ConstantScoreWeight::new(boost);
      Ok(Box::new(DummyQueryWeight1::new(*score_mode, base, self)))
    }

    fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
      IRC: IndexReaderContext,
      Self: Sized,
    {
      Ok(self.into())
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
      QV: QueryVisitor,
    {
      todo!()
    }
  }

  struct DummyQueryWeight1 {
    score_mode: ScoreMode,
    base: ConstantScoreWeight,
    query: Arc<Query>,
  }
  impl DummyQueryWeight1 {
    fn new(score_mode: ScoreMode, base: ConstantScoreWeight, query: DummyQuery1) -> Self {
      let query = Arc::new(query.into());
      Self {
        score_mode,
        base,
        query,
      }
    }
  }

  impl<IRC> SegmentCacheable<IRC> for DummyQueryWeight1
  where
    IRC: IndexReaderContext,
  {
    fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
      Ok(true)
    }
  }

  impl<IRC> Weight<IRC> for DummyQueryWeight1
  where
    IRC: IndexReaderContext,
  {
    type Matches = MatchWithNoTerms;

    fn matches(
      &self,
      context: &LeafReaderContext<IRCLeafReader<IRC>>,
      doc: i32,
      searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::Matches>> {
      self.default_matches(context, doc, searcher)
    }

    fn explain(
      &self,
      context: &LeafReaderContext<IRCLeafReader<IRC>>,
      doc: i32,
      searcher: &IndexSearcher<IRC>,
    ) -> Result<Explanation> {
      let scorer = self.scorer(context, searcher)?;
      self.base.explain(scorer, doc, self.query.as_string("")?)
    }

    fn get_query(&self) -> Arc<Query> {
      self.query.clone()
    }

    type ScorerSupplier = QueryWeightSs<IRC>;

    fn scorer_supplier(
      &self,
      _context: &LeafReaderContext<IRCLeafReader<IRC>>,
      _searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::ScorerSupplier>> {
      let scorer =
        ConstantScoreScorer::from_disi(self.base.score(), self.score_mode, AllDISI::new(1));
      Ok(Some(Box::new(DefaultScorerSupplier::new(scorer))))
    }
  }
}
