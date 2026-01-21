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
use crate::core::search::query::{BaseQuery, Query};
use crate::core::search::query_caching_policy::QueryCachingPolicy;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::frequency_tracking_ring_buffer::FrequencyTrackingRingBuffer;
use parking_lot::Mutex;
use std::hash::{DefaultHasher, Hash, Hasher};

/// the hash code that we use as a sentinel in the ring buffer.
const SENTINEL: i32 = i32::MAX;
pub struct UsageTrackingQueryCachingPolicy {
    recently_used_filters: Mutex<FrequencyTrackingRingBuffer>,
}
impl UsageTrackingQueryCachingPolicy {
    pub fn new() -> Result<Self> {
        Self::with_history_size(256)
    }
    pub fn with_history_size(history_size: usize) -> Result<Self> {
        Ok(Self {
            recently_used_filters: Mutex::new(FrequencyTrackingRingBuffer::new(
                history_size,
                SENTINEL,
            )?),
        })
    }
    pub(crate) fn min_frequency_to_cache(&self, _query: &Query) -> i32 {
        // TODO IMPORTANT
        2
    }
    fn should_never_cache(&self, _query: &Query) -> bool {
        // TODO IMPORTANT
        false
    }
    pub(crate) fn frequency(&self, query: &Query) -> i32 {
        debug_assert!(!matches!(query, Query::Base(BaseQuery::Boost(_))));
        debug_assert!(!matches!(query, Query::ConstantScore(_)));

        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        let hash_code = hasher.finish();
        let hash_code = (hash_code & 0x7FFF_FFFF) as i32;
        let recently_used_filters = self.recently_used_filters.lock();
        recently_used_filters.frequency(hash_code)
    }
}
impl QueryCachingPolicy for UsageTrackingQueryCachingPolicy {
    fn on_use(&self, query: &Query) {
        debug_assert!(
            !matches!(query, Query::Base(BaseQuery::Boost(_))),
            "BoostQuery should not be passed to on_use()"
        );
        debug_assert!(
            !matches!(query, Query::ConstantScore(_)),
            "ConstantScoreQuery should not be passed to on_use()"
        );
        if self.should_never_cache(query) {
            return;
        }

        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        let hash_code = hasher.finish();
        let hash_code = (hash_code & 0x7FFF_FFFF) as i32;

        // we only track hash codes to avoid holding references to possible
        // large queries; this may cause rare false positives, but at worse
        // this just means we cache a query that was not in fact used enough:
        let mut recently_used_filters = self.recently_used_filters.lock();
        recently_used_filters.add(hash_code);
    }

    fn should_cache(&self, query: &Query) -> Result<bool> {
        if self.should_never_cache(query) {
            return Ok(false);
        }
        let frequency = self.frequency(query);
        let min_frequency = self.min_frequency_to_cache(query);
        Ok(frequency >= min_frequency)
    }
}
pub(crate) fn is_costly(_query: &Query) -> bool {
    // TODO IMPORTANT 有些QueryCache还未实现
    false
}
