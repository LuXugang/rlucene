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
use crate::core::index::index_reader_context::{IRCTermState, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::query_timeout::QueryTimeout;
use crate::core::index::term_states::TermStates;
use crate::core::search::QueryCache;
use crate::core::search::bulk_scorer::{BulkScorer, BulkScorerEnum2, BulkScorerEnum9};
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_stream::DocIdStream;

use crate::core::document::sorted_numeric_doc_values_range_query::{
    SNDVRQSs, SortedNumericDocValuesRangeQueryWeight,
};
use crate::core::document::sorted_numeric_doc_values_set_query::{
    SNDVSQSs, SortedNumericDocValuesSetQueryWeight,
};
use crate::core::document::sorted_set_doc_values_range_query::{
    SSDVRQSs, SortedSetDocValuesRangeQueryWeight,
};
use crate::core::index::index_reader::Identity;
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::dummy::dummy_matches::DummyMatches;
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::field_exists_query::{FieldExistsESs, FieldExistsWeight};
use crate::core::search::filter_leaf_collector::FilterLeafCollector;
use crate::core::search::filter_scorable::FilterScorable;
use crate::core::search::index_searcher::{
    IndexSearcher, IndexSearcherWeight, IndexSearcherWeightSs, IndexSearcherWeightSsBulkScorer,
    IndexSearcherWeightSsScorerDisi, IndexSearcherWeightSsScorerTpi,
};
use crate::core::search::index_sort_sorted_numeric_doc_values_range_query::{
    ISSNDVRQSs, IndexSortSortedNumericDocValuesRangeQueryWeight,
};
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::match_all_docs_query::{MatchAllSs, MatchAllWeight};
use crate::core::search::match_no_docs_query::{MatchNoDocsSs, MatchNoDocsWeight};
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::point_range_query::{PointRangeSs, PointRangeWeight};
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::query_caching_policy::QueryCachingPolicy;
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, ScorerEnum2, ScorerEnum9, TwoPhaseState};
use crate::core::search::scorer_supplier::{ScorerSupplier, ScorerSupplierEnum9};
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::term_query::{TermSs, TermWeight};
use crate::core::search::weight::{DefaultBulkScorer, Weight, WeightEnum2, WeightEnum10};
use crate::core::util::bits::Bits;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// A query that wraps another query and simply returns a constant score equal to 1 for every document that matches the query.
/// It therefore simply strips of all scores and always returns 1.
#[derive(Debug)]
pub struct ConstantScoreQuery {
    id: Identity,
    query: Box<Query>,
}
impl ConstantScoreQuery {
    /// Strips off scores from the passed in Query. The hits will get a constant score of 1.
    pub fn new<T>(query: T) -> Self
    where
        T: Into<Box<Query>>,
    {
        let query = query.into();
        Self {
            id: Identity::new(),
            query,
        }
    }

    pub(crate) fn into_inner(self) -> Query {
        *self.query
    }
}
#[cfg(test)]
impl Clone for ConstantScoreQuery {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            query: self.query.clone(),
        }
    }
}

impl Eq for ConstantScoreQuery {}

impl PartialEq<Self> for ConstantScoreQuery {
    fn eq(&self, other: &Self) -> bool {
        self.query == other.query
    }
}

impl Hash for ConstantScoreQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::any::type_name::<Self>().to_string().hash(state);
        self.query.hash(state);
    }
}

impl HasIdentity for ConstantScoreQuery {
    fn identity(&self) -> &Identity {
        &self.id
    }
}
impl QueryBase for ConstantScoreQuery {
    fn as_string(&self, field: &str) -> String {
        let inner = self.query.as_string(field);
        format!("ConstantScore({})", inner)
    }

    type Weight<S, IRC, QCP, QC>
        = ConstantScoreQueryWeight<BaseQueryWeight<S, IRC>, IRC, QCP, QC>
    where
        S: Similarity,
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
        QC: QueryCache;
    fn create_weight<S, IRC, QT, QCP, QC>(
        self,
        searcher: &IndexSearcher<IRC, S, QT, QCP, QC>,
        score_mode: &ScoreMode,
        boost: f32,
        per_reader_term_state: Option<TermStates<IRCTermState<IRC>>>,
    ) -> Result<Self::Weight<S, IRC, QCP, QC>>
    where
        IRC: IndexReaderContext,
        S: Similarity,
        QT: QueryTimeout,
        QCP: QueryCachingPolicy,
        QC: QueryCache,
        Self: Sized,
    {
        let inner_score_mode = if score_mode.is_exhaustive() {
            ScoreMode::CompleteNoScores
        } else {
            ScoreMode::TopDocs
        };
        let query = *self.query;
        let inner_weight = query.create_weight_no_constant_score(
            searcher,
            &inner_score_mode,
            1.0,
            per_reader_term_state,
        )?;
        let inner_weight = searcher.wrap_weight(inner_weight, inner_score_mode);
        let v = if score_mode.needs_scores() {
            CSQWType::A(WeightImpl::<_, IRC, QCP, QC>::new(
                boost,
                inner_weight,
                *score_mode,
            ))
        } else {
            CSQWType::B(inner_weight)
        };
        let v = ConstantScoreQueryWeight::new(v);
        Ok(v)
    }

    fn rewrite<IRC, S, QT, QCP, QC>(
        mut self,
        searcher: &IndexSearcher<IRC, S, QT, QCP, QC>,
    ) -> Result<Query>
    where
        IRC: IndexReaderContext,
        S: Similarity,
        QT: QueryTimeout,
        QCP: QueryCachingPolicy,
        QC: QueryCache,
    {
        let query_id = self.query.identity().clone();
        let rewritten = self.query.rewrite(searcher)?;

        let rewritten = match rewritten {
            Query::Boost(b) => *b.query,
            Query::ConstantScore(cs) => *cs.query,
            // TODO IMPORTANT
            // Query::Boolean(bq) => bq.rewrite_no_scoring()?,
            q => q,
        };

        if let Query::MatchNoDoc(v) = rewritten {
            return Ok(v.into());
        }

        if rewritten.identity() != &query_id {
            return Ok(ConstantScoreQuery::new(rewritten).into());
        }

        if matches!(rewritten, Query::ConstantScore(_)) {
            return Ok(rewritten);
        }
        if let Query::Boost(v) = rewritten {
            return Ok(ConstantScoreQuery::new(v.query).into());
        }
        self.query = Box::new(rewritten);
        Ok(self.into())
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

pub struct WeightImpl<W, IRC, QCP, QC>
where
    IRC: IndexReaderContext,
    QCP: QueryCachingPolicy,
    QC: QueryCache,
    W: Weight<IRC::LeafReader>,
{
    base: ConstantScoreWeight,
    inner_weight: IndexSearcherWeight<W, IRC, QCP, QC>,
    score_mode: ScoreMode,
}
impl<W, IRC, QCP, QC> WeightImpl<W, IRC, QCP, QC>
where
    IRC: IndexReaderContext,
    QCP: QueryCachingPolicy,
    QC: QueryCache,
    W: Weight<IRC::LeafReader>,
{
    pub fn new(
        boost: f32,
        inner_weight: IndexSearcherWeight<W, IRC, QCP, QC>,
        score_mode: ScoreMode,
    ) -> Self {
        Self {
            base: ConstantScoreWeight::new(boost),
            inner_weight,
            score_mode,
        }
    }
}
impl<W, IRC, QCP, QC> SegmentCacheable<IRC::LeafReader> for WeightImpl<W, IRC, QCP, QC>
where
    IRC: IndexReaderContext,
    QCP: QueryCachingPolicy,
    QC: QueryCache,
    W: Weight<IRC::LeafReader>,
{
    fn is_cacheable(&self, ctx: &LeafReaderContext<IRC::LeafReader>) -> Result<bool> {
        self.inner_weight.is_cacheable(ctx)
    }
}

impl<W, IRC, QCP, QC> Weight<IRC::LeafReader> for WeightImpl<W, IRC, QCP, QC>
where
    IRC: IndexReaderContext,
    QCP: QueryCachingPolicy,
    QC: QueryCache,
    W: Weight<IRC::LeafReader>,
{
    type Matches = <IndexSearcherWeight<W, IRC, QCP, QC> as Weight<IRC::LeafReader>>::Matches;

    fn matches(
        &self,
        context: &LeafReaderContext<IRC::LeafReader>,
        doc: i32,
    ) -> Result<Option<Self::Matches>> {
        self.inner_weight.matches(context, doc)
    }

    fn explain(
        &self,
        context: &LeafReaderContext<IRC::LeafReader>,
        doc: i32,
    ) -> Result<Explanation> {
        let scorer = self.scorer(context)?;
        self.base
            .explain(scorer, doc, self.get_query().as_string(""))
    }

    fn get_query(&self) -> Arc<Query> {
        self.inner_weight.get_query()
    }

    type ScorerSupplier = ScorerSupplierImpl<W, IRC, QCP, QC>;

    fn scorer_supplier(
        &self,
        context: &LeafReaderContext<IRC::LeafReader>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        match self.inner_weight.scorer_supplier(context)? {
            Some(inner_scorer_supplier) => Ok(Some(ScorerSupplierImpl::new(
                self.score_mode,
                inner_scorer_supplier,
                self.base.score(),
            ))),
            None => Ok(None),
        }
    }

    fn count(&self, context: &LeafReaderContext<IRC::LeafReader>) -> Result<i32> {
        self.inner_weight.count(context)
    }
}

pub type ConstantScoreScorerEnum<W, IRC, QCP, QC> = ScorerEnum2<
    ConstantScoreScorer<IndexSearcherWeightSsScorerDisi<W, IRC, QCP, QC>, DummyTwoPhaseIterator>,
    ConstantScoreScorer<DummyDISI, IndexSearcherWeightSsScorerTpi<W, IRC, QCP, QC>>,
>;
pub type BulkScorerEnum<W, IRC, QCP, QC> = BulkScorerEnum2<
    DefaultBulkScorer<ConstantScoreScorerEnum<W, IRC, QCP, QC>>,
    ConstantBulkScorer<IndexSearcherWeightSsBulkScorer<W, IRC, QCP, QC>>,
>;
pub struct ScorerSupplierImpl<W, IRC, QCP, QC>
where
    W: Weight<IRC::LeafReader>,
    IRC: IndexReaderContext,
    QCP: QueryCachingPolicy,
    QC: QueryCache,
{
    score_mode: ScoreMode,
    inner_scorer_supplier: IndexSearcherWeightSs<W, IRC, QCP, QC>,
    score: f32,
}
impl<W, IRC, QCP, QC> ScorerSupplierImpl<W, IRC, QCP, QC>
where
    W: Weight<IRC::LeafReader>,
    IRC: IndexReaderContext,
    QCP: QueryCachingPolicy,
    QC: QueryCache,
{
    fn new(
        score_mode: ScoreMode,
        inner_scorer_supplier: IndexSearcherWeightSs<W, IRC, QCP, QC>,
        score: f32,
    ) -> Self {
        Self {
            score_mode,
            inner_scorer_supplier,
            score,
        }
    }
}
impl<W, IRC, QCP, QC> ScorerSupplier<IRC::LeafReader> for ScorerSupplierImpl<W, IRC, QCP, QC>
where
    W: Weight<IRC::LeafReader>,
    IRC: IndexReaderContext,
    QCP: QueryCachingPolicy,
    QC: QueryCache,
{
    type Scorer = ConstantScoreScorerEnum<W, IRC, QCP, QC>;
    type BulkScorer = BulkScorerEnum<W, IRC, QCP, QC>;

    fn get(
        &mut self,
        lead_cost: i64,
        context: &LeafReaderContext<IRC::LeafReader>,
    ) -> Result<Self::Scorer> {
        let inner_scorer = self.inner_scorer_supplier.get(lead_cost, context)?;
        let has_tpi = inner_scorer.has_two_phase_iterator() == TwoPhaseState::Yes
            || inner_scorer.two_phase_iterator()?.is_some();
        match has_tpi {
            true => {
                let tpi = inner_scorer.take_two_phase_iterator()?.unwrap();
                let v = ConstantScoreScorer::with_tpi(self.score, self.score_mode, tpi);
                Ok(ConstantScoreScorerEnum::<W, IRC, QCP, QC>::B(v))
            },
            false => {
                let disi = inner_scorer.take_iterator();
                let v = ConstantScoreScorer::with_disi(self.score, self.score_mode, disi);
                Ok(ConstantScoreScorerEnum::<W, IRC, QCP, QC>::A(v))
            },
        }
    }

    fn bulk_scorer(
        &mut self,
        context: &LeafReaderContext<IRC::LeafReader>,
    ) -> Result<Option<Self::BulkScorer>> {
        if !self.score_mode.is_exhaustive() {
            let v = self.default_bulk_scorer(context)?;
            return Ok(Some(BulkScorerEnum::<W, IRC, QCP, QC>::A(v)));
        }
        match self.inner_scorer_supplier.bulk_scorer(context)? {
            Some(v) => {
                let v = ConstantBulkScorer::new(v, self.score);
                Ok(Some(BulkScorerEnum::<W, IRC, QCP, QC>::B(v)))
            },
            None => Ok(None),
        }
    }

    fn cost(&mut self, context: &LeafReaderContext<IRC::LeafReader>) -> Result<i64> {
        self.inner_scorer_supplier.cost(context)
    }
}
/// We return this as our BulkScorer so that if the CSQ wraps a query with its own optimized top-level scorer (e.g. BooleanScorer) we can use that top-level scorer.
pub struct ConstantBulkScorer<BS>
where
    BS: BulkScorer,
{
    bulk_scorer: BS,
    the_score: f32,
}
impl<BS> ConstantBulkScorer<BS>
where
    BS: BulkScorer,
{
    pub fn new(bulk_scorer: BS, the_score: f32) -> Self {
        Self {
            bulk_scorer,
            the_score,
        }
    }
    fn wrap_collector<LC>(collector: LC, the_score: f32) -> FilterLeafCollectorImpl<LC>
    where
        LC: LeafCollector,
    {
        FilterLeafCollectorImpl::new(collector, the_score)
    }
}
impl<BS> BulkScorer for ConstantBulkScorer<BS>
where
    BS: BulkScorer,
{
    fn score<LC, B>(
        &mut self,
        collector: &mut LC,
        accept_docs: Option<&B>,
        min: i32,
        max: i32,
    ) -> Result<i32>
    where
        LC: LeafCollector,
        B: Bits,
    {
        self.bulk_scorer.score(
            &mut Self::wrap_collector(collector, self.the_score),
            accept_docs,
            min,
            max,
        )
    }

    fn cost(&mut self) -> Result<i64> {
        self.bulk_scorer.cost()
    }
}

pub struct FilterLeafCollectorImpl<LC>
where
    LC: LeafCollector,
{
    in_: FilterLeafCollector<LC>,
    the_score: f32,
}

impl<LC> FilterLeafCollectorImpl<LC>
where
    LC: LeafCollector,
{
    pub fn new(in_: LC, the_score: f32) -> Self {
        let base = FilterLeafCollector::new(in_);
        Self {
            in_: base,
            the_score,
        }
    }
}

impl<LC> Display for FilterLeafCollectorImpl<LC>
where
    LC: LeafCollector + Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", std::any::type_name::<Self>(), self.in_)
    }
}

impl<LC> LeafCollector for FilterLeafCollectorImpl<LC>
where
    LC: LeafCollector,
{
    fn set_scorer<S>(&mut self, scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        let mut v = FilterScorableImpl::new(self.the_score, scorer);
        self.in_.set_scorer(&mut v)
    }

    fn collect<S>(&mut self, doc: i32, scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        self.in_.collect(doc, scorer)
    }

    fn collect_stream<DS>(&mut self, stream: &mut DS) -> Result<()>
    where
        DS: DocIdStream,
    {
        self.in_.collect_stream(stream)
    }

    type DocIdSetIteratorRef<'b>
        = <FilterLeafCollector<LC> as LeafCollector>::DocIdSetIteratorRef<'b>
    where
        Self: 'b;

    fn competitive_iterator(&mut self) -> Result<Option<Self::DocIdSetIteratorRef<'_>>> {
        self.in_.competitive_iterator()
    }

    fn finish(&mut self) -> Result<()> {
        self.in_.finish()
    }
}

pub struct FilterScorableImpl<'a, S>
where
    S: Scorable,
{
    the_score: f32,
    base: FilterScorable<'a, S>,
}
impl<'a, S> FilterScorableImpl<'a, S>
where
    S: Scorable,
{
    pub(crate) fn new(the_score: f32, s: &'a mut S) -> Self {
        let base = FilterScorable::new(s);
        Self { the_score, base }
    }
}
impl<'a, S> Scorable for FilterScorableImpl<'a, S>
where
    S: Scorable,
{
    fn score(&mut self) -> Result<f32> {
        Ok(self.the_score)
    }

    fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
        self.base.smoothing_score(doc_id)
    }

    fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
        self.base.set_min_competitive_score(min_score)
    }

    type Scorable = <FilterScorable<'a, S> as Scorable>::Scorable;

    fn get_children(&self) -> Result<Vec<ChildScorable<Self::Scorable>>> {
        self.base.get_children()
    }

    fn cost(&mut self) -> Result<i64> {
        self.base.cost()
    }
}
pub type CSQWType<W, IRC, QCP, QC> =
    WeightEnum2<WeightImpl<W, IRC, QCP, QC>, IndexSearcherWeight<W, IRC, QCP, QC>>;

pub type ConstantScoreSs<W, IRC, QCP, QC> =
    <CSQWType<W, IRC, QCP, QC> as Weight<<IRC as IndexReaderContext>::LeafReader>>::ScorerSupplier;
pub type ConstantScoreSsScorer<W, IRC, QCP, QC> = <ConstantScoreSs<W, IRC, QCP, QC> as ScorerSupplier<<IRC as IndexReaderContext>::LeafReader>>::Scorer;
pub type ConstantScoreSsScorerDisi<W, IRC, QCP, QC> =
    <ConstantScoreSsScorer<W, IRC, QCP, QC> as Scorer>::DocIdSetIterator;
pub type ConstantScoreSsScorerDisiRef<'a, W, IRC, QCP, QC> =
    <ConstantScoreSsScorer<W, IRC, QCP, QC> as Scorer>::DocIdSetIteratorRef<'a>;
pub type ConstantScoreSsScorerDisiMut<'a, W, IRC, QCP, QC> =
    <ConstantScoreSsScorer<W, IRC, QCP, QC> as Scorer>::DocIdSetIteratorMut<'a>;
pub type ConstantScoreSsScorerTpi<W, IRC, QCP, QC> =
    <ConstantScoreSsScorer<W, IRC, QCP, QC> as Scorer>::TwoPhaseIter;
pub type ConstantScoreSsBulkScorer<W, IRC, QCP, QC> = <ConstantScoreSs<W, IRC, QCP, QC> as ScorerSupplier<<IRC as IndexReaderContext>::LeafReader>>::BulkScorer;
pub struct ConstantScoreQueryWeight<W, IRC, QCP, QC>
where
    W: Weight<IRC::LeafReader>,
    IRC: IndexReaderContext,
    QCP: QueryCachingPolicy,
    QC: QueryCache,
{
    inner: CSQWType<W, IRC, QCP, QC>,
}
impl<W, IRC, QCP, QC> ConstantScoreQueryWeight<W, IRC, QCP, QC>
where
    W: Weight<IRC::LeafReader>,
    IRC: IndexReaderContext,
    QCP: QueryCachingPolicy,
    QC: QueryCache,
{
    pub fn new(inner: CSQWType<W, IRC, QCP, QC>) -> Self {
        Self { inner }
    }
}
impl<W, IRC, QCP, QC> SegmentCacheable<IRC::LeafReader>
    for ConstantScoreQueryWeight<W, IRC, QCP, QC>
where
    W: Weight<IRC::LeafReader>,
    IRC: IndexReaderContext,
    QCP: QueryCachingPolicy,
    QC: QueryCache,
{
    fn is_cacheable(&self, ctx: &LeafReaderContext<IRC::LeafReader>) -> Result<bool> {
        self.inner.is_cacheable(ctx)
    }
}
impl<W, IRC, QCP, QC> Weight<IRC::LeafReader> for ConstantScoreQueryWeight<W, IRC, QCP, QC>
where
    W: Weight<IRC::LeafReader>,
    IRC: IndexReaderContext,
    QCP: QueryCachingPolicy,
    QC: QueryCache,
{
    type Matches = <CSQWType<W, IRC, QCP, QC> as Weight<IRC::LeafReader>>::Matches;

    fn matches(
        &self,
        context: &LeafReaderContext<IRC::LeafReader>,
        doc: i32,
    ) -> Result<Option<Self::Matches>> {
        self.inner.matches(context, doc)
    }

    fn default_matches(
        &self,
        _context: &LeafReaderContext<IRC::LeafReader>,
        _doc: i32,
    ) -> Result<Option<MatchWithNoTerms>> {
        self.inner.default_matches(_context, _doc)
    }

    fn explain(
        &self,
        context: &LeafReaderContext<IRC::LeafReader>,
        doc: i32,
    ) -> Result<Explanation> {
        self.inner.explain(context, doc)
    }

    fn get_query(&self) -> Arc<Query> {
        self.inner.get_query()
    }

    fn scorer(
        &self,
        _context: &LeafReaderContext<IRC::LeafReader>,
    ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<IRC::LeafReader>>::Scorer>> {
        self.inner.scorer(_context)
    }

    type ScorerSupplier = ConstantScoreSs<W, IRC, QCP, QC>;

    fn scorer_supplier(
        &self,
        _context: &LeafReaderContext<IRC::LeafReader>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        self.inner.scorer_supplier(_context)
    }

    fn bulk_scorer(
        &self,
        _context: &LeafReaderContext<IRC::LeafReader>,
    ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<IRC::LeafReader>>::BulkScorer>> {
        self.inner.bulk_scorer(_context)
    }

    fn count(&self, context: &LeafReaderContext<IRC::LeafReader>) -> Result<i32> {
        self.inner.count(context)
    }

    fn default_count(&self, _context: &LeafReaderContext<IRC::LeafReader>) -> Result<i32> {
        self.inner.default_count(_context)
    }

    fn is_weight_cacheable(&self) -> bool {
        self.inner.is_weight_cacheable()
    }
}

pub type QueryBaseWeight<Q, S, IRC, QCP, QC> = <Q as QueryBase>::Weight<S, IRC, QCP, QC>;

pub type QueryWeight1<S, IRC, QCP, QC> = WeightEnum10<
    TermWeight<S, IRC>,
    MatchAllWeight<<IRC as IndexReaderContext>::LeafReader>,
    PointRangeWeight<<IRC as IndexReaderContext>::LeafReader>,
    MatchNoDocsWeight<<IRC as IndexReaderContext>::LeafReader>,
    SortedNumericDocValuesSetQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    SortedNumericDocValuesRangeQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    SortedSetDocValuesRangeQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    IndexSortSortedNumericDocValuesRangeQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    FieldExistsWeight<<IRC as IndexReaderContext>::LeafReader>,
    ConstantScoreQueryWeight<BaseQueryWeight<S, IRC>, IRC, QCP, QC>,
>;
impl<S, IRC> SegmentCacheable<IRC::LeafReader> for BaseQueryWeight<S, IRC>
where
    IRC: IndexReaderContext,
    S: Similarity,
{
    fn is_cacheable(&self, ctx: &LeafReaderContext<IRC::LeafReader>) -> Result<bool> {
        match self {
            BaseQueryWeight::Term(w) => w.is_cacheable(ctx),
            BaseQueryWeight::MatchAll(w) => w.is_cacheable(ctx),
            BaseQueryWeight::PointRange(w) => w.is_cacheable(ctx),
            BaseQueryWeight::MatchNoDocs(w) => w.is_cacheable(ctx),
            BaseQueryWeight::SortedNumericDocValuesSet(w) => w.is_cacheable(ctx),
            BaseQueryWeight::SortedNumericDocValuesRange(w) => w.is_cacheable(ctx),
            BaseQueryWeight::SortedSetDocValuesRange(w) => w.is_cacheable(ctx),
            BaseQueryWeight::IndexSortSortedNumericDocValuesRange(w) => w.is_cacheable(ctx),
            BaseQueryWeight::FieldExists(w) => w.is_cacheable(ctx),
        }
    }
}

pub enum BaseQueryWeight<S, IRC>
where
    IRC: IndexReaderContext,
    S: Similarity,
{
    Term(TermWeight<S, IRC>),
    MatchAll(MatchAllWeight<<IRC as IndexReaderContext>::LeafReader>),
    PointRange(PointRangeWeight<<IRC as IndexReaderContext>::LeafReader>),
    MatchNoDocs(MatchNoDocsWeight<<IRC as IndexReaderContext>::LeafReader>),
    SortedNumericDocValuesSet(
        SortedNumericDocValuesSetQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    ),
    SortedNumericDocValuesRange(
        SortedNumericDocValuesRangeQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    ),
    SortedSetDocValuesRange(
        SortedSetDocValuesRangeQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    ),
    IndexSortSortedNumericDocValuesRange(
        IndexSortSortedNumericDocValuesRangeQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    ),
    FieldExists(FieldExistsWeight<<IRC as IndexReaderContext>::LeafReader>),
}

impl<S, IRC> Weight<IRC::LeafReader> for BaseQueryWeight<S, IRC>
where
    IRC: IndexReaderContext,
    S: Similarity,
{
    type Matches = DummyMatches;

    fn matches(
        &self,
        _context: &LeafReaderContext<IRC::LeafReader>,
        _doc: i32,
    ) -> Result<Option<Self::Matches>> {
        todo!()
    }

    fn default_matches(
        &self,
        context: &LeafReaderContext<IRC::LeafReader>,
        doc: i32,
    ) -> Result<Option<MatchWithNoTerms>> {
        match self {
            BaseQueryWeight::Term(w) => w.default_matches(context, doc),
            BaseQueryWeight::MatchAll(w) => w.default_matches(context, doc),
            BaseQueryWeight::PointRange(w) => w.default_matches(context, doc),
            BaseQueryWeight::MatchNoDocs(w) => w.default_matches(context, doc),
            BaseQueryWeight::SortedNumericDocValuesSet(w) => w.default_matches(context, doc),
            BaseQueryWeight::SortedNumericDocValuesRange(w) => w.default_matches(context, doc),
            BaseQueryWeight::SortedSetDocValuesRange(w) => w.default_matches(context, doc),
            BaseQueryWeight::IndexSortSortedNumericDocValuesRange(w) => {
                w.default_matches(context, doc)
            },
            BaseQueryWeight::FieldExists(w) => w.default_matches(context, doc),
        }
    }

    fn explain(
        &self,
        context: &LeafReaderContext<IRC::LeafReader>,
        doc: i32,
    ) -> Result<Explanation> {
        match self {
            BaseQueryWeight::Term(w) => w.explain(context, doc),
            BaseQueryWeight::MatchAll(w) => w.explain(context, doc),
            BaseQueryWeight::PointRange(w) => w.explain(context, doc),
            BaseQueryWeight::MatchNoDocs(w) => w.explain(context, doc),
            BaseQueryWeight::SortedNumericDocValuesSet(w) => w.explain(context, doc),
            BaseQueryWeight::SortedNumericDocValuesRange(w) => w.explain(context, doc),
            BaseQueryWeight::SortedSetDocValuesRange(w) => w.explain(context, doc),
            BaseQueryWeight::IndexSortSortedNumericDocValuesRange(w) => w.explain(context, doc),
            BaseQueryWeight::FieldExists(w) => w.explain(context, doc),
        }
    }

    fn get_query(&self) -> Arc<Query> {
        match self {
            BaseQueryWeight::Term(w) => w.get_query(),
            BaseQueryWeight::MatchAll(w) => w.get_query(),
            BaseQueryWeight::PointRange(w) => w.get_query(),
            BaseQueryWeight::MatchNoDocs(w) => w.get_query(),
            BaseQueryWeight::SortedNumericDocValuesSet(w) => w.get_query(),
            BaseQueryWeight::SortedNumericDocValuesRange(w) => w.get_query(),
            BaseQueryWeight::SortedSetDocValuesRange(w) => w.get_query(),
            BaseQueryWeight::IndexSortSortedNumericDocValuesRange(w) => w.get_query(),
            BaseQueryWeight::FieldExists(w) => w.get_query(),
        }
    }

    fn scorer(
        &self,
        context: &LeafReaderContext<IRC::LeafReader>,
    ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<IRC::LeafReader>>::Scorer>> {
        match self {
            BaseQueryWeight::Term(w) => Ok(w.scorer(context)?.map(ScorerEnum9::A)),
            BaseQueryWeight::MatchAll(w) => Ok(w.scorer(context)?.map(ScorerEnum9::B)),
            BaseQueryWeight::PointRange(w) => Ok(w.scorer(context)?.map(ScorerEnum9::C)),
            BaseQueryWeight::MatchNoDocs(w) => Ok(w.scorer(context)?.map(ScorerEnum9::D)),
            BaseQueryWeight::SortedNumericDocValuesSet(w) => {
                Ok(w.scorer(context)?.map(ScorerEnum9::E))
            },
            BaseQueryWeight::SortedNumericDocValuesRange(w) => {
                Ok(w.scorer(context)?.map(ScorerEnum9::F))
            },
            BaseQueryWeight::SortedSetDocValuesRange(w) => {
                Ok(w.scorer(context)?.map(ScorerEnum9::G))
            },
            BaseQueryWeight::IndexSortSortedNumericDocValuesRange(w) => {
                Ok(w.scorer(context)?.map(ScorerEnum9::H))
            },
            BaseQueryWeight::FieldExists(w) => Ok(w.scorer(context)?.map(ScorerEnum9::I)),
        }
    }

    type ScorerSupplier = ScorerSupplierEnum9<
        TermSs<IRC, S>,
        MatchAllSs,
        PointRangeSs<IRC::LeafReader>,
        MatchNoDocsSs,
        SNDVSQSs<IRC::LeafReader>,
        SNDVRQSs<IRC::LeafReader>,
        SSDVRQSs<IRC::LeafReader>,
        ISSNDVRQSs<IRC::LeafReader>,
        FieldExistsESs<IRC::LeafReader>,
    >;

    fn scorer_supplier(
        &self,
        context: &LeafReaderContext<IRC::LeafReader>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        match self {
            BaseQueryWeight::Term(w) => Ok(w.scorer_supplier(context)?.map(ScorerSupplierEnum9::A)),
            BaseQueryWeight::MatchAll(w) => {
                Ok(w.scorer_supplier(context)?.map(ScorerSupplierEnum9::B))
            },
            BaseQueryWeight::PointRange(w) => {
                Ok(w.scorer_supplier(context)?.map(ScorerSupplierEnum9::C))
            },
            BaseQueryWeight::MatchNoDocs(w) => {
                Ok(w.scorer_supplier(context)?.map(ScorerSupplierEnum9::D))
            },
            BaseQueryWeight::SortedNumericDocValuesSet(w) => {
                Ok(w.scorer_supplier(context)?.map(ScorerSupplierEnum9::E))
            },
            BaseQueryWeight::SortedNumericDocValuesRange(w) => {
                Ok(w.scorer_supplier(context)?.map(ScorerSupplierEnum9::F))
            },
            BaseQueryWeight::SortedSetDocValuesRange(w) => {
                Ok(w.scorer_supplier(context)?.map(ScorerSupplierEnum9::G))
            },
            BaseQueryWeight::IndexSortSortedNumericDocValuesRange(w) => {
                Ok(w.scorer_supplier(context)?.map(ScorerSupplierEnum9::H))
            },
            BaseQueryWeight::FieldExists(w) => {
                Ok(w.scorer_supplier(context)?.map(ScorerSupplierEnum9::I))
            },
        }
    }

    fn bulk_scorer(
        &self,
        context: &LeafReaderContext<IRC::LeafReader>,
    ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<IRC::LeafReader>>::BulkScorer>> {
        match self {
            BaseQueryWeight::Term(w) => Ok(w.bulk_scorer(context)?.map(BulkScorerEnum9::A)),
            BaseQueryWeight::MatchAll(w) => Ok(w.bulk_scorer(context)?.map(BulkScorerEnum9::B)),
            BaseQueryWeight::PointRange(w) => Ok(w.bulk_scorer(context)?.map(BulkScorerEnum9::C)),
            BaseQueryWeight::MatchNoDocs(w) => Ok(w.bulk_scorer(context)?.map(BulkScorerEnum9::D)),
            BaseQueryWeight::SortedNumericDocValuesSet(w) => {
                Ok(w.bulk_scorer(context)?.map(BulkScorerEnum9::E))
            },
            BaseQueryWeight::SortedNumericDocValuesRange(w) => {
                Ok(w.bulk_scorer(context)?.map(BulkScorerEnum9::F))
            },
            BaseQueryWeight::SortedSetDocValuesRange(w) => {
                Ok(w.bulk_scorer(context)?.map(BulkScorerEnum9::G))
            },
            BaseQueryWeight::IndexSortSortedNumericDocValuesRange(w) => {
                Ok(w.bulk_scorer(context)?.map(BulkScorerEnum9::H))
            },
            BaseQueryWeight::FieldExists(w) => Ok(w.bulk_scorer(context)?.map(BulkScorerEnum9::I)),
        }
    }

    fn count(&self, context: &LeafReaderContext<IRC::LeafReader>) -> Result<i32> {
        match self {
            BaseQueryWeight::Term(w) => w.count(context),
            BaseQueryWeight::MatchAll(w) => w.count(context),
            BaseQueryWeight::PointRange(w) => w.count(context),
            BaseQueryWeight::MatchNoDocs(w) => w.count(context),
            BaseQueryWeight::SortedNumericDocValuesSet(w) => w.count(context),
            BaseQueryWeight::SortedNumericDocValuesRange(w) => w.count(context),
            BaseQueryWeight::SortedSetDocValuesRange(w) => w.count(context),
            BaseQueryWeight::IndexSortSortedNumericDocValuesRange(w) => w.count(context),
            BaseQueryWeight::FieldExists(w) => w.count(context),
        }
    }

    fn default_count(&self, _context: &LeafReaderContext<IRC::LeafReader>) -> Result<i32> {
        match self {
            BaseQueryWeight::Term(w) => w.default_count(_context),
            BaseQueryWeight::MatchAll(w) => w.default_count(_context),
            BaseQueryWeight::PointRange(w) => w.default_count(_context),
            BaseQueryWeight::MatchNoDocs(w) => w.default_count(_context),
            BaseQueryWeight::SortedNumericDocValuesSet(w) => w.default_count(_context),
            BaseQueryWeight::SortedNumericDocValuesRange(w) => w.default_count(_context),
            BaseQueryWeight::SortedSetDocValuesRange(w) => w.default_count(_context),
            BaseQueryWeight::IndexSortSortedNumericDocValuesRange(w) => w.default_count(_context),
            BaseQueryWeight::FieldExists(w) => w.default_count(_context),
        }
    }

    fn is_weight_cacheable(&self) -> bool {
        match self {
            BaseQueryWeight::Term(w) => w.is_weight_cacheable(),
            BaseQueryWeight::MatchAll(w) => w.is_weight_cacheable(),
            BaseQueryWeight::PointRange(w) => w.is_weight_cacheable(),
            BaseQueryWeight::MatchNoDocs(w) => w.is_weight_cacheable(),
            BaseQueryWeight::SortedNumericDocValuesSet(w) => w.is_weight_cacheable(),
            BaseQueryWeight::SortedNumericDocValuesRange(w) => w.is_weight_cacheable(),
            BaseQueryWeight::SortedSetDocValuesRange(w) => w.is_weight_cacheable(),
            BaseQueryWeight::IndexSortSortedNumericDocValuesRange(w) => w.is_weight_cacheable(),
            BaseQueryWeight::FieldExists(w) => w.is_weight_cacheable(),
        }
    }
}
impl Query {
    pub(crate) fn create_weight_no_constant_score<S, IRC, QT, QCP, QC>(
        self,
        searcher: &IndexSearcher<IRC, S, QT, QCP, QC>,
        score_mode: &ScoreMode,
        boost: f32,
        per_reader_term_state: Option<TermStates<IRCTermState<IRC>>>,
    ) -> Result<BaseQueryWeight<S, IRC>>
    where
        IRC: IndexReaderContext,
        S: Similarity,
        QT: QueryTimeout,
        QCP: QueryCachingPolicy,
        QC: QueryCache,
    {
        match self {
            Query::Term(t) => Ok(BaseQueryWeight::Term(t.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            Query::MatchAll(m) => Ok(BaseQueryWeight::MatchAll(m.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            Query::PointRange(p) => Ok(BaseQueryWeight::PointRange(p.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            Query::MatchNoDoc(p) => Ok(BaseQueryWeight::MatchNoDocs(p.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            Query::SortedNumericDocValuesSet(p) => Ok(BaseQueryWeight::SortedNumericDocValuesSet(
                p.create_weight(searcher, score_mode, boost, per_reader_term_state)?,
            )),
            Query::SortedNumericDocValuesRange(p) => {
                Ok(BaseQueryWeight::SortedNumericDocValuesRange(
                    p.create_weight(searcher, score_mode, boost, per_reader_term_state)?,
                ))
            },
            Query::SortedSetDocValuesRange(p) => Ok(BaseQueryWeight::SortedSetDocValuesRange(
                p.create_weight(searcher, score_mode, boost, per_reader_term_state)?,
            )),
            Query::IndexSortSortedNumericDocValuesRange(p) => {
                Ok(BaseQueryWeight::IndexSortSortedNumericDocValuesRange(
                    p.create_weight(searcher, score_mode, boost, per_reader_term_state)?,
                ))
            },
            Query::FieldExists(p) => Ok(BaseQueryWeight::FieldExists(p.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            Query::ConstantScore(p) => p.into_inner().create_weight_no_constant_score(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            ),
            _ => Err(LuceneError::illegal_argument("")),
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::core::index::multi_reader::MultiReader;
    use crate::core::search::constant_score_query::ConstantScoreQuery;
    use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
    use crate::core::search::query::Query;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::util::lucene_test_case::lucene_test_case_util::new_searcher_with_reader;

    #[test]
    fn test_csq() -> Result<()> {
        // TODO TermRangeQuery未实现
        Ok(())
    }
    #[test]
    fn test_wrapped_2_times() -> Result<()> {
        // TODO BooleanQuery未实现
        Ok(())
    }

    #[test]
    fn test_constant_score_query_and_filter() -> Result<()> {
        // TODO BooleanQuery未实现
        Ok(())
    }
    #[test]
    fn test_propagates_approximations() -> Result<()> {
        // TODO PhraseQuery未实现
        Ok(())
    }
    #[test]
    fn test_rewrite_bubbles_up_match_no_docs_query() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;
        let query: Query = MatchNoDocsQuery::new().into();
        let query = ConstantScoreQuery::new(query);
        let rewritten = searcher.rewrite(query.into())?;
        assert_eq!(rewritten, Query::MatchNoDoc(MatchNoDocsQuery::new()));
        Ok(())
    }
}
