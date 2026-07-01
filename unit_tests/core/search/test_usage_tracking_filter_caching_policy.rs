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
use crate::core::search::query::{IntoQuery, Query, QueryBase, QueryWeight, QueryWeightSs};
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
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
pub use crate::test::support::core::search::query::DummyQuery1;
use crate::test::support::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_reader, random,
};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestUsageTrackingFilterCachingPolicy;
#[test]
fn test_costly_filter() -> Result<()> {
  let prefix: Query = PrefixQuery::new(Term::from_text("field", "prefix"))?.into_query();
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

#[test]
fn test_boolean_queries() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone())?;

  w.add_document(&mut random, Document::new())?;
  let reader = w.get_reader(&mut random)?;
  w.close(&mut random)?;

  let mut searcher = new_searcher_with_reader(reader)?;
  let policy = UsageTrackingQueryCachingPolicy::new()?;
  let cache = Arc::new(LRUQueryCache::with_skip_cache_factor(
    10,
    i64::MAX,
    f32::INFINITY,
    PredicateImpl,
  )?);
  searcher.set_query_cache(Some(QueryCacheEnum::custom(cache.clone())));
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
