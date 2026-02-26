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
use crate::core::index::dummy::dummy_terms_enum::DummyTermsEnum2;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::terms::Terms;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::MultiTermQuery;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct PrefixQuery;

impl QueryBase for PrefixQuery {
    fn as_string(&self, _field: &str) -> String {
        todo!()
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
        todo!()
    }

    fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        Self: Sized,
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

impl Debug for PrefixQuery {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl HasIdentity for PrefixQuery {
    fn identity(&self) -> &Identity {
        todo!()
    }
}

impl MultiTermQuery for PrefixQuery {
    fn get_field(&self) -> &str {
        todo!()
    }

    type TermsEnum<T>
        = DummyTermsEnum2<T>
    where
        T: Terms;

    fn get_terms_enum<T>(&self, _terms: T) -> Result<Self::TermsEnum<T>>
    where
        T: Terms + Clone,
    {
        todo!()
    }

    fn get_terms_count(&self) -> i64 {
        todo!()
    }
}
impl Eq for PrefixQuery {}
impl PartialEq for PrefixQuery {
    fn eq(&self, _other: &Self) -> bool {
        todo!()
    }
}
impl Hash for PrefixQuery {
    fn hash<H: Hasher>(&self, _state: &mut H) {
        todo!()
    }
}
