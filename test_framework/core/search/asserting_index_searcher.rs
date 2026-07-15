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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::search::collector::Collector;
use crate::core::search::index_searcher::{
  IndexSearcher, IndexSearcherBase, IndexSearcherDefaults, LeafReaderContextPartition,
};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::search::asserting_collector::AssertingCollector;
use crate::test_framework::core::search::asserting_weight::AssertingWeight;
use crate::test_framework::core::search::query_utils::QueryUtils;
use crate::test_framework::core::util::lucene_test_case::random_from_seed;
use parking_lot::Mutex;
use rand::{RngExt, rngs::StdRng};

/// Helper type that adds some extra checks to ensure correct usage of `IndexSearcher` and
/// `Weight`.
#[allow(dead_code)] // for quick search
pub(crate) struct AssertingIndexSearcher {
  random: Mutex<StdRng>,
}

impl AssertingIndexSearcher {
  pub(crate) fn new(random_seed: u64) -> Self {
    Self {
      random: Mutex::new(random_from_seed(random_seed)),
    }
  }
}

impl<IRC> IndexSearcherBase<IRC> for AssertingIndexSearcher
where
  IRC: IndexReaderContext,
{
  fn create_weight<T>(
    &self,
    searcher: &IndexSearcher<IRC>,
    query: T,
    score_mode: ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    T: QueryBase,
  {
    // this adds assertions to the inner weights/scorers too
    let weight = IndexSearcherDefaults::create_weight(searcher, query, score_mode, boost)?;
    Ok(Box::new(AssertingWeight::new(
      self.random.lock().random(),
      weight,
      score_mode,
    )))
  }

  fn rewrite(&self, searcher: &IndexSearcher<IRC>, original: Query) -> Result<Query> {
    // TODO: use the more sophisticated QueryUtils.check sometimes!
    QueryUtils::check_from_query(&original);
    let rewritten = IndexSearcherDefaults::rewrite(searcher, original)?;
    QueryUtils::check_from_query(&rewritten);
    Ok(rewritten)
  }

  fn search_partitions<W, C>(
    &self,
    searcher: &IndexSearcher<IRC>,
    partitions: &[LeafReaderContextPartition],
    weight: &W,
    collector: &mut C,
  ) -> Result<()>
  where
    C: Collector,
    W: Weight<IRC> + ?Sized,
  {
    let mut asserting_collector = AssertingCollector::wrap(collector);
    IndexSearcherDefaults::search_partitions(
      searcher,
      partitions,
      weight,
      &mut asserting_collector,
    )?;
    assert!(asserting_collector.has_finished_collecting_previous_leaf);
    Ok(())
  }
}
