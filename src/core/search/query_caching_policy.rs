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
use crate::core::search::usage_tracking_query_caching_policy::UsageTrackingQueryCachingPolicy;
use crate::core::util::error::lucene_error::Result;
use crate::impl_from_for_enum;
use std::sync::Arc;

/// A policy defining which filters should be cached.
///
/// Implementations of this trait must be thread-safe.
///
/// See also: [`UsageTrackingQueryCachingPolicy`], `LRUQueryCache`.
// TODO: add APIs for integration with `IndexWriter::IndexReaderWarmer`
pub trait QueryCachingPolicy {
  /// Callback that is called every time that a cached filter is used.
  /// This is typically useful if the policy wants to track usage statistics
  /// in order to make decisions.
  fn on_use(&self, query: &Query);

  /// Whether the given [`Query`] is worth caching.
  ///
  /// This method will be called by the `QueryCache` to know whether to cache.
  /// It will first attempt to load a [`DocIdSet`](crate::core::search::doc_id_set::DocIdSet) from the cache. If it is not cached yet
  /// and this method returns `true` then a cache entry will be generated.
  /// Otherwise an uncached scorer will be returned.
  fn should_cache(&self, query: &Query) -> Result<bool>;
}

impl<T> QueryCachingPolicy for &T
where
  T: QueryCachingPolicy,
{
  fn on_use(&self, query: &Query) {
    (**self).on_use(query)
  }

  fn should_cache(&self, query: &Query) -> Result<bool> {
    (**self).should_cache(query)
  }
}
impl<T> QueryCachingPolicy for Arc<T>
where
  T: QueryCachingPolicy,
{
  fn on_use(&self, query: &Query) {
    (**self).on_use(query)
  }

  fn should_cache(&self, query: &Query) -> Result<bool> {
    (**self).should_cache(query)
  }
}
pub type DynQueryCachingPolicy = dyn QueryCachingPolicy + Send + Sync;
pub type CustomQueryCachingPolicy = Box<DynQueryCachingPolicy>;

pub enum QueryCachingPolicyEnum {
  UsageTracking(UsageTrackingQueryCachingPolicy),
  Custom(CustomQueryCachingPolicy),
}

impl QueryCachingPolicyEnum {
  pub fn custom<P>(p: P) -> Self
  where
    P: QueryCachingPolicy + Send + Sync + 'static,
  {
    Self::Custom(Box::new(p))
  }
}
impl_from_for_enum!(QueryCachingPolicyEnum, UsageTrackingQueryCachingPolicy => UsageTracking);
impl QueryCachingPolicy for QueryCachingPolicyEnum {
  fn on_use(&self, query: &Query) {
    match self {
      Self::UsageTracking(inner) => inner.on_use(query),
      Self::Custom(inner) => inner.on_use(query),
    }
  }

  fn should_cache(&self, query: &Query) -> Result<bool> {
    match self {
      Self::UsageTracking(inner) => inner.should_cache(query),
      Self::Custom(inner) => inner.should_cache(query),
    }
  }
}

pub trait QueryCachingPolicyArc {
  fn into_query_cache_policy_arc(self) -> Arc<QueryCachingPolicyEnum>;
}

impl QueryCachingPolicyArc for Arc<QueryCachingPolicyEnum> {
  fn into_query_cache_policy_arc(self) -> Arc<QueryCachingPolicyEnum> {
    self
  }
}

impl<T> QueryCachingPolicyArc for T
where
  T: QueryCachingPolicy + Into<QueryCachingPolicyEnum>,
{
  fn into_query_cache_policy_arc(self) -> Arc<QueryCachingPolicyEnum> {
    Arc::new(self.into())
  }
}
