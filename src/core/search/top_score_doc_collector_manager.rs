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
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::max_score_accumulator::MaxScoreAccumulator;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::top_docs::TopDocs;
use crate::core::search::top_docs_collector::TopDocsCollector;
use crate::core::search::top_score_doc_collector::TopScoreDocCollector;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
/// Create a [`TopScoreDocCollectorManager`] which uses:
/// - a shared hit counter to maintain the number of hits, and
/// - a shared [`MaxScoreAccumulator`] to propagate the minimum score across segments.
///
///
/// # Notes
/// A new collector manager should be created for each search due to its internal states.
pub struct TopScoreDocCollectorManager {
    num_hits: i32,
    after: Option<ScoreDoc>,
    total_hits_threshold: i32,
    min_score_acc: Option<MaxScoreAccumulator>,
}
impl TopScoreDocCollectorManager {
    /// Creates a new [`TopScoreDocCollectorManager`] given the number of hits to collect
    /// and the number of hits to count accurately, with thread-safe internal states.
    ///
    ///
    /// **NOTE:**
    /// - If the total hit count of the top docs is less than or exactly `total_hits_threshold`
    ///   then this value is accurate.
    /// - On the other hand, if the [`TopDocs::total_hits`](TopDocs) value is greater than `total_hits_threshold`
    ///   then its value is a lower bound of the hit count.
    /// - A value of `i32::MAX` will make the hit count accurate, but will also likely make query
    ///   processing slower.
    ///
    ///
    /// **NOTE:**
    /// The instances returned by this method pre-allocate a full array of length `num_hits`,
    /// and fill the array with sentinel objects.
    ///
    /// # Parameters
    /// - `num_hits`: the number of results to collect.
    /// - `after`: the previous doc after which matching docs will be collected.
    /// - `total_hits_threshold`: the number of docs to count accurately. If the query matches more
    ///   than `total_hits_threshold` hits then its hit count will be a lower bound. On the other hand
    ///   if the query matches less than or exactly `total_hits_threshold` hits then the hit count
    ///   of the result will be accurate. `i32::MAX` may be used to make the hit count accurate,
    ///   but this will also make query processing slower.
    pub fn with_after(
        num_hits: i32,
        after: Option<ScoreDoc>,
        total_hits_threshold: i32,
    ) -> Result<Self> {
        if total_hits_threshold < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "totalHitsThreshold must be >= 0, got {}",
                total_hits_threshold
            )));
        }

        if num_hits <= 0 {
            return Err(LuceneError::illegal_argument(
                "numHits must be > 0; please use TotalHitCountCollectorManager \
                 if you just need the total hit count"
                    .to_string(),
            ));
        }

        let total_hits_threshold = std::cmp::max(total_hits_threshold, num_hits);

        let min_score_acc = if total_hits_threshold != i32::MAX {
            Some(MaxScoreAccumulator::new())
        } else {
            None
        };

        Ok(Self {
            num_hits,
            after,
            total_hits_threshold,
            min_score_acc,
        })
    }
    /// Creates a new [`TopScoreDocCollectorManager`] given the number of hits to collect
    /// and the number of hits to count accurately, with thread-safe internal states.
    ///
    ///
    /// **NOTE:**
    /// - If the total hit count of the top docs is less than or exactly `total_hits_threshold`,
    ///   then this value is accurate.
    /// - If the [`TopDocs::total_hits`](TopDocs) value is greater than `total_hits_threshold`,
    ///   then its value is a lower bound of the hit count.
    /// - A value of `i32::MAX` will make the hit count accurate,
    ///   but will also likely make query processing slower.
    ///
    ///
    /// **NOTE:**
    /// The instances returned by this method pre-allocate a full array of length `num_hits`
    /// and fill the array with sentinel objects.
    ///
    /// # Parameters
    /// - `num_hits`: the number of results to collect.
    /// - `total_hits_threshold`: the number of docs to count accurately.
    ///   - If the query matches more than `total_hits_threshold` hits then its hit count
    ///     will be a lower bound.
    ///   - If the query matches less than or exactly `total_hits_threshold` hits then
    ///     the hit count of the result will be accurate.
    ///   - `i32::MAX` may be used to make the hit count accurate,
    ///     but this will also make query processing slower.
    pub fn new(num_hits: i32, total_hits_threshold: i32) -> Result<Self> {
        Self::with_after(num_hits, None, total_hits_threshold)
    }
}
impl CollectorManager for TopScoreDocCollectorManager {
    type Collector = TopScoreDocCollector;
    type Output = TopDocs<ScoreDoc>;

    fn new_collector(&self) -> Result<Self::Collector> {
        TopScoreDocCollector::new(
            self.num_hits,
            self.after.clone(),
            self.total_hits_threshold,
            self.min_score_acc.clone(),
        )
    }

    fn reduce<I>(&self, collectors: I) -> Result<Self::Output>
    where
        I: IntoIterator<Item = Self::Collector>,
    {
        let mut top_docs_parts = Vec::new();

        for mut collector in collectors {
            top_docs_parts.push(collector.top_docs()?);
        }
        todo!()

        // Ok(TopDocs::merge(0, self.num_hits, top_docs_parts))
    }
}
