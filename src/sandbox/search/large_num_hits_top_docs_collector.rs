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
use crate::core::search::collector::Collector;
use crate::core::search::hit_queue::{self, HitQueueComparator};
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::top_docs::TopDocs;
use crate::core::search::top_docs_collector::EMPTY_TOP_DOCS;
use crate::core::search::total_hits::{Relation, TotalHits};
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::PriorityQueue;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

/// Optimized collector for large number of hits.
///
/// The collector maintains a vector of hits until it accumulates the requested
/// number of hits. After that, it builds a priority queue and starts filtering
/// further hits based on the minimum competitive score.
pub struct LargeNumHitsTopDocsCollector {
  requested_hit_count: usize,
  hits: Option<Vec<ScoreDoc>>,
  pub(crate) pq: Option<PriorityQueue<ScoreDoc, HitQueueComparator>>,
  pub(crate) total_hits: usize,
}

impl LargeNumHitsTopDocsCollector {
  pub fn new(requested_hit_count: usize) -> Self {
    Self {
      requested_hit_count,
      hits: Some(Vec::new()),
      pq: None,
      total_hits: 0,
    }
  }

  pub(crate) fn total_hits(&self) -> usize {
    self.total_hits
  }

  pub(crate) fn pq(&self) -> Option<&PriorityQueue<ScoreDoc, HitQueueComparator>> {
    self.pq.as_ref()
  }

  pub(crate) fn pq_top(&self) -> Option<&ScoreDoc> {
    self.pq.as_ref().and_then(|pq| pq.top())
  }

  /** Returns the top docs that were collected by this collector. */
  pub fn top_docs_with_how_many(&mut self, how_many: usize) -> Result<TopDocs<ScoreDoc>> {
    if how_many == 0 || how_many > self.total_hits {
      return Err(LuceneError::illegal_argument(
        "Incorrect number of hits requested",
      ));
    }

    let mut results = vec![ScoreDoc::default(); how_many];
    self.populate_results(&mut results, how_many)?;

    Ok(self.new_top_docs(Some(results)))
  }
  /**
   * Populates the results array with the ScoreDoc instances. This can be
   * overridden in case a different ScoreDoc type should be returned.
   */
  fn populate_results(&mut self, results: &mut [ScoreDoc], how_many: usize) -> Result<()> {
    if let Some(pq) = &mut self.pq {
      debug_assert!(self.total_hits > self.requested_hit_count);
      for i in (0..how_many).rev() {
        results[i] = pq.pop_unchecked()?;
      }
      return Ok(());
    }

    // Total number of hits collected were less than requestedHitCount
    debug_assert!(self.total_hits <= self.requested_hit_count);
    let hits = self
      .hits
      .as_mut()
      .ok_or_else(|| LuceneError::illegal_state("hits list has already been converted to a PQ"))?;
    hits.sort_by(|a, b| {
      b.score
        .partial_cmp(&a.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| a.doc.cmp(&b.doc))
    });

    results[..how_many].clone_from_slice(&hits[..how_many]);
    Ok(())
  }

  /**
   * Returns a TopDocs instance containing the given results. If results is
   * None it means there are no results to return.
   */
  fn new_top_docs(&self, results: Option<Vec<ScoreDoc>>) -> TopDocs<ScoreDoc> {
    match results {
      None => EMPTY_TOP_DOCS.clone(),
      Some(results) => TopDocs::new(TotalHits::new(self.total_hits, Relation::EqualTo), results),
    }
  }

  /** Returns the top docs that were collected by this collector. */
  pub fn top_docs(&mut self) -> Result<TopDocs<ScoreDoc>> {
    self.top_docs_with_how_many(std::cmp::min(self.total_hits, self.requested_hit_count))
  }
}

impl Collector for LargeNumHitsTopDocsCollector {
  type LeafCollector<'a, IRC>
    = LargeNumHitsTopDocsLeafCollector<'a>
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    Ok(LargeNumHitsTopDocsLeafCollector::new(
      self,
      context.doc_base,
    ))
  }

  // We always return COMPLETE since this collector should ideally
  // be used only with large number of hits case.
  fn score_mode(&self) -> ScoreMode {
    ScoreMode::Complete
  }
}

pub struct LargeNumHitsTopDocsLeafCollector<'a> {
  base: &'a mut LargeNumHitsTopDocsCollector,
  doc_base: usize,
}

impl<'a> LargeNumHitsTopDocsLeafCollector<'a> {
  fn new(base: &'a mut LargeNumHitsTopDocsCollector, doc_base: usize) -> Self {
    Self { base, doc_base }
  }
}

impl Display for LargeNumHitsTopDocsLeafCollector<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl LeafCollector for LargeNumHitsTopDocsLeafCollector<'_> {
  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let score = scorer.score()?;

    // This collector relies on the fact that scorers produce positive values:
    debug_assert!(score >= 0.0); // NOTE: false for NaN

    if self.base.total_hits < self.base.requested_hit_count {
      self
        .base
        .hits
        .as_mut()
        .ok_or_else(|| LuceneError::illegal_state("hits list has already been converted to a PQ"))?
        .push(ScoreDoc::new(doc + self.doc_base as i32, score));
      self.base.total_hits += 1;
      return Ok(());
    } else if self.base.total_hits == self.base.requested_hit_count {
      // Convert the list to a priority queue.

      // We should get here only when priority queue has not been built.
      debug_assert!(self.base.pq.is_none());
      let mut pq = hit_queue::new(self.base.requested_hit_count, false)?;
      if let Some(hits) = self.base.hits.take() {
        for score_doc in hits {
          pq.add(score_doc)?;
        }
      }
      self.base.pq = Some(pq);
    }

    let pq = self
      .base
      .pq
      .as_mut()
      .ok_or_else(|| LuceneError::illegal_state("priority queue has not been built"))?;
    let pq_top_score = pq
      .top()
      .ok_or_else(|| LuceneError::illegal_state("priority queue is empty"))?
      .score;

    if score > pq_top_score {
      let pq_top = pq
        .top_mut()
        .ok_or_else(|| LuceneError::illegal_state("priority queue is empty"))?;
      pq_top.doc = doc + self.doc_base as i32;
      pq_top.score = score;
      pq.update_top()?;
    }
    self.base.total_hits += 1;
    Ok(())
  }
}
