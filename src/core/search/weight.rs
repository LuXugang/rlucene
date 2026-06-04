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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::bulk_scorer::BulkScorer;
#[cfg(test)]
use crate::core::search::bulk_scorer::BulkScorerKind;
#[cfg(test)]
use crate::core::search::bulk_scorer::BulkScorerKind::Default;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::matches::Matches;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryWeightSsBulkScorer, QueryWeightSsScorer};
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::scorer_util::ScorerUtil;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::sync::Arc;

/// Expert: Calculate query weights and build query scorers.
///
/// The purpose of [`Weight`] is to ensure searching does not modify a [`Query`],
/// so that a [`Query`] instance can be reused.
///
/// - `IndexSearcher`-dependent state of the query should reside in the `Weight`.
/// - [`LeafReader`]-dependent state should reside in the `Scorer`.
///
/// Since [`Weight`] creates `Scorer` instances for a given [`LeafReaderContext`]
/// (via [`Weight::scorer`]), callers must maintain the relationship between the
/// searcher's top-level `IndexReaderContext` and the context used to create a
/// `Scorer`.
///
/// A `Weight` is used in the following way:
///
/// 1. A `Weight` is constructed by a top-level query, given an `IndexSearcher`
///    (see `Query::create_weight`).
/// 2. A `Scorer` is constructed by [`Weight::scorer`].
pub trait Weight<IRC>: SegmentCacheable<IRC>
where
  IRC: IndexReaderContext,
{
  type Matches: Matches;
  /// Returns [`Matches`] for a specific document, or `None` if the document
  /// does not match the parent query.
  ///
  /// A query match that contains no position information (for example, a
  /// Point or DocValues query) will return
  /// `MatchesUtils::MATCH_WITH_NO_TERMS`.
  ///
  /// # Parameters
  /// - `context`: the reader's context to create the [`Matches`] for
  /// - `doc`: the document's id relative to the given context's reader
  fn matches(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::Matches>>;
  fn default_matches(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<MatchWithNoTerms>> {
    let scorer_supplier = self.scorer_supplier(context, searcher)?;
    let mut scorer_supplier = match scorer_supplier {
      None => return Ok(None),
      Some(s) => s,
    };

    let mut scorer = scorer_supplier.get(1, context, searcher)?;
    if let Some(mut two_phase) = scorer.two_phase_iterator_mut() {
      if two_phase.approximation_mut().advance(doc)? != doc || !two_phase.matches()? {
        return Ok(None);
      }
    } else if scorer.iterator_mut().advance(doc)? != doc {
      return Ok(None);
    }
    Ok(Some(MatchWithNoTerms))
  }

  /// An explanation of the score computation for the named document.
  ///
  /// # Parameters
  /// - `context`: the reader's context to create the [`Explanation`] for
  /// - `doc`: the document's id relative to the given context's reader
  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation>;

  fn get_query(&self) -> Arc<Query>;

  /// Optional method that delegates to [`Weight::scorer_supplier`].
  ///
  /// Returns a `Scorer` which can iterate in order over all matching documents
  /// and assign them a score. A scorer for the same [`LeafReaderContext`] instance
  /// may be requested multiple times as part of a single search call.
  ///
  /// # Notes
  ///
  /// - May return `None` if no documents will be scored by this query.
  /// - The returned `Scorer` does **not** have [`LeafReader::get_live_docs`]
  ///   applied; callers must check live docs on top.
  ///
  /// # Parameters
  ///
  /// - `context`: the [`LeafReaderContext`] for which to return the `Scorer`.
  ///
  /// # Returns
  ///
  /// An optional `Scorer` which scores documents in/out-of-order.
  ///
  /// # Errors
  ///
  /// Returns an error if a low-level I/O error occurs.
  fn scorer(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<IRC>>::Scorer>> {
    let mut scorer_supplier = match self.scorer_supplier(context, searcher)? {
      None => return Ok(None),
      Some(s) => s,
    };
    Ok(Some(scorer_supplier.get(i64::MAX, context, searcher)?))
  }

  type ScorerSupplier: ScorerSupplier<IRC>;
  /// Get a [`ScorerSupplier`], which allows knowing the cost of the `Scorer`
  /// before building it.
  ///
  /// A scorer supplier for the same [`LeafReaderContext`] instance may be requested
  /// multiple times as part of a single search call.
  ///
  /// # Notes
  ///
  /// - Must return `None` if the scorer is `None`.
  ///
  /// # Parameters
  ///
  /// - `context`: the leaf reader context
  ///
  /// # Returns
  ///
  /// A [`ScorerSupplier`] providing the scorer, or `None` if the scorer is absent.
  ///
  /// # Errors
  ///
  /// Returns an error if a low-level I/O error occurs.
  ///
  /// # See also
  ///
  /// - `Scorer`
  /// - [`DefaultScorerSupplier`]
  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>>;
  /// Helper method that delegates to [`Weight::scorer_supplier`].
  ///
  /// A bulk scorer for the same [`LeafReaderContext`] instance may be requested
  /// multiple times as part of a single search call.
  fn bulk_scorer(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<IRC>>::BulkScorer>> {
    let mut scorer_supplier = match self.scorer_supplier(context, searcher)? {
      None => return Ok(None),
      Some(s) => s,
    };

    scorer_supplier.set_top_level_scoring_clause()?;
    scorer_supplier.bulk_scorer(context, searcher)
  }

  /// Counts the number of live documents that match this weight's parent query
  /// in a leaf.
  ///
  /// # Default
  ///
  /// The default implementation returns `-1` for every query. This indicates
  /// that the count could not be computed in sub-linear time.
  ///
  /// # Notes
  ///
  /// - Specific query classes should override this to provide other accurate
  ///   sub-linear implementations (that actually return the count).
  ///   For example, see how `MatchAllDocsQuery::create_weight` does it.
  /// - This method is used by [`IndexSearcher::count`](crate::core::search::index_searcher::IndexSearcher::count) to count hits.
  ///
  /// # Parameters
  ///
  /// - `context`: the [`LeafReaderContext`] for which to return the count.
  ///
  /// # Returns
  ///
  /// An integer count of the number of matches, or `-1` if it cannot be
  /// determined efficiently.
  ///
  /// # Errors
  ///
  /// Returns an error if a low-level I/O error occurs.
  fn count(&self, context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
    self.default_count(context)
  }
  fn default_count(&self, _context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
    Ok(-1)
  }

  #[cfg(test)]
  fn as_any(&mut self) -> &mut dyn std::any::Any {
    unreachable!("")
  }
}
impl<IRC, T> Weight<IRC> for Box<T>
where
  IRC: IndexReaderContext,
  T: Weight<IRC> + ?Sized,
{
  type Matches = T::Matches;

  fn matches(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::Matches>> {
    (**self).matches(context, doc, searcher)
  }

  fn default_matches(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<MatchWithNoTerms>> {
    (**self).default_matches(context, doc, searcher)
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    (**self).explain(context, doc, searcher)
  }

  fn get_query(&self) -> Arc<Query> {
    (**self).get_query()
  }

  fn scorer(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<IRC>>::Scorer>> {
    (**self).scorer(context, searcher)
  }

  type ScorerSupplier = T::ScorerSupplier;

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    (**self).scorer_supplier(context, searcher)
  }

  fn bulk_scorer(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<IRC>>::BulkScorer>> {
    (**self).bulk_scorer(context, searcher)
  }

  fn count(&self, context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
    (**self).count(context)
  }

  fn default_count(&self, _context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
    (**self).default_count(_context)
  }
  #[cfg(test)]
  fn as_any(&mut self) -> &mut dyn std::any::Any {
    (**self).as_any()
  }
}

/// Just wraps a Scorer and performs top scoring using it.
pub struct DefaultBulkScorer<S>
where
  S: Scorer,
{
  scorer: S,
}
impl<S> DefaultBulkScorer<S>
where
  S: Scorer,
{
  pub fn new(scorer: S) -> Self {
    Self { scorer }
  }
}
impl<S> BulkScorer for DefaultBulkScorer<S>
where
  S: Scorer,
{
  fn score(
    &mut self,
    collector: &mut dyn LeafCollector,
    accept_docs: Option<&dyn Bits>,
    min: i32,
    max: i32,
  ) -> Result<i32> {
    collector.set_scorer(&mut self.scorer)?;
    let has_two_phase = self.scorer.has_two_phase_iterator() == TwoPhaseState::Yes
      || self.scorer.two_phase_iterator().is_some();
    let doc_id = self.scorer.approximation().doc_id();

    let has_competitive_iterator = {
      let opt = collector.competitive_iterator()?;
      opt.is_some()
    };

    if !has_competitive_iterator
      && doc_id == -1
      && accept_docs.is_none()
      && min == 0
      && max == NO_MORE_DOCS
    {
      score_all(collector, accept_docs, &mut self.scorer, has_two_phase)?;
      Ok(NO_MORE_DOCS)
    } else {
      score_range(
        collector,
        accept_docs,
        min,
        max,
        &mut self.scorer,
        has_competitive_iterator,
        has_two_phase,
      )
    }
  }

  fn cost(&mut self) -> Result<i64> {
    self.scorer.iterator_mut().cost()
  }

  #[cfg(test)]
  fn kind(&self) -> BulkScorerKind {
    Default
  }
}
pub struct DefaultScorerSupplier<S>
where
  S: Scorer,
{
  scorer: Option<S>,
}
impl<S> DefaultScorerSupplier<S>
where
  S: Scorer,
{
  pub fn new(scorer: S) -> Self {
    Self {
      scorer: Some(scorer),
    }
  }
}
impl<S, IRC> ScorerSupplier<IRC> for DefaultScorerSupplier<S>
where
  IRC: IndexReaderContext,
  IRCLeafReader<IRC>: LeafReader,
  S: Scorer + 'static,
{
  type Scorer = QueryWeightSsScorer;
  type BulkScorer = QueryWeightSsBulkScorer;

  fn get(
    &mut self,
    _lead_cost: i64,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::Scorer> {
    let v = self
      .scorer
      .take()
      .ok_or_else(|| LuceneError::illegal_state("ScorerSupplier::get returned None"))?;
    Ok(Box::new(v))
  }

  fn bulk_scorer(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::BulkScorer>> {
    Ok(Some(Box::new(self.default_bulk_scorer(context, searcher)?)))
  }

  fn cost(
    &mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<i64> {
    let scorer = self
      .scorer
      .as_mut()
      .ok_or_else(|| LuceneError::illegal_state("DefaultScorer::get returned None"))?;
    scorer.iterator().cost()
  }
}
/// Specialized method to bulk-score all hits;
/// we separate this from scoreRange to help out hotspot. See [`LUCENE-5487`](https://issues.apache.org/jira/browse/LUCENE-5487">LUCENE-5487)
fn score_all<S>(
  collector: &mut dyn LeafCollector,
  accept_docs: Option<&dyn Bits>,
  scorer: &mut S,
  has_two_phase: bool,
) -> Result<()>
where
  S: Scorer,
{
  if !has_two_phase {
    loop {
      let doc = ScorerUtil::next_doc(scorer)?;
      if doc == NO_MORE_DOCS {
        break;
      }
      let is_accept = match accept_docs {
        None => true,
        Some(a) => a.get(doc as usize)?,
      };
      if is_accept {
        collector.collect(doc, scorer)?;
      }
    }
  } else {
    // The scorer has an approximation, so run the approximation first, then check acceptDocs,
    // then confirm
    loop {
      let doc = ScorerUtil::next_doc(scorer)?;
      if doc == NO_MORE_DOCS {
        break;
      }
      let is_accept = match accept_docs {
        None => true,
        Some(a) => a.get(doc as usize)?,
      };
      if is_accept {
        let matches = {
          let tpi = scorer.two_phase_iterator_mut();
          match tpi {
            Some(mut two_phase) => two_phase.matches()?,
            None => {
              return Err(LuceneError::illegal_state(
                "TwoPhaseIterator should not None",
              ));
            },
          }
        };
        if matches {
          collector.collect(doc, scorer)?;
        }
      }
    }
  }
  Ok(())
}
/// Specialized method to bulk-score a range of hits;
/// we separate this from scoreAll to help out hotspot. See [`LUCENE-5487`](https://issues.apache.org/jira/browse/LUCENE-5487">LUCENE-5487)
fn score_range<S>(
  collector: &mut dyn LeafCollector,
  accept_docs: Option<&dyn Bits>,
  mut min: i32,
  max: i32,
  scorer: &mut S,
  has_competitive: bool,
  has_two_phase: bool,
) -> Result<i32>
where
  S: Scorer,
{
  if has_competitive {
    let mut opt = collector.competitive_iterator()?;
    if let Some(iterator) = opt.as_mut() {
      if iterator.doc_id() > min {
        // The competitive iterator may not match any docs in the range.
        min = iterator.doc_id().min(max);
      }
    } else {
      return Err(LuceneError::illegal_state(
        "has_competitive is true but competitive_iterator is None",
      ));
    }
  }
  let mut doc = {
    let mut iterator = scorer.approximation_mut();
    let mut doc = iterator.doc_id();
    if doc < min {
      if doc == min - 1 {
        doc = iterator.next_doc()?;
      } else {
        doc = iterator.advance(min)?
      }
    }
    doc
  };

  if !has_two_phase && !has_competitive {
    // Optimize simple iterators with collectors that can't skip
    while doc < max {
      let is_accept = match accept_docs {
        None => true,
        Some(a) => a.get(doc as usize)?,
      };
      if is_accept {
        collector.collect(doc, scorer)?;
      }
      doc = ScorerUtil::next_doc(scorer)?;
    }
  } else {
    while doc < max {
      // competitive_iterator may be updated by collector.collect
      if let Some(mut competitive_iterator) = collector.competitive_iterator()? {
        debug_assert!(competitive_iterator.doc_id() <= doc);
        let mut competitive_doc = competitive_iterator.doc_id();
        if competitive_doc < doc {
          competitive_iterator.advance(doc)?;
        }
        competitive_doc = competitive_iterator.doc_id();
        if competitive_doc != doc {
          doc = ScorerUtil::advance(scorer, competitive_doc)?;
          continue;
        }
      }
      let is_accept = match accept_docs {
        None => true,
        Some(a) => a.get(doc as usize)?,
      };
      if is_accept {
        let matches = if has_two_phase {
          let mut two_phase = scorer.two_phase_iterator_mut().unwrap();
          two_phase.matches()?
        } else {
          true
        };
        if matches {
          collector.collect(doc, scorer)?;
        }
      }
      doc = ScorerUtil::next_doc(scorer)?;
    }
  }

  Ok(doc)
}
