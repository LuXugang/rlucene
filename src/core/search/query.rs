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
use crate::core::document::sorted_numeric_doc_values_range_query::{
    SortedNumericDocValuesRangeQuery, SortedNumericDocValuesRangeQueryWeight,
};
use crate::core::document::sorted_numeric_doc_values_set_query::{
    SortedNumericDocValuesSetQuery, SortedNumericDocValuesSetQueryWeight,
};
use crate::core::document::sorted_set_doc_values_range_query::{
    SortedSetDocValuesRangeQuery, SortedSetDocValuesRangeQueryWeight,
};
use crate::core::index::index_reader_context::{IRCLeafReader, IRCTermState, IndexReaderContext};
use crate::core::index::query_timeout::QueryTimeout;
use crate::core::index::term_states::TermStates;
use crate::core::search::QueryCache;
use crate::core::search::boolean_query::BooleanQuery;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::dummy::dummy_query::DummyQuery;
use crate::core::search::field_exists_query::{FieldExistsQuery, FieldExistsWeight};
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::index_sort_sorted_numeric_doc_values_range_query::{
    IndexSortSortedNumericDocValuesRangeQuery, IndexSortSortedNumericDocValuesRangeQueryWeight,
};
use crate::core::search::match_all_docs_query::{MatchAllDocsQuery, MatchAllWeight};
use crate::core::search::match_no_docs_query::{MatchNoDocsQuery, MatchNoDocsWeight};
use crate::core::search::point_range_query::{PointRangeQuery, PointRangeWeight};
use crate::core::search::query_caching_policy::QueryCachingPolicy;
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::term_query::{TermQuery, TermWeight};
use crate::core::search::weight::{Weight, WeightEnum9};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::cmp::PartialEq;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub trait QueryBase: Eq + Hash + Debug {
    fn as_string(&self, field: &str) -> String;
    type Weight<S, IRC, QCP, QC>: Weight<IRC::LeafReader>
    where
        S: Similarity,
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
        QC: QueryCache;
    fn create_weight<S, IRC, QT, QCP, QC>(
        self,
        _searcher: &IndexSearcher<IRC, S, QT, QCP, QC>,
        _score_mode: &ScoreMode,
        _boost: f32,
        _per_reader_term_state: Option<TermStates<IRCTermState<IRC>>>,
    ) -> Result<Self::Weight<S, IRC, QCP, QC>>
    where
        IRC: IndexReaderContext,
        S: Similarity,
        QT: QueryTimeout,
        QCP: QueryCachingPolicy,
        QC: QueryCache,
        Self: Sized,
    {
        Err(LuceneError::unsupported_operation(format!(
            "Query {} does not implement create_weight",
            std::any::type_name::<Self>()
        )))
    }
    type RewriteQuery: QueryBase;
    fn rewrite<IRC, S, QT, QCP, QC>(
        &self,
        _searcher: &IndexSearcher<IRC, S, QT, QCP, QC>,
    ) -> Result<Option<Self::RewriteQuery>>
    where
        IRC: IndexReaderContext,
        S: Similarity,
        QT: QueryTimeout,
        QCP: QueryCachingPolicy,
        QC: QueryCache,
    {
        Ok(None)
    }
    fn visit<QV>(&self, visitor: &QV)
    where
        QV: QueryVisitor;
}
pub enum BaseQuery {
    Term(TermQuery),
    MatchAll(MatchAllDocsQuery),
    MatchNoDoc(MatchNoDocsQuery),
    Dummy(DummyQuery),
    PointRange(PointRangeQuery),
    SortedNumericDocValuesSet(SortedNumericDocValuesSetQuery),
    SortedNumericDocValuesRange(SortedNumericDocValuesRangeQuery),
    SortedSetDocValuesRange(SortedSetDocValuesRangeQuery),
    IndexSortSortedNumericDocValuesRange(IndexSortSortedNumericDocValuesRangeQuery),
    FieldExists(FieldExistsQuery),
}
#[cfg(test)]
impl Clone for BaseQuery {
    fn clone(&self) -> Self {
        match self {
            BaseQuery::Term(t) => BaseQuery::Term(t.clone()),
            BaseQuery::MatchAll(m) => BaseQuery::MatchAll(m.clone()),
            BaseQuery::MatchNoDoc(m) => BaseQuery::MatchNoDoc(m.clone()),
            BaseQuery::Dummy(d) => BaseQuery::Dummy(d.clone()),
            BaseQuery::PointRange(c) => BaseQuery::PointRange(c.clone()),
            BaseQuery::SortedNumericDocValuesSet(c) => {
                BaseQuery::SortedNumericDocValuesSet(c.clone())
            },
            BaseQuery::SortedNumericDocValuesRange(c) => {
                BaseQuery::SortedNumericDocValuesRange(c.clone())
            },
            BaseQuery::SortedSetDocValuesRange(c) => BaseQuery::SortedSetDocValuesRange(c.clone()),
            BaseQuery::IndexSortSortedNumericDocValuesRange(c) => {
                BaseQuery::IndexSortSortedNumericDocValuesRange(c.clone())
            },
            BaseQuery::FieldExists(c) => BaseQuery::FieldExists(c.clone()),
        }
    }
}

impl Eq for BaseQuery {}

impl PartialEq<Self> for BaseQuery {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (BaseQuery::Term(t1), BaseQuery::Term(t2)) => t1 == t2,
            (BaseQuery::MatchAll(m1), BaseQuery::MatchAll(m2)) => m1 == m2,
            (BaseQuery::MatchNoDoc(m1), BaseQuery::MatchNoDoc(m2)) => m1 == m2,
            (BaseQuery::Dummy(d1), BaseQuery::Dummy(d2)) => d1 == d2,
            (BaseQuery::PointRange(c1), BaseQuery::PointRange(c2)) => c1 == c2,
            (
                BaseQuery::SortedNumericDocValuesSet(c1),
                BaseQuery::SortedNumericDocValuesSet(c2),
            ) => c1 == c2,
            (
                BaseQuery::SortedNumericDocValuesRange(c1),
                BaseQuery::SortedNumericDocValuesRange(c2),
            ) => c1 == c2,
            (
                BaseQuery::IndexSortSortedNumericDocValuesRange(c1),
                BaseQuery::IndexSortSortedNumericDocValuesRange(c2),
            ) => c1 == c2,
            (BaseQuery::FieldExists(c1), BaseQuery::FieldExists(c2)) => c1 == c2,
            _ => false,
        }
    }
}

impl Hash for BaseQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            BaseQuery::Term(t) => {
                t.hash(state);
            },
            BaseQuery::MatchAll(m) => {
                m.hash(state);
            },
            BaseQuery::MatchNoDoc(m) => {
                m.hash(state);
            },
            BaseQuery::Dummy(d) => {
                d.hash(state);
            },
            BaseQuery::PointRange(c) => {
                c.hash(state);
            },
            BaseQuery::SortedNumericDocValuesSet(c) => {
                c.hash(state);
            },
            BaseQuery::SortedNumericDocValuesRange(c) => {
                c.hash(state);
            },
            BaseQuery::SortedSetDocValuesRange(c) => {
                c.hash(state);
            },
            BaseQuery::IndexSortSortedNumericDocValuesRange(c) => {
                c.hash(state);
            },
            BaseQuery::FieldExists(c) => {
                c.hash(state);
            },
        }
    }
}

impl Debug for BaseQuery {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BaseQuery::Term(t) => {
                write!(f, "BaseQuery::Term({:?})", t)
            },
            BaseQuery::MatchAll(m) => {
                write!(f, "BaseQuery::MatchAll({:?})", m)
            },
            BaseQuery::MatchNoDoc(m) => {
                write!(f, "BaseQuery::MatchNoDoc({:?})", m)
            },
            BaseQuery::Dummy(d) => {
                write!(f, "BaseQuery::Dummy({:?})", d)
            },
            BaseQuery::PointRange(c) => {
                write!(f, "BaseQuery::PointRange({:?})", c)
            },
            BaseQuery::SortedNumericDocValuesSet(c) => {
                write!(f, "BaseQuery::SortedNumericDocValuesSet({:?})", c)
            },
            BaseQuery::SortedNumericDocValuesRange(c) => {
                write!(f, "BaseQuery::SortedNumericDocValuesRange({:?})", c)
            },
            BaseQuery::SortedSetDocValuesRange(c) => {
                write!(f, "BaseQuery::SortedSetDocValuesRange({:?})", c)
            },
            BaseQuery::IndexSortSortedNumericDocValuesRange(c) => {
                write!(
                    f,
                    "BaseQuery::IndexSortSortedNumericDocValuesRange({:?})",
                    c
                )
            },
            BaseQuery::FieldExists(c) => {
                write!(f, "BaseQuery::FieldExists({:?})", c)
            },
        }
    }
}

impl QueryBase for BaseQuery {
    fn as_string(&self, field: &str) -> String {
        match self {
            BaseQuery::Term(t) => t.as_string(field),
            BaseQuery::MatchAll(m) => m.as_string(field),
            BaseQuery::MatchNoDoc(m) => m.as_string(field),
            BaseQuery::Dummy(d) => d.as_string(field),
            BaseQuery::PointRange(c) => c.as_string(field),
            BaseQuery::SortedNumericDocValuesSet(c) => c.as_string(field),
            BaseQuery::SortedNumericDocValuesRange(c) => c.as_string(field),
            BaseQuery::SortedSetDocValuesRange(c) => c.as_string(field),
            BaseQuery::IndexSortSortedNumericDocValuesRange(c) => c.as_string(field),
            BaseQuery::FieldExists(c) => c.as_string(field),
        }
    }

    type Weight<S, IRC, QCP, QC>
        = BaseQueryWeight<S, IRC>
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
        match self {
            BaseQuery::Term(d) => Ok(BaseQueryWeight::A(d.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            BaseQuery::MatchAll(d) => Ok(BaseQueryWeight::B(d.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            BaseQuery::PointRange(d) => Ok(BaseQueryWeight::C(d.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            BaseQuery::MatchNoDoc(d) => Ok(BaseQueryWeight::D(d.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            BaseQuery::SortedNumericDocValuesSet(d) => Ok(BaseQueryWeight::E(d.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            BaseQuery::SortedNumericDocValuesRange(d) => Ok(BaseQueryWeight::F(d.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            BaseQuery::SortedSetDocValuesRange(d) => Ok(BaseQueryWeight::G(d.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            BaseQuery::IndexSortSortedNumericDocValuesRange(d) => Ok(BaseQueryWeight::H(
                d.create_weight(searcher, score_mode, boost, per_reader_term_state)?,
            )),
            BaseQuery::FieldExists(d) => Ok(BaseQueryWeight::I(d.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            BaseQuery::Dummy(_) => Err(LuceneError::unsupported_operation(
                "DummyQuery cannot create weight",
            )),
        }
    }

    type RewriteQuery = DummyQuery;

    fn rewrite<IRC, S, QT, QCP, QC>(
        &self,
        _searcher: &IndexSearcher<IRC, S, QT, QCP, QC>,
    ) -> Result<Option<Self::RewriteQuery>>
    where
        IRC: IndexReaderContext,
        S: Similarity,
        QT: QueryTimeout,
        QCP: QueryCachingPolicy,
        QC: QueryCache,
    {
        todo!()
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}
// To BaseQuery
macro_rules! impl_from_for_base_query {
    ( $( $ty:ty => $variant:ident ),+ $(,)? ) => {
        $(
            impl From<$ty> for BaseQuery {
                #[inline]
                fn from(value: $ty) -> Self {
                    BaseQuery::$variant(value)
                }
            }
        )+
    };
}

impl_from_for_base_query! {
    TermQuery => Term,
    MatchAllDocsQuery => MatchAll,
    MatchNoDocsQuery => MatchNoDoc,
    DummyQuery => Dummy,
    PointRangeQuery => PointRange,
    SortedNumericDocValuesSetQuery => SortedNumericDocValuesSet,
    SortedNumericDocValuesRangeQuery => SortedNumericDocValuesRange,
    SortedSetDocValuesRangeQuery => SortedSetDocValuesRange,
    IndexSortSortedNumericDocValuesRangeQuery => IndexSortSortedNumericDocValuesRange,
    FieldExistsQuery => FieldExists,
}

// To Query
macro_rules! impl_from_query_via_base {
    ( $( $ty:ty => $variant:ident ),+ $(,)? ) => {
        $(
            impl From<$ty> for Query {
                #[inline]
                fn from(value: $ty) -> Self {
                    BaseQuery::$variant(value).into()
                }
            }
        )+
    };
}

impl_from_query_via_base! {
    TermQuery => Term,
    MatchAllDocsQuery => MatchAll,
    MatchNoDocsQuery => MatchNoDoc,
    DummyQuery => Dummy,
    PointRangeQuery => PointRange,
    SortedNumericDocValuesSetQuery => SortedNumericDocValuesSet,
    SortedNumericDocValuesRangeQuery => SortedNumericDocValuesRange,
    SortedSetDocValuesRangeQuery => SortedSetDocValuesRange,
    IndexSortSortedNumericDocValuesRangeQuery => IndexSortSortedNumericDocValuesRange,
    FieldExistsQuery => FieldExists,
}

pub enum Query {
    Base(BaseQuery),
    ConstantScore(ConstantScoreQuery),
    Boolean(BooleanQuery),
    Boost(BoostQuery),
}
#[cfg(test)]
impl Clone for Query {
    fn clone(&self) -> Self {
        match self {
            Query::Base(b) => Query::Base(b.clone()),
            Query::ConstantScore(c) => Query::ConstantScore(c.clone()),
            Query::Boolean(c) => Query::Boolean(c.clone()),
            Query::Boost(c) => Query::Boost(c.clone()),
        }
    }
}
impl Eq for Query {}

impl PartialEq for Query {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Query::Base(c1), Query::Base(c2)) => c1 == c2,
            (Query::ConstantScore(c1), Query::ConstantScore(c2)) => c1 == c2,
            (Query::Boolean(c1), Query::Boolean(c2)) => c1 == c2,
            (Query::Boost(c1), Query::Boost(c2)) => c1 == c2,
            _ => false,
        }
    }
}

impl Hash for Query {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Query::Base(c) => {
                c.hash(state);
            },
            Query::ConstantScore(c) => {
                c.hash(state);
            },
            Query::Boolean(c) => {
                c.hash(state);
            },
            Query::Boost(c) => {
                c.hash(state);
            },
        }
    }
}
impl Debug for Query {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Query::Base(c) => {
                write!(f, "Query::Base({:?})", c)
            },
            Query::ConstantScore(c) => {
                write!(f, "Query::ConstantScore({:?})", c)
            },
            Query::Boolean(c) => {
                write!(f, "Query::Boolean({:?})", c)
            },
            Query::Boost(c) => {
                write!(f, "Query::Boost({:?})", c)
            },
        }
    }
}

impl QueryBase for Query {
    fn as_string(&self, field: &str) -> String {
        match self {
            Query::Base(c) => c.as_string(field),
            Query::ConstantScore(c) => c.as_string(field),
            Query::Boolean(c) => c.as_string(field),
            Query::Boost(c) => c.as_string(field),
        }
    }

    type Weight<S, IRC, QCP, QC>
        = QueryWeight<S, IRC>
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
        match self {
            Query::Base(b) => b.create_weight(searcher, score_mode, boost, per_reader_term_state),
            Query::ConstantScore(_) | Query::Boolean(_) | Query::Boost(_) => todo!(),
        }
    }

    type RewriteQuery = DummyQuery;

    fn rewrite<IRC, S, QT, QCP, QC>(
        &self,
        _searcher: &IndexSearcher<IRC, S, QT, QCP, QC>,
    ) -> Result<Option<Self::RewriteQuery>>
    where
        IRC: IndexReaderContext,
        S: Similarity,
        QT: QueryTimeout,
        QCP: QueryCachingPolicy,
        QC: QueryCache,
    {
        todo!()
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}
pub type BaseQueryWeight<S, IRC> = WeightEnum9<
    TermWeight<S, IRC>,
    MatchAllWeight<<IRC as IndexReaderContext>::LeafReader>,
    PointRangeWeight<<IRC as IndexReaderContext>::LeafReader>,
    MatchNoDocsWeight<<IRC as IndexReaderContext>::LeafReader>,
    SortedNumericDocValuesSetQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    SortedNumericDocValuesRangeQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    SortedSetDocValuesRangeQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    IndexSortSortedNumericDocValuesRangeQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    FieldExistsWeight<<IRC as IndexReaderContext>::LeafReader>,
>;
pub type QueryWeight<S, IRC> = BaseQueryWeight<S, IRC>;
pub type QueryWeightScorerSupplier<S, IRC> =
    <QueryWeight<S, IRC> as Weight<IRCLeafReader<IRC>>>::ScorerSupplier;

impl From<BaseQuery> for Query {
    fn from(value: BaseQuery) -> Self {
        Query::Base(value)
    }
}
impl From<ConstantScoreQuery> for Query {
    fn from(value: ConstantScoreQuery) -> Self {
        Query::ConstantScore(value)
    }
}
impl From<BooleanQuery> for Query {
    fn from(value: BooleanQuery) -> Self {
        Query::Boolean(value)
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
    Q: QueryBase + ?Sized,
{
    fn as_string(&self, field: &str) -> String {
        (**self).as_string(field)
    }

    type Weight<S, IRC, QCP, QC>
        = Q::Weight<S, IRC, QCP, QC>
    where
        S: Similarity,
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
        QC: QueryCache;

    fn create_weight<S, IRC, QT, QCP, QC>(
        self,
        _searcher: &IndexSearcher<IRC, S, QT, QCP, QC>,
        _score_mode: &ScoreMode,
        _boost: f32,
        _per_reader_term_state: Option<TermStates<IRCTermState<IRC>>>,
    ) -> Result<Self::Weight<S, IRC, QCP, QC>>
    where
        IRC: IndexReaderContext,
        S: Similarity,
        QT: QueryTimeout,
        QCP: QueryCachingPolicy,
        QC: QueryCache,
        Self: Sized,
    {
        Err(LuceneError::unsupported_operation(format!(
            "Arc<QueryBase> cannot be used to create_weight directly: {}",
            std::any::type_name::<Q>()
        )))
    }

    type RewriteQuery = Q::RewriteQuery;

    fn rewrite<IRC, S, QT, QCP, QC>(
        &self,
        searcher: &IndexSearcher<IRC, S, QT, QCP, QC>,
    ) -> Result<Option<Self::RewriteQuery>>
    where
        IRC: IndexReaderContext,
        S: Similarity,
        QT: QueryTimeout,
        QCP: QueryCachingPolicy,
        QC: QueryCache,
    {
        (**self).rewrite(searcher)
    }

    fn visit<QV>(&self, visitor: &QV)
    where
        QV: QueryVisitor,
    {
        (**self).visit(visitor)
    }
}
