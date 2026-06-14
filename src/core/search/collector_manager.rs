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
use crate::core::search::collector::Collector;

/// A manager of collectors. This trait is useful to parallelize execution of search requests and
/// has two main methods:
///
/// - [`new_collector`](CollectorManager::new_collector) which must return a **new** collector that will be used to collect a certain
///   set of leaves.
/// - [`reduce`](CollectorManager::reduce) which will be used to reduce the results of individual collections into a
///   meaningful result. This method is only called **after all leaves have been fully collected**.
///
/// **Note:** Multiple [`LeafCollector`](crate::core::search::leaf_collector::LeafCollector)s may be requested for the same [`LeafReaderContext`](crate::core::index::leaf_reader_context::LeafReaderContext)
/// via `Collector::get_leaf_collector(...)` across the different collectors returned by
/// [`new_collector`](CollectorManager::new_collector). Any computation or logic that needs to happen **once per segment**
/// requires specific handling in the collector manager implementation, because the collection of an
/// entire segment may be split across threads.
///
/// See also: `IndexSearcher::search(query, manager)`
use crate::core::util::error::lucene_error::Result;
/// A manager of collectors. This trait supports parallel search execution and has
/// two main methods:
///
/// - [`CollectorManager::new_collector`], which must return a NEW collector which will be used to
///   collect a certain set of leaves.
/// - [`CollectorManager::reduce`], which will be used to reduce the results of individual
///   collections into a meaningful result. This method is only called after all leaves have been
///   fully collected.
///
/// **Note:** Multiple [`LeafCollector`]s may be requested for the same [`LeafReaderContext`] via
/// [`Collector::get_leaf_collector`] across the different [`Collector`]s returned by
/// [`CollectorManager::new_collector`]. Any computation or logic that needs to happen once per
/// segment requires specific handling in the collector manager implementation, because the
/// collection of an entire segment may be split across threads.
///
/// See also [`IndexSearcher::search`].
pub trait CollectorManager {
  /// The per-shard/per-task collector type to create.
  type C: Collector;

  /// The final reduced result type.
  type T;

  /// Return a new collector. This **must return a different instance on each call**.
  fn new_collector(&self) -> Result<Self::C>;

  /// Reduce the results of individual collectors into a meaningful result.
  ///
  /// For instance, a `TopDocsCollector` would compute the `top_docs()` of each collector
  /// and then merge them, similar to `TopDocs::merge(...)`. This **must be called after**
  /// collection is finished on all provided collectors.
  fn reduce(&self, collectors: Vec<Self::C>) -> Result<Self::T>;
}

impl<M> CollectorManager for &M
where
  M: CollectorManager + ?Sized,
{
  type C = M::C;
  type T = M::T;

  fn new_collector(&self) -> Result<Self::C> {
    (**self).new_collector()
  }

  fn reduce(&self, collectors: Vec<Self::C>) -> Result<Self::T> {
    (**self).reduce(collectors)
  }
}
