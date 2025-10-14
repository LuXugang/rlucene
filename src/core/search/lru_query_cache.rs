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
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set::{DocIdSet, EmptyDocIdSet};
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{
    DocIdSetIterator, Either2DocIdSetIterator, Either3DocIdSetIterator, EmptyDISI,
};
use crate::core::search::dummy::dummy_doc_id_set_iterator::DummyDocIdSetIterator;
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::query::QueryEnum;
use crate::core::search::query_caching_policy::QueryCachingPolicy;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::Either2Scorer;
use crate::core::search::scorer_supplier::{Either3ScorerSupplier, ScorerSupplier};
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::{DefaultBulkScorer, Weight};
use crate::core::util::accountable::Accountable;
use crate::core::util::bit_doc_id_set::BitDocIdSet;
use crate::core::util::bit_set::BitSet;
use crate::core::util::dummy::dummy_bits::DummyBits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::roaring_doc_id_set::RoaringDocIdSet;
use crate::core::util::roaring_doc_id_set::builder::Builder;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub struct LRUQueryCache;

pub(crate) struct CachingWrapperWeight<W, QCP, LR>
where
    W: Weight<LR>,
    QCP: QueryCachingPolicy,
    LR: LeafReader,
{
    in_: W,
    base: ConstantScoreWeight,
    policy: Rc<QCP>,
    used: AtomicBool,
    _marker: std::marker::PhantomData<LR>,
}
impl<W, QCP, LR> CachingWrapperWeight<W, QCP, LR>
where
    W: Weight<LR>,
    QCP: QueryCachingPolicy,
    LR: LeafReader,
{
    pub fn new(in_: W, policy: Rc<QCP>) -> Self {
        Self {
            in_,
            base: ConstantScoreWeight::new(1.0),
            policy,
            used: AtomicBool::new(false),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<W, QCP, LR> SegmentCacheable<LR> for CachingWrapperWeight<W, QCP, LR>
where
    LR: LeafReader,
    QCP: QueryCachingPolicy,
    W: Weight<LR>,
{
    fn is_cacheable(&self, ctx: &LeafReaderContext<LR>) -> bool {
        self.in_.is_cacheable(ctx)
    }
}

impl<W, QCP, LR> Weight<LR> for CachingWrapperWeight<W, QCP, LR>
where
    W: Weight<LR>,
    QCP: QueryCachingPolicy,
    LR: LeafReader,
{
    type Matches = W::Matches;

    fn matches(
        &mut self,
        context: &LeafReaderContext<LR>,
        doc: i32,
    ) -> Result<Option<Self::Matches>> {
        self.in_.matches(context, doc)
    }

    fn explain(&mut self, context: &LeafReaderContext<LR>, doc: i32) -> Result<Explanation> {
        let scorer = self.scorer(context)?;
        self.base.explain(scorer, doc, self.get_query().to_string())
    }

    type Query = W::Query;

    fn get_query(&self) -> &Self::Query {
        self.in_.get_query()
    }

    fn get_query_enum(&self) -> Arc<QueryEnum> {
        self.in_.get_query_enum()
    }

    type ScorerSupplier = CachingWrapperWeightSupplier<W, LR>;

    fn scorer_supplier(
        &mut self,
        context: &LeafReaderContext<LR>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        todo!()
    }
}
pub(crate) struct ScorerSupplierImpl1<S> {
    cost: i64,
    skip_cache_factor: f32,
    supplier: S,
    max_doc: i32,
}
impl<S> ScorerSupplierImpl1<S>
where
    S: ScorerSupplier,
{
    pub(crate) fn new(cost: i64, skip_cache_factor: f32, supplier: S, max_doc: i32) -> Result<Self>
    where
        S: ScorerSupplier,
    {
        Ok(Self {
            cost,
            skip_cache_factor,
            supplier,
            max_doc,
        })
    }
}
pub type DISI = Either2DocIdSetIterator<EmptyDISI, CacheAndCountDISI>;
impl<S> ScorerSupplier for ScorerSupplierImpl1<S>
where
    S: ScorerSupplier,
{
    type Scorer = Either2Scorer<S::Scorer, ConstantScoreScorer<DISI, DummyTwoPhaseIterator>>;
    type BulkScorer = DefaultBulkScorer<Self::Scorer>;

    fn get(&mut self, lead_cost: i64) -> Result<Option<Self::Scorer>> {
        if (self.cost as f32 / self.skip_cache_factor) > lead_cost as f32 {
            return match self.supplier.get(lead_cost)? {
                Some(scorer) => Ok(Some(Either2Scorer::A(scorer))),
                None => Ok(None),
            };
        };
        let cached = cache_impl(&mut self.supplier.bulk_scorer()?, self.max_doc)?;
        // TODO: 这里没有处理缓存
        let disi = match cached.iterator()? {
            Some(disi) => DISI::B(disi),
            None => DISI::A(EmptyDISI::default()),
        };
        Ok(Some(Either2Scorer::B(ConstantScoreScorer::with_disi(
            0.0,
            ScoreMode::CompleteNoScores,
            disi,
        ))))
    }

    fn bulk_scorer(&mut self) -> Result<Self::BulkScorer> {
        todo!()
    }

    fn cost(&mut self) -> Result<i64> {
        Ok(self.cost)
    }
}

pub(crate) struct ScorerSupplierImpl2 {
    disi: CacheAndCountDISI,
    cost: i64,
}
impl ScorerSupplierImpl2 {
    pub(crate) fn new(disi: CacheAndCountDISI) -> Result<Self> {
        let cost = disi.cost()?;
        Ok(Self { disi, cost })
    }
}
impl ScorerSupplier for ScorerSupplierImpl2 {
    type Scorer = ConstantScoreScorer<CacheAndCountDISI, DummyTwoPhaseIterator>;
    type BulkScorer = DefaultBulkScorer<Self::Scorer>;

    fn get(&mut self, _lead_cost: i64) -> Result<Option<Self::Scorer>> {
        Ok(Some(ConstantScoreScorer::with_disi(
            0.0,
            ScoreMode::CompleteNoScores,
            std::mem::take(&mut self.disi),
        )))
    }

    fn bulk_scorer(&mut self) -> Result<Self::BulkScorer> {
        self.default_bulk_scorer()
    }

    fn cost(&mut self) -> Result<i64> {
        Ok(self.cost)
    }
}
pub type CachingWrapperWeightSupplier<W, LR> = Either3ScorerSupplier<
    <W as Weight<LR>>::ScorerSupplier,
    ScorerSupplierImpl1<<W as Weight<LR>>::ScorerSupplier>,
    ScorerSupplierImpl2,
>;
/// Cache of doc ids with a count.
pub(crate) struct CacheAndCount<D>
where
    D: DocIdSet,
{
    cache: D,
    count: i32,
}
impl CacheAndCount<EmptyDocIdSet> {
    pub(crate) fn empty() -> Self {
        Self {
            cache: EmptyDocIdSet,
            count: 0,
        }
    }
}

impl<D> CacheAndCount<D>
where
    D: DocIdSet,
{
    pub(crate) fn new(cache: D, count: i32) -> Self {
        Self { cache, count }
    }

    pub(crate) fn iterator(&self) -> Result<Option<D::DocIdSetIterator>> {
        self.cache.iterator()
    }
    pub(crate) fn count(&self) -> i32 {
        self.count
    }
}
impl<D> Accountable for CacheAndCount<D>
where
    D: DocIdSet,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

fn cache_into_bit_set<BS>(
    scorer: &mut BS,
    max_doc: i32,
) -> Result<CacheAndCount<BitDocIdSet<FixedBitSet>>>
where
    BS: BulkScorer,
{
    let mut collector = LeafCollectorImpl::new(max_doc);
    scorer.score(&mut collector, None::<&DummyBits>, 0, NO_MORE_DOCS)?;
    let v = BitDocIdSet::with_cost(
        Some(std::mem::take(&mut collector.bit_set)),
        collector.count as i64,
    )?;
    Ok(CacheAndCount::new(v, collector.count))
}

struct LeafCollectorImpl {
    bit_set: FixedBitSet,
    count: i32,
}
impl LeafCollectorImpl {
    fn new(max_doc: i32) -> Self {
        Self {
            bit_set: FixedBitSet::new(max_doc),
            count: 0,
        }
    }
}

impl Display for LeafCollectorImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::any::type_name::<Self>())
    }
}

impl LeafCollector for LeafCollectorImpl {
    fn collect<S>(&mut self, doc: i32, _scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        self.count += 1;
        self.bit_set.set(doc);
        Ok(())
    }

    type DocIdSetIterator = DummyDocIdSetIterator;
}

fn cache_into_roaring_doc_id_set<BS>(
    scorer: &mut BS,
    max_doc: i32,
) -> Result<CacheAndCount<RoaringDocIdSet>>
where
    BS: BulkScorer,
{
    let mut collector = RoaringCollectorImpl::new(max_doc);
    scorer.score(&mut collector, None::<&DummyBits>, 0, NO_MORE_DOCS)?;
    let cache = collector.builder.build();
    let cardinality = cache.cardinality();
    Ok(CacheAndCount::new(cache, cardinality))
}

struct RoaringCollectorImpl {
    builder: Builder,
}

impl RoaringCollectorImpl {
    fn new(max_doc: i32) -> Self {
        Self {
            builder: Builder::new(max_doc),
        }
    }
}

impl Display for RoaringCollectorImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::any::type_name::<Self>())
    }
}

impl LeafCollector for RoaringCollectorImpl {
    fn collect<S>(&mut self, doc: i32, _scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        self.builder.add(doc)?;
        Ok(())
    }

    type DocIdSetIterator = DummyDocIdSetIterator;
}
pub(crate) enum CacheAndCountEnum {
    BitSet(CacheAndCount<BitDocIdSet<FixedBitSet>>),
    Roaring(CacheAndCount<RoaringDocIdSet>),
    Empty(CacheAndCount<EmptyDocIdSet>),
}
impl CacheAndCountEnum {
    pub(crate) fn count(&self) -> i32 {
        match self {
            CacheAndCountEnum::BitSet(c) => c.count(),
            CacheAndCountEnum::Roaring(c) => c.count(),
            CacheAndCountEnum::Empty(c) => c.count(),
        }
    }
    pub(crate) fn iterator(&self) -> Result<Option<CacheAndCountDISI>> {
        match self {
            CacheAndCountEnum::BitSet(c) => Ok(c.iterator()?.map(Either3DocIdSetIterator::B)),
            CacheAndCountEnum::Roaring(c) => Ok(c.iterator()?.map(Either3DocIdSetIterator::C)),
            CacheAndCountEnum::Empty(c) => Ok(c.iterator()?.map(Either3DocIdSetIterator::A)),
        }
    }
}
pub type CacheAndCountDISI = Either3DocIdSetIterator<
    <EmptyDocIdSet as DocIdSet>::DocIdSetIterator,
    <BitDocIdSet<FixedBitSet> as DocIdSet>::DocIdSetIterator,
    <RoaringDocIdSet as DocIdSet>::DocIdSetIterator,
>;
// for std::mem::take
impl Default for CacheAndCountDISI {
    fn default() -> Self {
        Either3DocIdSetIterator::A(EmptyDISI::default())
    }
}

fn cache_impl<BS>(scorer: &mut BS, max_doc: i32) -> Result<CacheAndCountEnum>
where
    BS: BulkScorer,
{
    let cost = scorer.cost()?;
    if cost * 100 >= max_doc as i64 {
        // FixedBitSet is faster for dense sets and will enable the random-access
        // optimization in ConjunctionDISI
        let v = cache_into_bit_set(scorer, max_doc)?;
        Ok(CacheAndCountEnum::BitSet(v))
    } else {
        let v = cache_into_roaring_doc_id_set(scorer, max_doc)?;
        Ok(CacheAndCountEnum::Roaring(v))
    }
}
