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
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::weight::Weight;
/// Expert: Collectors are primarily meant to be used to gather raw results from a search,
/// and implement sorting or custom result filtering, collation, etc.
///
/// Lucene's core collectors are derived from [`Collector`] and [`SimpleCollector`](crate::core::search::simple_collector::SimpleCollector).
/// Applications can usually use one of these collector types or implement [`TopDocsCollector`](crate::core::search::top_docs_collector::TopDocsCollector),
/// instead of implementing Collector directly:
///
/// - [`TopDocsCollector`](crate::core::search::top_docs_collector::TopDocsCollector) is a base trait that assumes you will retrieve the top N
///   docs, according to some criteria, after collection is done.
/// - `TopScoreDocCollector` implements [`TopDocsCollector`](crate::core::search::top_docs_collector::TopDocsCollector) and sorts
///   according to score + docID. This is used internally by the [`IndexSearcher`](crate::core::search::index_searcher::IndexSearcher) search
///   methods that do not take an explicit `Sort`(crate::core::index::sort::Sort). It is likely the most frequently used
///   collector.
/// - [`TopFieldCollector`](crate::core::search::top_field_collector::TopFieldCollector) implements [`TopDocsCollector`](crate::core::search::top_docs_collector::TopDocsCollector) and sorts according to a
///   specified `Sort`(crate::core::index::sort::Sort) object (sort by field). This is used internally by the
///   [`IndexSearcher`](crate::core::search::index_searcher::IndexSearcher) search methods that take an explicit `Sort`(crate::core::index::sort::Sort).
/// - [`PositiveScoresOnlyCollector`](crate::core::search::positive_scores_only_collector::PositiveScoresOnlyCollector) wraps any other Collector and prevents collection of
///   hits whose score is <= 0.0
use crate::core::util::error::lucene_error::Result;

pub trait Collector {
  type LeafCollector<'a, IRC>: LeafCollector
  where
    Self: 'a,
    IRC: IndexReaderContext;
  /// Create a new [`LeafCollector`] to collect the given context.
  ///
  /// Set the [`Weight`] that will be used to produce scorers that will feed [`LeafCollector`]s.
  /// This is typically useful to have access to [`Weight::count`] from [`Collector::get_leaf_collector`].
  /// # Arguments
  /// * `context` - next atomic reader context
  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized;

  /// Indicates what features are required from the scorer.
  fn score_mode(&self) -> ScoreMode;

  fn set_weight<W, IRC>(&self, _weight: Option<&W>) -> Result<()>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    Ok(())
  }
}
