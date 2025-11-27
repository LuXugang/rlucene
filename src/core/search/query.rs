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
use crate::core::index::index_reader_context::{IRCTermState, IndexReaderContext};
use crate::core::index::query_timeout::QueryTimeout;
use crate::core::index::term_states::TermStates;
use crate::core::search::QueryCache;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::dummy::dummy_query::DummyQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_all_docs_query::{MatchAllDocsQuery, MatchAllWeight};
use crate::core::search::match_no_docs_query::{MatchNoDocsQuery, MatchNoDocsWeight};
use crate::core::search::point_range_query::{PointRangeQuery, PointRangeWeight};
use crate::core::search::query_caching_policy::QueryCachingPolicy;
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::term_query::{TermQuery, TermWeight};
use crate::core::search::weight::{Either6Weight, Weight};
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
        QC: QueryCache<IRC::LeafReader>;
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
        QC: QueryCache<IRC::LeafReader>,
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
        QC: QueryCache<IRC::LeafReader>,
    {
        Ok(None)
    }
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
        }
    }

    type Weight<S, IRC, QCP, QC>
        = QueryWeight<S, IRC>
    where
        S: Similarity,
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
        QC: QueryCache<IRC::LeafReader>;

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
        QC: QueryCache<IRC::LeafReader>,
        Self: Sized,
    {
        match self {
            Query::Term(t) => Ok(QueryWeight::A(t.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            Query::MatchAll(m) => Ok(QueryWeight::B(m.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            Query::PointRange(p) => Ok(QueryWeight::C(p.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            Query::MatchNoDoc(p) => Ok(QueryWeight::D(p.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            Query::SortedNumericDocValuesSet(p) => Ok(QueryWeight::E(p.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            Query::SortedNumericDocValuesRange(p) => Ok(QueryWeight::F(p.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            _ => Err(LuceneError::illegal_argument("")),
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
        QC: QueryCache<IRC::LeafReader>,
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
pub type QueryWeight<S, IRC> = Either6Weight<
    TermWeight<S, IRC>,
    MatchAllWeight<<IRC as IndexReaderContext>::LeafReader>,
    PointRangeWeight<<IRC as IndexReaderContext>::LeafReader>,
    MatchNoDocsWeight<<IRC as IndexReaderContext>::LeafReader>,
    SortedNumericDocValuesSetQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    SortedNumericDocValuesRangeQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
>;

impl From<TermQuery> for Query {
    fn from(value: TermQuery) -> Self {
        Query::Term(value)
    }
}
impl From<MatchAllDocsQuery> for Query {
    fn from(value: MatchAllDocsQuery) -> Self {
        Query::MatchAll(value)
    }
}
impl From<MatchNoDocsQuery> for Query {
    fn from(value: MatchNoDocsQuery) -> Self {
        Query::MatchNoDoc(value)
    }
}
impl From<DummyQuery> for Query {
    fn from(value: DummyQuery) -> Self {
        Query::Dummy(value)
    }
}
impl From<BoostQuery> for Query {
    fn from(value: BoostQuery) -> Self {
        Query::Boost(value)
    }
}
impl From<ConstantScoreQuery> for Query {
    fn from(value: ConstantScoreQuery) -> Self {
        Query::ConstantScore(value)
    }
}
impl From<PointRangeQuery> for Query {
    fn from(value: PointRangeQuery) -> Self {
        Query::PointRange(value)
    }
}
impl From<SortedNumericDocValuesSetQuery> for Query {
    fn from(value: SortedNumericDocValuesSetQuery) -> Self {
        Query::SortedNumericDocValuesSet(value)
    }
}
impl From<SortedNumericDocValuesRangeQuery> for Query {
    fn from(value: SortedNumericDocValuesRangeQuery) -> Self {
        Query::SortedNumericDocValuesRange(value)
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
        QC: QueryCache<IRC::LeafReader>;

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
        QC: QueryCache<IRC::LeafReader>,
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
        QC: QueryCache<IRC::LeafReader>,
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
