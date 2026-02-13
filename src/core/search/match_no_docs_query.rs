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
use crate::core::index::leaf_reader::{LRTermState, LeafReader};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::term_states::TermStates;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::Weight;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;

/// A query that matches no documents.
#[derive(Clone, Debug)]
pub struct MatchNoDocsQuery {
    id: Identity,
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
            id: Identity::new(),
            reason: "".to_string(),
        }
    }
    pub fn with_message<T>(reason: T) -> Self
    where
        T: Into<String>,
    {
        let reason = reason.into();
        Self {
            id: Identity::new(),
            reason,
        }
    }

    /// Provides a reason explaining why this query was used
    pub fn with_reason(reason: String) -> Self {
        Self {
            id: Identity::new(),
            reason,
        }
    }
}

impl PartialEq for MatchNoDocsQuery {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for MatchNoDocsQuery {}

impl Hash for MatchNoDocsQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.reason.hash(state);
    }
}

impl HasIdentity for MatchNoDocsQuery {
    fn identity(&self) -> &Identity {
        &self.id
    }
}

impl QueryBase for MatchNoDocsQuery {
    fn as_string(&self, _field: &str) -> String {
        format!("MatchNoDocsQuery(\"{}\")", self.reason)
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
        Ok(Box::new(MatchNoDocsWeight::new(self)))
    }

    fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        Ok(self.into())
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

pub struct MatchNoDocsWeight<LR>
where
    LR: LeafReader,
{
    parent_query: Arc<Query>,
    _leaf_reader: PhantomData<LR>,
}

impl<LR> MatchNoDocsWeight<LR>
where
    LR: LeafReader,
{
    pub fn new(query: MatchNoDocsQuery) -> Self {
        Self {
            parent_query: Arc::new(query.into()),
            _leaf_reader: PhantomData,
        }
    }
}

impl<LR> SegmentCacheable<LR> for MatchNoDocsWeight<LR>
where
    LR: LeafReader,
{
    fn is_cacheable(&self, _ctx: &LeafReaderContext<LR>) -> Result<bool> {
        Ok(true)
    }
}

impl<LR> Weight<LR> for MatchNoDocsWeight<LR>
where
    LR: LeafReader + 'static,
{
    type Matches = MatchWithNoTerms;

    fn matches(
        &self,
        _context: &LeafReaderContext<LR>,
        _doc: i32,
    ) -> Result<Option<Self::Matches>> {
        Ok(None)
    }

    fn explain(&self, _context: &LeafReaderContext<LR>, _doc: i32) -> Result<Explanation> {
        let parent_query = if let Query::MatchNoDoc(v) = self.parent_query.as_ref() {
            v
        } else {
            return Err(LuceneError::illegal_state(""));
        };
        Ok(Explanation::no_match_no_details(parent_query.reason.clone()))
    }

    fn get_query(&self) -> Arc<Query> {
        self.parent_query.clone()
    }

    type ScorerSupplier = QueryWeightSs<LR>;

    fn scorer_supplier(
        &self,
        _context: &LeafReaderContext<LR>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        Ok(None)
    }

    fn count(&self, _context: &LeafReaderContext<LR>) -> Result<i32> {
        Ok(0)
    }
}

impl<LR> std::fmt::Debug for MatchNoDocsWeight<LR>
where
    LR: LeafReader,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "weight({:?})", self.parent_query)
    }
}
