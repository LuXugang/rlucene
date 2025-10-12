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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::collector::Collector;
use crate::core::search::doc_id_stream::DocIdStream;
use crate::core::search::dummy::dummy_doc_id_set_iterator::DummyDocIdSetIterator;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// Just counts the total number of hits. This is the collector behind [`IndexSearcher::count`](crate::core::search::index_searcher::IndexSearcher::count).
/// When the [`Weight`] implements [`Weight::count`], this collector will skip collecting segments.
pub struct TotalHitCountCollector {
    pub(crate) total_hit: i32,
}
impl TotalHitCountCollector {
    pub fn new() -> Self {
        Self { total_hit: 0 }
    }
    /// Returns how many hits matched the search.
    pub fn get_total_hits(&self) -> i32 {
        self.total_hit
    }
}
impl Collector for TotalHitCountCollector {
    type LeafCollector<'a>
        = TotalHitCountLeafCollector<'a>
    where
        Self: 'a;

    fn get_leaf_collector<'a, W, LR>(
        &'a mut self,
        context: &LeafReaderContext<LR>,
        weight: Option<&mut W>,
    ) -> Result<Self::LeafCollector<'a>>
    where
        LR: LeafReader,
        W: Weight<LR>,
    {
        let leaf_count = match weight {
            Some(w) => w.count(context)?,
            None => -1,
        };
        if leaf_count != -1 {
            self.total_hit += leaf_count;
            return Err(LuceneError::collection_terminated(""));
        }
        Ok(TotalHitCountLeafCollector::new(self))
    }

    fn score_mode(&self) -> ScoreMode {
        ScoreMode::CompleteNoScores
    }
}

pub struct TotalHitCountLeafCollector<'a> {
    collector: &'a mut TotalHitCountCollector,
}

impl<'a> TotalHitCountLeafCollector<'a> {
    fn new(collector: &'a mut TotalHitCountCollector) -> Self {
        Self { collector }
    }
}

impl<'a> LeafCollector for TotalHitCountLeafCollector<'a> {
    fn collect<S>(&mut self, _doc: i32, _scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        self.collector.total_hit += 1;
        Ok(())
    }

    fn collect_stream<DS, S>(&mut self, stream: &mut DS, _scorer: &mut S) -> Result<()>
    where
        DS: DocIdStream,
        S: Scorable,
    {
        self.collector.total_hit += stream.count()?;
        Ok(())
    }

    type DocIdSetIterator = DummyDocIdSetIterator;
}
