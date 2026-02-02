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
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::{LRTermState, LeafReader};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::term_states::TermStates;
use crate::core::search::QueryCache;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::dummy::dummy_matches::DummyMatches;
use crate::core::search::dummy::dummy_scorer_supplier::DummyScorerSupplier;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::Weight;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::Result;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// A query that wraps another query and simply returns a constant score equal to 1 for every document that matches the query.
/// It therefore simply strips of all scores and always returns 1.
#[derive(Debug, Clone)]
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

    fn create_weight<IRC, QC>(
        self,
        _searcher: &IndexSearcher<IRC, QC>,
        _score_mode: &ScoreMode,
        _boost: f32,
        _per_reader_term_state: Option<TermStates<LRTermState<IRCLeafReader<IRC>>>>,
    ) -> Result<QueryWeight<IRCLeafReader<IRC>>>
    where
        IRC: IndexReaderContext,
        QC: QueryCache,
        Self: Sized,
        <IRC as IndexReaderContext>::LeafReader: 'static,
    {
        // let inner_score_mode = if score_mode.is_exhaustive() {
        //     ScoreMode::CompleteNoScores
        // } else {
        //     ScoreMode::TopDocs
        // };
        // let query = *self.query;
        // let inner_weight =
        //     query.create_weight(searcher, &inner_score_mode, 1.0, per_reader_term_state)?;
        // let inner_weight = searcher.wrap_weight(inner_weight, inner_score_mode);
        // let v = if score_mode.needs_scores() {
        //     CSQWType::A(WeightImpl::new(boost, inner_weight, *score_mode))
        // } else {
        //     CSQWType::B(inner_weight)
        // };
        // let v = ConstantScoreQueryWeight::new(v);
        // Ok(v)
        todo!()
    }

    fn rewrite<IRC, QC>(mut self, searcher: &IndexSearcher<IRC, QC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        QC: QueryCache,
    {
        let query_id = self.query.identity().clone();
        let rewritten = self.query.rewrite(searcher)?;

        let rewritten = match rewritten {
            Query::Boost(b) => *b.query,
            Query::ConstantScore(cs) => cs.into_inner(),
            // TODO IMPORTANT BooleanQuery
            q => q,
        };

        if let Query::MatchNoDoc(v) = rewritten {
            return Ok(v.into());
        }

        if rewritten.identity() != &query_id {
            return Ok(ConstantScoreQuery::new(rewritten).into());
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

pub struct WeightImpl<LR>
where
    LR: LeafReader,
{
    base: ConstantScoreWeight,
    inner_weight: QueryWeight<LR>,
    score_mode: ScoreMode,
}
impl<LR> WeightImpl<LR>
where
    LR: LeafReader,
{
    pub fn new(boost: f32, inner_weight: QueryWeight<LR>, score_mode: ScoreMode) -> Self {
        Self {
            base: ConstantScoreWeight::new(boost),
            inner_weight,
            score_mode,
        }
    }
}
impl<LR> SegmentCacheable<LR> for WeightImpl<LR>
where
    LR: LeafReader,
{
    fn is_cacheable(&self, ctx: &LeafReaderContext<LR>) -> Result<bool> {
        self.inner_weight.is_cacheable(ctx)
    }
}

impl<LR> Weight<LR> for WeightImpl<LR>
where
    LR: LeafReader,
{
    // TODO IMPORTANT
    type Matches = DummyMatches;

    fn matches(
        &self,
        _context: &LeafReaderContext<LR>,
        _doc: i32,
    ) -> Result<Option<Self::Matches>> {
        todo!()
        // self.inner_weight.matches(context, doc)
    }

    fn explain(&self, context: &LeafReaderContext<LR>, doc: i32) -> Result<Explanation> {
        let scorer = self.scorer(context)?;
        self.base
            .explain(scorer, doc, self.get_query().as_string(""))
    }

    fn get_query(&self) -> Arc<Query> {
        self.inner_weight.get_query()
    }

    // type ScorerSupplier = ScorerSupplierImpl<LR>;
    type ScorerSupplier = DummyScorerSupplier;

    fn scorer_supplier(
        &self,
        _context: &LeafReaderContext<LR>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        // match self.inner_weight.scorer_supplier(context)? {
        //     Some(inner_scorer_supplier) => Ok(Some(ScorerSupplierImpl::new(
        //         self.score_mode,
        //         inner_scorer_supplier,
        //         self.base.score(),
        //     ))),
        //     None => Ok(None),
        // }
        todo!()
    }

    fn count(&self, context: &LeafReaderContext<LR>) -> Result<i32> {
        self.inner_weight.count(context)
    }
}

// pub struct ScorerSupplierImpl<LR>
// where
//     LR: LeafReader,
// {
//     score_mode: ScoreMode,
//     inner_scorer_supplier: QueryWeightSs<LR>,
//     score: f32,
// }
// impl<LR> ScorerSupplierImpl<LR>
// where
//     LR: LeafReader,
// {
//     fn new(
//         score_mode: ScoreMode,
//         inner_scorer_supplier: QueryWeightSs<LR>,
//         score: f32,
//     ) -> Self {
//         Self {
//             score_mode,
//             inner_scorer_supplier,
//             score,
//         }
//     }
// }
// impl<LR> ScorerSupplier<LR> for ScorerSupplierImpl<LR>
// where
//     LR: LeafReader,
// {
//     type Scorer = DummyScorer;
//     type BulkScorer = DummyBulkScorer;
//
//     fn get(&mut self, lead_cost: i64, context: &LeafReaderContext<LR>) -> Result<Self::Scorer> {
//         // let inner_scorer = self.inner_scorer_supplier.get(lead_cost, context)?;
//         // let has_tpi = inner_scorer.has_two_phase_iterator() == TwoPhaseState::Yes
//         //     || inner_scorer.two_phase_iterator()?.is_some();
//         // match has_tpi {
//         //     true => {
//         //         let tpi = inner_scorer.take_two_phase_iterator()?.unwrap();
//         //         let v = ConstantScoreScorer::with_tpi(self.score, self.score_mode, tpi);
//         //         Ok(ConstantScoreScorerEnum::<LR>::B(v))
//         //     },
//         //     false => {
//         //         let disi = inner_scorer.take_iterator();
//         //         let v = ConstantScoreScorer::with_disi(self.score, self.score_mode, disi);
//         //         Ok(ConstantScoreScorerEnum::<LR>::A(v))
//         //     },
//         // }
//         todo!()
//     }
//
//     fn bulk_scorer(&mut self, context: &LeafReaderContext<LR>) -> Result<Option<Self::BulkScorer>> {
//         // if !self.score_mode.is_exhaustive() {
//         //     let v = self.default_bulk_scorer(context)?;
//         //     return Ok(Some(BulkScorerEnum::<LR>::A(v)));
//         // }
//         // match self.inner_scorer_supplier.bulk_scorer(context)? {
//         //     Some(v) => {
//         //         let v = ConstantBulkScorer::new(v, self.score);
//         //         Ok(Some(BulkScorerEnum::<LR>::B(v)))
//         //     },
//         //     None => Ok(None),
//         // }
//         todo!()
//     }
//
//     fn cost(&mut self, context: &LeafReaderContext<LR>) -> Result<i64> {
//         // self.inner_scorer_supplier.cost(context)
//         todo!()
//     }
// }
// /// We return this as our BulkScorer so that if the CSQ wraps a query with its own optimized top-level scorer (e.g. BooleanScorer) we can use that top-level scorer.
// pub struct ConstantBulkScorer<BS>
// where
//     BS: BulkScorer,
// {
//     bulk_scorer: BS,
//     the_score: f32,
// }
// impl<BS> ConstantBulkScorer<BS>
// where
//     BS: BulkScorer,
// {
//     pub fn new(bulk_scorer: BS, the_score: f32) -> Self {
//         Self {
//             bulk_scorer,
//             the_score,
//         }
//     }
//     fn wrap_collector<LC>(collector: LC, the_score: f32) -> FilterLeafCollectorImpl<LC>
//     where
//         LC: LeafCollector,
//     {
//         FilterLeafCollectorImpl::new(collector, the_score)
//     }
// }
// impl<BS> BulkScorer for ConstantBulkScorer<BS>
// where
//     BS: BulkScorer,
// {
//     fn score<LC, B>(
//         &mut self,
//         collector: &mut LC,
//         accept_docs: Option<&B>,
//         min: i32,
//         max: i32,
//     ) -> Result<i32>
//     where
//         LC: LeafCollector,
//         B: Bits,
//     {
//         self.bulk_scorer.score(
//             &mut Self::wrap_collector(collector, self.the_score),
//             accept_docs,
//             min,
//             max,
//         )
//     }
//
//     fn cost(&mut self) -> Result<i64> {
//         self.bulk_scorer.cost()
//     }
// }
//
// pub struct FilterLeafCollectorImpl<LC>
// where
//     LC: LeafCollector,
// {
//     in_: FilterLeafCollector<LC>,
//     the_score: f32,
// }
//
// impl<LC> FilterLeafCollectorImpl<LC>
// where
//     LC: LeafCollector,
// {
//     pub fn new(in_: LC, the_score: f32) -> Self {
//         let base = FilterLeafCollector::new(in_);
//         Self {
//             in_: base,
//             the_score,
//         }
//     }
// }
//
// impl<LC> Display for FilterLeafCollectorImpl<LC>
// where
//     LC: LeafCollector + Display,
// {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{} ({})", std::any::type_name::<Self>(), self.in_)
//     }
// }
//
// impl<LC> LeafCollector for FilterLeafCollectorImpl<LC>
// where
//     LC: LeafCollector,
// {
//     fn set_scorer<S>(&mut self, scorer: &mut S) -> Result<()>
//     where
//         S: Scorable,
//     {
//         let mut v = FilterScorableImpl::new(self.the_score, scorer);
//         self.in_.set_scorer(&mut v)
//     }
//
//     fn collect<S>(&mut self, doc: i32, scorer: &mut S) -> Result<()>
//     where
//         S: Scorable,
//     {
//         self.in_.collect(doc, scorer)
//     }
//
//     fn collect_stream<DS>(&mut self, stream: &mut DS) -> Result<()>
//     where
//         DS: DocIdStream,
//     {
//         self.in_.collect_stream(stream)
//     }
//
//     type DocIdSetIteratorRef<'b>
//         = <FilterLeafCollector<LC> as LeafCollector>::DocIdSetIteratorRef<'b>
//     where
//         Self: 'b;
//
//     fn competitive_iterator(&mut self) -> Result<Option<Self::DocIdSetIteratorRef<'_>>> {
//         self.in_.competitive_iterator()
//     }
//
//     fn finish(&mut self) -> Result<()> {
//         self.in_.finish()
//     }
// }
//
// pub struct FilterScorableImpl<'a, S>
// where
//     S: Scorable,
// {
//     the_score: f32,
//     base: FilterScorable<'a, S>,
// }
// impl<'a, S> FilterScorableImpl<'a, S>
// where
//     S: Scorable,
// {
//     pub(crate) fn new(the_score: f32, s: &'a mut S) -> Self {
//         let base = FilterScorable::new(s);
//         Self { the_score, base }
//     }
// }
// impl<'a, S> Scorable for FilterScorableImpl<'a, S>
// where
//     S: Scorable,
// {
//     fn score(&mut self) -> Result<f32> {
//         Ok(self.the_score)
//     }
//
//     fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
//         self.base.smoothing_score(doc_id)
//     }
//
//     fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
//         self.base.set_min_competitive_score(min_score)
//     }
//
//     type Scorable = <FilterScorable<'a, S> as Scorable>::Scorable;
//
//     fn get_children(&self) -> Result<Vec<ChildScorable<Self::Scorable>>> {
//         self.base.get_children()
//     }
//
//     fn cost(&mut self) -> Result<i64> {
//         self.base.cost()
//     }
// }
// pub type CSQWType<LR> = WeightEnum2<WeightImpl<LR>, QueryWeight<LR>>;
//
// pub type ConstantScoreSs<LR> = <CSQWType<LR> as Weight<LR>>::ScorerSupplier;
// pub type ConstantScoreSsScorer<LR> =
//     <ConstantScoreSs<LR> as ScorerSupplier<LR>>::Scorer;
// pub type ConstantScoreSsScorerDisi<LR> =
//     <ConstantScoreSsScorer<LR> as Scorer>::DocIdSetIterator;
// pub type ConstantScoreSsScorerDisiRef<'a, LR> =
//     <ConstantScoreSsScorer<LR> as Scorer>::DocIdSetIteratorRef<'a>;
// pub type ConstantScoreSsScorerDisiMut<'a, LR> =
//     <ConstantScoreSsScorer<LR> as Scorer>::DocIdSetIteratorMut<'a>;
// pub type ConstantScoreSsScorerTpi<LR> =
//     <ConstantScoreSsScorer<LR> as Scorer>::TwoPhaseIter;
// pub type ConstantScoreSsBulkScorer<LR> =
//     <ConstantScoreSs<LR> as ScorerSupplier<LR>>::BulkScorer;
pub struct ConstantScoreQueryWeight<LR>
where
    LR: LeafReader,
{
    // inner: CSQWType<LR>,
    inner: QueryWeight<LR>,
}
impl<LR> ConstantScoreQueryWeight<LR>
where
    LR: LeafReader,
{
    pub fn new(inner: QueryWeight<LR>) -> Self {
        Self { inner }
    }
}
impl<LR> SegmentCacheable<LR> for ConstantScoreQueryWeight<LR>
where
    LR: LeafReader,
{
    fn is_cacheable(&self, ctx: &LeafReaderContext<LR>) -> Result<bool> {
        self.inner.is_cacheable(ctx)
    }
}
impl<LR> Weight<LR> for ConstantScoreQueryWeight<LR>
where
    LR: LeafReader,
{
    type Matches = DummyMatches;

    fn matches(
        &self,
        _context: &LeafReaderContext<LR>,
        _doc: i32,
    ) -> Result<Option<Self::Matches>> {
        // self.inner.matches(context, doc)
        todo!()
    }

    fn default_matches(
        &self,
        _context: &LeafReaderContext<LR>,
        _doc: i32,
    ) -> Result<Option<MatchWithNoTerms>> {
        self.inner.default_matches(_context, _doc)
    }

    fn explain(&self, context: &LeafReaderContext<LR>, doc: i32) -> Result<Explanation> {
        self.inner.explain(context, doc)
    }

    fn get_query(&self) -> Arc<Query> {
        self.inner.get_query()
    }

    fn scorer(
        &self,
        _context: &LeafReaderContext<LR>,
    ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<LR>>::Scorer>> {
        todo!()
        // self.inner.scorer(_context)
    }

    // type ScorerSupplier = ConstantScoreSs<LR>;
    type ScorerSupplier = DummyScorerSupplier;

    fn scorer_supplier(
        &self,
        _context: &LeafReaderContext<LR>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        todo!()
        // self.inner.scorer_supplier(_context)
    }

    fn bulk_scorer(
        &self,
        _context: &LeafReaderContext<LR>,
    ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier<LR>>::BulkScorer>> {
        todo!()
        // self.inner.bulk_scorer(_context)
    }

    fn count(&self, context: &LeafReaderContext<LR>) -> Result<i32> {
        self.inner.count(context)
    }

    fn default_count(&self, _context: &LeafReaderContext<LR>) -> Result<i32> {
        self.inner.default_count(_context)
    }

    fn is_weight_cacheable(&self) -> bool {
        self.inner.is_weight_cacheable()
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
