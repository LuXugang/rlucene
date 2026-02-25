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
use crate::core::index::leaf_reader::LRTermState;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::term_states::TermStates;
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_stream::DocIdStream;
use crate::core::search::explanation::Explanation;
use crate::core::search::filter_scorable::FilterScorable;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{
    Query, QueryBase, QueryWeight, QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer,
};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::Weight;
use crate::core::util::bits::Bits;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Debug, Display, Formatter};
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

    fn create_weight<IRC>(
        self,
        searcher: &IndexSearcher<IRC>,
        score_mode: &ScoreMode,
        boost: f32,
        per_reader_term_state: Option<TermStates<LRTermState<IRCLeafReader<IRC>>>>,
    ) -> Result<QueryWeight<IRC>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
        IRCLeafReader<IRC>: 'static,
    {
        let inner_score_mode = if score_mode.is_exhaustive() {
            ScoreMode::CompleteNoScores
        } else {
            ScoreMode::TopDocs
        };
        let query = *self.query;
        let inner_weight =
            query.create_weight(searcher, &inner_score_mode, 1.0, per_reader_term_state)?;
        let v: QueryWeight<IRC> = if score_mode.needs_scores() {
            Box::new(WeightImpl::new(boost, inner_weight, *score_mode))
        } else {
            inner_weight
        };
        Ok(Box::new(ConstantScoreQueryWeight::new(v)))
    }

    fn rewrite<IRC>(mut self, searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
    {
        let query_id = self.query.identity().clone();
        let rewritten = self.query.rewrite(searcher)?;

        let rewritten = match rewritten {
            Query::Boost(b) => b.into_inner(),
            Query::ConstantScore(cs) => cs.into_inner(),
            Query::Boolean(cs) => cs.rewrite_no_scoring()?,
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

pub struct WeightImpl<IRC>
where
    IRC: IndexReaderContext,
    IRCLeafReader<IRC>: 'static,
{
    base: ConstantScoreWeight,
    inner_weight: QueryWeight<IRC>,
    score_mode: ScoreMode,
}
impl<IRC> WeightImpl<IRC>
where
    IRC: IndexReaderContext,
    IRCLeafReader<IRC>: 'static,
{
    pub fn new(boost: f32, inner_weight: QueryWeight<IRC>, score_mode: ScoreMode) -> Self {
        Self {
            base: ConstantScoreWeight::new(boost),
            inner_weight,
            score_mode,
        }
    }
}
impl<IRC> SegmentCacheable for WeightImpl<IRC>
where
    IRC: IndexReaderContext,
    IRCLeafReader<IRC>: 'static,
{
    type IRC = IRC;

    fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
        self.inner_weight.is_cacheable(ctx)
    }
}

impl<IRC> Weight for WeightImpl<IRC>
where
    IRC: IndexReaderContext,
    IRCLeafReader<IRC>: 'static,
{
    // TODO IMPORTANT
    type Matches = MatchWithNoTerms;

    fn matches(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        doc: i32,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::Matches>> {
        self.inner_weight.matches(context, doc, searcher)
    }

    fn explain(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        doc: i32,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Explanation> {
        let scorer = self.scorer(context, searcher)?;
        self.base
            .explain(scorer, doc, self.get_query().as_string(""))
    }

    fn get_query(&self) -> Arc<Query> {
        self.inner_weight.get_query()
    }

    type ScorerSupplier = QueryWeightSs<IRC>;

    fn scorer_supplier(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        match self.inner_weight.scorer_supplier(context, searcher)? {
            Some(inner_scorer_supplier) => Ok(Some(Box::new(ScorerSupplierImpl::new(
                self.score_mode,
                inner_scorer_supplier,
                self.base.score(),
            )))),
            None => Ok(None),
        }
    }

    fn count(&self, context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
        self.inner_weight.count(context)
    }
}
pub struct ScorerSupplierImpl<IRC>
where
    IRC: IndexReaderContext,
{
    score_mode: ScoreMode,
    inner_scorer_supplier: QueryWeightSs<IRC>,
    score: f32,
}
impl<IRC> ScorerSupplierImpl<IRC>
where
    IRC: IndexReaderContext,
{
    fn new(score_mode: ScoreMode, inner_scorer_supplier: QueryWeightSs<IRC>, score: f32) -> Self {
        Self {
            score_mode,
            inner_scorer_supplier,
            score,
        }
    }
}
impl<IRC> ScorerSupplier for ScorerSupplierImpl<IRC>
where
    IRC: IndexReaderContext,
{
    type Scorer = QueryWeightSsScorer;
    type BulkScorer = QueryWeightSsBulkScorer;
    type IRC = IRC;

    fn get(
        &mut self,
        lead_cost: i64,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
    ) -> Result<Self::Scorer> {
        let inner_scorer = self.inner_scorer_supplier.get(lead_cost, context)?;
        match inner_scorer.has_two_phase_iterator() {
            TwoPhaseState::Yes => {
                let tpi = inner_scorer
                    .take_two_phase_iterator()
                    .ok_or_else(|| LuceneError::illegal_state("no tpi?"))?;
                let v = ConstantScoreScorer::from_tpi(self.score, self.score_mode, tpi);
                Ok(Box::new(v))
            },
            TwoPhaseState::No => {
                let disi = inner_scorer.take_iterator();
                let v = ConstantScoreScorer::from_disi(self.score, self.score_mode, disi);
                Ok(Box::new(v))
            },
        }
    }

    fn bulk_scorer(
        &mut self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
    ) -> Result<Option<Self::BulkScorer>> {
        if !self.score_mode.is_exhaustive() {
            let v = self.default_bulk_scorer(context)?;
            return Ok(Some(Box::new(v)));
        }
        match self.inner_scorer_supplier.bulk_scorer(context)? {
            Some(v) => {
                let v = ConstantBulkScorer::new(v, self.score);
                Ok(Some(Box::new(v)))
            },
            None => Ok(None),
        }
    }

    fn cost(&mut self, context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i64> {
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
    fn score(
        &mut self,
        collector: &mut dyn LeafCollector,
        accept_docs: Option<&dyn Bits>,
        min: i32,
        max: i32,
    ) -> Result<i32> {
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
    in_: LC,
    the_score: f32,
}

impl<LC> FilterLeafCollectorImpl<LC>
where
    LC: LeafCollector,
{
    pub fn new(in_: LC, the_score: f32) -> Self {
        Self { in_, the_score }
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
    fn finish(&mut self) -> Result<()> {
        self.in_.finish()
    }

    fn set_scorer(&mut self, scorer: &mut dyn Scorable) -> Result<()> {
        let mut v = FilterScorableImpl::new(self.the_score, scorer);
        self.in_.set_scorer(&mut v)
    }

    fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
        self.in_.collect(doc, scorer)
    }

    fn collect_stream(&mut self, stream: &mut dyn DocIdStream) -> Result<()> {
        self.in_.collect_stream(stream)
    }

    fn competitive_iterator(&mut self) -> Result<Option<Box<dyn DocIdSetIterator + '_>>> {
        self.in_.competitive_iterator()
    }
}

pub struct FilterScorableImpl<'a, S>
where
    S: Scorable + ?Sized,
{
    the_score: f32,
    base: FilterScorable<'a, S>,
}
impl<'a, S> FilterScorableImpl<'a, S>
where
    S: Scorable + ?Sized,
{
    pub(crate) fn new(the_score: f32, s: &'a mut S) -> Self {
        let base = FilterScorable::new(s);
        Self { the_score, base }
    }
}
impl<'a, S> Scorable for FilterScorableImpl<'a, S>
where
    S: Scorable + ?Sized,
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

    fn get_children(&self) -> Result<Vec<ChildScorable<Box<dyn Scorable>>>> {
        self.base.get_children()
    }

    fn cost(&self) -> Result<i64> {
        self.base.cost()
    }
}
pub struct ConstantScoreQueryWeight<IRC>
where
    IRC: IndexReaderContext,
    IRCLeafReader<IRC>: 'static,
{
    inner: QueryWeight<IRC>,
}
impl<IRC> ConstantScoreQueryWeight<IRC>
where
    IRC: IndexReaderContext,
    IRCLeafReader<IRC>: 'static,
{
    pub fn new(inner: QueryWeight<IRC>) -> Self {
        Self { inner }
    }
}
impl<IRC> SegmentCacheable for ConstantScoreQueryWeight<IRC>
where
    IRC: IndexReaderContext,
    IRCLeafReader<IRC>: 'static,
{
    type IRC = IRC;

    fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
        self.inner.is_cacheable(ctx)
    }
}
impl<IRC> Weight for ConstantScoreQueryWeight<IRC>
where
    IRC: IndexReaderContext,
    IRCLeafReader<IRC>: 'static,
{
    type Matches = MatchWithNoTerms;

    fn matches(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        doc: i32,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::Matches>> {
        self.inner.matches(context, doc, searcher)
    }

    fn default_matches(
        &self,
        _context: &LeafReaderContext<IRCLeafReader<IRC>>,
        _doc: i32,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<MatchWithNoTerms>> {
        self.inner.default_matches(_context, _doc, searcher)
    }

    fn explain(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        doc: i32,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Explanation> {
        self.inner.explain(context, doc, searcher)
    }

    fn get_query(&self) -> Arc<Query> {
        self.inner.get_query()
    }

    type ScorerSupplier = QueryWeightSs<IRC>;

    fn scorer_supplier(
        &self,
        _context: &LeafReaderContext<IRCLeafReader<IRC>>,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        self.inner.scorer_supplier(_context, searcher)
    }

    fn bulk_scorer(
        &self,
        _context: &LeafReaderContext<IRCLeafReader<IRC>>,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<<Self::ScorerSupplier as ScorerSupplier>::BulkScorer>> {
        self.inner.bulk_scorer(_context, searcher)
    }

    fn count(&self, context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
        self.inner.count(context)
    }

    fn default_count(&self, _context: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<i32> {
        self.inner.default_count(_context)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::document::document::Document;
    use crate::core::document::field::Store;
    use crate::core::index::multi_reader::MultiReader;
    use crate::core::index::term::Term;
    use crate::core::search::boolean_clause::Occur;
    use crate::core::search::boolean_query::Builder;
    use crate::core::search::constant_score_query::ConstantScoreQuery;
    use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
    use crate::core::search::phrase_query::PhraseQuery;
    use crate::core::search::query::{Query, QueryBase};
    use crate::core::search::score_mode::ScoreMode;
    use crate::core::search::term_query::TermQuery;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::index::random_index_writer::RandomIndexWriter;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        new_directory_shared, new_searcher_with_reader, new_string_field, new_text_field, random,
    };
    use std::collections::HashMap;

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
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let w = RandomIndexWriter::new(&mut random, dir.clone());
        let mut field_to_type = HashMap::new();
        let mut doc = Document::new();
        doc.add(new_string_field(
            "field",
            "a",
            Store::No,
            &mut field_to_type,
        )?);
        w.add_document(doc)?;

        let mut doc = Document::new();
        doc.add(new_string_field(
            "field",
            "b",
            Store::No,
            &mut field_to_type,
        )?);
        w.add_document(doc)?;

        let reader = w.get_reader()?;
        w.close()?;

        let searcher = new_searcher_with_reader(reader)?;

        let filter_b: Query = TermQuery::new(Term::from_text("field", "b")).into();
        let query: Query = ConstantScoreQuery::new(filter_b.clone()).into();

        let mut builder = Builder::new();
        builder
            .add(query, Occur::Must)?
            .add(filter_b.clone(), Occur::Filter)?;
        let mut filtered: Query = builder.build().into();

        assert_eq!(1, searcher.count(filtered)?); // Query for field:b, Filter field:b

        let filter_a: Query = TermQuery::new(Term::from_text("field", "a")).into();
        let query: Query = ConstantScoreQuery::new(filter_a).into();

        builder = Builder::new();
        builder
            .add(query, Occur::Must)?
            .add(filter_b, Occur::Filter)?;
        filtered = builder.build().into();

        assert_eq!(0, searcher.count(filtered)?); // Query field:b, Filter field:a

        Ok(())
    }

    #[test]
    fn test_propagates_approximations() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let writer = RandomIndexWriter::new(&mut random, dir.clone());
        let mut field_to_type = HashMap::new();

        let mut doc = Document::new();
        doc.add(new_text_field(
            "field",
            "a b",
            Store::No,
            &mut field_to_type,
        )?);
        writer.add_document(doc)?;
        writer.commit()?;

        let reader = writer.get_reader()?;
        let mut searcher = new_searcher_with_reader(reader)?;
        searcher.set_query_cache(None); // to still have approximations

        let pq: Query = PhraseQuery::from_terms(0, "field", &["a", "b"])?.into();
        let csq: Query = ConstantScoreQuery::new(pq).into();

        let rewritten = searcher.rewrite(csq)?;
        let weight = rewritten.create_weight(&searcher, &ScoreMode::Complete, 1.0, None)?;

        let ctx = &searcher.get_leaf_contexts()?[0];
        let scorer = weight.scorer(ctx, &searcher)?.unwrap();

        assert!(scorer.two_phase_iterator().is_some());

        Ok(())
    }

    #[test]
    fn test_rewrite_bubbles_up_match_no_docs_query() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;
        let query: Query = MatchNoDocsQuery::new().into();
        let query = ConstantScoreQuery::new(query);
        let rewritten = searcher.rewrite(query)?;
        assert_eq!(rewritten, Query::MatchNoDoc(MatchNoDocsQuery::new()));
        Ok(())
    }
}
