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
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryBase, QueryWeight, QueryWeightSs};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::HasIdentity;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bit_set_iterator::BitSetIterator;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub struct TestScorerPerf;

#[derive(Clone, Debug)]
pub struct BitSetQuery {
    docs: Arc<FixedBitSet>,
    id: Identity,
}
impl BitSetQuery {
    pub fn new(docs: Arc<FixedBitSet>) -> Self {
        Self {
            docs,
            id: Identity::new(),
        }
    }
}

impl HasIdentity for BitSetQuery {
    fn identity(&self) -> &Identity {
        &self.id
    }
}

impl QueryBase for BitSetQuery {
    fn as_string(&self, _field: &str) -> Result<String> {
        Ok("randomBitSetFilter".to_string())
    }

    fn create_weight<IRC>(
        self,
        _searcher: &IndexSearcher<IRC>,
        score_mode: &ScoreMode,
        boost: f32,
    ) -> Result<QueryWeight<IRC>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        Ok(Box::new(BitSetQueryWeight::new(boost, self, *score_mode)))
    }

    fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        Ok(Query::BitSet(self))
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
    }
}
impl PartialEq for BitSetQuery {
    fn eq(&self, other: &Self) -> bool {
        self.docs == other.docs
    }
}
impl Hash for BitSetQuery {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.docs.hash(state);
    }
}
impl Eq for BitSetQuery {}

pub struct BitSetQueryWeight {
    docs: Arc<FixedBitSet>,
    score_mode: ScoreMode,
    query: Arc<Query>,
    base: ConstantScoreWeight,
}
impl BitSetQueryWeight {
    pub fn new(score: f32, query: BitSetQuery, score_mode: ScoreMode) -> Self {
        let docs = query.docs.clone();
        Self {
            docs,
            score_mode,
            query: Arc::new(Query::BitSet(query)),
            base: ConstantScoreWeight::new(score),
        }
    }
}

impl<IRC> SegmentCacheable<IRC> for BitSetQueryWeight
where
    IRC: IndexReaderContext,
{
    fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
        Ok(false)
    }
}

impl<IRC> Weight<IRC> for BitSetQueryWeight
where
    IRC: IndexReaderContext,
{
    type Matches = MatchWithNoTerms;

    fn matches(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        doc: i32,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::Matches>> {
        self.default_matches(context, doc, searcher)
    }

    fn explain(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        doc: i32,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Explanation> {
        let scorer = self.scorer(context, searcher)?;
        self.base.explain(scorer, doc, self.query.as_string("")?)
    }

    fn get_query(&self) -> Arc<Query> {
        self.query.clone()
    }

    type ScorerSupplier = QueryWeightSs<IRC>;

    fn scorer_supplier(
        &self,
        _context: &LeafReaderContext<IRCLeafReader<IRC>>,
        _searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        let iter = BitSetIterator::new(
            self.docs.clone(),
            self.docs.approximate_cardinality() as i64,
        )?;
        let s = ConstantScoreScorer::from_disi(self.base.score(), self.score_mode, iter);
        let ss = DefaultScorerSupplier::new(s);
        Ok(Some(Box::new(ss)))
    }
}
