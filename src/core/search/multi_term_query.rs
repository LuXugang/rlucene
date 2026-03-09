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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::terms::{Terms, TermsPosting};
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::automaton_query::AutomatonQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query_constant_score_blended_wrapper::MultiTermQueryConstantScoreBlendedWrapper;
use crate::core::search::multi_term_query_constant_score_wrapper::MultiTermQueryConstantScoreWrapper;
use crate::core::search::prefix_query::PrefixQuery;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::regexp_query::RegexpQuery;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::term_range_query::TermRangeQuery;
use crate::core::search::wildcard_query::WildcardQuery;
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::impl_from_for_enum;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

pub trait MultiTermQuery: QueryBase + Clone {
    fn get_field(&self) -> &str;
    type TermsEnum<T>: TermsEnum<PostingsEnum = TermsPosting<T>>
    where
        T: Terms;
    fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
    where
        T: Terms + Clone;
    fn get_terms_count(&self) -> i64 {
        -1
    }

    fn as_query(&self) -> Query;
}
pub trait RewriteMethod {
    fn rewrite<IRC>(
        self,
        index_searcher: &IndexSearcher<IRC>,
        query: MultiTermQueryEnum,
    ) -> Result<Query>
    where
        IRC: IndexReaderContext;
}
/// A rewrite method where documents are assigned a constant score equal to the query's boost.
/// Maintains a boolean query-like implementation over the most costly terms while pre-processing
/// the less costly terms into a filter bitset. Enforces an upper-limit on the number of terms
/// allowed in the boolean query-like implementation.
///
/// This method aims to balance the benefits of both [`ConstantScoreRewrite`] and
/// [`ConstantScoreRewrite`] by enabling skipping and early termination over costly terms
/// while limiting the overhead of a BooleanQuery with many terms. It also ensures you cannot hit
/// `IndexSearcher.TooManyClauses`. For some use-cases with all low
/// cost terms, [`ConstantScoreRewrite`] may be more performant. While for some use-cases
/// with all high cost terms, [`ConstantScoreBooleanRewrite`] may be better.
#[derive(Default, Clone)]
pub struct ConstantScoreBlendedRewrite;
impl RewriteMethod for ConstantScoreBlendedRewrite {
    fn rewrite<IRC>(
        self,
        _index_searcher: &IndexSearcher<IRC>,
        query: MultiTermQueryEnum,
    ) -> Result<Query>
    where
        IRC: IndexReaderContext,
    {
        Ok(MultiTermQueryConstantScoreBlendedWrapper::new(query).into())
    }
}
/// A rewrite method that first creates a private Filter, by visiting each term in sequence and
/// marking all docs for that term. Matching documents are assigned a constant score equal to the
/// query's boost.
///
/// This method is faster than the BooleanQuery rewrite methods when the number of matched terms
/// or matched documents is non-trivial. Also, it will never hit an errant `IndexSearcher.TooManyClauses`
/// exception.
#[derive(Default, Clone)]
pub struct ConstantScoreRewrite;
impl RewriteMethod for ConstantScoreRewrite {
    fn rewrite<IRC>(
        self,
        _index_searcher: &IndexSearcher<IRC>,
        query: MultiTermQueryEnum,
    ) -> Result<Query>
    where
        IRC: IndexReaderContext,
    {
        Ok(MultiTermQueryConstantScoreWrapper::new(query).into())
    }
}
#[derive(Clone)]
pub enum RewriteMethodEnum {
    Standard(ConstantScoreRewrite),
    Blended(ConstantScoreBlendedRewrite),
}
impl RewriteMethod for RewriteMethodEnum {
    fn rewrite<IRC>(
        self,
        index_searcher: &IndexSearcher<IRC>,
        query: MultiTermQueryEnum,
    ) -> Result<Query>
    where
        IRC: IndexReaderContext,
    {
        match self {
            RewriteMethodEnum::Standard(r) => r.rewrite(index_searcher, query),
            RewriteMethodEnum::Blended(r) => r.rewrite(index_searcher, query),
        }
    }
}
impl_from_for_enum!(
    RewriteMethodEnum,
    ConstantScoreRewrite => Standard,
    ConstantScoreBlendedRewrite => Blended,
);

#[derive(Clone)]
pub enum MultiTermQueryEnum {
    Prefix(PrefixQuery),
    TermRange(TermRangeQuery),
    Automaton(AutomatonQuery),
    Wildcard(WildcardQuery),
    Regexp(RegexpQuery),
}

impl QueryBase for MultiTermQueryEnum {
    fn as_string(&self, field: &str) -> Result<String> {
        match self {
            MultiTermQueryEnum::Prefix(q) => q.as_string(field),
            MultiTermQueryEnum::TermRange(q) => q.as_string(field),
            MultiTermQueryEnum::Automaton(q) => q.as_string(field),
            MultiTermQueryEnum::Wildcard(q) => q.as_string(field),
            MultiTermQueryEnum::Regexp(q) => q.as_string(field),
        }
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
        Err(LuceneError::unsupported_operation(""))
    }

    fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        Err(LuceneError::unsupported_operation(""))
    }

    fn visit<QV>(&self, visitor: &QV)
    where
        QV: QueryVisitor,
    {
        match self {
            MultiTermQueryEnum::Prefix(q) => q.visit(visitor),
            MultiTermQueryEnum::TermRange(q) => q.visit(visitor),
            MultiTermQueryEnum::Automaton(q) => q.visit(visitor),
            MultiTermQueryEnum::Wildcard(q) => q.visit(visitor),
            MultiTermQueryEnum::Regexp(q) => q.visit(visitor),
        }
    }
}

impl Debug for MultiTermQueryEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MultiTermQueryEnum::Prefix(q) => write!(f, "Prefix({:?})", q),
            MultiTermQueryEnum::TermRange(q) => write!(f, "TermRange({:?})", q),
            MultiTermQueryEnum::Automaton(q) => write!(f, "Automaton({:?})", q),
            MultiTermQueryEnum::Wildcard(q) => write!(f, "Wildcard({:?})", q),
            MultiTermQueryEnum::Regexp(q) => write!(f, "Regexp({:?})", q),
        }
    }
}

impl HasIdentity for MultiTermQueryEnum {
    fn identity(&self) -> &Identity {
        match self {
            MultiTermQueryEnum::Prefix(q) => q.identity(),
            MultiTermQueryEnum::TermRange(q) => q.identity(),
            MultiTermQueryEnum::Automaton(q) => q.identity(),
            MultiTermQueryEnum::Wildcard(q) => q.identity(),
            MultiTermQueryEnum::Regexp(q) => q.identity(),
        }
    }
}

impl_from_for_enum!(
    MultiTermQueryEnum,
    PrefixQuery => Prefix,
    TermRangeQuery => TermRange,
    AutomatonQuery => Automaton,
    WildcardQuery => Wildcard,
    RegexpQuery => Regexp,
);
impl Hash for MultiTermQueryEnum {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            MultiTermQueryEnum::Prefix(q) => q.hash(state),
            MultiTermQueryEnum::TermRange(q) => q.hash(state),
            MultiTermQueryEnum::Automaton(q) => q.hash(state),
            MultiTermQueryEnum::Wildcard(q) => q.hash(state),
            MultiTermQueryEnum::Regexp(q) => q.hash(state),
        }
    }
}
impl Eq for MultiTermQueryEnum {}

impl PartialEq for MultiTermQueryEnum {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MultiTermQueryEnum::Prefix(q1), MultiTermQueryEnum::Prefix(q2)) => q1 == q2,
            (MultiTermQueryEnum::TermRange(q1), MultiTermQueryEnum::TermRange(q2)) => q1 == q2,
            (MultiTermQueryEnum::Automaton(q1), MultiTermQueryEnum::Automaton(q2)) => q1 == q2,
            (MultiTermQueryEnum::Wildcard(q1), MultiTermQueryEnum::Wildcard(q2)) => q1 == q2,
            (MultiTermQueryEnum::Regexp(q1), MultiTermQueryEnum::Regexp(q2)) => q1 == q2,
            _ => false,
        }
    }
}
