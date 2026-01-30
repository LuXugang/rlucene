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
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCTermState, IndexReaderContext};
use crate::core::index::term_states::TermStates;
use crate::core::search::QueryCache;
use crate::core::search::boolean_query::BooleanQuery;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::{
    BaseQueryWeight, ConstantScoreQuery, ConstantScoreQueryWeight,
};
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
use crate::core::search::term_query::{TermQuery, TermWeight};
use crate::core::search::weight::{Weight, WeightEnum10};
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::cmp::PartialEq;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub trait QueryBase: Eq + Hash + Debug + HasIdentity {
    fn as_string(&self, field: &str) -> String;
    type Weight<IRC, QCP, QC>: Weight<IRC::LeafReader>
    where
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
        QC: QueryCache;
    fn create_weight<IRC, QCP, QC>(
        self,
        _searcher: &IndexSearcher<IRC, QCP, QC>,
        _score_mode: &ScoreMode,
        _boost: f32,
        _per_reader_term_state: Option<TermStates<IRCTermState<IRC>>>,
    ) -> Result<Self::Weight<IRC, QCP, QC>>
    where
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
        QC: QueryCache,
        Self: Sized,
    {
        Err(LuceneError::unsupported_operation(format!(
            "Query {} does not implement create_weight",
            std::any::type_name::<Self>()
        )))
    }
    fn rewrite<IRC, QCP, QC>(self, _searcher: &IndexSearcher<IRC, QCP, QC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
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
#[cfg(test)]
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

    type Weight<IRC, QCP, QC>
        = QueryWeight<IRC, QCP, QC>
    where
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
        QC: QueryCache;

    fn create_weight<IRC, QCP, QC>(
        self,
        searcher: &IndexSearcher<IRC, QCP, QC>,
        score_mode: &ScoreMode,
        boost: f32,
        per_reader_term_state: Option<TermStates<IRCTermState<IRC>>>,
    ) -> Result<Self::Weight<IRC, QCP, QC>>
    where
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
        QC: QueryCache,
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
            Query::SortedSetDocValuesRange(p) => Ok(QueryWeight::G(p.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            Query::IndexSortSortedNumericDocValuesRange(p) => Ok(QueryWeight::H(p.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            Query::FieldExists(p) => Ok(QueryWeight::I(p.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            Query::ConstantScore(p) => Ok(QueryWeight::J(p.create_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?)),
            _ => Err(LuceneError::illegal_argument("")),
        }
    }

    fn rewrite<IRC, QCP, QC>(self, searcher: &IndexSearcher<IRC, QCP, QC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
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

    type Weight<IRC, QCP, QC>
        = Q::Weight<IRC, QCP, QC>
    where
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
        QC: QueryCache;

    fn create_weight<IRC, QCP, QC>(
        self,
        _searcher: &IndexSearcher<IRC, QCP, QC>,
        _score_mode: &ScoreMode,
        _boost: f32,
        _per_reader_term_state: Option<TermStates<IRCTermState<IRC>>>,
    ) -> Result<Self::Weight<IRC, QCP, QC>>
    where
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
        QC: QueryCache,
        Self: Sized,
    {
        Err(LuceneError::unsupported_operation(format!(
            "Arc<QueryBase> cannot be used to create_weight directly: {}",
            std::any::type_name::<Q>()
        )))
    }

    fn rewrite<IRC, QCP, QC>(self, _searcher: &IndexSearcher<IRC, QCP, QC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
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
pub type QueryWeight<IRC, QCP, QC> = WeightEnum10<
    TermWeight<IRC>,
    MatchAllWeight<<IRC as IndexReaderContext>::LeafReader>,
    PointRangeWeight<<IRC as IndexReaderContext>::LeafReader>,
    MatchNoDocsWeight<<IRC as IndexReaderContext>::LeafReader>,
    SortedNumericDocValuesSetQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    SortedNumericDocValuesRangeQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    SortedSetDocValuesRangeQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    IndexSortSortedNumericDocValuesRangeQueryWeight<<IRC as IndexReaderContext>::LeafReader>,
    FieldExistsWeight<<IRC as IndexReaderContext>::LeafReader>,
    ConstantScoreQueryWeight<BaseQueryWeight<IRC>, IRC, QCP, QC>,
>;
