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
use crate::core::index::query_timeout::QueryTimeout;
use crate::core::index::term_states::TermStates;
use crate::core::search::QueryCache;
use crate::core::search::dummy::dummy_query::DummyQuery;
use crate::core::search::dummy::dummy_weight::DummyWeight;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::query_caching_policy::QueryCachingPolicy;
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::term_query::TermQuery;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::cmp::PartialEq;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub trait Query: Eq + Hash + Debug {
    fn as_string(&self, field: &str) -> String;
    type Weight<S, IRC>: Weight<IRC::LeafReader>
    where
        S: Similarity,
        IRC: IndexReaderContext;
    fn create_weight<S, IRC, QT, QCP, QC>(
        self,
        _search: &IndexSearcher<IRC, S, QT, QCP, QC>,
        _score_mod: &ScoreMode,
        _boost: f32,
        _per_reader_term_state: Option<TermStates<IRCTermState<IRC>>>,
    ) -> Result<Self::Weight<S, IRC>>
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
    type RewriteQuery: Query;
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

pub enum QueryEnum {
    Term(TermQuery),
    MatchAll(MatchAllDocsQuery),
    MatchNoDoc(MatchNoDocsQuery),
    Dummy(DummyQuery),
}

impl Eq for QueryEnum {}

impl PartialEq for QueryEnum {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (QueryEnum::Term(t1), QueryEnum::Term(t2)) => t1 == t2,
            (QueryEnum::MatchAll(m1), QueryEnum::MatchAll(m2)) => m1 == m2,
            (QueryEnum::MatchNoDoc(m1), QueryEnum::MatchNoDoc(m2)) => m1 == m2,
            (QueryEnum::Dummy(d1), QueryEnum::Dummy(d2)) => d1 == d2,
            _ => false,
        }
    }
}

impl Hash for QueryEnum {
    fn hash<H: Hasher>(&self, _state: &mut H) {
        match self {
            QueryEnum::Term(t) => {
                t.hash(_state);
            },
            QueryEnum::MatchAll(m) => {
                m.hash(_state);
            },
            QueryEnum::MatchNoDoc(m) => {
                m.hash(_state);
            },
            QueryEnum::Dummy(d) => {
                d.hash(_state);
            },
        }
    }
}
impl Debug for QueryEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryEnum::Term(t) => {
                write!(f, "QueryEnum::Term({:?})", t)
            },
            QueryEnum::MatchAll(m) => {
                write!(f, "QueryEnum::MatchAll({:?})", m)
            },
            QueryEnum::MatchNoDoc(m) => {
                write!(f, "QueryEnum::MatchNoDoc({:?})", m)
            },
            QueryEnum::Dummy(d) => {
                write!(f, "QueryEnum::Dummy({:?})", d)
            },
        }
    }
}

impl Query for QueryEnum {
    fn as_string(&self, field: &str) -> String {
        match self {
            QueryEnum::Term(t) => t.as_string(field),
            QueryEnum::MatchAll(m) => m.as_string(field),
            QueryEnum::MatchNoDoc(m) => m.as_string(field),
            QueryEnum::Dummy(d) => d.as_string(field),
        }
    }

    type Weight<S, IRC>
        = DummyWeight<IRC::LeafReader>
    where
        S: Similarity,
        IRC: IndexReaderContext;

    fn create_weight<S, IRC, QT, QCP, QC>(
        self,
        _search: &IndexSearcher<IRC, S, QT, QCP, QC>,
        _score_mod: &ScoreMode,
        _boost: f32,
        _per_reader_term_state: Option<TermStates<IRCTermState<IRC>>>,
    ) -> Result<Self::Weight<S, IRC>>
    where
        IRC: IndexReaderContext,
        S: Similarity,
        QT: QueryTimeout,
        QCP: QueryCachingPolicy,
        QC: QueryCache,
        Self: Sized,
    {
        todo!()
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

impl From<TermQuery> for QueryEnum {
    fn from(value: TermQuery) -> Self {
        QueryEnum::Term(value)
    }
}
impl From<MatchAllDocsQuery> for QueryEnum {
    fn from(value: MatchAllDocsQuery) -> Self {
        QueryEnum::MatchAll(value)
    }
}
impl From<MatchNoDocsQuery> for QueryEnum {
    fn from(value: MatchNoDocsQuery) -> Self {
        QueryEnum::MatchNoDoc(value)
    }
}
impl From<DummyQuery> for QueryEnum {
    fn from(value: DummyQuery) -> Self {
        QueryEnum::Dummy(value)
    }
}
#[derive(Clone, Debug)]
pub struct IdentityQuery {
    pub(crate) query: Arc<QueryEnum>,
}
impl IdentityQuery {
    pub fn new(query: Arc<QueryEnum>) -> Self {
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
impl<Q> Query for Arc<Q>
where
    Q: Query + ?Sized,
{
    fn as_string(&self, field: &str) -> String {
        (**self).as_string(field)
    }

    type Weight<S, IRC>
        = Q::Weight<S, IRC>
    where
        S: Similarity,
        IRC: IndexReaderContext;

    fn create_weight<S, IRC, QT, QCP, QC>(
        self,
        _search: &IndexSearcher<IRC, S, QT, QCP, QC>,
        _score_mode: &ScoreMode,
        _boost: f32,
        _per_reader_term_state: Option<TermStates<IRCTermState<IRC>>>,
    ) -> Result<Self::Weight<S, IRC>>
    where
        IRC: IndexReaderContext,
        S: Similarity,
        QT: QueryTimeout,
        QCP: QueryCachingPolicy,
        QC: QueryCache,
        Self: Sized,
    {
        Err(LuceneError::unsupported_operation(format!(
            "Arc<Query> cannot be used to create_weight directly: {}",
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
