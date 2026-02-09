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
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LRTermState;
use crate::core::index::term_states::TermStates;
use crate::core::search::boolean_query::BooleanQuery;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::dummy::dummy_query::DummyQuery;
use crate::core::search::field_exists_query::FieldExistsQuery;
use crate::core::search::index_searcher::IndexSearcher;

use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::index_sort_sorted_numeric_doc_values_range_query::IndexSortSortedNumericDocValuesRangeQuery;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::point_range_query::PointRangeQuery;
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::Scorer;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::term_query::TermQuery;
#[cfg(test)]
use crate::core::search::wand_scorer::tests::MaxScoreWrapperQuery;
#[cfg(test)]
use crate::core::search::wand_scorer::tests::WANDScorerQuery;
use crate::core::search::weight::Weight;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
#[cfg(test)]
use crate::test::search::random_approximation_query::RandomApproximationQuery;
use std::cmp::PartialEq;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub type QueryWeight<LR> =
    Box<dyn Weight<LR, Matches = MatchWithNoTerms, ScorerSupplier = QueryWeightSs<LR>>>;
pub type QueryWeightSs<LR> =
    Box<dyn ScorerSupplier<LR, BulkScorer = QueryWeightSsBulkScorer, Scorer = QueryWeightSsScorer>>;
pub type QueryWeightSsBulkScorer = Box<dyn BulkScorer>;
pub type QueryWeightSsScorer = Box<dyn Scorer>;

macro_rules! impl_from_for_query {
    ( $( $ty:ty => $variant:ident ),+ $(,)? ) => {
        $(
            impl From<$ty> for Query {
                #[inline]
                fn from(value: $ty) -> Self {
                    Query::$variant(value)
                }
            }
        )+
    };
}
macro_rules! dispatch_query {
    ($self:expr, |$inner:ident| $body:expr) => {{
        match $self {
            Query::Boolean($inner) => $body,
            Query::Boost($inner) => $body,
            Query::ConstantScore($inner) => $body,
            Query::Dummy($inner) => $body,
            Query::FieldExists($inner) => $body,
            Query::IndexSortSortedNumericDocValuesRange($inner) => $body,
            Query::MatchAll($inner) => $body,
            Query::MatchNoDoc($inner) => $body,
            Query::PointRange($inner) => $body,
            Query::SortedNumericDocValuesRange($inner) => $body,
            Query::SortedNumericDocValuesSet($inner) => $body,
            Query::SortedSetDocValuesRange($inner) => $body,
            Query::Phrase($inner) => $body,
            Query::Term($inner) => $body,
            #[cfg(test)]
            Query::WANDScorer($inner) => $body,
            #[cfg(test)]
            Query::MaxScoreWrapper($inner) => $body,
            #[cfg(test)]
            Query::RandomApproximation($inner) => $body,
        }
    }};
}

// Implement From<T> for Query for all query types
impl_from_for_query! {
    BooleanQuery => Boolean,
    BoostQuery => Boost,
    ConstantScoreQuery => ConstantScore,
    DummyQuery => Dummy,
    FieldExistsQuery => FieldExists,
    IndexSortSortedNumericDocValuesRangeQuery => IndexSortSortedNumericDocValuesRange,
    MatchAllDocsQuery => MatchAll,
    MatchNoDocsQuery => MatchNoDoc,
    PointRangeQuery => PointRange,
    SortedNumericDocValuesRangeQuery => SortedNumericDocValuesRange,
    SortedNumericDocValuesSetQuery => SortedNumericDocValuesSet,
    SortedSetDocValuesRangeQuery => SortedSetDocValuesRange,
    TermQuery => Term,
    PhraseQuery=> Phrase,
}

pub trait QueryBase: Debug + HasIdentity {
    fn as_string(&self, field: &str) -> String;

    fn create_weight<IRC>(
        self,
        _searcher: &IndexSearcher<IRC>,
        _score_mode: &ScoreMode,
        _boost: f32,
        _per_reader_term_state: Option<TermStates<LRTermState<IRCLeafReader<IRC>>>>,
    ) -> Result<QueryWeight<IRCLeafReader<IRC>>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
        <IRC as IndexReaderContext>::LeafReader: 'static;

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
    Boolean(BooleanQuery),
    Boost(BoostQuery),
    ConstantScore(ConstantScoreQuery),
    Dummy(DummyQuery),
    FieldExists(FieldExistsQuery),
    IndexSortSortedNumericDocValuesRange(IndexSortSortedNumericDocValuesRangeQuery),
    MatchAll(MatchAllDocsQuery),
    MatchNoDoc(MatchNoDocsQuery),
    PointRange(PointRangeQuery),
    SortedNumericDocValuesRange(SortedNumericDocValuesRangeQuery),
    SortedNumericDocValuesSet(SortedNumericDocValuesSetQuery),
    SortedSetDocValuesRange(SortedSetDocValuesRangeQuery),
    Phrase(PhraseQuery),
    Term(TermQuery),
    #[cfg(test)]
    WANDScorer(WANDScorerQuery),
    #[cfg(test)]
    MaxScoreWrapper(MaxScoreWrapperQuery),
    #[cfg(test)]
    RandomApproximation(RandomApproximationQuery),
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
    fn as_string(&self, field: &str) -> String {
        dispatch_query!(self, |q| q.as_string(field))
    }

    fn create_weight<IRC>(
        self,
        searcher: &IndexSearcher<IRC>,
        score_mode: &ScoreMode,
        boost: f32,
        per_reader_term_state: Option<TermStates<LRTermState<IRCLeafReader<IRC>>>>,
    ) -> Result<QueryWeight<IRCLeafReader<IRC>>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
        <IRC as IndexReaderContext>::LeafReader: 'static,
    {
        dispatch_query!(self, |q| q.create_weight(
            searcher,
            score_mode,
            boost,
            per_reader_term_state
        ))
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
    fn as_string(&self, field: &str) -> String {
        (**self).as_string(field)
    }

    fn create_weight<IRC>(
        self,
        _searcher: &IndexSearcher<IRC>,
        _score_mode: &ScoreMode,
        _boost: f32,
        _per_reader_term_state: Option<TermStates<LRTermState<IRCLeafReader<IRC>>>>,
    ) -> Result<QueryWeight<IRCLeafReader<IRC>>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
        <IRC as IndexReaderContext>::LeafReader: 'static,
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
