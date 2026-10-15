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

use crate::core::search::approximation_disi::ApproximationDISI;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;

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
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryWeight, QueryWeightMatches};
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::scorer_util::ScorerUtil;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::sync::Arc;

/// Expert: Calculate query weights and build query scorers.
///
/// The purpose of [`Weight`] is to ensure searching does not modify a [`Query`],
/// so that a [`Query`] instance can be reused.
///
/// - [`IndexSearcher`]-dependent state of the query should reside in the [`Weight`].
/// - [`LeafReader`]-dependent state should reside in the [`Scorer`].
///
/// Since [`Weight`] creates [`Scorer`] instances for a given [`LeafReaderContext`]
/// (via [`Weight::scorer`]), callers must maintain the relationship between the
/// searcher's top-level [`IndexReaderContext`] and the context used to create a
/// [`Scorer`].
///
/// A [`Weight`] is used in the following way:
///
/// 1. A [`Weight`] is constructed by a top-level query, given an [`IndexSearcher`]
///    (see [`QueryBase::create_weight`](crate::core::search::query::QueryBase::create_weight)).
/// 2. A [`Scorer`] is constructed by [`Weight::scorer`].
pub trait Weight<IRC, SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity>:
  SegmentCacheable<IRC>
where
  SIM: Similarity,
  IRC: IndexReaderContext,
{
  /// Returns [`Matches`](crate::core::search::matches::Matches) for a specific document, or `None`
  /// if the document does not match the parent query.
  ///
  /// A query match that contains no position information (for example, a
  /// Point or DocValues query) will return
  /// [`MATCH_WITH_NO_TERMS`](crate::core::search::matches_utils::MATCH_WITH_NO_TERMS).
  ///
  /// # Parameters
  /// - `context`: the reader's context to create the
  ///   [`Matches`](crate::core::search::matches::Matches) for
  /// - `doc`: the document's id relative to the given context's reader
  fn matches<'a>(
    &'a self,
    context: &'a LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &'a IndexSearcher<IRC, SIM>,
  ) -> Result<Option<QueryWeightMatches<'a, IRC, SIM>>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    let scorer_supplier = self.scorer_supplier(context, searcher)?;
    let mut scorer_supplier = match scorer_supplier {
      None => return Ok(None),
      Some(s) => s,
    };

    let mut scorer = scorer_supplier.get(1, context, searcher)?;
    if scorer.has_two_phase_iterator() == crate::core::search::scorer::TwoPhaseState::Yes {
      if scorer.approx_advance(doc)? != doc || !scorer.matches()? {
        return Ok(None);
      }
    } else if scorer.advance(doc)? != doc {
      return Ok(None);
    }
    Ok(Some(QueryWeightMatches::MatchWithNoTerms(MatchWithNoTerms)))
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
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Explanation>
  where
    IndexSearcher<IRC, SIM>: Sync;

  fn get_query(&self) -> Arc<Query>;

  /// Optional method that delegates to [`Weight::scorer_supplier`].
  ///
  /// Returns a [`Scorer`] which can iterate in order over all matching documents
  /// and assign them a score. A scorer for the same [`LeafReaderContext`] instance
  /// may be requested multiple times as part of a single search call.
  ///
  /// # Notes
  ///
  /// - May return `None` if no documents will be scored by this query.
  /// - The returned [`Scorer`] does **not** have [`LeafReader::get_live_docs`]
  ///   applied; callers must check live docs on top.
  ///
  /// # Parameters
  ///
  /// - `context`: the [`LeafReaderContext`] for which to return the [`Scorer`].
  ///
  /// # Returns
  ///
  /// An optional [`Scorer`] which scores documents in/out-of-order.
  ///
  /// # Errors
  ///
  /// Returns an error if a low-level I/O error occurs.
  fn scorer(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<IRC, SIM>>::Scorer>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    let mut scorer_supplier = match self.scorer_supplier(context, searcher)? {
      None => return Ok(None),
      Some(s) => s,
    };
    Ok(Some(scorer_supplier.get(i64::MAX, context, searcher)?))
  }

  type ScorerSupplier: ScorerSupplier<IRC, SIM>;
  /// Get a [`ScorerSupplier`], which allows knowing the cost of the [`Scorer`]
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
  /// - [`Scorer`]
  /// - [`DefaultScorerSupplier`]
  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<Self::ScorerSupplier>>
  where
    IndexSearcher<IRC, SIM>: Sync;
  /// Helper method that delegates to [`Weight::scorer_supplier`].
  ///
  /// A bulk scorer for the same [`LeafReaderContext`] instance may be requested
  /// multiple times as part of a single search call.
  fn bulk_scorer(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<IRC, SIM>>::BulkScorer>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
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
  /// - Specific query types should implement this to provide other accurate
  ///   sub-linear implementations (that actually return the count).
  ///   For example, see how [`QueryBase::create_weight`](crate::core::search::query::QueryBase::create_weight) does it.
  /// - This method is used by [`IndexSearcher::count`](crate::core::search::index_searcher::IndexSearcher::count) to count hits.
  ///
  /// # Parameters
  ///
  /// - `context`: the [`LeafReaderContext`] for which to return the count.
  /// - `searcher`: the searcher to use when counting requires creating a scorer.
  ///
  /// # Returns
  ///
  /// An integer count of the number of matches, or `-1` if it cannot be
  /// determined efficiently.
  ///
  /// # Errors
  ///
  /// Returns an error if a low-level I/O error occurs.
  fn count(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<i32>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    self.default_count(context)
  }
  fn default_count(&self, _context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
    Ok(-1)
  }

  fn is_cache_wrapper(&self) -> bool {
    false
  }

  fn into_inner_weight(self: Box<Self>) -> Option<QueryWeight<IRC, SIM>>
  where
    IRC: 'static,
  {
    None
  }

  #[cfg(test)]
  fn as_any(&mut self) -> &mut dyn std::any::Any {
    unreachable!("")
  }
}
impl<IRC, T, SIM> Weight<IRC, SIM> for Box<T>
where
  SIM: Similarity,
  IRC: IndexReaderContext,
  T: Weight<IRC, SIM> + ?Sized,
{
  fn matches<'a>(
    &'a self,
    context: &'a LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &'a IndexSearcher<IRC, SIM>,
  ) -> Result<Option<QueryWeightMatches<'a, IRC, SIM>>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    (**self).matches(context, doc, searcher)
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Explanation>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    (**self).explain(context, doc, searcher)
  }

  fn get_query(&self) -> Arc<Query> {
    (**self).get_query()
  }

  fn scorer(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<IRC, SIM>>::Scorer>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    (**self).scorer(context, searcher)
  }

  type ScorerSupplier = T::ScorerSupplier;

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<Self::ScorerSupplier>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    (**self).scorer_supplier(context, searcher)
  }

  fn bulk_scorer(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<IRC, SIM>>::BulkScorer>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    (**self).bulk_scorer(context, searcher)
  }

  fn count(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<i32>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    (**self).count(context, searcher)
  }

  fn default_count(&self, _context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
    (**self).default_count(_context)
  }

  fn is_cache_wrapper(&self) -> bool {
    (**self).is_cache_wrapper()
  }

  fn into_inner_weight(self: Box<Self>) -> Option<QueryWeight<IRC, SIM>>
  where
    IRC: 'static,
  {
    (*self).into_inner_weight()
  }

  #[cfg(test)]
  fn as_any(&mut self) -> &mut dyn std::any::Any {
    (**self).as_any()
  }
}

/// Just wraps a Scorer and performs top scoring using it.
pub struct DefaultBulkScorer<S> {
  pub(crate) scorer: S,
}
impl<S> DefaultBulkScorer<S> {
  pub fn new(scorer: S) -> Self {
    Self { scorer }
  }
}
impl<S> BulkScorer for DefaultBulkScorer<S>
where
  S: Scorer,
{
  fn score<CLC: crate::core::search::leaf_collector::LeafCollector, AB: Bits + ?Sized>(
    &mut self,
    collector: &mut CLC,
    accept_docs: Option<&AB>,
    min: i32,
    max: i32,
  ) -> Result<i32> {
    collector.set_scorer(&mut self.scorer)?;
    let has_two_phase = self.scorer.has_two_phase_iterator() == TwoPhaseState::Yes;
    let doc_id = self.scorer.approx_doc_id();

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
    DocIdSetIterator::cost(&self.scorer)
  }

  #[cfg(test)]
  fn kind(&self) -> BulkScorerKind {
    Default
  }
}
pub struct DefaultScorerSupplier<S> {
  pub(crate) scorer: Option<S>,
}
impl<S> DefaultScorerSupplier<S> {
  pub fn new(scorer: S) -> Self {
    Self {
      scorer: Some(scorer),
    }
  }
}
impl<S, IRC, SIM> ScorerSupplier<IRC, SIM> for DefaultScorerSupplier<S>
where
  SIM: Similarity,
  IRC: IndexReaderContext + 'static,
  IRCLeafReader<IRC>: LeafReader,
  S: Scorer + 'static,
{
  type Scorer = S;
  type BulkScorer = DefaultBulkScorer<S>;

  fn get(
    &mut self,
    _lead_cost: i64,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Self::Scorer>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    let v = self
      .scorer
      .take()
      .ok_or_else(|| LuceneError::illegal_state("ScorerSupplier::get returned None"))?;
    Ok(v)
  }

  fn bulk_scorer(
    &mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<Option<Self::BulkScorer>>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    // Select the concrete bulk scorer before document iteration.
    let scorer = self
      .scorer
      .take()
      .ok_or_else(|| LuceneError::illegal_state("ScorerSupplier::get returned None"))?;
    Ok(Some(DefaultBulkScorer::new(scorer)))
  }

  fn cost(
    &mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC, SIM>,
  ) -> Result<i64>
  where
    IndexSearcher<IRC, SIM>: Sync,
  {
    let scorer = self
      .scorer
      .as_mut()
      .ok_or_else(|| LuceneError::illegal_state("DefaultScorer::get returned None"))?;
    scorer.cost()
  }
}
/// Specialized method to bulk-score all hits;
/// we separate this from scoreRange to help out hotspot. See [`LUCENE-5487`](https://issues.apache.org/jira/browse/LUCENE-5487">LUCENE-5487)
fn score_all<S, CLC: crate::core::search::leaf_collector::LeafCollector, AB: Bits + ?Sized>(
  collector: &mut CLC,
  accept_docs: Option<&AB>,
  scorer: &mut S,
  has_two_phase: bool,
) -> Result<()>
where
  S: Scorer,
{
  if !has_two_phase {
    loop {
      let doc = scorer.next_doc()?;
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
        let matches = scorer.matches()?;
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
fn score_range<S, CLC: crate::core::search::leaf_collector::LeafCollector, AB: Bits + ?Sized>(
  collector: &mut CLC,
  accept_docs: Option<&AB>,
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
    let mut doc = scorer.approx_doc_id();
    if doc < min {
      if doc == min - 1 {
        doc = scorer.approx_next_doc()?;
      } else {
        doc = scorer.approx_advance(min)?
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
      doc = scorer.next_doc()?;
    }
  } else {
    while doc < max {
      // Reborrow a present iterator after collecting, but retain initial absence for the window.
      if has_competitive && let Some(mut competitive_iterator) = collector.competitive_iterator()? {
        debug_assert!(competitive_iterator.doc_id() <= doc);
        let mut competitive_doc = competitive_iterator.doc_id();
        if competitive_doc < doc {
          competitive_doc = competitive_iterator.advance(doc)?;
        }
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
        let matches = !has_two_phase || scorer.matches()?;
        if matches {
          collector.collect(doc, scorer)?;
        }
      }
      doc = ScorerUtil::next_doc(scorer)?;
    }
  }

  Ok(doc)
}
