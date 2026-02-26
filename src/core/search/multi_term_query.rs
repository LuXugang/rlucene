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
use crate::core::index::terms::{Terms, TermsPosting};
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::prefix_query::PrefixQuery;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::term_range_query::TermRangeQuery;
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

pub trait MultiTermQuery: QueryBase {
    fn get_field(&self) -> &str;
    type TermsEnum<T>: TermsEnum<PostingsEnum = TermsPosting<T>>
    where
        T: Terms;
    fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
    where
        T: Terms + Clone;
    fn get_terms_count(&self) -> i64;
}

pub trait RewriteMethod {
    fn rewrite<IRC, Q>(self, index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Q>
    where
        IRC: IndexReaderContext,
        Q: MultiTermQuery + Sized;
}
#[derive(Clone)]
pub enum MultiTermQueryEnum {
    Prefix(PrefixQuery),
    TermRange(TermRangeQuery),
}

impl QueryBase for MultiTermQueryEnum {
    fn as_string(&self, field: &str) -> String {
        match self {
            MultiTermQueryEnum::Prefix(q) => q.as_string(field),
            MultiTermQueryEnum::TermRange(q) => q.as_string(field),
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
        IRCLeafReader<IRC>: 'static,
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
        }
    }
}

impl Debug for MultiTermQueryEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MultiTermQueryEnum::Prefix(q) => write!(f, "Prefix({:?})", q),
            MultiTermQueryEnum::TermRange(q) => write!(f, "TermRange({:?})", q),
        }
    }
}

impl HasIdentity for MultiTermQueryEnum {
    fn identity(&self) -> &Identity {
        match self {
            MultiTermQueryEnum::Prefix(q) => q.identity(),
            MultiTermQueryEnum::TermRange(q) => q.identity(),
        }
    }
}

impl From<PrefixQuery> for MultiTermQueryEnum {
    fn from(v: PrefixQuery) -> Self {
        MultiTermQueryEnum::Prefix(v)
    }
}
impl From<TermRangeQuery> for MultiTermQueryEnum {
    fn from(v: TermRangeQuery) -> Self {
        MultiTermQueryEnum::TermRange(v)
    }
}
impl Hash for MultiTermQueryEnum {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            MultiTermQueryEnum::Prefix(q) => q.hash(state),
            MultiTermQueryEnum::TermRange(q) => q.hash(state),
        }
    }
}
impl Eq for MultiTermQueryEnum {}

impl PartialEq for MultiTermQueryEnum {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MultiTermQueryEnum::Prefix(q1), MultiTermQueryEnum::Prefix(q2)) => q1 == q2,
            (MultiTermQueryEnum::TermRange(q1), MultiTermQueryEnum::TermRange(q2)) => q1 == q2,
            _ => false,
        }
    }
}
