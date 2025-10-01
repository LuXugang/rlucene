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
use crate::core::search::score_doc::{ScoreDoc, ScoreDocLike};
use crate::core::search::top_docs::TopDocs;
use crate::core::search::total_hits::{Relation, TotalHits};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};
/// A base trait for all collectors that return a [`TopDocs`] output.
///
/// This collector allows easy extension by providing a constructor that accepts a [`PriorityQueue`],
/// as well as protected-like members for that priority queue and a counter of the number of total hits.
///
/// Extending implementations can override any of the methods to provide their own behavior.
/// It is also possible to avoid the use of the priority queue entirely by passing `None` instead of a [`PriorityQueue`].
/// In that case, however, you should consider overriding all relevant methods in order to avoid errors.
///
/// # Notes
/// - This trait is analogous to Lucene's `TopDocsCollector` abstract base class.
/// - The associated [`TopDocs`] represents the search results (hits + metadata).
/// - The `total_hits` counter and the `PriorityQueue` are the common state shared by all implementations.
pub trait TopDocsCollector {
    type Item: ScoreDocLike;
    type Cmp: Compare<Self::Item>;
    fn pq(&self) -> &PriorityQueue<Self::Item, Self::Cmp>;
    fn pq_mut(&mut self) -> &mut PriorityQueue<Self::Item, Self::Cmp>;
    fn total_hits(&self) -> usize;
    /// The total number of documents that matched this query.
    fn get_total_hits_relation(&self) -> Relation;

    /// Populates the results array with the ScoreDoc instances.
    /// This can be overridden in case a different ScoreDoc type should be returned.
    fn populate_results(&mut self, results: &mut [ScoreDoc], how_many: i32) -> Result<()> {
        let pq = self.pq_mut();
        for i in (0..how_many).rev() {
            if let Some(item) = pq.pop()? {
                results[i as usize] = item.convert_score_doc();
            }
        }
        Ok(())
    }
    /// Returns a [`TopDocs`] instance containing the given results.
    ///
    /// If `results` is `None`, it means there are no results to return:
    /// - either because there were 0 calls to `collect()`,
    /// - or because the arguments to [`TopDocsCollector::top_docs`] were invalid.
    ///
    /// # Notes
    /// This method is the Rust equivalent of Lucene's `TopDocsCollector.newTopDocs(ScoreDoc[] results, int start)`.
    fn new_top_docs(&self, results: Option<Vec<ScoreDoc>>, _start: i32) -> TopDocs {
        match results {
            None => empty_top_docs(),
            Some(res) => TopDocs::new(
                TotalHits::new(self.total_hits(), self.get_total_hits_relation()),
                res,
            ),
        }
    }

    fn top_docs_size(&self) -> usize {
        let total = self.total_hits();
        let pq_size = self.pq().size();
        if total < pq_size { total } else { pq_size }
    }

    /// Returns the top docs that were collected by this collector.
    fn top_docs(&mut self) -> Result<TopDocs> {
        let size = self.top_docs_size() as i32;
        self.top_docs_with_start_limit(0, size)
    }
    /// Returns the documents in the range `[start .. pq.size())` that were collected by this collector.
    ///
    /// If `start >= pq.size()`, an empty [`TopDocs`] is returned.
    ///
    /// This method is convenient to call if the application always asks for the last results,
    /// starting from the last "page".
    ///
    /// **NOTE:** you cannot call this method more than once for each search execution.
    /// If you need to call it more than once, passing each time a different `start`,
    /// you should call [`TopDocsCollector::top_docs`] and work with the returned [`TopDocs`] object,
    /// which will contain all the results this search execution collected.
    fn top_docs_with_start(&mut self, start: i32) -> Result<TopDocs> {
        // In case pq was populated with sentinel values, there might be less
        // results than pq.size(). Therefore return all results until either
        // pq.size() or totalHits.
        let size = self.top_docs_size() as i32;
        self.top_docs_with_start_limit(start, size)
    }
    /// Returns the documents in the range `[start .. start+how_many)` that were collected by this collector.
    ///
    /// If `start >= pq.size()`, an empty [`TopDocs`] is returned.
    /// If `pq.size() - start < how_many`, then only the available documents in `[start .. pq.size())` are returned.
    ///
    /// This method is useful in cases where pagination of search results is allowed by the application,
    /// and it attempts to optimize memory usage by allocating only as much space as requested by `how_many`.
    ///
    /// **NOTE:** you cannot call this method more than once for each search execution.
    /// If you need to call it more than once, passing each time a different range,
    /// you should call [`TopDocsCollector::top_docs`] and work with the returned [`TopDocs`] object,
    /// which will contain all the results this search execution collected.
    fn top_docs_with_start_limit(&mut self, start: i32, mut how_many: i32) -> Result<TopDocs> {
        let size = self.top_docs_size() as i32;

        if how_many < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "Number of hits requested must be greater than 0 but value was {}",
                how_many
            )));
        }

        if start < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "Expected value of starting position is between 0 and {}, got {}",
                size, start
            )));
        }

        if start >= size || how_many == 0 {
            return Ok(self.new_top_docs(None, start));
        }

        how_many = std::cmp::min(size - start, how_many);

        let mut results = Vec::with_capacity(how_many as usize);

        let discard_count = self.pq().size() as i32 - start - how_many;
        let pq = self.pq_mut();
        for _ in 0..discard_count {
            pq.pop()?;
        }

        self.populate_results(&mut results, how_many)?;

        Ok(self.new_top_docs(Some(results), start))
    }
}
/// This is used in case topDocs() is called with illegal parameters, or there simply aren't (enough) results.
pub fn empty_top_docs() -> TopDocs {
    TopDocs::new(TotalHits::new(0, Relation::EqualTo), vec![])
}

pub struct TopDocsCollectorBase<T, C>
where
    T: ScoreDocLike,
    C: Compare<T>,
{
    /// The total number of documents that the collector encountered.
    total_hits: i32,
    /// The priority queue which holds the top documents.
    /// Note that different implementations of PriorityQueue give different meaning to 'top documents'.
    /// HitQueue for example aggregates the top scoring documents,
    /// while other PQ implementations may hold documents sorted by other criteria.
    pq: PriorityQueue<T, C>,
    /// Whether totalHits is exact or a lower bound.
    total_hits_relation: Relation,
}
