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
use crate::core::search::collector::Collector;
use crate::core::search::score_doc::ScoreDocLike;
use crate::core::search::top_docs::{TopDocs, TopDocsLike};
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
pub trait TopDocsCollector: Collector {
    type Item: ScoreDocLike + Default;
    type Cmp: Compare<Self::Item>;
    type TopDocsLike: TopDocsLike;
    fn pq(&self) -> &PriorityQueue<Self::Item, Self::Cmp>;
    fn pq_mut(&mut self) -> &mut PriorityQueue<Self::Item, Self::Cmp>;
    fn total_hits(&self) -> usize;
    /// The total number of documents that matched this query.
    fn get_total_hits_relation(&self) -> Relation;

    /// Populates the results array with the ScoreDoc instances.
    /// This can be overridden in case a different ScoreDoc type should be returned.
    fn populate_results(&mut self, results: &mut [Self::Item], how_many: usize) -> Result<()> {
        let pq = self.pq_mut();
        debug_assert!(how_many <= pq.size());
        for i in (0..how_many).rev() {
            results[i] = pq.pop_unchecked()?
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
    fn new_top_docs(&self, results: Option<Vec<Self::Item>>, _start: i32) -> Self::TopDocsLike
    where
        Self: Sized;
    fn default_new_top_docs(
        &self,
        results: Option<Vec<Self::Item>>,
        _start: i32,
    ) -> TopDocs<Self::Item>
    where
        Self: Sized,
    {
        match results {
            None => Self::empty_top_docs(),
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
    fn top_docs(&mut self) -> Result<Self::TopDocsLike>
    where
        Self: Sized,
    {
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
    fn top_docs_with_start(&mut self, start: i32) -> Result<Self::TopDocsLike>
    where
        Self: Sized,
    {
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
    fn top_docs_with_start_limit(
        &mut self,
        start: i32,
        mut how_many: i32,
    ) -> Result<Self::TopDocsLike>
    where
        Self: Sized,
    {
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

        let mut results = vec![Default::default(); how_many as usize];

        let discard_count = self.pq().size() as i32 - start - how_many;
        let pq = self.pq_mut();
        for _ in 0..discard_count {
            pq.pop_unchecked()?;
        }

        self.populate_results(&mut results, how_many as usize)?;

        Ok(self.new_top_docs(Some(results), start))
    }
    /// This is used in case topDocs() is called with illegal parameters, or there simply aren't (enough) results.
    fn empty_top_docs() -> TopDocs<Self::Item>
    where
        Self: Sized,
    {
        TopDocs::new(TotalHits::new(0, Relation::EqualTo), vec![])
    }
}

pub struct TopDocsCollectorBase<T, C>
where
    T: ScoreDocLike,
    C: Compare<T>,
{
    /// The total number of documents that the collector encountered.
    pub(crate) total_hits: usize,
    /// The priority queue which holds the top documents.
    /// Note that different implementations of PriorityQueue give different meaning to 'top documents'.
    /// HitQueue for example aggregates the top scoring documents,
    /// while other PQ implementations may hold documents sorted by other criteria.
    pub(crate) pq: PriorityQueue<T, C>,
    /// Whether totalHits is exact or a lower bound.
    pub(crate) total_hits_relation: Relation,
}
impl<T, C> TopDocsCollectorBase<T, C>
where
    T: ScoreDocLike,
    C: Compare<T>,
{
    pub fn new(pq: PriorityQueue<T, C>) -> Self {
        Self {
            total_hits: 0,
            pq,
            total_hits_relation: Relation::EqualTo,
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::core::document::document::Document;
    use crate::core::index::composite_reader::{CompositeReader, get_context};
    use crate::core::index::directory_reader::directory_reader_util;
    use crate::core::index::index_writer::IndexWriter;
    use crate::core::index::leaf_reader::LeafReader;
    use crate::core::index::leaf_reader_context::LeafReaderContext;
    use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
    use crate::core::search::collector::Collector;
    use crate::core::search::collector_manager::CollectorManager;
    use crate::core::search::dummy::dummy_doc_id_set_iterator::DummyDocIdSetIterator;
    use crate::core::search::hit_queue::{HitQueue, HitQueueComparator};
    use crate::core::search::leaf_collector::LeafCollector;
    use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
    use crate::core::search::scorable::Scorable;
    use crate::core::search::score_doc::ScoreDoc;
    use crate::core::search::score_mode::ScoreMode;
    use crate::core::search::score_mode::ScoreMode::CompleteNoScores;
    use crate::core::search::top_docs::TopDocs;
    use crate::core::search::top_docs_collector::{TopDocsCollector, TopDocsCollectorBase};
    use crate::core::search::total_hits::{Relation, TotalHits};
    use crate::core::search::weight::Weight;
    use crate::core::store::directory::Directory;
    use crate::core::util::error::lucene_error::{LuceneError, Result};
    use crate::core::util::priority_queue::PriorityQueue;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        new_directory, new_index_writer_config, new_searcher, random,
    };
    use rand::Rng;
    use std::fmt::{Display, Formatter};
    use std::sync::Arc;

    use crate::core::index::index_reader_context::IndexReaderContext;

    use crate::core::search::dummy::dummy_weight::DummyWeight;

    use crate::core::search::query::Query;

    use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
    use crate::core::search::total_hits::Relation::{EqualTo, GreaterThanOrEqualTo};
    use crate::test::search::check_hits::CheckHits;

    #[allow(dead_code)] // for quick search
    struct TestTopDocsCollector;

    struct MyTopDocsCollectorMananger {
        num_hits: i32,
    }
    impl MyTopDocsCollectorMananger {
        fn new(num_hits: i32) -> Self {
            Self { num_hits }
        }
    }
    impl CollectorManager for MyTopDocsCollectorMananger {
        type C = MyTopDocsCollector;
        type T = MyTopDocsCollector;

        fn new_collector(&self) -> Result<Self::C> {
            MyTopDocsCollector::new(self.num_hits)
        }

        fn reduce(&self, collectors: Vec<Self::C>) -> Result<Self::T> {
            let mut total_hits = 0;
            let mut my_top_docs_collector = MyTopDocsCollector::new(self.num_hits)?;
            for collector in collectors {
                total_hits += collector.base.total_hits;
                for score_doc in collector.base.pq.iter() {
                    my_top_docs_collector
                        .pq_mut()
                        .insert_with_overflow(score_doc)?;
                }
            }
            my_top_docs_collector.base.total_hits = total_hits;
            Ok(my_top_docs_collector)
        }
    }

    pub const SCORES: [f32; 30] = [
        0.7767749, 1.7839992, 8.9925785, 7.9608946, 0.07948637, 2.6356435, 7.4950366, 7.1490803,
        8.108544, 4.961808, 2.2423935, 7.285586, 4.6699767, 2.9655676, 6.953706, 5.383931,
        6.9916306, 8.365894, 7.888485, 8.723962, 3.1796896, 0.39971232, 1.3077754, 6.8489285,
        9.17561, 5.060466, 7.9793315, 8.601509, 4.1858315, 0.28146625,
    ];

    struct LeafCollectorImpl<'a> {
        base: &'a mut MyTopDocsCollector,
        doc_base: i32,
        scores: [f32; 30],
    }
    impl<'a> LeafCollectorImpl<'a> {
        fn new(base: &'a mut MyTopDocsCollector, doc_base: i32, scores: [f32; 30]) -> Self {
            Self {
                base,
                doc_base,
                scores,
            }
        }
    }

    impl<'a> Display for LeafCollectorImpl<'a> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "LeafCollectorImpl")
        }
    }

    impl<'a> LeafCollector for LeafCollectorImpl<'a> {
        fn collect<S>(&mut self, doc: i32, _scorer: &mut S) -> Result<()>
        where
            S: Scorable,
        {
            self.base.base.total_hits += 1;
            let sd = ScoreDoc::new(
                doc + self.doc_base,
                self.scores[(self.doc_base + doc) as usize],
            );
            self.base.pq_mut().insert_with_overflow(sd)?;
            Ok(())
        }

        type DocIdSetIterator = DummyDocIdSetIterator;
        type DocIdSetIteratorRef<'b>
            = DummyDocIdSetIterator
        where
            Self: 'b;
    }
    struct MyTopDocsCollector {
        base: TopDocsCollectorBase<ScoreDoc, HitQueueComparator>,
    }
    impl MyTopDocsCollector {
        fn new(size: i32) -> Result<Self> {
            let pq = HitQueue::new(size, true)?;
            let base = TopDocsCollectorBase::new(pq);
            Ok(Self { base })
        }
    }

    impl Collector for MyTopDocsCollector {
        type LeafCollector<'a, LR>
            = LeafCollectorImpl<'a>
        where
            Self: 'a,
            LR: LeafReader;

        fn get_leaf_collector<'a, W, LR>(
            &'a mut self,
            context: &LeafReaderContext<LR>,
            _weight: Option<&W>,
        ) -> Result<Self::LeafCollector<'a, LR>>
        where
            LR: LeafReader,
            W: Weight<LR>,
        {
            let base = context.doc_base;
            Ok(LeafCollectorImpl::new(self, base, SCORES))
        }

        fn score_mode(&self) -> ScoreMode {
            CompleteNoScores
        }
    }

    impl TopDocsCollector for MyTopDocsCollector {
        type Item = ScoreDoc;
        type Cmp = HitQueueComparator;
        type TopDocsLike = TopDocs<Self::Item>;

        fn pq(&self) -> &PriorityQueue<Self::Item, Self::Cmp> {
            &self.base.pq
        }

        fn pq_mut(&mut self) -> &mut PriorityQueue<Self::Item, Self::Cmp> {
            &mut self.base.pq
        }

        fn total_hits(&self) -> usize {
            self.base.total_hits
        }

        fn get_total_hits_relation(&self) -> Relation {
            self.base.total_hits_relation
        }

        fn new_top_docs(&self, results: Option<Vec<Self::Item>>, _start: i32) -> Self::TopDocsLike
        where
            Self: Sized,
        {
            match results {
                None => Self::empty_top_docs(),
                Some(res) => TopDocs::new(
                    TotalHits::new(self.base.total_hits, self.base.total_hits_relation),
                    res,
                ),
            }
        }
    }
    fn get_reader<D>(dir: Arc<D>) -> Result<StandardDirectoryReaderType<D>>
    where
        D: Directory,
    {
        let mut random = random();
        // TODO IMPORTANT：RandomIndexWriter
        let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
        for _ in 0..30 {
            let _ = writer.add_document(Document::new())?;
        }
        let reader = writer.get_reader(true, false)?;
        writer.close()?;
        Ok(reader)
    }
    fn do_search<R: Rng + ?Sized>(random: &mut R, num_results: i32) -> Result<MyTopDocsCollector> {
        let query = MatchAllDocsQuery::new();
        let dir = Arc::new(new_directory(random)?);
        let reader = Arc::new(get_reader(dir)?);
        let mut searcher = new_searcher(reader)?;
        let cm = MyTopDocsCollectorMananger::new(num_results);
        searcher.search_with_collector_manager(query, &cm)
    }
    fn do_search_with_threshold<CR>(
        num_results: i32,
        threshold: i32,
        query: Query,
        index_reader: CR,
    ) -> Result<TopDocs<ScoreDoc>>
    where
        CR: CompositeReader + Clone,
        CR::LeafReader: LeafReader<ParentReader = CR>,
    {
        // TODO：这里应该使用new_searcher的另一个变体
        let mut searcher = new_searcher(index_reader)?;
        let collector_manager =
            TopScoreDocCollectorManager::with_after(num_results, None, threshold)?;
        searcher.search_with_collector_manager(query, &collector_manager)
    }
    fn do_concurrent_search_with_threshold<CR>(
        num_results: i32,
        threshold: i32,
        query: Query,
        index_reader: CR,
    ) -> Result<TopDocs<ScoreDoc>>
    where
        CR: CompositeReader + Clone + 'static,
        CR::LeafReader: LeafReader<ParentReader = CR>,
    {
        // TODO：这里应该使用new_searcher的另一个变体
        let mut searcher = new_searcher(index_reader)?;
        let collector_manager =
            TopScoreDocCollectorManager::with_after(num_results, None, threshold)?;
        searcher.search_with_collector_manager(query, &collector_manager)
    }

    #[test]
    fn test_invalid_arguments() -> Result<()> {
        let mut random = random();
        let num_results = 5;
        let mut tdc = do_search(&mut random, num_results)?;

        // start < 0
        let result = tdc.top_docs_with_start(-1);
        assert!(
            matches!(result, Err(LuceneError::IllegalArgument(msg)) if msg.message.eq(
                    "Expected value of starting position is between 0 and 5, got -1",
            ))
        );

        // start == pq.size()
        let td = tdc.top_docs_with_start(num_results)?;
        assert_eq!(td.score_docs.len(), 0);

        // howMany < 0
        let result = tdc.top_docs_with_start_limit(0, -1);
        assert!(
            matches!(result, Err(LuceneError::IllegalArgument(msg)) if msg.message.eq(
                    "Number of hits requested must be greater than 0 but value was -1",
            ))
        );

        Ok(())
    }
    #[test]
    fn test_zero_results() -> Result<()> {
        let mut tdc = MyTopDocsCollector::new(5)?;
        let td = tdc.top_docs_with_start_limit(0, 1)?;
        assert_eq!(td.score_docs.len(), 0);
        Ok(())
    }
    #[test]
    fn test_first_results_page() -> Result<()> {
        let mut random = random();
        let mut tdc = do_search(&mut random, 15)?;
        let td = tdc.top_docs_with_start_limit(0, 10)?;
        assert_eq!(td.score_docs.len(), 10);
        Ok(())
    }
    #[test]
    fn test_second_results_pages() -> Result<()> {
        let mut random = random();

        // ask for more results than are available
        let mut tdc = do_search(&mut random, 15)?;
        let td = tdc.top_docs_with_start_limit(10, 10)?;
        assert_eq!(td.score_docs.len(), 5);

        // ask for 5 results (exactly what there should be)
        let mut tdc = do_search(&mut random, 15)?;
        let td = tdc.top_docs_with_start_limit(10, 5)?;
        assert_eq!(td.score_docs.len(), 5);

        // ask for less results than there are
        let mut tdc = do_search(&mut random, 15)?;
        let td = tdc.top_docs_with_start_limit(10, 4)?;
        assert_eq!(td.score_docs.len(), 4);

        Ok(())
    }
    #[test]
    fn test_get_all_results() -> Result<()> {
        let mut random = random();
        let mut tdc = do_search(&mut random, 15)?;
        let td = tdc.top_docs()?;
        assert_eq!(td.score_docs.len(), 15);
        Ok(())
    }

    #[test]
    fn test_get_results_from_start() -> Result<()> {
        let mut random = random();

        // should bring all results
        let mut tdc = do_search(&mut random, 15)?;
        let td = tdc.top_docs_with_start(0)?;
        assert_eq!(td.score_docs.len(), 15);

        // get the last 5 only
        let mut tdc = do_search(&mut random, 15)?;
        let td = tdc.top_docs_with_start(10)?;
        assert_eq!(td.score_docs.len(), 5);

        Ok(())
    }
    #[test]
    fn test_illegal_arguments() -> Result<()> {
        let mut random = random();
        let mut tdc = do_search(&mut random, 15)?;

        // start < 0
        let result = tdc.top_docs_with_start(-1);
        assert!(
            matches!(result, Err(LuceneError::IllegalArgument(msg)) if msg.message.eq(
                "Expected value of starting position is between 0 and 15, got -1",
            ))
        );

        // how_many < 0
        let result = tdc.top_docs_with_start_limit(9, -1);
        assert!(
            matches!(result, Err(LuceneError::IllegalArgument(msg)) if msg.message.eq(
                "Number of hits requested must be greater than 0 but value was -1",
            ))
        );

        Ok(())
    }
    #[test]
    fn test_results_order() -> Result<()> {
        let mut random = random();
        let mut tdc = do_search(&mut random, 15)?;
        let td = tdc.top_docs()?;
        let sd = td.score_docs;

        assert_eq!(MAX_SCORE, sd[0].score);
        for i in 1..sd.len() {
            assert!(sd[i - 1].score >= sd[i].score);
        }

        Ok(())
    }
    const MAX_SCORE: f32 = 9.17561;

    struct Score {
        score: f32,
        min_competitive_score: Option<f32>,
    }
    impl Score {
        fn new() -> Self {
            Self {
                score: 0.0,
                min_competitive_score: None,
            }
        }
    }
    impl Scorable for Score {
        fn score(&mut self) -> Result<f32> {
            Ok(self.score)
        }

        fn set_min_competitive_score(&mut self, score: f32) -> Result<()> {
            assert!(
                self.min_competitive_score.is_none()
                    || score >= *self.min_competitive_score.as_ref().unwrap()
            );
            self.min_competitive_score = Some(score);
            Ok(())
        }

        type Scorable = Score;
    }
    #[test]
    fn test_set_min_competitive_score() -> Result<()> {
        let mut random = random();
        let dir = Arc::new(new_directory(&mut random)?);
        // TODO: 这里没有定义合并策略
        let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

        writer.add_documents(vec![
            Document::new(),
            Document::new(),
            Document::new(),
            Document::new(),
        ])?;
        writer.flush()?;
        writer.add_documents(vec![Document::new(), Document::new()])?;
        writer.flush()?;

        let reader = Arc::new(directory_reader_util::open_with_writer(&writer)?);
        let v = get_context(reader)?;
        assert_eq!(v.leaves()?.len(), 2);
        writer.close()?;

        let collector_manager = TopScoreDocCollectorManager::new(2, 2)?;
        let mut collector = collector_manager.new_collector()?;
        let mut scorer = Score::new();
        let dummy_weight = DummyWeight::new(v.leaves()?[0].reader().clone());
        let mut leaf_collector =
            collector.get_leaf_collector(&v.leaves()?[0], Some(&dummy_weight))?;
        leaf_collector.set_scorer(&mut scorer)?;
        assert!(scorer.min_competitive_score.is_none());

        scorer.score = 1.0;
        leaf_collector.collect(0, &mut scorer)?;
        assert!(scorer.min_competitive_score.is_none());

        scorer.score = 2.0;
        leaf_collector.collect(1, &mut scorer)?;
        assert!(scorer.min_competitive_score.is_none());

        scorer.score = 3.0;
        leaf_collector.collect(2, &mut scorer)?;
        assert_eq!(
            scorer.min_competitive_score,
            Some(f32::from_bits((2.0f32).to_bits() + 1))
        );

        scorer.score = 0.5;
        scorer.min_competitive_score = None;
        leaf_collector.collect(3, &mut scorer)?;
        assert!(scorer.min_competitive_score.is_none());

        scorer.score = 4.0;
        leaf_collector.collect(4, &mut scorer)?;
        assert_eq!(
            scorer.min_competitive_score,
            Some(f32::from_bits((3.0f32).to_bits() + 1))
        );

        // Make sure the min score is set on scorers on new segments
        scorer = Score::new();
        let mut leaf_collector =
            collector.get_leaf_collector(&v.leaves()?[1], Some(&dummy_weight))?;
        leaf_collector.set_scorer(&mut scorer)?;
        assert_eq!(
            scorer.min_competitive_score,
            Some(f32::from_bits((3.0f32).to_bits() + 1))
        );

        scorer.score = 1.0;
        leaf_collector.collect(0, &mut scorer)?;
        assert_eq!(
            scorer.min_competitive_score,
            Some(f32::from_bits((3.0f32).to_bits() + 1))
        );

        scorer.score = 4.0;
        leaf_collector.collect(1, &mut scorer)?;
        assert_eq!(
            scorer.min_competitive_score,
            Some(f32::from_bits((4.0f32).to_bits() + 1))
        );

        Ok(())
    }
    #[test]
    fn test_shared_count_collector_manager() -> Result<()> {
        let mut random = random();

        let dir = Arc::new(new_directory(&mut random)?);
        // TODO: 这里没有定义合并策略
        let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

        writer.add_documents(vec![
            Document::new(),
            Document::new(),
            Document::new(),
            Document::new(),
        ])?;
        writer.flush()?;
        writer.add_documents(vec![Document::new(), Document::new()])?;
        writer.flush()?;

        let reader = Arc::new(directory_reader_util::open_with_writer(&writer)?);
        let v = get_context(reader.clone())?;
        assert_eq!(v.leaves()?.len(), 2);
        writer.close()?;

        let query = MatchAllDocsQuery::new();
        let tdc = do_concurrent_search_with_threshold(5, 10, query.into(), reader.clone())?;
        let query = MatchAllDocsQuery::new();
        let tdc2 = do_search_with_threshold(5, 10, query.into(), reader.clone())?;

        let query = MatchAllDocsQuery::new();
        CheckHits::check_equal(&query.into(), &tdc.score_docs, &tdc2.score_docs)?;
        Ok(())
    }
    #[test]
    fn test_total_hits() -> Result<()> {
        let mut random = random();
        let dir = Arc::new(new_directory(&mut random)?);
        // TODO: 这里没有定义合并策略
        let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

        writer.add_documents(vec![
            Document::new(),
            Document::new(),
            Document::new(),
            Document::new(),
        ])?;
        writer.flush()?;
        writer.add_documents(vec![
            Document::new(),
            Document::new(),
            Document::new(),
            Document::new(),
            Document::new(),
            Document::new(),
        ])?;
        writer.flush()?;

        let reader = Arc::new(directory_reader_util::open_with_writer(&writer)?);
        let v = get_context(reader.clone())?;
        assert_eq!(v.leaves()?.len(), 2);
        writer.close()?;
        let dummy_weight = DummyWeight::new(v.leaves()?[0].reader().clone());

        for total_hits_threshold in 0..20 {
            let collector_manager = TopScoreDocCollectorManager::new(2, total_hits_threshold)?;
            let mut collector = collector_manager.new_collector()?;
            let mut scorer = Score::new();
            let mut leaf_collector =
                collector.get_leaf_collector(&v.leaves()?[0], Some(&dummy_weight))?;
            leaf_collector.set_scorer(&mut scorer)?;

            scorer.score = 3.0;
            leaf_collector.collect(0, &mut scorer)?;

            scorer.score = 3.0;
            leaf_collector.collect(1, &mut scorer)?;

            let mut leaf_collector =
                collector.get_leaf_collector(&v.leaves()?[1], Some(&dummy_weight))?;
            leaf_collector.set_scorer(&mut scorer)?;

            scorer.score = 3.0;
            leaf_collector.collect(1, &mut scorer)?;

            scorer.score = 4.0;
            leaf_collector.collect(1, &mut scorer)?;

            let top_docs = collector.top_docs()?;
            assert_eq!(top_docs.total_hits.value, 4);
            assert_eq!(
                scorer.min_competitive_score.is_some(),
                total_hits_threshold < 4
            );
            assert_eq!(
                top_docs.total_hits,
                if total_hits_threshold < 4 {
                    TotalHits::new(4, GreaterThanOrEqualTo)
                } else {
                    TotalHits::new(4, EqualTo)
                }
            );
        }
        Ok(())
    }
    // // TODO: 这里需要调整TextField不可变使用后再来调整这个测试
    #[test]
    fn test_relation_vs_top_docs_count() -> Result<()> {
        // let mut random = random();
        // let dir = Arc::new(new_directory(&mut random)?);
        // // TODO: 这里没有定义合并策略
        // let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
        //
        // let mut doc = Document::new();
        // doc.add(TextField::with_string("f", "foo bar", Store::No)?);
        // writer.add_documents(vec![Document::new(), Document::new(), Document::new(), Document::new(), Document::new()])?;
        // writer.flush()?;
        // writer.add_documents(vec![Document::new(), Document::new(), Document::new(), Document::new(), Document::new()])?;
        // writer.flush()?;
        //
        // let reader = Arc::new(directory_reader_util::open_with_writer(&writer)?);
        // let irc = get_context(reader.clone())?;
        // let mut searcher = IndexSearcher::new(irc)?;
        //
        // let cm = TopScoreDocCollectorManager::new(2, 10)?;
        // let top_docs = searcher.search_with_collector_manager(
        //     TermQuery::new(Term::from_text("f", "foo")),
        //     &cm,
        //     None,
        // )?;
        // assert_eq!(top_docs.total_hits.value, 10);
        // assert_eq!(top_docs.total_hits.relation, EqualTo);
        //
        // let cm = TopScoreDocCollectorManager::new(2, 2)?;
        // let top_docs = searcher.search_with_collector_manager(
        //     TermQuery::new(Term::from_text("f", "foo")),
        //     &cm,
        //     None,
        // )?;
        // assert!(10 >= top_docs.total_hits.value);
        // assert_eq!(top_docs.total_hits.relation, GreaterThanOrEqualTo);
        //
        // let cm = TopScoreDocCollectorManager::new(10, 2)?;
        // let top_docs = searcher.search_with_collector_manager(
        //     TermQuery::new(Term::from_text("f", "foo")),
        //     &cm,
        //     None,
        // )?;
        // assert_eq!(top_docs.total_hits.value, 10);
        // assert_eq!(top_docs.total_hits.relation, EqualTo);

        Ok(())
    }

    fn test_concurrent_min_score() -> Result<()> {
        // TODO
        Ok(())
    }
    fn test_random_min_competitive_score() -> Result<()> {
        // TODO
        Ok(())
    }
    fn test_realistic_concurrent_minimum_score() -> Result<()> {
        // TODO
        Ok(())
    }
}
