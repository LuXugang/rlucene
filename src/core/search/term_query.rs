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
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::{
    LRImpactsEnum, LRNormNumericDocValues, LRPosting, LRTermState, LRTermsEnum, LeafReader,
};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::postings_enum::{FREQS, NONE};
use crate::core::index::reader_util::ReaderUtil;
use crate::core::index::term::Term;
use crate::core::index::term_states::{PrepareState, TermStateEnum, TermStates, build};
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, EmptyDISI};
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{
    Query, QueryBase, QueryWeight, QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer,
};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, ScorerEnum2};
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::{
    SimScorer, SimScorerEnum2, Similarity, SimilarityEnum, SimilaritySimScorer,
};
use crate::core::search::term_scorer::TermScorer;
use crate::core::search::term_statistics::TermStatistics;
use crate::core::search::weight::Weight;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// A Query that matches documents containing a term. This may be combined with other terms with a [`BooleanQuery`](crate::core::search::boolean_query::BooleanQuery).
#[derive(Clone)]
pub struct TermQuery {
    id: Identity,
    term: Arc<Term>,
}
impl TermQuery {
    pub fn new<T>(term: T) -> Self
    where
        T: Into<Arc<Term>>,
    {
        Self {
            id: Identity::new(),
            term: term.into(),
        }
    }
    pub fn get_term(&self) -> Arc<Term> {
        self.term.clone()
    }
}

impl PartialEq for TermQuery {
    fn eq(&self, other: &Self) -> bool {
        self.term == other.term
    }
}

impl Hash for TermQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.term.hash(state);
    }
}
impl Eq for TermQuery {}

impl Debug for TermQuery {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string(""))
    }
}

impl HasIdentity for TermQuery {
    fn identity(&self) -> &Identity {
        &self.id
    }
}

impl QueryBase for TermQuery {
    fn as_string(&self, field: &str) -> String {
        let mut buffer = String::new();
        if self.term.field != field {
            buffer.push_str(&self.term.field);
            buffer.push(':');
        }
        match self.term.text() {
            Ok(text) => {
                buffer.push_str(&text);
            },
            Err(_) => {
                buffer.push_str("<?>");
            },
        }
        buffer
    }

    fn create_weight<IRC>(
        self,
        searcher: &IndexSearcher<IRC>,
        score_mode: &ScoreMode,
        boost: f32,
        per_reader_term_state: Option<TermStates<LRTermState<IRCLeafReader<IRC>>>>,
    ) -> Result<QueryWeight<IRCLeafReader<IRC>>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
        <IRC as IndexReaderContext>::LeafReader: 'static,
    {
        let context = searcher.get_top_reader_context();
        let term_state = match per_reader_term_state {
            Some(states) if states.was_built_for_some(context.base().id()) => states,
            _ => build(searcher, self.term.clone(), score_mode.needs_scores())?,
        };
        Ok(Box::new(TermWeight::new(
            searcher,
            *score_mode,
            boost,
            term_state,
            self,
        )?))
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
pub struct TermWeight<LR>
where
    LR: LeafReader,
{
    similarity: Arc<SimilarityEnum>,
    sim_scorer: Option<Arc<TermQuerySimScorer>>,
    term_states: Arc<Mutex<TermStates<LRTermState<LR>>>>,
    score_mode: ScoreMode,
    parent_query: Arc<Query>,
}
impl<LR> TermWeight<LR>
where
    LR: LeafReader,
{
    pub fn new<IRC>(
        searcher: &IndexSearcher<IRC>,
        score_mode: ScoreMode,
        boost: f32,
        term_states: TermStates<LRTermState<LR>>,
        query: TermQuery,
    ) -> Result<Self>
    where
        IRC: IndexReaderContext,
    {
        let similarity = searcher.get_similarity();

        let (collection_stats, term_stats) = if score_mode.needs_scores() {
            let collection_stats = searcher.collection_statistics(query.term.field())?;
            let term_stats = if term_states.doc_freq()? > 0 {
                Some(searcher.term_statistics(
                    query.term.clone(),
                    term_states.doc_freq()?,
                    term_states.total_term_freq()?,
                )?)
            } else {
                None
            };
            (collection_stats, term_stats)
        } else {
            // we do not need the actual stats, use fake stats with docFreq=maxDoc=ttf=1
            let collection_stats = Some(CollectionStatistics::new(query.term.field(), 1, 1, 1, 1)?);
            let term_stats = Some(TermStatistics::new(query.term.clone(), 1, 1)?);
            (collection_stats, term_stats)
        };

        // Assigning a dummy simScorer in case score is not needed to avoid unnecessary float[]
        // allocations in case default BM25Scorer is used.
        // See: https://github.com/apache/lucene/issues/12297
        let sim_scorer = if let Some(term_stats) = term_stats {
            debug_assert!(collection_stats.is_some());
            if score_mode.needs_scores() {
                Some(Arc::new(TermQuerySimScorer::A(similarity.scorer(
                    boost,
                    collection_stats.as_ref().unwrap(),
                    &[term_stats],
                )?)))
            } else {
                Some(Arc::new(TermQuerySimScorer::B(SimScorerImpl)))
            }
        } else {
            None
        };

        Ok(Self {
            similarity,
            sim_scorer,
            term_states: Arc::new(Mutex::new(term_states)),
            score_mode,
            parent_query: Arc::new(query.into()),
        })
    }
    /// Returns a TermsEnum positioned at this weights Term or None if the term does not exist in the given context
    fn get_terms_enum(&self, context: &LeafReaderContext<LR>) -> Result<Option<LRTermsEnum<LR>>> {
        debug_assert!(
            {
                let v = ReaderUtil::get_top_level_context(context);
                self.term_states.lock().was_built_for(v)
            },
            "The top-reader used to create Weight is not the same as the current reader's top-reader"
        );
        let mut term_states = self.term_states.lock();
        let mut supplier = term_states.get(context)?;

        let state = match supplier {
            Some(ref mut s) => term_states.resolve(s)?,
            None => None,
        };
        let parent_query = if let Query::Term(v) = self.parent_query.as_ref() {
            v
        } else {
            return Err(LuceneError::illegal_state(""));
        };

        let Some(state) = state else {
            debug_assert!(
                self.term_not_in_reader(context.reader(), parent_query.term.as_ref())?,
                "no termstate found but term exists in reader"
            );
            return Ok(None);
        };
        let mut terms_enum = context
            .reader()
            .terms(parent_query.term.field())?
            .as_ref()
            .unwrap()
            .iterator()?;
        match state.as_ref() {
            TermStateEnum::A(s) => {
                terms_enum.seek_exact_with_state(parent_query.term.bytes(), s)?;
                Ok(Some(terms_enum))
            },
            TermStateEnum::B(_) => Err(LuceneError::illegal_state(
                "should never get empty term state here",
            )),
        }
    }
    fn term_not_in_reader(&self, reader: &LR, term: &Term) -> Result<bool>
    where
        LR: LeafReader,
    {
        Ok(LeafReader::doc_freq(reader, term)? == 0)
    }
}

impl<LR> SegmentCacheable<LR> for TermWeight<LR>
where
    LR: LeafReader,
{
    fn is_cacheable(&self, _ctx: &LeafReaderContext<LR>) -> Result<bool> {
        Ok(true)
    }
}

impl<LR> Weight<LR> for TermWeight<LR>
where
    LR: LeafReader + 'static,
{
    type Matches = MatchWithNoTerms;

    fn matches(
        &self,
        _context: &LeafReaderContext<LR>,
        _doc: i32,
    ) -> Result<Option<Self::Matches>> {
        todo!()
    }

    fn explain(&self, context: &LeafReaderContext<LR>, doc: i32) -> Result<Explanation> {
        let scorer_opt = self.build_term_scorer(context)?;
        if let Some(mut scorer) = scorer_opt {
            let new_doc = scorer.iterator_mut().advance(doc)?;
            if new_doc == doc {
                let freq = match &mut scorer {
                    ScorerEnum2::A(ts) => ts.freq()?,
                    ScorerEnum2::B(_) => {
                        return Err(LuceneError::illegal_state("should TermScorer here"));
                    },
                };

                let mut norm: i64 = 1;
                let parent_query = if let Query::Term(v) = self.parent_query.as_ref() {
                    v
                } else {
                    return Err(LuceneError::illegal_state(""));
                };

                if let Some(mut norms) =
                    context.reader().get_norm_values(&parent_query.term.field)?
                    && norms.advance_exact(doc)?
                {
                    norm = norms.long_value()?;
                }

                let freq_explanation = Explanation::match_no_details(
                    freq,
                    "freq, occurrences of term within document".to_string(),
                );

                let score_explanation = self
                    .sim_scorer
                    .as_ref()
                    .unwrap()
                    .explain(freq_explanation, norm);

                return Ok(Explanation::match_(
                    score_explanation.value,
                    format!(
                        "weight({:?} in {}) [{}], result of:",
                        self.get_query(),
                        doc,
                        self.similarity,
                    ),
                    vec![score_explanation],
                ));
            }
        }

        Ok(Explanation::no_match_no_details(
            "no matching term".to_string(),
        ))
    }

    fn get_query(&self) -> Arc<Query> {
        self.parent_query.clone()
    }

    type ScorerSupplier = QueryWeightSs<LR>;

    fn scorer_supplier(
        &self,
        context: &LeafReaderContext<LR>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        debug_assert!(
            {
                let v = ReaderUtil::get_top_level_context(context);
                self.term_states.lock().was_built_for(v)
            },
            "The top-reader used to create Weight is not the same as the current reader's top-reader"
        );
        let state_supplier = self.term_states.lock().get(context)?;
        let parent_query = if let Query::Term(v) = self.parent_query.as_ref() {
            v
        } else {
            return Err(LuceneError::illegal_state(""));
        };

        match state_supplier {
            None => Ok(None),
            Some(v) => {
                debug_assert!(self.sim_scorer.is_some());
                let v = TermScorerSupplier::new(
                    false,
                    self.term_states.clone(),
                    v,
                    parent_query.term.clone(),
                    self.sim_scorer.as_ref().unwrap().clone(),
                    self.score_mode,
                );
                let v = Box::new(v);
                Ok(Some(v))
            },
        }
    }

    fn count(&self, context: &LeafReaderContext<LR>) -> Result<i32> {
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

impl<LR> TermWeight<LR>
where
    LR: LeafReader + 'static,
{
    fn build_term_scorer(
        &self,
        context: &LeafReaderContext<LR>,
    ) -> Result<Option<TermScorerEnum<LR, EmptyDISI, DummyTwoPhaseIterator>>> {
        match self.get_terms_enum(context)? {
            Some(mut terms_enum) => {
                let norms = if self.score_mode.needs_scores() {
                    let parent_query = if let Query::Term(v) = self.parent_query.as_ref() {
                        v
                    } else {
                        return Err(LuceneError::illegal_state(""));
                    };
                    context.reader().get_norm_values(&parent_query.term.field)?
                } else {
                    None
                };

                if self.score_mode == ScoreMode::TopScores {
                    let v = TermScorerEnum::<LR, EmptyDISI, DummyTwoPhaseIterator>::A(
                        TermScorer::from_impacts(
                            terms_enum.impacts(FREQS as i32)?,
                            self.sim_scorer.as_ref().unwrap().clone(),
                            norms,
                            false,
                        ),
                    );
                    Ok(Some(v))
                } else {
                    let flags = if self.score_mode.needs_scores() {
                        FREQS
                    } else {
                        NONE
                    };
                    let v = TermScorerEnum::<LR, EmptyDISI, DummyTwoPhaseIterator>::A(
                        TermScorer::from_postings(
                            terms_enum.postings_with_flags(None, flags as i32)?,
                            self.sim_scorer.as_ref().unwrap().clone(),
                            norms,
                        ),
                    );
                    Ok(Some(v))
                }
            },
            None => {
                let v = TermScorerEnum::<LR, EmptyDISI, DummyTwoPhaseIterator>::B(
                    ConstantScoreScorer::from_disi(0.0, self.score_mode, EmptyDISI::default()),
                );
                Ok(Some(v))
            },
        }
    }
}
pub struct TermScorerSupplier<LR>
where
    LR: LeafReader,
{
    top_level_scoring_clause: bool,
    term_states: Arc<Mutex<TermStates<LRTermState<LR>>>>,
    prepare_state: PrepareState<LR>,
    term: Arc<Term>,
    sim_scorer: Arc<TermQuerySimScorer>,
    score_mode: ScoreMode,
    terms_enum: Option<LRTermsEnum<LR>>,
}
impl<LR> TermScorerSupplier<LR>
where
    LR: LeafReader,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        top_level_scoring_clause: bool,
        term_states: Arc<Mutex<TermStates<LRTermState<LR>>>>,
        prepare_state: PrepareState<LR>,
        term: Arc<Term>,
        sim_scorer: Arc<TermQuerySimScorer>,
        score_mode: ScoreMode,
    ) -> Self {
        Self {
            top_level_scoring_clause,
            term_states,
            prepare_state,
            term,
            sim_scorer,
            score_mode,
            terms_enum: None,
        }
    }

    pub(crate) fn get_terms_enum(&mut self, context: &LeafReaderContext<LR>) -> Result<Option<()>> {
        if self.terms_enum.is_none() {
            // TODO IMPORTANT 如果state_opt为None 那么terms_enum仍然为None 如果执行cost会再次尝试resolve 是不是可以增加一个flag避免重复resolve
            let state_opt = self.term_states.lock().resolve(&mut self.prepare_state)?;
            match state_opt {
                None => return Ok(None),
                Some(s) => match s.as_ref() {
                    TermStateEnum::A(s) => {
                        let mut terms_enum = match context.reader().terms(self.term.field())? {
                            Some(term) => term.iterator()?,
                            None => {
                                return Err(LuceneError::illegal_argument(format!(
                                    "term should exist here {}",
                                    self.term
                                )));
                            },
                        };
                        terms_enum.seek_exact_with_state(self.term.bytes(), s)?;
                        self.terms_enum = Some(terms_enum);
                    },
                    TermStateEnum::B(_) => {
                        return Err(LuceneError::illegal_state(
                            "should never get empty term state here",
                        ));
                    },
                },
            };
        }
        Ok(Some(()))
    }
}
impl<LR> ScorerSupplier<LR> for TermScorerSupplier<LR>
where
    LR: LeafReader + 'static,
{
    type Scorer = QueryWeightSsScorer;
    type BulkScorer = QueryWeightSsBulkScorer;

    fn get(&mut self, _lead_cost: i64, context: &LeafReaderContext<LR>) -> Result<Self::Scorer> {
        match self.get_terms_enum(context)? {
            Some(_) => {
                debug_assert!(self.terms_enum.is_some());
                let norms = if self.score_mode.needs_scores() {
                    context.reader().get_norm_values(&self.term.field)?
                } else {
                    None
                };

                if self.score_mode == ScoreMode::TopScores {
                    let v = TermScorerEnum::<LR, EmptyDISI, DummyTwoPhaseIterator>::A(
                        TermScorer::from_impacts(
                            self.terms_enum.as_mut().unwrap().impacts(FREQS as i32)?,
                            self.sim_scorer.clone(),
                            norms,
                            self.top_level_scoring_clause,
                        ),
                    );
                    Ok(Box::new(v))
                } else {
                    let flags = if self.score_mode.needs_scores() {
                        FREQS
                    } else {
                        NONE
                    };
                    let v = TermScorerEnum::<LR, EmptyDISI, DummyTwoPhaseIterator>::A(
                        TermScorer::from_postings(
                            self.terms_enum
                                .as_mut()
                                .unwrap()
                                .postings_with_flags(None, flags as i32)?,
                            self.sim_scorer.clone(),
                            norms,
                        ),
                    );
                    Ok(Box::new(v))
                }
            },
            None => {
                let v = TermScorerEnum::<LR, EmptyDISI, DummyTwoPhaseIterator>::B(
                    ConstantScoreScorer::from_disi(0.0, self.score_mode, EmptyDISI::default()),
                );
                Ok(Box::new(v))
            },
        }
    }

    fn bulk_scorer(&mut self, context: &LeafReaderContext<LR>) -> Result<Option<Self::BulkScorer>> {
        Ok(Some(Box::new(self.default_bulk_scorer(context)?)))
    }

    fn cost(&mut self, context: &LeafReaderContext<LR>) -> Result<i64> {
        let result: Result<i32> = (|| match self.get_terms_enum(context)? {
            None => Ok(0),
            Some(_) => Ok(self.terms_enum.as_mut().unwrap().doc_freq()?),
        })();
        match result {
            Ok(v) => Ok(v as i64),
            Err(e) => Err(LuceneError::unchecked_io_error(e)),
        }
    }
}

pub struct SimScorerImpl;
impl SimScorer for SimScorerImpl {
    fn score(&self, _freq: f32, _norm: i64) -> f32 {
        0f32
    }
}
pub(crate) type TermQuerySimScorer = SimScorerEnum2<SimilaritySimScorer, SimScorerImpl>;

pub type TermScorerEnum<LR, DISI, TPI> = ScorerEnum2<
    TermScorer<
        LRPosting<LR>,
        Arc<TermQuerySimScorer>,
        LRNormNumericDocValues<LR>,
        LRImpactsEnum<LR>,
    >,
    ConstantScoreScorer<DISI, TPI>,
>;
