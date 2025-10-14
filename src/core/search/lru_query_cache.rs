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
use crate::core::index::index_reader::{CacheHelper, CacheKey};
use crate::core::index::index_reader_context::IndexReaderContext;
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
use crate::core::search::query::{IdentityQuery, QueryEnum};
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
use crate::core::util::predicate::Predicate;
use crate::core::util::roaring_doc_id_set::RoaringDocIdSet;
use crate::core::util::roaring_doc_id_set::builder::Builder;
use linked_hash_map::LinkedHashMap;
use parking_lot::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};

pub struct LRUQueryCache {
    max_size: i32,
    max_ram_bytes_used: i64,
    rwlock: RwLock<()>,
    skip_cache_factor: f32,
    hit_count: AtomicU64,
    miss_count: AtomicU64,
    // these variables are volatile so that we do not need to sync reads
    // but increments need to be performed under the lock
    ram_bytes_used: AtomicI64,
    cache_count: AtomicI64,
    cache_size: AtomicI64,
    inner: RwLock<Inner>,
}
pub struct Inner {
    unique_queries: Mutex<LinkedHashMap<Arc<QueryEnum>, IdentityQuery>>,
    cache: HashMap<CacheKey, LeafCache>,
}

impl LRUQueryCache {
    pub(crate) fn on_hit(&self, _reader_core_key: &CacheKey, _query: &QueryEnum) {
        self.hit_count.fetch_add(1, Ordering::Relaxed);
    }
    pub(crate) fn on_miss(&self, _reader_core_key: &CacheKey, _query: &QueryEnum) {
        self.miss_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    pub(crate) fn on_query_cache(
        &self,
        _query: &QueryEnum,
        ram_bytes_used: i64,
        _rwlock: &RwLockWriteGuard<Inner>,
    ) {
        self.ram_bytes_used
            .fetch_add(ram_bytes_used, Ordering::Relaxed);
    }
    pub(crate) fn on_query_eviction(
        &self,
        _query: &QueryEnum,
        ram_bytes_used: i64,
        _guard: &RwLockWriteGuard<Inner>,
    ) {
        self.ram_bytes_used
            .fetch_sub(ram_bytes_used, Ordering::Relaxed);
    }
    pub(crate) fn on_doc_id_set_cache(&self, _reader_core_key: &CacheKey, ram_bytes_used: i64) {
        self.cache_size.fetch_add(1, Ordering::Relaxed);
        self.cache_count.fetch_add(1, Ordering::Relaxed);
        self.ram_bytes_used
            .fetch_add(ram_bytes_used, Ordering::Relaxed);
    }

    pub(crate) fn on_doc_id_set_eviction(
        &self,
        _reader_core_key: &CacheKey,
        num_entries: i64,
        sum_ram_bytes_used: i64,
    ) {
        self.ram_bytes_used
            .fetch_sub(sum_ram_bytes_used, Ordering::Relaxed);
        self.cache_size.fetch_sub(num_entries, Ordering::Relaxed);
    }

    pub(crate) fn on_clear(&self, _guard: &RwLockWriteGuard<Inner>) {
        self.ram_bytes_used.store(0, Ordering::Relaxed);
        self.cache_size.store(0, Ordering::Relaxed);
    }
    pub(crate) fn requires_eviction(&self, guard: &RwLockWriteGuard<Inner>) -> bool {
        let size = guard.unique_queries.lock().len();
        if size == 0 {
            return false;
        }
        size as i32 > self.max_size
            || self.ram_bytes_used.load(Ordering::Relaxed) > self.max_ram_bytes_used
    }
    pub(crate) fn get<C>(
        &self,
        key: &QueryEnum,
        cache_helper: &C,
        inner: &RwLockReadGuard<Inner>,
    ) -> Option<Arc<CacheAndCountEnum>>
    where
        C: CacheHelper,
    {
        // TODO: 这里没有assert

        let reader_key = cache_helper.get_key();

        let leaf_cache = match inner.cache.get(&reader_key) {
            Some(c) => c,
            None => {
                self.on_miss(&reader_key, key);
                return None;
            },
        };
        // this get call moves the query to the most-recently-used position
        let mut unique_queries = inner.unique_queries.lock();
        let singleton = match unique_queries.get_refresh(key) {
            Some(c) => c,
            None => {
                self.on_miss(&reader_key, key);
                return None;
            },
        };

        match leaf_cache.get(singleton) {
            Some(c) => {
                self.on_hit(&reader_key, singleton.query.as_ref());
                Some(c)
            },
            None => {
                self.on_miss(&reader_key, singleton.query.as_ref());
                None
            },
        }
    }

    pub(crate) fn put_if_absent<C>(
        &self,
        query: Arc<QueryEnum>,
        cached: CacheAndCountEnum,
        cache_helper: &C,
    ) where
        C: CacheHelper,
    {
        // TODO: 这里没有assert
        // under a lock to make sure that mostRecentlyUsedQueries and cache remain sync'ed
        let mut inner = self.inner.write();

        let (singleton, inserted) = {
            let mut uq = inner.unique_queries.lock();
            if let Some(iq) = uq.get_refresh(query.as_ref()) {
                (iq.clone(), false)
            } else {
                let iq = IdentityQuery::new(query.clone());
                let prev = uq.insert(query, iq.clone());
                debug_assert!(prev.is_none());
                (iq, true)
            }
        };

        if inserted {
            self.on_query_cache(
                singleton.query.as_ref(),
                self.get_ram_bytes_used(singleton.query.as_ref()),
                &inner,
            );
        }

        let key = cache_helper.get_key();
        let leaf_cache = match inner.cache.entry(key.clone()) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(cache) => {
                let leaf_cache = LeafCache::new(key);
                let lc_ref = cache.insert(leaf_cache);
                self.ram_bytes_used.fetch_add(
                    // TODO: memory calculation not implemented
                    0,
                    std::sync::atomic::Ordering::Relaxed,
                );
                // TODO: IMPORTANT 这里没有调用add_close_listener
                lc_ref
            },
        };

        leaf_cache.put_if_absent(singleton, cached, self);
        self.evict_if_necessary(&mut inner);
    }
    pub(crate) fn evict_if_necessary(&self, guard: &mut RwLockWriteGuard<Inner>) {
        loop {
            if !self.requires_eviction(guard) {
                break;
            }

            let singleton = {
                let mut unique_queries = guard.unique_queries.lock();
                match unique_queries.pop_front() {
                    Some((_key, singleton)) => singleton,
                    None => break,
                }
            };
            self.on_eviction(singleton, guard);
        }
    }

    /// Remove all cache entries for the given core cache key.
    pub(crate) fn clear_core_cache_key(&self, core_key: &CacheKey) {
        let mut inner = self.inner.write();

        if let Some(leaf_cache) = inner.cache.remove(core_key) {
            // TODO: memory calculation not implemented
            self.ram_bytes_used
                .fetch_sub(0, std::sync::atomic::Ordering::Relaxed);

            let num_entries = leaf_cache.cache.len();
            debug_assert!(num_entries <= i64::MAX as usize);
            if num_entries > 0 {
                self.on_doc_id_set_eviction(
                    core_key,
                    num_entries as i64,
                    leaf_cache
                        .ram_bytes_used
                        .load(std::sync::atomic::Ordering::Relaxed),
                );
            } else {
                debug_assert_eq!(num_entries, 0);
                debug_assert_eq!(
                    leaf_cache
                        .ram_bytes_used
                        .load(std::sync::atomic::Ordering::Relaxed),
                    0
                );
            }
        }
    }
    /// Remove all cache entries for the given query.
    pub fn clear_query(&self, query: &QueryEnum) {
        let mut inner = self.inner.write();
        let v = {
            let mut unique_queries = inner.unique_queries.lock();
            unique_queries.remove(query)
        };
        if let Some(singleton) = v {
            self.on_eviction(singleton, &mut inner);
        }
    }

    pub(crate) fn on_eviction(
        &self,
        singleton: IdentityQuery,
        guard: &mut RwLockWriteGuard<Inner>,
    ) {
        self.on_query_eviction(
            singleton.query.as_ref(),
            self.get_ram_bytes_used(singleton.query.as_ref()),
            guard,
        );

        for leaf_cache in guard.cache.values_mut() {
            leaf_cache.remove(&singleton, self);
        }
    }

    pub(crate) fn clear(&self) {
        let mut inner = self.inner.write();
        inner.cache.clear();
        inner.unique_queries.lock().clear();
        self.on_clear(&inner);
    }
    fn get_ram_bytes_used(&self, _query: &QueryEnum) -> i64 {
        // TODO: memory calculation not implemented
        0
    }

    pub fn get_total_count(&self) -> u64 {
        self.get_hit_count() + self.get_miss_count()
    }

    pub fn get_hit_count(&self) -> u64 {
        self.hit_count.load(Ordering::Relaxed)
    }

    pub fn get_miss_count(&self) -> u64 {
        self.miss_count.load(Ordering::Relaxed)
    }

    pub fn get_cache_size(&self) -> i64 {
        self.cache_size.load(Ordering::Relaxed)
    }

    pub fn get_cache_count(&self) -> i64 {
        self.cache_count.load(Ordering::Relaxed)
    }

    pub fn get_eviction_count(&self) -> i64 {
        self.get_cache_count() - self.get_cache_size()
    }
    #[cfg(test)]
    pub(crate) fn assert_consistent(&self) -> Result<()> {
        use std::collections::HashSet;
        let inner = self.inner.write();

        if self.requires_eviction(&inner) {
            debug_assert!(
                false,
                "requires evictions: size={}, maxSize={}, ramBytesUsed={}, maxRamBytesUsed={}",
                inner.unique_queries.lock().len(),
                self.max_size,
                self.ram_bytes_used.load(Ordering::Relaxed),
                self.max_ram_bytes_used
            );
        }

        let mru_id_set: HashSet<IdentityQuery> = {
            let uq = inner.unique_queries.lock();
            uq.values().cloned().collect()
        };

        for leaf_cache in inner.cache.values() {
            let mut keys: HashSet<IdentityQuery> = leaf_cache.cache.keys().cloned().collect();
            keys.retain(|k| !mru_id_set.contains(k));
            if !keys.is_empty() {
                debug_assert!(
                    false,
                    "One leaf cache contains more keys than the top-level cache: {:?}",
                    keys
                );
            }
        }

        // TODO: memory calculation not implemented
        let mut recomputed_ram_bytes_used = 0 * (inner.cache.len() as i64);

        {
            let uq = inner.unique_queries.lock();
            for singleton in uq.values() {
                recomputed_ram_bytes_used += self.get_ram_bytes_used(singleton.query.as_ref());
            }
        }

        for leaf_cache in inner.cache.values() {
            recomputed_ram_bytes_used +=
                // TODO: memory calculation not implemented
                0 * (leaf_cache.cache.len() as i64);
            for cached in leaf_cache.cache.values() {
                recomputed_ram_bytes_used += cached.ram_bytes_used()?;
            }
        }

        let current_ram = self.ram_bytes_used.load(Ordering::Relaxed);
        if recomputed_ram_bytes_used != current_ram {
            debug_assert!(
                false,
                "ramBytesUsed mismatch : {} != {}",
                current_ram, recomputed_ram_bytes_used
            );
        }

        let mut recomputed_cache_size: i64 = 0;
        for leaf_cache in inner.cache.values() {
            recomputed_cache_size += leaf_cache.cache.len() as i64;
        }
        if recomputed_cache_size != self.get_cache_size() {
            panic!(
                "cacheSize mismatch : {} != {}",
                self.get_cache_size(),
                recomputed_cache_size
            );
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn cached_queries(&self) -> Vec<Arc<QueryEnum>> {
        let inner = self.inner.read();
        let uq = inner.unique_queries.lock();
        uq.keys().cloned().collect()
    }
}
impl Accountable for LRUQueryCache {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

pub(crate) struct LeafCache {
    key: CacheKey,
    cache: HashMap<IdentityQuery, Arc<CacheAndCountEnum>>,
    ram_bytes_used: AtomicI64,
}
impl LeafCache {
    pub(crate) fn new(key: CacheKey) -> Self {
        Self {
            key,
            cache: HashMap::new(),
            ram_bytes_used: AtomicI64::new(0),
        }
    }
    pub(crate) fn on_doc_id_set_cache(&self, ram_bytes_used: i64, parent: &LRUQueryCache) {
        self.ram_bytes_used
            .fetch_add(ram_bytes_used, std::sync::atomic::Ordering::Relaxed);
        parent.on_doc_id_set_cache(&self.key, ram_bytes_used);
    }
    pub(crate) fn on_doc_id_set_eviction(&self, ram_bytes_used: i64, parent: &LRUQueryCache) {
        self.ram_bytes_used
            .fetch_sub(ram_bytes_used, std::sync::atomic::Ordering::Relaxed);
        parent.on_doc_id_set_eviction(&self.key, 1, ram_bytes_used);
    }

    pub(crate) fn get(&self, query: &IdentityQuery) -> Option<Arc<CacheAndCountEnum>> {
        // TODO: 没有assert
        self.cache.get(query).cloned()
    }

    pub(crate) fn put_if_absent(
        &mut self,
        query: IdentityQuery,
        cached: CacheAndCountEnum,
        parent: &LRUQueryCache,
    ) {
        // TODO: 没有assert
        match self.cache.entry(query) {
            Entry::Vacant(e) => {
                e.insert(Arc::new(cached));
                self.on_doc_id_set_cache(
                    // TODO: memory calculation not implemented
                    0, parent,
                );
            },
            Entry::Occupied(_) => {},
        }
    }

    pub(crate) fn remove(&mut self, query: &IdentityQuery, parent: &LRUQueryCache) {
        if let Some(removed) = self.cache.remove(query) {
            self.on_doc_id_set_eviction(
                // TODO: memory calculation not implemented
                0, parent,
            );
        }
    }
}
impl Accountable for LeafCache {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}
pub(crate) struct CachingWrapperWeight<W, QCP, LR>
where
    W: Weight<LR>,
    QCP: QueryCachingPolicy,
    LR: LeafReader,
{
    in_: W,
    base: ConstantScoreWeight,
    policy: Arc<QCP>,
    used: AtomicBool,
    _marker: std::marker::PhantomData<LR>,
}
impl<W, QCP, LR> CachingWrapperWeight<W, QCP, LR>
where
    W: Weight<LR>,
    QCP: QueryCachingPolicy,
    LR: LeafReader,
{
    pub fn new(in_: W, policy: Arc<QCP>) -> Self {
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
impl Accountable for CacheAndCountEnum {
    fn ram_bytes_used(&self) -> Result<i64> {
        match self {
            CacheAndCountEnum::BitSet(c) => c.ram_bytes_used(),
            CacheAndCountEnum::Roaring(c) => c.ram_bytes_used(),
            CacheAndCountEnum::Empty(c) => c.ram_bytes_used(),
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
pub(crate) struct MinSegmentSizePredicate<LR> {
    min_size: i32,
    _marker: std::marker::PhantomData<LR>,
}
impl<LR> MinSegmentSizePredicate<LR>
where
    LR: LeafReader,
{
    pub fn new(min_size: i32) -> Self {
        Self {
            min_size,
            _marker: std::marker::PhantomData,
        }
    }
}
impl<LR> Predicate<LeafReaderContext<LR>> for MinSegmentSizePredicate<LR>
where
    LR: LeafReader,
{
    fn test(&self, context: &LeafReaderContext<LR>) -> Result<bool> {
        let max_doc = context.reader().max_doc()?;
        if max_doc < self.min_size {
            return Ok(false);
        }
        todo!()
    }
}
