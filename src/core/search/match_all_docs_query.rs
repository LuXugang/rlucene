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
use crate::core::search::bulk_scorer::{BulkScorer, Either2BulkScorer};
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set_iterator::AllDISI;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::Query;
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score::Score;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::weight::{DefaultBulkScorer, Weight};
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
/// A query that matches all documents.
#[derive(Hash, PartialEq, Eq, Debug)]
pub struct MatchAllDocsQuery;
impl Default for MatchAllDocsQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl MatchAllDocsQuery {
    pub fn new() -> Self {
        MatchAllDocsQuery
    }
}

impl Query for MatchAllDocsQuery {
    fn as_string(&self, _field: &str) -> String {
        "*:*".to_string()
    }

    type Weight<S, IRC>
        = MatchAllWeight
    where
        S: Similarity,
        IRC: IndexReaderContext;
    type RewriteQuery = MatchAllDocsQuery;

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}
impl Display for MatchAllDocsQuery {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string(""))
    }
}

pub struct MatchAllWeight {
    base: ConstantScoreWeight,
    parent_query: MatchAllDocsQuery,
    score_mode: ScoreMode,
}
impl MatchAllWeight {
    pub fn new(score: f32, query: MatchAllDocsQuery, score_mode: ScoreMode) -> Self {
        Self {
            base: ConstantScoreWeight::new(score),
            parent_query: query,
            score_mode,
        }
    }
}

impl<LR> SegmentCacheable<LR> for MatchAllWeight
where
    LR: LeafReader,
{
    fn is_cacheable(&self, _ctx: &LeafReaderContext<LR>) -> bool {
        true
    }
}

impl<LR> Weight<LR> for MatchAllWeight
where
    LR: LeafReader,
{
    type Matches = MatchWithNoTerms;

    fn matches(
        &mut self,
        context: &LeafReaderContext<LR>,
        doc: i32,
    ) -> Result<Option<Self::Matches>> {
        self.default_matches(context, doc)
    }

    fn explain(&mut self, context: &LeafReaderContext<LR>, doc: i32) -> Result<Explanation> {
        let scorer = self.scorer(context)?;
        self.base
            .explain(scorer, doc, self.parent_query.to_string())
    }

    type Query = MatchAllDocsQuery;

    fn get_query(&self) -> &Self::Query {
        &self.parent_query
    }

    type ScorerSupplier = MatchAllDocsScorerSupplier;

    fn scorer_supplier(
        &mut self,
        context: &LeafReaderContext<LR>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        Ok(Some(MatchAllDocsScorerSupplier::new(
            self.score_mode,
            std::mem::take(&mut self.base),
            context.reader().max_doc()?,
        )))
    }

    fn count(&mut self, context: &LeafReaderContext<LR>) -> Result<i32> {
        context.reader().num_docs()
    }
}
impl std::fmt::Debug for MatchAllWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "weight({})", MatchAllDocsQuery)
    }
}

pub struct MatchAllDocsScorerSupplier {
    score_mode: ScoreMode,
    weight: ConstantScoreWeight,
    max_doc: i32,
}
impl MatchAllDocsScorerSupplier {
    pub fn new(score_mode: ScoreMode, weight: ConstantScoreWeight, max_doc: i32) -> Self {
        Self {
            score_mode,
            weight,
            max_doc,
        }
    }
}
impl ScorerSupplier for MatchAllDocsScorerSupplier {
    type Scorer = ConstantScoreScorer<AllDISI, DummyTwoPhaseIterator>;
    type BulkScorer = MatchAllBulkScorerEnum<Self::Scorer>;

    fn get(&mut self, _lead_cost: i64) -> Result<Option<Self::Scorer>> {
        let score = self.weight.score();
        Ok(Some(ConstantScoreScorer::with_disi(
            score,
            self.score_mode,
            AllDISI::new(self.max_doc),
        )))
    }

    fn bulk_scorer(&mut self) -> Result<Self::BulkScorer> {
        if !self.score_mode.is_exhaustive() {
            Ok(MatchAllBulkScorerEnum::B(self.default_bulk_scorer()?))
        } else {
            let score = self.weight.score();
            Ok(MatchAllBulkScorerEnum::A(MatchAllBulkScorer::new(
                self.score_mode,
                self.max_doc,
                score,
            )))
        }
    }

    fn cost(&mut self) -> Result<i64> {
        Ok(self.max_doc as i64)
    }
}
pub struct MatchAllBulkScorer {
    score_mode: ScoreMode,
    max_doc: i32,
    score: f32,
}
impl MatchAllBulkScorer {
    pub fn new(score_mode: ScoreMode, max_doc: i32, score: f32) -> Self {
        Self {
            score_mode,
            max_doc,
            score,
        }
    }
}
impl BulkScorer for MatchAllBulkScorer {
    fn score<LC, B>(
        &mut self,
        collector: &mut LC,
        accept_docs: Option<&B>,
        min: i32,
        max: i32,
    ) -> Result<i32>
    where
        LC: LeafCollector,
        B: Bits,
    {
        let max = std::cmp::min(max, self.max_doc);
        let scorer = Score::new(self.score).into();
        collector.set_scorer(scorer)?;
        for doc in min..max {
            if accept_docs.is_none_or(|bits| bits.get(doc)) {
                collector.collect(doc)?;
            }
        }
        if max == self.max_doc {
            Ok(NO_MORE_DOCS)
        } else {
            Ok(max)
        }
    }

    fn cost(&mut self) -> Result<i64> {
        Ok(self.max_doc as i64)
    }
}
pub type MatchAllBulkScorerEnum<T> = Either2BulkScorer<MatchAllBulkScorer, DefaultBulkScorer<T>>;
