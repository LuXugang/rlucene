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
use crate::core::document::document::Document;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::simple_collector::SimpleCollector;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::HasIdentity;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    at_least_usize, is_night_mode, new_directory_shared, new_index_writer_config_with_analyzer,
    new_searcher_with_reader, random,
};
use rand::{Rng, RngExt};
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub struct TestScorerPerf;

fn rand_bit_set<R: Rng + ?Sized>(random: &mut R, sz: usize, num_bits_to_set: usize) -> FixedBitSet {
    let mut set = FixedBitSet::new(sz);
    for _ in 0..num_bits_to_set {
        set.set(random.random_range(0..sz));
    }
    set
}

fn rand_bit_sets<R: Rng + ?Sized>(
    random: &mut R,
    num_sets: usize,
    set_size: usize,
) -> Vec<Arc<FixedBitSet>> {
    let mut sets = Vec::with_capacity(num_sets);
    for _ in 0..num_sets {
        let num_bits_to_set = random.random_range(0..set_size);
        sets.push(Arc::new(rand_bit_set(random, set_size, num_bits_to_set)));
    }
    sets
}

fn add_clause<R: Rng + ?Sized>(
    random: &mut R,
    sets: &[Arc<FixedBitSet>],
    bq: &mut Builder,
    result: Option<FixedBitSet>,
    validate: bool,
) -> Result<Option<FixedBitSet>> {
    let rnd = sets[random.random_range(0..sets.len())].clone();
    let q = BitSetQuery::new(rnd.clone());
    bq.add(Query::BitSet(q), Occur::Must)?;

    if validate {
        let result = if let Some(mut v) = result {
            v.and(rnd.as_ref());
            Some(v)
        } else {
            Some(rnd.as_ref().clone())
        };
        Ok(result)
    } else {
        Ok(result)
    }
}

fn do_conjunctions<R: Rng + ?Sized, IRC: IndexReaderContext>(
    random: &mut R,
    s: &IndexSearcher<IRC>,
    sets: &[Arc<FixedBitSet>],
    iter: usize,
    max_clauses: usize,
    validate: bool,
) -> Result<()> {
    for _ in 0..iter {
        let n_clauses = random.random_range(2..=max_clauses);
        let mut bq = Builder::new();
        let mut result: Option<FixedBitSet> = None;

        for _ in 0..n_clauses {
            result = add_clause(random, sets, &mut bq, result, validate)?;
        }

        let hc =
            s.search_with_collector_manager(bq.build(), &CountingHitCollectorManager::new())?;

        if validate {
            assert_eq!(result.unwrap().cardinality(), hc.get_count());
        }
    }
    Ok(())
}
fn do_nested_conjunctions<R: Rng + ?Sized, IRC: IndexReaderContext>(
    random: &mut R,
    s: &IndexSearcher<IRC>,
    sets: &[Arc<FixedBitSet>],
    iter: usize,
    max_outer_clauses: usize,
    max_clauses: usize,
    validate: bool,
) -> Result<()> {
    let mut n_matches = 0i64;

    for _ in 0..iter {
        let o_clauses = random.random_range(2..=max_outer_clauses);
        let mut oq = Builder::new();
        let mut result: Option<FixedBitSet> = None;

        for _ in 0..o_clauses {
            let n_clauses = random.random_range(2..=max_clauses);
            let mut bq = Builder::new();
            for _ in 0..n_clauses {
                result = add_clause(random, sets, &mut bq, result, validate)?;
            }

            oq.add(bq.build(), Occur::Must)?;
        }

        let hc =
            s.search_with_collector_manager(oq.build(), &CountingHitCollectorManager::new())?;
        n_matches += hc.get_count() as i64;

        if validate {
            assert_eq!(result.unwrap().cardinality(), hc.get_count());
        }
    }
    if cfg!(feature = "test_log_verbose") {
        println!("Average number of matches={}", n_matches / iter as i64);
    }
    Ok(())
}
#[test]
fn test_conjunctions() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let analyzer = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let iw = IndexWriter::new(dir.clone(), iwc)?;
    // set to false when doing performance testing
    let validate = true;

    iw.add_document(Document::new())?;
    iw.close()?;

    let r = directory_reader_util::open(dir)?;
    let mut s = new_searcher_with_reader(r)?;
    s.set_query_cache(None);
    let num_sets = at_least_usize(&mut random, 1000);
    let set_size = at_least_usize(&mut random, 10);
    let sets = rand_bit_sets(&mut random, num_sets, set_size);
    let iterations = if is_night_mode() {
        at_least_usize(&mut random, 10000)
    } else {
        at_least_usize(&mut random, 500)
    };
    let max_clauses = at_least_usize(&mut random, 5);
    do_conjunctions(&mut random, &s, &sets, iterations, max_clauses, validate)?;
    let max_outer_clauses = at_least_usize(&mut random, 3);
    let max_clauses = at_least_usize(&mut random, 3);
    do_nested_conjunctions(
        &mut random,
        &s,
        &sets,
        iterations,
        max_outer_clauses,
        max_clauses,
        validate,
    )?;

    Ok(())
}

struct CountingHitCollectorManager;
impl CountingHitCollectorManager {
    fn new() -> Self {
        Self
    }
}
impl CollectorManager for CountingHitCollectorManager {
    type C = CountingHitCollector;
    type T = CountingHitCollector;

    fn new_collector(&self) -> Result<Self::C> {
        Ok(CountingHitCollector::new())
    }

    fn reduce(&self, collectors: Vec<Self::C>) -> Result<Self::T> {
        let mut result = CountingHitCollector::new();
        collectors.into_iter().for_each(|c| {
            result.count += c.count;
            result.sum += c.sum;
        });
        Ok(result)
    }
}
struct CountingHitCollector {
    count: usize,
    sum: usize,
    doc_base: usize,
}
impl CountingHitCollector {
    fn new() -> Self {
        Self {
            count: 0,
            sum: 0,
            doc_base: 0,
        }
    }
    fn get_count(&self) -> usize {
        self.count
    }
}

impl Collector for CountingHitCollector {
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

impl LeafCollector for CountingHitCollector {
    fn collect(&mut self, doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
        self.count += 1;
        self.sum += doc as usize + self.doc_base;
        Ok(())
    }
}

impl Display for CountingHitCollector {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CountingHitCollector(count={}, sum={})",
            self.count, self.sum
        )
    }
}

impl SimpleCollector for CountingHitCollector {
    fn do_set_next_reader<LR>(&mut self, context: &LeafReaderContext<LR>) -> Result<()>
    where
        LR: LeafReader,
    {
        self.doc_base = context.doc_base;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct BitSetQuery {
    docs: Arc<FixedBitSet>,
    id: Identity,
}
impl BitSetQuery {
    pub fn new(docs: Arc<FixedBitSet>) -> Self {
        Self {
            docs,
            id: Identity::new(),
        }
    }
}

impl HasIdentity for BitSetQuery {
    fn identity(&self) -> &Identity {
        &self.id
    }
}

impl QueryBase for BitSetQuery {
    fn as_string(&self, _field: &str) -> Result<String> {
        Ok("randomBitSetFilter".to_string())
    }

    fn create_weight<IRC>(
        self,
        _searcher: &IndexSearcher<IRC>,
        score_mode: &ScoreMode,
        boost: f32,
    ) -> Result<QueryWeight<IRC>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        Ok(Box::new(BitSetQueryWeight::new(boost, self, *score_mode)))
    }

    fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        Ok(Query::BitSet(self))
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
    }
}
impl PartialEq for BitSetQuery {
    fn eq(&self, other: &Self) -> bool {
        self.docs == other.docs
    }
}
impl Hash for BitSetQuery {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.docs.hash(state);
    }
}
impl Eq for BitSetQuery {}

pub struct BitSetQueryWeight {
    docs: Arc<FixedBitSet>,
    score_mode: ScoreMode,
    query: Arc<Query>,
    base: ConstantScoreWeight,
}
impl BitSetQueryWeight {
    pub fn new(score: f32, query: BitSetQuery, score_mode: ScoreMode) -> Self {
        let docs = query.docs.clone();
        Self {
            docs,
            score_mode,
            query: Arc::new(Query::BitSet(query)),
            base: ConstantScoreWeight::new(score),
        }
    }
}

impl<IRC> SegmentCacheable<IRC> for BitSetQueryWeight
where
    IRC: IndexReaderContext,
{
    fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
        Ok(false)
    }
}

impl<IRC> Weight<IRC> for BitSetQueryWeight
where
    IRC: IndexReaderContext,
{
    type Matches = MatchWithNoTerms;

    fn matches(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        doc: i32,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::Matches>> {
        self.default_matches(context, doc, searcher)
    }

    fn explain(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        doc: i32,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Explanation> {
        let scorer = self.scorer(context, searcher)?;
        self.base.explain(scorer, doc, self.query.as_string("")?)
    }

    fn get_query(&self) -> Arc<Query> {
        self.query.clone()
    }

    type ScorerSupplier = QueryWeightSs<IRC>;

    fn scorer_supplier(
        &self,
        _context: &LeafReaderContext<IRCLeafReader<IRC>>,
        _searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        let iter = BitSetIterator::new(
            self.docs.clone(),
            self.docs.approximate_cardinality() as i64,
        )?;
        let s = ConstantScoreScorer::from_disi(self.base.score(), self.score_mode, iter);
        let ss = DefaultScorerSupplier::new(s);
        Ok(Some(Box::new(ss)))
    }
}
