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
use crate::core::document::sorted_numeric_doc_values_range_query::SortedNumericDocValuesRangeQuery;
use crate::core::document::sorted_numeric_doc_values_set_query::SortedNumericDocValuesSetQuery;
use crate::core::document::sorted_set_doc_values_range_query::SortedSetDocValuesRangeQuery;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::search::boolean_query::BooleanQuery;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::dummy::dummy_query::DummyQuery;
use crate::core::search::field_exists_query::FieldExistsQuery;
use crate::core::search::index_searcher::IndexSearcher;

use crate::core::search::automaton_query::AutomatonQuery;
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::disjunction_max_query::DisjunctionMaxQuery;
use crate::core::search::index_or_doc_values_query::IndexOrDocValuesQuery;
use crate::core::search::index_sort_sorted_numeric_doc_values_range_query::IndexSortSortedNumericDocValuesRangeQuery;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::multi_term_query_constant_score_blended_wrapper::MultiTermQueryConstantScoreBlendedWrapper;
use crate::core::search::multi_term_query_constant_score_wrapper::MultiTermQueryConstantScoreWrapper;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::point_range_query::PointRangeQuery;
use crate::core::search::prefix_query::PrefixQuery;
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::regexp_query::RegexpQuery;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::Scorer;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::term_in_set_query::TermInSetQuery;
use crate::core::search::term_query::TermQuery;
use crate::core::search::term_range_query::TermRangeQuery;
#[cfg(test)]
use crate::core::search::usage_tracking_query_caching_policy::tests::DummyQuery1;
#[cfg(test)]
use crate::core::search::wand_scorer::tests::MaxScoreWrapperQuery;
#[cfg(test)]
use crate::core::search::wand_scorer::tests::WANDScorerQuery;
use crate::core::search::weight::Weight;
use crate::core::search::wildcard_query::WildcardQuery;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::impl_from_for_enum;
#[cfg(test)]
use crate::test::core::search::block_score_query_wrapper::BlockScoreQueryWrapper;
#[cfg(test)]
use crate::test::core::search::random_approximation_query::RandomApproximationQuery;
#[cfg(test)]
use crate::test::core::search::test_boolean_rewrites::TestRewriteQuery;
#[cfg(test)]
use crate::test::core::search::test_scorer_perf::BitSetQuery;
use std::cmp::PartialEq;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub type QueryWeight<IRC> =
    Box<dyn Weight<IRC, Matches = MatchWithNoTerms, ScorerSupplier = QueryWeightSs<IRC>>>;
pub type QueryWeightSs<IRC> = Box<
    dyn ScorerSupplier<IRC, BulkScorer = QueryWeightSsBulkScorer, Scorer = QueryWeightSsScorer>,
>;
pub type QueryWeightSsBulkScorer = Box<dyn BulkScorer>;
pub type QueryWeightSsScorer = Box<dyn Scorer>;
macro_rules! impl_into_box_query {
    ($($ty:ty),* $(,)?) => {
        $(
            impl IntoBoxQuery for $ty {
                fn into_box_query(self) -> Box<Query> {
                    Box::new(self.into())
                }
            }
        )*
    };
}
macro_rules! dispatch_query {
    ($self:expr, |$inner:ident| $body:expr) => {{
        match $self {
            Query::Automaton($inner) => $body,
            Query::Boolean($inner) => $body,
            Query::Boost($inner) => $body,
            Query::ConstantScore($inner) => $body,
            Query::Dummy($inner) => $body,
            Query::DisjunctionMax($inner) => $body,
            Query::FieldExists($inner) => $body,
            Query::IndexOrDocValues($inner) => $body,
            Query::IndexSortSortedNumericDocValuesRange($inner) => $body,
            Query::MatchAllDocs($inner) => $body,
            Query::MatchNoDocs($inner) => $body,
            Query::MultiTermQueryConstantScoreBlendedWrapper($inner) => $body,
            Query::MultiTermQueryConstantScoreWrapper($inner) => $body,
            Query::PointRange($inner) => $body,
            Query::Prefix($inner) => $body,
            Query::Regexp($inner) => $body,
            Query::SortedNumericDocValuesRange($inner) => $body,
            Query::SortedNumericDocValuesSet($inner) => $body,
            Query::SortedSetDocValuesRange($inner) => $body,
            Query::Phrase($inner) => $body,
            Query::Term($inner) => $body,
            Query::TermInSet($inner) => $body,
            Query::TermRange($inner) => $body,
            Query::Wildcard($inner) => $body,
            #[cfg(test)]
            Query::BitSet($inner) => $body,
            #[cfg(test)]
            Query::BlockScoreQueryWrapper($inner) => $body,
            #[cfg(test)]
            Query::Dummy1($inner) => $body,
            #[cfg(test)]
            Query::MaxScoreWrapper($inner) => $body,
            #[cfg(test)]
            Query::RandomApproximation($inner) => $body,
            #[cfg(test)]
            Query::TestRewrite($inner) => $body,
            #[cfg(test)]
            Query::WANDScorer($inner) => $body,
        }
    }};
}
impl_from_for_enum!(
    Query,
    AutomatonQuery=> Automaton,
    BooleanQuery => Boolean,
    BoostQuery => Boost,
    ConstantScoreQuery => ConstantScore,
    DummyQuery => Dummy,
    DisjunctionMaxQuery => DisjunctionMax,
    FieldExistsQuery => FieldExists,
    IndexOrDocValuesQuery => IndexOrDocValues,
    IndexSortSortedNumericDocValuesRangeQuery => IndexSortSortedNumericDocValuesRange,
    MatchAllDocsQuery => MatchAllDocs,
    MatchNoDocsQuery => MatchNoDocs,
    MultiTermQueryConstantScoreBlendedWrapper => MultiTermQueryConstantScoreBlendedWrapper,
    MultiTermQueryConstantScoreWrapper => MultiTermQueryConstantScoreWrapper,
    PointRangeQuery => PointRange,
    PrefixQuery => Prefix,
    RegexpQuery => Regexp,
    SortedNumericDocValuesRangeQuery => SortedNumericDocValuesRange,
    SortedNumericDocValuesSetQuery => SortedNumericDocValuesSet,
    SortedSetDocValuesRangeQuery => SortedSetDocValuesRange,
    TermQuery => Term,
    TermInSetQuery => TermInSet,
    TermRangeQuery => TermRange,
    PhraseQuery=> Phrase,
    WildcardQuery => Wildcard,
);
#[cfg(test)]
impl_from_for_enum!(
    Query,
    BitSetQuery => BitSet,
    BlockScoreQueryWrapper => BlockScoreQueryWrapper,
    DummyQuery1=> Dummy1,
    MaxScoreWrapperQuery => MaxScoreWrapper,
    RandomApproximationQuery => RandomApproximation,
    TestRewriteQuery => TestRewrite,
    WANDScorerQuery => WANDScorer
);
impl_into_box_query!(
    AutomatonQuery,
    BooleanQuery,
    BoostQuery,
    ConstantScoreQuery,
    DummyQuery,
    DisjunctionMaxQuery,
    FieldExistsQuery,
    IndexOrDocValuesQuery,
    IndexSortSortedNumericDocValuesRangeQuery,
    MatchAllDocsQuery,
    MatchNoDocsQuery,
    MultiTermQueryConstantScoreBlendedWrapper,
    MultiTermQueryConstantScoreWrapper,
    PointRangeQuery,
    RegexpQuery,
    SortedNumericDocValuesRangeQuery,
    SortedNumericDocValuesSetQuery,
    SortedSetDocValuesRangeQuery,
    PhraseQuery,
    PrefixQuery,
    TermQuery,
    TermInSetQuery,
    TermRangeQuery,
    WildcardQuery,
);

pub trait QueryBase: Debug + HasIdentity {
    fn as_string(&self, field: &str) -> Result<String>;

    fn create_weight<IRC>(
        self,
        _searcher: &IndexSearcher<IRC>,
        _score_mode: &ScoreMode,
        _boost: f32,
    ) -> Result<QueryWeight<IRC>>
    where
        IRC: IndexReaderContext,
        Self: Sized;

    fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        Self: Sized;

    fn visit<QV>(&self, visitor: &QV)
    where
        QV: QueryVisitor;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Query {
    Automaton(AutomatonQuery),
    Boolean(BooleanQuery),
    Boost(BoostQuery),
    ConstantScore(ConstantScoreQuery),
    Dummy(DummyQuery),
    DisjunctionMax(DisjunctionMaxQuery),
    FieldExists(FieldExistsQuery),
    IndexOrDocValues(IndexOrDocValuesQuery),
    IndexSortSortedNumericDocValuesRange(IndexSortSortedNumericDocValuesRangeQuery),
    MatchAllDocs(MatchAllDocsQuery),
    MatchNoDocs(MatchNoDocsQuery),
    MultiTermQueryConstantScoreBlendedWrapper(MultiTermQueryConstantScoreBlendedWrapper),
    MultiTermQueryConstantScoreWrapper(MultiTermQueryConstantScoreWrapper),
    PointRange(PointRangeQuery),
    Regexp(RegexpQuery),
    SortedNumericDocValuesRange(SortedNumericDocValuesRangeQuery),
    SortedNumericDocValuesSet(SortedNumericDocValuesSetQuery),
    SortedSetDocValuesRange(SortedSetDocValuesRangeQuery),
    Phrase(PhraseQuery),
    Prefix(PrefixQuery),
    Term(TermQuery),
    TermInSet(TermInSetQuery),
    TermRange(TermRangeQuery),
    Wildcard(WildcardQuery),
    #[cfg(test)]
    BitSet(BitSetQuery),
    #[cfg(test)]
    BlockScoreQueryWrapper(BlockScoreQueryWrapper),
    #[cfg(test)]
    Dummy1(DummyQuery1),
    #[cfg(test)]
    MaxScoreWrapper(MaxScoreWrapperQuery),
    #[cfg(test)]
    RandomApproximation(RandomApproximationQuery),
    #[cfg(test)]
    TestRewrite(TestRewriteQuery),
    #[cfg(test)]
    WANDScorer(WANDScorerQuery),
}
macro_rules! query_variant_name {
    (
        $self:expr;
        normal: [ $( $variant:ident ),* $(,)? ];
        test: [ $( $test_variant:ident ),* $(,)? ]
    ) => {
        match $self {
            $(
                Query::$variant(_) => stringify!($variant),
            )*
            $(
                #[cfg(test)]
                Query::$test_variant(_) => stringify!($test_variant),
            )*
        }
    };
}
impl Query {
    pub fn name(&self) -> &'static str {
        query_variant_name!(
            self;
            normal: [
                Automaton,
                Boolean,
                Boost,
                ConstantScore,
                Dummy,
                DisjunctionMax,
                FieldExists,
                IndexOrDocValues,
                IndexSortSortedNumericDocValuesRange,
                MatchAllDocs,
                MatchNoDocs,
                MultiTermQueryConstantScoreBlendedWrapper,
                MultiTermQueryConstantScoreWrapper,
                PointRange,
                Regexp,
                SortedNumericDocValuesRange,
                SortedNumericDocValuesSet,
                SortedSetDocValuesRange,
                Phrase,
                Prefix,
                Term,
                TermInSet,
                TermRange,
                Wildcard,
            ];
            test: [
                BitSet,
                BlockScoreQueryWrapper,
                Dummy1,
                MaxScoreWrapper,
                RandomApproximation,
                TestRewrite,
                WANDScorer,
            ]
        )
    }
}
impl Default for Query {
    fn default() -> Self {
        Query::Dummy(DummyQuery::default())
    }
}

impl HasIdentity for Query {
    fn identity(&self) -> &Identity {
        dispatch_query!(self, |q| q.identity())
    }
}
impl QueryBase for Query {
    fn as_string(&self, field: &str) -> Result<String> {
        dispatch_query!(self, |q| q.as_string(field))
    }

    fn create_weight<IRC>(
        self,
        searcher: &IndexSearcher<IRC>,
        score_mode: &ScoreMode,
        boost: f32,
    ) -> Result<QueryWeight<IRC>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        dispatch_query!(self, |q| q.create_weight(searcher, score_mode, boost,))
    }

    fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
    {
        dispatch_query!(self, |q| q.rewrite(searcher))
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub struct IdentityQuery {
    pub(crate) query: Arc<Query>,
}
impl IdentityQuery {
    pub fn new(query: Arc<Query>) -> Self {
        Self { query }
    }
}

impl PartialEq for IdentityQuery {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.query, &other.query)
    }
}
impl Eq for IdentityQuery {}

impl Hash for IdentityQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.query).hash(state);
    }
}
impl<Q> QueryBase for Arc<Q>
where
    Q: QueryBase,
{
    fn as_string(&self, field: &str) -> Result<String> {
        (**self).as_string(field)
    }

    fn create_weight<IRC>(
        self,
        _searcher: &IndexSearcher<IRC>,
        _score_mode: &ScoreMode,
        _boost: f32,
    ) -> Result<QueryWeight<IRC>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        Err(LuceneError::unsupported_operation(format!(
            "Arc<QueryBase> cannot be used to create_weight directly: {}",
            std::any::type_name::<Q>()
        )))
    }

    fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
    {
        Err(LuceneError::unsupported_operation(format!(
            "Arc<QueryBase> cannot be used to rewrite directly: {}",
            std::any::type_name::<Q>()
        )))
    }

    fn visit<QV>(&self, visitor: &QV)
    where
        QV: QueryVisitor,
    {
        (**self).visit(visitor)
    }
}
pub trait IntoBoxQuery {
    fn into_box_query(self) -> Box<Query>;
}
impl IntoBoxQuery for Query {
    fn into_box_query(self) -> Box<Query> {
        Box::new(self)
    }
}

impl IntoBoxQuery for Box<Query> {
    fn into_box_query(self) -> Box<Query> {
        self
    }
}
