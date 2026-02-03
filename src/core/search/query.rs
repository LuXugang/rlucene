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
use crate::core::index::leaf_reader::{LRTermState, LeafReader};
use crate::core::index::term_states::TermStates;
use crate::core::search::QueryCache;
use crate::core::search::boolean_query::BooleanQuery;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::dummy::dummy_query::DummyQuery;
use crate::core::search::field_exists_query::FieldExistsQuery;
use crate::core::search::index_searcher::IndexSearcher;

use crate::core::search::boolean_scorer_supplier::{BulkScorerType, GetType};
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::doc_id_set_iterator::EmptyDISI;
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::dummy::dummy_scorable::DummyScorable;
use crate::core::search::dummy::dummy_scorer::DummyScorer;
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::index_sort_sorted_numeric_doc_values_range_query::IndexSortSortedNumericDocValuesRangeQuery;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::match_all_docs_query::{
    MatchAllBulkScorerEnum, MatchAllDocsQuery, MatchAllSsScorer,
};
use crate::core::search::match_no_docs_query::{
    MatchNoDocsQuery, MatchNoDocsSsBulkScorer, MatchNoDocsSsScorer,
};
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::point_range_query::PointRangeQuery;
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::term_query::{TermQuery, TermScorerEnum};
use crate::core::search::weight::{DefaultBulkScorer, Weight};
use crate::core::util::bits::Bits;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::cmp::PartialEq;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub type QueryWeight<LR> =
    Box<dyn Weight<LR, Matches = MatchWithNoTerms, ScorerSupplier = QueryWeightSs<LR>>>;
pub type QueryWeightSs<LR> =
    Box<dyn ScorerSupplier<LR, BulkScorer = QueryWeightSsBs<LR>, Scorer = QueryWeightSsS<LR>>>;

pub enum QueryWeightSsBs<LR>
where
    LR: LeafReader,
{
    Term(DefaultBulkScorer<QueryWeightSsS<LR>>),
    MatchAll(MatchAllBulkScorerEnum<LR>),
    MatchNo(MatchNoDocsSsBulkScorer<LR>),
    Boolean(Box<BulkScorerType<QueryWeightSsBs<LR>, QueryWeightSsS<LR>>>),
}
impl<LR> BulkScorer for QueryWeightSsBs<LR>
where
    LR: LeafReader,
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
        match self {
            QueryWeightSsBs::Term(scorer) => scorer.score(collector, accept_docs, min, max),
            QueryWeightSsBs::MatchAll(scorer) => scorer.score(collector, accept_docs, min, max),
            QueryWeightSsBs::MatchNo(scorer) => scorer.score(collector, accept_docs, min, max),
            QueryWeightSsBs::Boolean(scorer) => scorer.score(collector, accept_docs, min, max),
        }
    }

    fn cost(&mut self) -> Result<i64> {
        match self {
            QueryWeightSsBs::Term(scorer) => scorer.cost(),
            QueryWeightSsBs::MatchAll(scorer) => scorer.cost(),
            QueryWeightSsBs::MatchNo(scorer) => scorer.cost(),
            QueryWeightSsBs::Boolean(scorer) => scorer.cost(),
        }
    }
}

pub enum QueryWeightSsS<LR>
where
    LR: LeafReader,
{
    Term(TermScorerEnum<LR, EmptyDISI, DummyTwoPhaseIterator>),
    MatchAll(MatchAllSsScorer),
    MatchNo(MatchNoDocsSsScorer<LR>),
    Boolean(Box<GetType<QueryWeightSsS<LR>>>),
    Dummy(DummyScorer),
}

impl<LR> Scorable for QueryWeightSsS<LR>
where
    LR: LeafReader,
{
    fn score(&mut self) -> Result<f32> {
        match self {
            QueryWeightSsS::Term(scorer) => scorer.score(),
            QueryWeightSsS::MatchAll(scorer) => scorer.score(),
            QueryWeightSsS::MatchNo(scorer) => scorer.score(),
            QueryWeightSsS::Boolean(scorer) => scorer.score(),
            QueryWeightSsS::Dummy(scorer) => scorer.score(),
        }
    }

    type Scorable = DummyScorable;
}

impl<LR> Scorer for QueryWeightSsS<LR>
where
    LR: LeafReader,
{
    type DocIdSetIterator = DummyDISI;
    type DocIdSetIteratorRef<'a>
        = DummyDISI
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>
        = DummyDISI
    where
        Self: 'a;
    type TwoPhaseIter = DummyTwoPhaseIterator;
    type TwoPhaseIterRef<'a>
        = DummyTwoPhaseIterator
    where
        Self: 'a;
    type TwoPhaseIterMut<'a>
        = DummyTwoPhaseIterator
    where
        Self: 'a;

    fn doc_id(&mut self) -> Result<i32> {
        match self {
            QueryWeightSsS::Term(scorer) => scorer.doc_id(),
            QueryWeightSsS::MatchAll(scorer) => scorer.doc_id(),
            QueryWeightSsS::MatchNo(scorer) => scorer.doc_id(),
            QueryWeightSsS::Boolean(scorer) => scorer.doc_id(),
            QueryWeightSsS::Dummy(scorer) => scorer.doc_id(),
        }
    }

    fn iterator(&self) -> Self::DocIdSetIteratorRef<'_> {
        todo!()
    }

    fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_> {
        todo!()
    }

    fn take_iterator(self) -> Self::DocIdSetIterator {
        todo!()
    }

    fn two_phase_iterator(&self) -> Result<Option<Self::TwoPhaseIterRef<'_>>> {
        todo!()
    }

    fn two_phase_iterator_mut(&mut self) -> Result<Option<Self::TwoPhaseIterMut<'_>>> {
        todo!()
    }

    fn take_two_phase_iterator(self) -> Result<Option<Self::TwoPhaseIter>>
    where
        Self: Sized,
    {
        todo!()
    }

    fn advance_shallow(&mut self, _target: i32) -> Result<i32> {
        match self {
            QueryWeightSsS::Term(scorer) => scorer.advance_shallow(_target),
            QueryWeightSsS::MatchAll(scorer) => scorer.advance_shallow(_target),
            QueryWeightSsS::MatchNo(scorer) => scorer.advance_shallow(_target),
            QueryWeightSsS::Boolean(scorer) => scorer.advance_shallow(_target),
            QueryWeightSsS::Dummy(scorer) => scorer.advance_shallow(_target),
        }
    }

    fn default_advance_shallow(&mut self, _target: i32) -> Result<i32> {
        match self {
            QueryWeightSsS::Term(scorer) => scorer.default_advance_shallow(_target),
            QueryWeightSsS::MatchAll(scorer) => scorer.default_advance_shallow(_target),
            QueryWeightSsS::MatchNo(scorer) => scorer.default_advance_shallow(_target),
            QueryWeightSsS::Boolean(scorer) => scorer.default_advance_shallow(_target),
            QueryWeightSsS::Dummy(scorer) => scorer.default_advance_shallow(_target),
        }
    }

    fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
        match self {
            QueryWeightSsS::Term(scorer) => scorer.get_max_score(up_to),
            QueryWeightSsS::MatchAll(scorer) => scorer.get_max_score(up_to),
            QueryWeightSsS::MatchNo(scorer) => scorer.get_max_score(up_to),
            QueryWeightSsS::Boolean(scorer) => scorer.get_max_score(up_to),
            QueryWeightSsS::Dummy(scorer) => scorer.get_max_score(up_to),
        }
    }

    fn default_cost(&mut self) -> Result<i64> {
        match self {
            QueryWeightSsS::Term(scorer) => scorer.default_cost(),
            QueryWeightSsS::MatchAll(scorer) => scorer.default_cost(),
            QueryWeightSsS::MatchNo(scorer) => scorer.default_cost(),
            QueryWeightSsS::Boolean(scorer) => scorer.default_cost(),
            QueryWeightSsS::Dummy(scorer) => scorer.default_cost(),
        }
    }

    fn has_two_phase_iterator(&self) -> TwoPhaseState {
        match self {
            QueryWeightSsS::Term(scorer) => scorer.has_two_phase_iterator(),
            QueryWeightSsS::MatchAll(scorer) => scorer.has_two_phase_iterator(),
            QueryWeightSsS::MatchNo(scorer) => scorer.has_two_phase_iterator(),
            QueryWeightSsS::Boolean(scorer) => scorer.has_two_phase_iterator(),
            QueryWeightSsS::Dummy(scorer) => scorer.has_two_phase_iterator(),
        }
    }
}

pub trait QueryBase: Eq + Hash + Debug + HasIdentity {
    fn as_string(&self, field: &str) -> String;

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
        Err(LuceneError::unsupported_operation(format!(
            "Query {} does not implement create_weight",
            std::any::type_name::<Self>()
        )))
    }
    fn rewrite<IRC, QC>(self, _searcher: &IndexSearcher<IRC, QC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        QC: QueryCache,
        Self: Sized;

    fn visit<QV>(&self, visitor: &QV)
    where
        QV: QueryVisitor;
}

pub enum Query {
    Term(TermQuery),
    MatchAll(MatchAllDocsQuery),
    MatchNoDoc(MatchNoDocsQuery),
    Dummy(DummyQuery),
    Boost(BoostQuery),
    ConstantScore(ConstantScoreQuery),
    PointRange(PointRangeQuery),
    SortedNumericDocValuesSet(SortedNumericDocValuesSetQuery),
    SortedNumericDocValuesRange(SortedNumericDocValuesRangeQuery),
    SortedSetDocValuesRange(SortedSetDocValuesRangeQuery),
    IndexSortSortedNumericDocValuesRange(IndexSortSortedNumericDocValuesRangeQuery),
    FieldExists(FieldExistsQuery),
    Boolean(BooleanQuery),
}
impl Clone for Query {
    fn clone(&self) -> Self {
        match self {
            Query::Term(t) => Query::Term(t.clone()),
            Query::MatchAll(m) => Query::MatchAll(m.clone()),
            Query::MatchNoDoc(m) => Query::MatchNoDoc(m.clone()),
            Query::Dummy(d) => Query::Dummy(d.clone()),
            Query::Boost(b) => Query::Boost(b.clone()),
            Query::ConstantScore(c) => Query::ConstantScore(c.clone()),
            Query::PointRange(c) => Query::PointRange(c.clone()),
            Query::SortedNumericDocValuesSet(c) => Query::SortedNumericDocValuesSet(c.clone()),
            Query::SortedNumericDocValuesRange(c) => Query::SortedNumericDocValuesRange(c.clone()),
            Query::SortedSetDocValuesRange(c) => Query::SortedSetDocValuesRange(c.clone()),
            Query::IndexSortSortedNumericDocValuesRange(c) => {
                Query::IndexSortSortedNumericDocValuesRange(c.clone())
            },
            Query::FieldExists(c) => Query::FieldExists(c.clone()),
            Query::Boolean(c) => Query::Boolean(c.clone()),
        }
    }
}
impl Default for Query {
    fn default() -> Self {
        Query::Dummy(DummyQuery::default())
    }
}

impl Eq for Query {}

impl PartialEq for Query {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Query::Term(t1), Query::Term(t2)) => t1 == t2,
            (Query::MatchAll(m1), Query::MatchAll(m2)) => m1 == m2,
            (Query::MatchNoDoc(m1), Query::MatchNoDoc(m2)) => m1 == m2,
            (Query::Dummy(d1), Query::Dummy(d2)) => d1 == d2,
            (Query::Boost(b1), Query::Boost(b2)) => b1 == b2,
            (Query::ConstantScore(c1), Query::ConstantScore(c2)) => c1 == c2,
            (Query::PointRange(c1), Query::PointRange(c2)) => c1 == c2,
            (Query::SortedNumericDocValuesSet(c1), Query::SortedNumericDocValuesSet(c2)) => {
                c1 == c2
            },
            (Query::SortedNumericDocValuesRange(c1), Query::SortedNumericDocValuesRange(c2)) => {
                c1 == c2
            },
            (
                Query::IndexSortSortedNumericDocValuesRange(c1),
                Query::IndexSortSortedNumericDocValuesRange(c2),
            ) => c1 == c2,
            (Query::FieldExists(c1), Query::FieldExists(c2)) => c1 == c2,
            (Query::Boolean(c1), Query::Boolean(c2)) => c1 == c2,
            _ => false,
        }
    }
}

impl Hash for Query {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Query::Term(t) => {
                t.hash(state);
            },
            Query::MatchAll(m) => {
                m.hash(state);
            },
            Query::MatchNoDoc(m) => {
                m.hash(state);
            },
            Query::Dummy(d) => {
                d.hash(state);
            },
            Query::Boost(b) => {
                b.hash(state);
            },
            Query::ConstantScore(c) => {
                c.hash(state);
            },
            Query::PointRange(c) => {
                c.hash(state);
            },
            Query::SortedNumericDocValuesSet(c) => {
                c.hash(state);
            },
            Query::SortedNumericDocValuesRange(c) => {
                c.hash(state);
            },
            Query::SortedSetDocValuesRange(c) => {
                c.hash(state);
            },
            Query::IndexSortSortedNumericDocValuesRange(c) => {
                c.hash(state);
            },
            Query::FieldExists(c) => {
                c.hash(state);
            },
            Query::Boolean(c) => {
                c.hash(state);
            },
        }
    }
}

impl HasIdentity for Query {
    fn identity(&self) -> &Identity {
        match self {
            Query::Term(t) => t.identity(),
            Query::MatchAll(m) => m.identity(),
            Query::MatchNoDoc(m) => m.identity(),
            Query::Dummy(d) => d.identity(),
            Query::Boost(b) => b.identity(),
            Query::ConstantScore(c) => c.identity(),
            Query::PointRange(c) => c.identity(),
            Query::SortedNumericDocValuesSet(c) => c.identity(),
            Query::SortedNumericDocValuesRange(c) => c.identity(),
            Query::SortedSetDocValuesRange(c) => c.identity(),
            Query::IndexSortSortedNumericDocValuesRange(c) => c.identity(),
            Query::FieldExists(c) => c.identity(),
            Query::Boolean(c) => c.identity(),
        }
    }
}
impl Debug for Query {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Query::Term(t) => {
                write!(f, "Query::Term({:?})", t)
            },
            Query::MatchAll(m) => {
                write!(f, "Query::MatchAll({:?})", m)
            },
            Query::MatchNoDoc(m) => {
                write!(f, "Query::MatchNoDoc({:?})", m)
            },
            Query::Dummy(d) => {
                write!(f, "Query::Dummy({:?})", d)
            },
            Query::Boost(b) => {
                write!(f, "Query::Boost({:?})", b)
            },
            Query::ConstantScore(c) => {
                write!(f, "Query::ConstantScore({:?})", c)
            },
            Query::PointRange(c) => {
                write!(f, "Query::PointRange({:?})", c)
            },
            Query::SortedNumericDocValuesSet(c) => {
                write!(f, "Query::SortedNumericDocValuesSet({:?})", c)
            },
            Query::SortedNumericDocValuesRange(c) => {
                write!(f, "Query::SortedNumericDocValuesRange({:?})", c)
            },
            Query::SortedSetDocValuesRange(c) => {
                write!(f, "Query::SortedSetDocValuesRange({:?})", c)
            },
            Query::IndexSortSortedNumericDocValuesRange(c) => {
                write!(f, "Query::IndexSortSortedNumericDocValuesRange({:?})", c)
            },
            Query::FieldExists(c) => {
                write!(f, "Query::FieldExists({:?})", c)
            },
            Query::Boolean(c) => {
                write!(f, "Query::Boolean({:?})", c)
            },
        }
    }
}

impl QueryBase for Query {
    fn as_string(&self, field: &str) -> String {
        match self {
            Query::Term(t) => t.as_string(field),
            Query::MatchAll(m) => m.as_string(field),
            Query::MatchNoDoc(m) => m.as_string(field),
            Query::Dummy(d) => d.as_string(field),
            Query::Boost(b) => b.as_string(field),
            Query::ConstantScore(c) => c.as_string(field),
            Query::PointRange(c) => c.as_string(field),
            Query::SortedNumericDocValuesSet(c) => c.as_string(field),
            Query::SortedNumericDocValuesRange(c) => c.as_string(field),
            Query::SortedSetDocValuesRange(c) => c.as_string(field),
            Query::IndexSortSortedNumericDocValuesRange(c) => c.as_string(field),
            Query::FieldExists(c) => c.as_string(field),
            Query::Boolean(c) => c.as_string(field),
        }
    }

    fn create_weight<IRC, QC>(
        self,
        searcher: &IndexSearcher<IRC, QC>,
        score_mode: &ScoreMode,
        boost: f32,
        per_reader_term_state: Option<TermStates<LRTermState<IRCLeafReader<IRC>>>>,
    ) -> Result<QueryWeight<IRCLeafReader<IRC>>>
    where
        IRC: IndexReaderContext,
        QC: QueryCache,
        Self: Sized,
        <IRC as IndexReaderContext>::LeafReader: 'static,
    {
        match self {
            Query::Term(t) => t.create_weight(searcher, score_mode, boost, per_reader_term_state),
            Query::MatchAll(m) => {
                m.create_weight(searcher, score_mode, boost, per_reader_term_state)
            },
            Query::MatchNoDoc(m) => {
                m.create_weight(searcher, score_mode, boost, per_reader_term_state)
            },
            Query::Dummy(d) => d.create_weight(searcher, score_mode, boost, per_reader_term_state),
            Query::Boost(b) => b.create_weight(searcher, score_mode, boost, per_reader_term_state),
            Query::ConstantScore(c) => {
                c.create_weight(searcher, score_mode, boost, per_reader_term_state)
            },
            Query::PointRange(c) => {
                c.create_weight(searcher, score_mode, boost, per_reader_term_state)
            },
            Query::SortedNumericDocValuesSet(c) => {
                c.create_weight(searcher, score_mode, boost, per_reader_term_state)
            },
            Query::SortedNumericDocValuesRange(c) => {
                c.create_weight(searcher, score_mode, boost, per_reader_term_state)
            },
            Query::SortedSetDocValuesRange(c) => {
                c.create_weight(searcher, score_mode, boost, per_reader_term_state)
            },
            Query::IndexSortSortedNumericDocValuesRange(c) => {
                c.create_weight(searcher, score_mode, boost, per_reader_term_state)
            },
            Query::FieldExists(c) => {
                c.create_weight(searcher, score_mode, boost, per_reader_term_state)
            },
            Query::Boolean(c) => {
                c.create_weight(searcher, score_mode, boost, per_reader_term_state)
            },
        }
    }

    fn rewrite<IRC, QC>(self, searcher: &IndexSearcher<IRC, QC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        QC: QueryCache,
    {
        match self {
            Query::Term(t) => t.rewrite(searcher),
            Query::MatchAll(m) => m.rewrite(searcher),
            Query::MatchNoDoc(m) => m.rewrite(searcher),
            Query::Dummy(d) => d.rewrite(searcher),
            Query::Boost(b) => b.rewrite(searcher),
            Query::ConstantScore(c) => c.rewrite(searcher),
            Query::PointRange(c) => c.rewrite(searcher),
            Query::SortedNumericDocValuesSet(c) => c.rewrite(searcher),
            Query::SortedNumericDocValuesRange(c) => c.rewrite(searcher),
            Query::SortedSetDocValuesRange(c) => c.rewrite(searcher),
            Query::IndexSortSortedNumericDocValuesRange(c) => c.rewrite(searcher),
            Query::FieldExists(c) => c.rewrite(searcher),
            Query::Boolean(c) => c.rewrite(searcher),
        }
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

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

impl_from_for_query! {
    TermQuery => Term,
    MatchAllDocsQuery => MatchAll,
    MatchNoDocsQuery => MatchNoDoc,
    DummyQuery => Dummy,
    BoostQuery => Boost,
    ConstantScoreQuery => ConstantScore,
    PointRangeQuery => PointRange,
    SortedNumericDocValuesSetQuery => SortedNumericDocValuesSet,
    SortedNumericDocValuesRangeQuery => SortedNumericDocValuesRange,
    SortedSetDocValuesRangeQuery => SortedSetDocValuesRange,
    IndexSortSortedNumericDocValuesRangeQuery => IndexSortSortedNumericDocValuesRange,
    FieldExistsQuery => FieldExists,
    BooleanQuery => Boolean,
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
        Err(LuceneError::unsupported_operation(format!(
            "Arc<QueryBase> cannot be used to create_weight directly: {}",
            std::any::type_name::<Q>()
        )))
    }

    fn rewrite<IRC, QC>(self, _searcher: &IndexSearcher<IRC, QC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        QC: QueryCache,
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
