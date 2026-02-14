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
use crate::core::search::scorable::Scorable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// Used by [`BulkScorers`](crate::core::search::bulk_scorer::BulkScorer) that need to pass a
/// [`Scorable`] to
/// [`LeafCollector::collect`](crate::core::search::leaf_collector::LeafCollector::collect).
pub struct Score {
    pub(crate) score: f32,
}
impl Score {
    pub fn new(score: f32) -> Self {
        Self { score }
    }
}
impl Scorable for Score {
    fn score(&mut self) -> Result<f32> {
        Ok(self.score)
    }

    fn cost(&self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }
}
