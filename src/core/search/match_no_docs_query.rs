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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::dummy::dummy_scorer_supplier::DummyScorerSupplier;
use crate::core::search::explanation::Explanation;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryEnum};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

/// A query that matches no documents.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct MatchNoDocsQuery {
    reason: String,
}

impl Default for MatchNoDocsQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl MatchNoDocsQuery {
    /// Default constructor
    pub fn new() -> Self {
        Self {
            reason: String::new(),
        }
    }

    /// Provides a reason explaining why this query was used
    pub fn with_reason(reason: String) -> Self {
        Self { reason }
    }
}

impl Query for MatchNoDocsQuery {
    fn as_string(&self, _field: &str) -> String {
        format!("MatchNoDocsQuery(\"{}\")", self.reason)
    }

    type Weight<S, IRC>
        = MatchNoDocsWeight
    where
        S: Similarity,
        IRC: IndexReaderContext;
    type RewriteQuery = MatchNoDocsQuery;

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

impl Display for MatchNoDocsQuery {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string(""))
    }
}

pub struct MatchNoDocsWeight {
    parent_query: QueryEnum,
}

impl MatchNoDocsWeight {
    pub fn new(query: MatchNoDocsQuery) -> Self {
        Self {
            parent_query: query.into(),
        }
    }
}

impl<LR> SegmentCacheable<LR> for MatchNoDocsWeight
where
    LR: LeafReader,
{
    fn is_cacheable(&self, _ctx: &LeafReaderContext<LR>) -> bool {
        true
    }
}

impl<LR> Weight<LR> for MatchNoDocsWeight
where
    LR: LeafReader,
{
    type Matches = MatchWithNoTerms;

    fn matches(
        &mut self,
        _context: &LeafReaderContext<LR>,
        _doc: i32,
    ) -> Result<Option<Self::Matches>> {
        Ok(None)
    }

    fn explain(&mut self, _context: &LeafReaderContext<LR>, _doc: i32) -> Result<Explanation> {
        let QueryEnum::MatchNoDoc(parent_query) = &self.parent_query else {
            unreachable!("should never happen");
        };
        Ok(Explanation::no_match(parent_query.reason.clone(), vec![]))
    }

    type Query = MatchNoDocsQuery;

    fn get_query(&self) -> &Self::Query {
        let QueryEnum::MatchNoDoc(parent_query) = &self.parent_query else {
            unreachable!("should never happen");
        };
        parent_query
    }

    type ScorerSupplier = DummyScorerSupplier;

    fn scorer_supplier(
        &mut self,
        _context: &LeafReaderContext<LR>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        Ok(None)
    }

    fn count(&mut self, _context: &LeafReaderContext<LR>) -> Result<i32> {
        Ok(0)
    }
}

impl std::fmt::Debug for MatchNoDocsWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "weight({})", self.parent_query)
    }
}
