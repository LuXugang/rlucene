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
use crate::core::index::query_timeout::QueryTimeout;
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub(crate) const INTERVAL: i32 = 10;
/// The [`TimeLimitingBulkScorer`] is used to timeout search requests that take longer than the
/// maximum allowed search time limit. After this time is exceeded, the search thread is stopped by
/// return a [`TimeLimitingBulkError`](crate::core::util::error::TimeExceededError).
///
/// See also [`ExitableDirectoryReader`](crate::core::index::exitable_directory_reader::ExitableDirectoryReader).
pub struct TimeLimitingBulkScorer<'a, BS, QT>
where
    BS: BulkScorer,
    QT: QueryTimeout,
{
    in_: BS,
    query_timeout: &'a QT,
}
impl<'a, BS, QT> TimeLimitingBulkScorer<'a, BS, QT>
where
    BS: BulkScorer,
    QT: QueryTimeout,
{
    /// Create a [`TimeLimitingBulkScorer`] wrapper over another [`BulkScorer`] with a specified timeout.
    ///
    /// # Arguments
    ///
    /// * `bulk_scorer` — the wrapped [`BulkScorer`]
    /// * `query_timeout` — max time allowed for collecting hits after which
    ///   [`TimeExceededError`](crate::core::util::error::TimeExceededError) is returned
    pub fn new(in_: BS, query_timeout: &'a QT) -> Self {
        Self { in_, query_timeout }
    }
}
impl<BS, QT> BulkScorer for TimeLimitingBulkScorer<'_, BS, QT>
where
    BS: BulkScorer,
    QT: QueryTimeout,
{
    fn score<LC, B>(
        &mut self,
        collector: &mut LC,
        accept_docs: Option<&B>,
        mut min: i32,
        max: i32,
    ) -> Result<i32>
    where
        LC: LeafCollector,
        B: Bits,
    {
        let mut interval = INTERVAL;
        while min < max {
            let new_max = ((min as i64 + interval as i64).min(max as i64)) as i32;
            let new_interval = interval + (interval >> 1); // increase the interval by 50% on each iteration
            if interval < new_interval {
                interval = new_interval;
            }

            if self.query_timeout.should_exit() {
                return Err(LuceneError::time_exceeded(""));
            }

            min = self.in_.score(collector, accept_docs, min, new_max)?; // in is the wrapped bulk scorer1
        }
        Ok(min)
    }

    fn cost(&mut self) -> Result<i64> {
        self.in_.cost()
    }
}
