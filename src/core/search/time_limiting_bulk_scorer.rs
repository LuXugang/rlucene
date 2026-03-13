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

pub(crate) const INTERVAL: i32 = 100;
/// The [`TimeLimitingBulkScorer`] is used to timeout search requests that take longer than the
/// maximum allowed search time limit. After this time is exceeded, the search thread is stopped by
/// return a [`TimeLimitingBulkError`](crate::core::util::error::TimeExceededError).
///
/// See also `ExitableDirectoryReader`.
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
    fn score(
        &mut self,
        collector: &mut dyn LeafCollector,
        accept_docs: Option<&dyn Bits>,
        mut min: i32,
        max: i32,
    ) -> Result<i32> {
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

struct BulkScorerImpl {
    expected_interval: i32,
    last_max: i32,
    last_interval: i32,
    max_docs: i32,
}
impl BulkScorerImpl {
    fn new(max_docs: i32) -> Self {
        Self {
            expected_interval: INTERVAL,
            last_max: 0,
            last_interval: 0,
            max_docs,
        }
    }
}
impl BulkScorer for BulkScorerImpl {
    fn score(
        &mut self,
        _collector: &mut dyn LeafCollector,
        _accept_docs: Option<&dyn Bits>,
        min: i32,
        max: i32,
    ) -> Result<i32> {
        let difference = max - min;

        assert!(difference >= self.last_interval, "Rate should only go up");
        assert_eq!(self.last_max, min, "Documents skipped");
        assert!(
            if max == self.max_docs {
                self.expected_interval >= difference
            } else {
                self.expected_interval == difference
            },
            "Incorrect rate encountered"
        );

        self.last_max = max;
        self.last_interval = difference;
        // use integer sum since the exponential growth formula yields different result due to
        // rounding
        self.expected_interval += self.expected_interval / 2;
        // overflow - stop at the previous one
        if self.expected_interval < 0 {
            self.expected_interval = self.last_interval;
        }

        Ok(max)
    }

    fn cost(&mut self) -> Result<i64> {
        Ok(1)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::document::document::Document;
    use crate::core::document::field::Store;
    use crate::core::index::directory_reader::directory_reader_util;
    use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
    use crate::core::index::index_writer::IndexWriter;
    use crate::core::index::leaf_reader_context::LeafReaderContext;
    use crate::core::index::query_timeout::{QueryTimeout, QueryTimeoutEnum};
    use crate::core::index::term::Term;
    use crate::core::search::collector::Collector;
    use crate::core::search::leaf_collector::LeafCollector;
    use crate::core::search::scorable::Scorable;
    use crate::core::search::score_mode::ScoreMode;
    use crate::core::search::simple_collector::SimpleCollector;
    use crate::core::search::term_query::TermQuery;
    use crate::core::search::weight::Weight;

    use crate::core::search::bulk_scorer::BulkScorer;
    use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::core::search::time_limiting_bulk_scorer::{BulkScorerImpl, TimeLimitingBulkScorer};
    use crate::core::util::bits::MatchAllBits;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::analysis::mock_analyzer::MockAnalyzer;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        new_directory_shared, new_index_writer_config_with_analyzer, new_searcher_with_reader,
        new_text_field, random,
    };
    use std::collections::HashMap;
    use std::fmt::{Display, Formatter};
    use std::sync::atomic::AtomicI32;

    #[allow(dead_code)] // for quick search
    struct TestTimeLimitingBulkScorer;

    #[test]
    fn test_time_limiting_bulk_scorer() -> Result<()> {
        let mut random = random();
        let directory = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let writer = IndexWriter::new(
            directory.clone(),
            new_index_writer_config_with_analyzer(&mut random, analyzer),
        )?;

        let n = 10000;
        let mut field_to_type = HashMap::new();

        for _ in 0..n {
            let mut d = Document::new();
            d.add(new_text_field(
                &mut random,
                "default",
                "ones ",
                Store::Yes,
                &mut field_to_type,
            )?);
            writer.add_document(d)?;
        }

        writer.force_merge(1)?;
        writer.commit()?;
        writer.close()?;

        let query = TermQuery::new(Term::from_text("default", "ones"));
        let directory_reader = directory_reader_util::open(directory.clone())?;
        let mut searcher = new_searcher_with_reader(directory_reader)?;
        searcher.set_timeout(QueryTimeoutEnum::Custom(Box::new(QueryTimeoutImpl::new(
            10,
        ))));

        let top = searcher.search(query, n)?;
        let hits = top.score_docs;

        assert!(
            !hits.is_empty() && hits.len() < n && searcher.timeout(),
            "Partial result and is aborted is true"
        );
        Ok(())
    }
    #[test]
    fn test_exponential_rate() -> Result<()> {
        let max_docs = NO_MORE_DOCS - 1;
        let bulk_scorer = BulkScorerImpl::new(max_docs);
        let qt = QueryTimeoutImpl::new(-1);
        let mut scorer = TimeLimitingBulkScorer::new(bulk_scorer, &qt);
        let mut c = LeafCollectorImpl;
        let bits = MatchAllBits::new(i32::MAX as usize);
        scorer.score(&mut c, Some(&bits), 0, max_docs)?;

        Ok(())
    }

    #[derive(Default)]
    struct LeafCollectorImpl;

    impl Collector for LeafCollectorImpl {
        type LeafCollector<'a, IRC>
            = &'a mut Self
        where
            Self: 'a,
            IRC: IndexReaderContext;

        fn get_leaf_collector<'a, W, IRC>(
            &'a mut self,
            context: &LeafReaderContext<IRCLeafReader<IRC>>,
            weight: Option<&W>,
        ) -> Result<Self::LeafCollector<'a, IRC>>
        where
            IRC: IndexReaderContext,
            W: Weight<IRC> + ?Sized,
        {
            SimpleCollector::get_leaf_collector(self, context, weight)?;
            Ok(self)
        }

        fn score_mode(&self) -> ScoreMode {
            ScoreMode::CompleteNoScores
        }
    }

    impl LeafCollector for LeafCollectorImpl {
        fn collect(&mut self, _doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
            Ok(())
        }
    }

    impl Display for LeafCollectorImpl {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", std::any::type_name::<Self>())
        }
    }

    impl SimpleCollector for LeafCollectorImpl {}

    struct QueryTimeoutImpl {
        counter: AtomicI32,
        time_allowed: i32,
    }
    impl QueryTimeoutImpl {
        fn new(time_allowed: i32) -> Self {
            Self {
                counter: AtomicI32::new(0),
                time_allowed,
            }
        }
    }
    impl QueryTimeout for QueryTimeoutImpl {
        fn should_exit(&self) -> bool {
            let v = self
                .counter
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            v == self.time_allowed
        }
    }
}
