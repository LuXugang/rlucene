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
use crate::core::index::leaf_reader::{LeafReader, LeafReaderTermStates, LeafReaderTermsEnum};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::term::Term;
use crate::core::index::term_state::TermState;
use crate::core::index::term_states::{PrepareState, TermStates};
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::dummy::dummy_matches::DummyMatches;
use crate::core::search::dummy::dummy_scorer_supplier::DummyScorerSupplier;
use crate::core::search::dummy::dummy_weight::DummyWeight;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{Query, QueryEnum};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::{
    Either2SimScorer, SimScorer, Similarity,
};
use crate::core::search::term_statistics::TermStatistics;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Eq, Debug)]
pub struct TermQuery {
    term: Term,
}
impl TermQuery {
    pub fn new(term: Term) -> Self {
        Self { term }
    }
}

impl PartialEq<Self> for TermQuery {
    fn eq(&self, other: &Self) -> bool {
        self.term == other.term
    }
}

impl Hash for TermQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // TODO
        self.term.hash(state);
    }
}

impl Query for TermQuery {
    fn wrap(self) -> QueryEnum {
        QueryEnum::Term(self)
    }

    type Weight = DummyWeight;

    fn crate_weight<IRC, LR, S>(
        &self,
        _search: &IndexSearcher<IRC, LR, S>,
        _score_mod: &ScoreMode,
        _boost: f32,
    ) -> Result<Self::Weight>
    where
        IRC: IndexReaderContext<LR>,
        LR: LeafReader,
        S: Similarity,
    {
        todo!()
    }

    type Query = TermQuery;

    fn rewrite<IRC, LR, S>(
        &self,
        _searcher: &IndexSearcher<IRC, LR, S>,
    ) -> Result<Option<Self::Query>>
    where
        IRC: IndexReaderContext<LR>,
        LR: LeafReader,
        S: Similarity,
    {
        todo!()
    }

    fn visit<QV>(&self, visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

impl Display for TermQuery {
    fn fmt(&self, _f: &mut std::fmt::Formatter) -> std::fmt::Result {
        todo!()
    }
}

pub struct TermWeight<S, TS>
where
    S: Similarity,
    TS: TermState,
{
    similarity: Rc<S>,
    sim_scorer: Option<Rc<TermQuerySimScorer<S::SimScorer>>>,
    term_states: Option<TermStates<TS>>,
    score_mode: ScoreMode,
    parent_query: Rc<TermQuery>,
}
impl<S, TS> TermWeight<S, TS>
where
    S: Similarity,
    TS: TermState,
{
    pub fn new<IRC, LR>(
        searcher: &IndexSearcher<IRC, LR, S>,
        score_mode: ScoreMode,
        boost: f32,
        term: Rc<Term>,
        term_states: Option<TermStates<TS>>,
        query: Rc<TermQuery>,
    ) -> Result<Self>
    where
        IRC: IndexReaderContext<LR>,
        LR: LeafReader,
    {
        if score_mode.needs_scores() && term_states.is_none() {
            return Err(LuceneError::illegal_argument(
                "termStates are required when scores are needed",
            ));
        }

        let similarity = searcher.get_similarity();

        // collectionStats 和 termStats
        let ts = term_states.as_ref().unwrap();
        let (collection_stats, term_stats) = if score_mode.needs_scores() {
            let collection_stats = searcher.collection_statistics(term.field());
            let term_stats = if ts.doc_freq()? > 0 {
                Some(searcher.term_statistics(
                    term.clone(),
                    ts.doc_freq()?,
                    ts.total_term_freq()?,
                )?)
            } else {
                None
            };
            (collection_stats, term_stats)
        } else {
            // we do not need the actual stats, use fake stats with docFreq=maxDoc=ttf=1
            let collection_stats = CollectionStatistics::new(term.field().to_string(), 1, 1, 1, 1)?;
            let term_stats = Some(TermStatistics::new(term.clone(), 1, 1)?);
            (collection_stats, term_stats)
        };

        // Assigning a dummy simScorer in case score is not needed to avoid unnecessary float[]
        // allocations in case default BM25Scorer is used.
        // See: https://github.com/apache/lucene/issues/12297
        let sim_scorer = if let Some(term_stats) = term_stats {
            if score_mode.needs_scores() {
                Some(Rc::new(TermQuerySimScorer::A(similarity.scorer(
                    boost,
                    &collection_stats,
                    &[term_stats],
                ))))
            } else {
                Some(Rc::new(TermQuerySimScorer::B(SimScorerImpl)))
            }
        } else {
            None
        };

        Ok(Self {
            similarity,
            sim_scorer,
            term_states,
            score_mode,
            parent_query: query,
        })
    }

    fn get_terms_enum<LR>(
        &self,
        _context: &LeafReaderContext<LR>,
    ) -> Result<Option<<LR::Terms as Terms>::TermsEnum>>
    where
        LR: LeafReader,
    {
        todo!()
    }
}

impl<S, TS> SegmentCacheable for TermWeight<S, TS>
where
    S: Similarity,
    TS: TermState,
{
    fn is_cacheable<LR>(&self, ctx: &LeafReaderContext<LR>) -> bool
    where
        LR: LeafReader,
    {
        todo!()
    }
}

impl<S, TS> Weight for TermWeight<S, TS>
where
    S: Similarity,
    TS: TermState,
{
    type Matches = DummyMatches;

    fn matches<LR>(
        &mut self,
        context: &LeafReaderContext<LR>,
        doc: i32,
    ) -> Result<Option<Self::Matches>>
    where
        LR: LeafReader,
    {
        todo!()
    }

    fn explain<LR>(&self, context: &LeafReaderContext<LR>, doc: i32) -> Result<Explanation>
    where
        LR: LeafReader,
    {
        todo!()
    }

    type Query = TermQuery;

    fn get_query(&self) -> &Self::Query {
        todo!()
    }

    type ScorerSupplier = DummyScorerSupplier;

    fn scorer_supplier<LR>(
        &mut self,
        context: &LeafReaderContext<LR>,
    ) -> Result<Option<Self::ScorerSupplier>>
    where
        LR: LeafReader,
    {
        todo!()
    }

    fn count<LR>(&self, context: &LeafReaderContext<LR>) -> Result<i32>
    where
        LR: LeafReader,
    {
        if !context.reader().has_deletions()? {
            if let Some(mut terms_enum) = self.get_terms_enum(context)? {
                terms_enum.doc_freq()
            } else {
                Ok(0)
            }
        } else {
            self.default_count(context)
        }
    }
}

pub(crate) struct ScorerSupplierImpl<LR>
where
    LR: LeafReader,
{
    terms_enum: Option<LeafReaderTermsEnum<LR>>,
    top_level_scoring_clause: bool,
    term_states: TermStates<LeafReaderTermStates<LR>>,
    prepare_state: PrepareState<LR>,
    context: Rc<LR>,
    term: Rc<Term>,
}
impl<LR> ScorerSupplierImpl<LR>
where
    LR: LeafReader,
{
    // pub(crate) fn get_terms_enum(
    //     &mut self,
    // ) -> Result<Option<&mut LeafReaderTermsEnum<LR>>>
    // {
    //     if self.terms_enum.is_none() {
    //         let state_opt = self.term_states.resolve()?;
    //         let state = match state_opt {
    //             None => return Ok(None),
    //             Some(s) => s,
    //         };
    //
    //         let mut te = self.context
    //             .terms(self.term.field())?
    //             .ok_or_else(|| LuceneError::IllegalState("missing terms".into()))?
    //             .iterator()?;
    //
    //         te.seek_exact_with_state(self.term.bytes(), &state)?;
    //
    //         self.terms_enum = Some(te);
    //     }
    //     Ok(self.terms_enum.as_mut())
    // }
}

pub(crate) struct SimScorerImpl;
impl SimScorer for SimScorerImpl {
    fn score(&self, _freq: f32, _norm: i64) -> f32 {
        0f32
    }
}
pub(crate) type TermQuerySimScorer<S> = Either2SimScorer<S, SimScorerImpl>;
