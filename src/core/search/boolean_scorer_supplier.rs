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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::block_max_conjunction_scorer::BlockMaxConjunctionScorer;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_scorer::BooleanScorer;
use crate::core::search::bulk_scorer::{BulkScorer, BulkScorerEnum2, BulkScorerEnum3};
use crate::core::search::conjunction_scorer::ConjunctionScorer;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::disjunction_scorer::DisjunctionScorer;
use crate::core::search::disjunction_sum_scorer::DisjunctionSumScorer;
use crate::core::search::dummy::dummy_bulk_scorer::DummyBulkScorer;
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::dummy::dummy_scorer::DummyScorer;
use crate::core::search::filter_scorer::FilterScorer;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::max_score_bulk_scorer::MaxScoreBulkScorer;
use crate::core::search::req_excl_scorer::ReqExclScorer;
use crate::core::search::scorable::Scorable;
use crate::core::search::score::Score;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::score_mode::ScoreMode::CompleteNoScores;
use crate::core::search::scorer::{Scorer, ScorerEnum2, ScorerEnum3, ScorerEnum4, TwoPhaseState};
use crate::core::search::scorer_supplier::{ScorerSupplier, SsBulkScorer, SsScorer};
use crate::core::search::scorer_util::ScorerUtil;
use crate::core::search::wand_scorer::WANDScorer;
use crate::core::search::weight::DefaultBulkScorer;
use crate::core::util::TryIntoInt;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;

pub struct BooleanScorerSupplier<SS, LR>
where
    SS: ScorerSupplier<LR>,
    LR: LeafReader,
{
    subs: HashMap<Occur, Vec<SS>>,
    score_mode: ScoreMode,
    min_should_match: i32,
    max_doc: i32,
    cost: i64,
    top_level_scoring_clause: bool,
    _phantom: PhantomData<LR>,
}
impl<SS, LR> BooleanScorerSupplier<SS, LR>
where
    SS: ScorerSupplier<LR>,
    LR: LeafReader,
{
    fn compute_cost(&mut self, context: &LeafReaderContext<LR>) -> Result<i64> {
        let mut min_required_cost: Option<i64> = None;

        if let Some(v) = self.subs.get_mut(&Occur::Must) {
            for ss in v.iter_mut() {
                let c = ss.cost(context)?;
                min_required_cost = Some(match min_required_cost {
                    Some(prev) => prev.min(c),
                    None => c,
                });
            }
        }

        if let Some(v) = self.subs.get_mut(&Occur::Filter) {
            for ss in v.iter_mut() {
                let c = ss.cost(context)?;
                min_required_cost = Some(match min_required_cost {
                    Some(prev) => prev.min(c),
                    None => c,
                });
            }
        }

        if self.min_should_match == 0
            && let Some(c) = min_required_cost
        {
            return Ok(c);
        }
        let should_cost = match self.subs.get_mut(&Occur::Should) {
            Some(v) => {
                let mut costs = Vec::with_capacity(v.len());
                for ss in v.iter_mut() {
                    costs.push(ss.cost(context)?);
                }

                ScorerUtil::cost_with_min_should_match(
                    costs.into_iter(),
                    v.len(),
                    self.min_should_match.try_convert()?,
                )?
            },
            None => i64::MAX,
        };

        Ok(std::cmp::min(
            min_required_cost.unwrap_or(i64::MAX),
            should_cost,
        ))
    }
    fn optional_bulk_scorer(
        &mut self,
        context: &LeafReaderContext<LR>,
    ) -> Result<
        Option<
            BulkScorerEnum3<
                SsBulkScorer<SS, LR>,
                MaxScoreBulkScorer<SsScorer<SS, LR>>,
                BooleanScorer<SsScorer<SS, LR>>,
            >,
        >,
    > {
        let should_len = self.subs.get(&Occur::Should).map(|v| v.len()).unwrap_or(0);

        if should_len == 0 {
            return Ok(None);
        } else if should_len == 1 && self.min_should_match <= 1 {
            return match self.subs.get_mut(&Occur::Should).unwrap()[0].bulk_scorer(context)? {
                None => Ok(None),
                Some(bs) => return Ok(Some(BulkScorerEnum3::A(bs))),
            };
        }

        if self.score_mode == ScoreMode::TopScores && self.min_should_match <= 1 {
            let mut optional_scorers = Vec::with_capacity(should_len);
            for ss in self.subs.get_mut(&Occur::Should).unwrap().iter_mut() {
                optional_scorers.push(ss.get(i64::MAX, context)?);
            }
            let v = BulkScorerEnum3::B(MaxScoreBulkScorer::new(
                self.max_doc,
                optional_scorers,
                None,
            )?);
            return Ok(Some(v));
        }

        let mut optional = Vec::with_capacity(should_len);
        for ss in self.subs.get_mut(&Occur::Should).unwrap().iter_mut() {
            optional.push(ss.get(i64::MAX, context)?);
        }

        let msm = std::cmp::max(1, self.min_should_match);
        let v = BulkScorerEnum3::C(BooleanScorer::new(
            optional,
            msm as usize,
            self.score_mode.needs_scores(),
        )?);
        Ok(Some(v))
    }
    fn filtered_optional_bulk_scorer(
        &mut self,
        context: &LeafReaderContext<LR>,
    ) -> Result<Option<DummyBulkScorer>> {
        let must_len = self.subs.get(&Occur::Must).map(|v| v.len()).unwrap_or(0);
        let filter_len = self.subs.get(&Occur::Filter).map(|v| v.len()).unwrap_or(0);
        let should_len = self.subs.get(&Occur::Should).map(|v| v.len()).unwrap_or(0);

        if must_len != 0
            || filter_len == 0
            || self.score_mode != ScoreMode::TopScores
            || should_len <= 1
            || self.min_should_match > 1
        {
            return Ok(None);
        }

        let cost = self.cost(context)?;

        let mut optional_scorers = Vec::with_capacity(should_len);
        if let Some(v) = self.subs.get_mut(&Occur::Should) {
            for ss in v.iter_mut() {
                optional_scorers.push(ss.get(cost, context)?);
            }
        }

        let mut filters = Vec::with_capacity(filter_len);
        if let Some(v) = self.subs.get_mut(&Occur::Filter) {
            for ss in v.iter_mut() {
                filters.push(ss.get(cost, context)?);
            }
        }

        let filter_scorer = if filters.len() == 1 {
            filters.pop().unwrap()
        } else {
            todo!()
        };

        let _v = Some(MaxScoreBulkScorer::new(
            self.max_doc,
            optional_scorers,
            Some(filter_scorer),
        )?);
        todo!()
    }
    fn required_bulk_scorer(
        &mut self,
        context: &LeafReaderContext<LR>,
    ) -> Result<Option<DummyBulkScorer>> {
        let must_len = {
            self.subs
                .get_mut(&Occur::Must)
                .map(|v| v.len())
                .unwrap_or(0)
        };
        let filter_len = self
            .subs
            .get_mut(&Occur::Filter)
            .map(|v| v.len())
            .unwrap_or(0);

        let required_cnt = must_len + filter_len;

        // no required clauses
        if required_cnt == 0 {
            return Ok(None);
        }

        if required_cnt == 1 {
            let _scorer = if must_len != 0 {
                let must = self.subs.get_mut(&Occur::Must).unwrap();
                must[0].bulk_scorer(context)?.map(BulkScorerEnum2::A)
            } else {
                let filter = self.subs.get_mut(&Occur::Filter).unwrap();
                if self.score_mode.needs_scores() {
                    match filter[0].bulk_scorer(context)? {
                        None => return Err(LuceneError::illegal_state("bulk_scorer is None"))?,
                        Some(s) => {
                            let v = disable_scoring(s);
                            Some(BulkScorerEnum2::B(v))
                        },
                    }
                } else {
                    filter[0].bulk_scorer(context)?.map(BulkScorerEnum2::A)
                }
            };
            // return Ok(scorer);
            return Ok(Some(DummyBulkScorer));
        }

        let mut lead_cost = i64::MAX;
        match self.subs.get_mut(&Occur::Must) {
            Some(v) if !v.is_empty() => {
                for ss in v.iter_mut() {
                    lead_cost = lead_cost.min(ss.cost(context)?);
                }
            },
            _ => {},
        }
        match self.subs.get_mut(&Occur::Filter) {
            Some(v) if !v.is_empty() => {
                for ss in v.iter_mut() {
                    lead_cost = lead_cost.min(ss.cost(context)?);
                }
            },
            _ => {},
        }

        let mut required_no_scoring = Vec::with_capacity(filter_len);
        if let Some(v) = self.subs.get_mut(&Occur::Filter) {
            for ss in v.iter_mut() {
                required_no_scoring.push(ss.get(lead_cost, context)?);
            }
        }

        let mut required_scoring = Vec::with_capacity(must_len);
        if let Some(v) = self.subs.get_mut(&Occur::Must) {
            for ss in v.iter_mut() {
                if must_len == 1 {
                    ss.set_top_level_scoring_clause()?;
                }
                required_scoring.push(ss.get(lead_cost, context)?);
            }
        }

        if self.score_mode == ScoreMode::TopScores && required_scoring.len() > 1 {
            let mut all_no_scoring_no_two_phase = true;
            for s in required_no_scoring.iter_mut() {
                if s.two_phase_iterator()?.is_some() {
                    all_no_scoring_no_two_phase = false;
                    break;
                }
            }

            let mut all_scoring_no_two_phase = true;
            for s in required_scoring.iter_mut() {
                if s.two_phase_iterator()?.is_some() {
                    all_scoring_no_two_phase = false;
                    break;
                }
            }

            if all_no_scoring_no_two_phase && all_scoring_no_two_phase {
                // Turn all filters into scoring clauses with a score of zero
                let mut wrap_required_scoring =
                    Vec::with_capacity(required_no_scoring.len() + required_scoring.len());
                for x in required_scoring.into_iter() {
                    wrap_required_scoring.push(ScorerEnum2::A(x))
                }
                for filter_scorer in required_no_scoring {
                    wrap_required_scoring.push(ScorerEnum2::B(ConstantScoreScorer::with_disi(
                        0.0,
                        ScoreMode::Complete,
                        filter_scorer.take_iterator(),
                    )));
                }

                // return Ok(Some(
                //     BlockMaxConjunctionBulkScorer::new(self.max_doc, wrap_required_scoring)?,
                // ));
                return Ok(Some(DummyBulkScorer));
            }
        }

        if self.score_mode != ScoreMode::TopScores
            && required_scoring.len() + required_no_scoring.len() >= 2
        {
            let mut all_scoring_no_two_phase = true;
            for s in required_scoring.iter_mut() {
                if s.two_phase_iterator()?.is_some() {
                    all_scoring_no_two_phase = false;
                    break;
                }
            }

            let mut all_no_scoring_no_two_phase = true;
            for s in required_no_scoring.iter_mut() {
                if s.two_phase_iterator()?.is_some() {
                    all_no_scoring_no_two_phase = false;
                    break;
                }
            }

            if all_scoring_no_two_phase && all_no_scoring_no_two_phase {
                // return Ok(Some(
                //     ConjunctionBulkScorer::new(required_scoring, required_no_scoring),
                // ));
                return Ok(Some(DummyBulkScorer));
            }
        }

        // fallback to scorer-based bulk scorer
        let mut required_scoring =
            if self.score_mode == ScoreMode::TopScores && required_scoring.len() > 1 {
                let v = BlockMaxConjunctionScorer::new(required_scoring)?;
                vec![ScorerEnum2::B(v)]
            } else {
                required_scoring.into_iter().map(ScorerEnum2::A).collect()
            };
        let mut required_no_scoring: Vec<
            ScorerEnum2<SsScorer<SS, LR>, BlockMaxConjunctionScorer<SsScorer<SS, LR>>>,
        > = required_no_scoring
            .into_iter()
            .map(ScorerEnum2::A)
            .collect();

        let _conjunction_scorer = if required_scoring.len() + required_no_scoring.len() == 1 {
            if required_scoring.len() == 1 {
                let v = match required_scoring.pop().unwrap() {
                    ScorerEnum2::A(s) => s,
                    ScorerEnum2::B(_) => {
                        return Err(LuceneError::illegal_state(""));
                    },
                };
                ScorerEnum2::A(v)
            } else {
                let inner = match required_no_scoring.pop().unwrap() {
                    ScorerEnum2::A(s) => s,
                    ScorerEnum2::B(_) => return Err(LuceneError::illegal_state("")),
                };
                if self.score_mode.needs_scores() {
                    ScorerEnum2::B(FilterScorerImpl::new(inner))
                } else {
                    ScorerEnum2::A(inner)
                }
            }
        } else {
            todo!()
        };

        // Ok(Some(BulkScorer::Default(DefaultBulkScorer::new(
        //     conjunction_scorer,
        // ))))
        todo!()
    }
    /// Create a new scorer for the given required clauses.
    /// Note that requiredScoring is a subset of required containing required clauses that should participate in scoring.
    fn req(
        &mut self,
        required_no_scoring: &mut [SS],
        required_scoring: &mut [SS],
        lead_cost: i64,
        top_level_scoring_clause: bool,
        context: &LeafReaderContext<LR>,
    ) -> Result<Req<SsScorer<SS, LR>>> {
        if required_no_scoring.len() + required_scoring.len() == 1 {
            let req = if required_no_scoring.is_empty() {
                required_scoring[0].get(lead_cost, context)?
            } else {
                required_no_scoring[0].get(lead_cost, context)?
            };

            if !self.score_mode.needs_scores() {
                return Ok(Req::A(req));
            }

            if required_scoring.is_empty() {
                // Scores are needed but we only have a filter clause
                // BooleanWeight expects that calling score() is ok so we need to wrap
                // to prevent score() from being propagated
                return Ok(Req::B(FilterScorerImpl::new(req)));
            }

            return Ok(Req::A(req));
        }

        let mut required_scorers =
            Vec::with_capacity(required_no_scoring.len() + required_scoring.len());
        let mut scoring_scorers = Vec::with_capacity(required_scoring.len());

        for s in required_no_scoring.iter_mut() {
            required_scorers.push(s.get(lead_cost, context)?);
        }

        for s in required_scoring.iter_mut() {
            let scorer = s.get(lead_cost, context)?;
            scoring_scorers.push(scorer);
        }
        if self.score_mode == ScoreMode::TopScores
            && scoring_scorers.len() > 1
            && top_level_scoring_clause
            && required_scorers.is_empty()
        {
            let block_max_scorer = BlockMaxConjunctionScorer::new(scoring_scorers)?;
            return Ok(Req::C(block_max_scorer));
        }

        // Ok(Req::D(ConjunctionScorer::new(
        //     required_scorers,
        //     scoring_scorers,
        // )?))
        todo!()
    }
    fn excl<S>(
        &mut self,
        main: S,
        prohibited: &mut [SS],
        lead_cost: i64,
        context: &LeafReaderContext<LR>,
    ) -> Result<Excl<S, Opt<SsScorer<SS, LR>>>>
    where
        S: Scorer,
    {
        if prohibited.is_empty() {
            Ok(Excl::A(main))
        } else {
            let opt = self.opt(prohibited, 1, CompleteNoScores, lead_cost, false, context)?;
            Ok(Excl::B(ReqExclScorer::new(main, opt)?))
        }
    }

    fn opt(
        &mut self,
        optional: &mut [SS],
        min_should_match: i32,
        score_mode: ScoreMode,
        lead_cost: i64,
        top_level_scoring_clause: bool,
        context: &LeafReaderContext<LR>,
    ) -> Result<Opt<SsScorer<SS, LR>>> {
        if optional.len() == 1 {
            return Ok(Opt::A(optional[0].get(lead_cost, context)?));
        }

        let mut optional_scorers = Vec::with_capacity(optional.len());
        for supplier in optional.iter_mut() {
            optional_scorers.push(supplier.get(lead_cost, context)?);
        }
        // Technically speaking, WANDScorer should be able to handle the following 3 conditions now
        // 1. Any ScoreMode (with scoring or not)
        // 2. Any minCompetitiveScore ( >= 0 )
        // 3. Any minShouldMatch ( >= 0 )
        //
        // However, as WANDScorer uses more complex algorithm and data structure, we would like to
        // still use DisjunctionSumScorer to handle exhaustive pure disjunctions, which may be faster
        let v = if (score_mode == ScoreMode::TopScores && top_level_scoring_clause)
            || min_should_match > 1
        {
            Opt::B(WANDScorer::new(
                optional_scorers,
                min_should_match,
                score_mode,
                lead_cost,
            )?)
        } else {
            Opt::C(DisjunctionScorer::new(
                optional_scorers,
                score_mode,
                DisjunctionSumScorer,
            )?)
        };
        Ok(v)
    }
}
pub type Excl<S1, S2> = ScorerEnum2<S1, ReqExclScorer<S1, S2>>;
pub type Opt<S> = ScorerEnum3<S, WANDScorer<S>, DisjunctionScorer<S, DisjunctionSumScorer>>;
pub type Req<S> = ScorerEnum4<
    S,
    FilterScorerImpl<S>,
    BlockMaxConjunctionScorer<S>,
    ConjunctionScorer<S, ScorerEnum2<S, BlockMaxConjunctionScorer<S>>>,
>;
impl<SS, LR> ScorerSupplier<LR> for BooleanScorerSupplier<SS, LR>
where
    SS: ScorerSupplier<LR>,
    LR: LeafReader,
{
    type Scorer = DummyScorer;
    type BulkScorer = DummyBulkScorer;

    fn get(&mut self, _lead_cost: i64, _context: &LeafReaderContext<LR>) -> Result<Self::Scorer> {
        todo!()
    }

    fn bulk_scorer(
        &mut self,
        _context: &LeafReaderContext<LR>,
    ) -> Result<Option<Self::BulkScorer>> {
        todo!()
    }

    fn default_bulk_scorer(
        &mut self,
        _context: &LeafReaderContext<LR>,
    ) -> Result<DefaultBulkScorer<Self::Scorer>> {
        todo!()
    }

    fn cost(&mut self, context: &LeafReaderContext<LR>) -> Result<i64> {
        if self.cost == -1 {
            self.cost = self.compute_cost(context)?;
        }
        Ok(self.cost)
    }

    fn set_top_level_scoring_clause(&mut self) -> Result<()> {
        self.top_level_scoring_clause = true;

        let should_len = self.subs.get(&Occur::Should).map(|v| v.len()).unwrap_or(0);
        let must_len = self.subs.get(&Occur::Must).map(|v| v.len()).unwrap_or(0);

        if should_len + must_len == 1 {
            // If there is a single scoring clause, propagate the call.
            if let Some(v) = self.subs.get_mut(&Occur::Should) {
                for ss in v.iter_mut() {
                    ss.set_top_level_scoring_clause()?;
                }
            }
            if let Some(v) = self.subs.get_mut(&Occur::Must) {
                for ss in v.iter_mut() {
                    ss.set_top_level_scoring_clause()?;
                }
            }
        }
        Ok(())
    }
}

pub struct FilterScorerImpl<S>
where
    S: Scorer,
{
    base: FilterScorer<S>,
}
impl<S> FilterScorerImpl<S>
where
    S: Scorer,
{
    fn new(inner: S) -> Self {
        Self {
            base: FilterScorer::new(inner),
        }
    }
}

impl<S> Scorable for FilterScorerImpl<S>
where
    S: Scorer,
{
    fn score(&mut self) -> Result<f32> {
        Ok(0f32)
    }

    type Scorable = <FilterScorer<S> as Scorable>::Scorable;
}

impl<S> Scorer for FilterScorerImpl<S>
where
    S: Scorer,
{
    type DocIdSetIterator = <FilterScorer<S> as Scorer>::DocIdSetIterator;
    type DocIdSetIteratorRef<'a>
        = <FilterScorer<S> as Scorer>::DocIdSetIteratorRef<'a>
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>
        = <FilterScorer<S> as Scorer>::DocIdSetIteratorMut<'a>
    where
        Self: 'a;
    type TwoPhaseIter = <FilterScorer<S> as Scorer>::TwoPhaseIter;
    type TwoPhaseIterRef<'a>
        = <FilterScorer<S> as Scorer>::TwoPhaseIterRef<'a>
    where
        Self: 'a;
    type TwoPhaseIterMut<'a>
        = <FilterScorer<S> as Scorer>::TwoPhaseIterMut<'a>
    where
        Self: 'a;

    fn doc_id(&mut self) -> Result<i32> {
        self.base.doc_id()
    }

    fn iterator(&self) -> Self::DocIdSetIteratorRef<'_> {
        self.base.iterator()
    }

    fn iterator_mut(&mut self) -> Self::DocIdSetIteratorMut<'_> {
        self.base.iterator_mut()
    }

    fn take_iterator(self) -> Self::DocIdSetIterator {
        self.base.take_iterator()
    }

    fn two_phase_iterator(&self) -> Result<Option<Self::TwoPhaseIterRef<'_>>> {
        self.base.two_phase_iterator()
    }

    fn two_phase_iterator_mut(&mut self) -> Result<Option<Self::TwoPhaseIterMut<'_>>> {
        self.base.two_phase_iterator_mut()
    }

    fn take_two_phase_iterator(self) -> Result<Option<Self::TwoPhaseIter>>
    where
        Self: Sized,
    {
        self.base.take_two_phase_iterator()
    }

    fn advance_shallow(&mut self, target: i32) -> Result<i32> {
        self.base.advance_shallow(target)
    }

    fn default_advance_shallow(&mut self, target: i32) -> Result<i32> {
        self.base.default_advance_shallow(target)
    }

    fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
        Ok(0f32)
    }

    fn default_cost(&mut self) -> Result<i64> {
        self.base.default_cost()
    }

    fn has_two_phase_iterator(&self) -> TwoPhaseState {
        self.base.has_two_phase_iterator()
    }
}

pub(crate) fn disable_scoring<BS>(scorer: BS) -> BulkScorerImpl<BS>
where
    BS: BulkScorer,
{
    BulkScorerImpl::new(scorer)
}

pub struct BulkScorerImpl<BS>
where
    BS: BulkScorer,
{
    scorer: BS,
}
impl<BS> BulkScorerImpl<BS>
where
    BS: BulkScorer,
{
    fn new(scorer: BS) -> Self {
        Self { scorer }
    }
}
impl<BS> BulkScorer for BulkScorerImpl<BS>
where
    BS: BulkScorer,
{
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
        let mut no_score_collector = LeafCollectorImpl::new(collector);
        self.scorer
            .score(&mut no_score_collector, accept_docs, min, max)
    }

    fn cost(&mut self) -> Result<i64> {
        self.scorer.cost()
    }
}

pub struct LeafCollectorImpl<LC>
where
    LC: LeafCollector,
{
    collector: LC,
    fake: Score,
}
impl<LC> LeafCollectorImpl<LC>
where
    LC: LeafCollector,
{
    fn new(collector: LC) -> Self {
        Self {
            collector,
            fake: Score::new(0.0),
        }
    }
}

impl<LC> Display for LeafCollectorImpl<LC>
where
    LC: LeafCollector,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "LeafCollectorImpl({})", self.collector)
    }
}

impl<LC> LeafCollector for LeafCollectorImpl<LC>
where
    LC: LeafCollector,
{
    fn set_scorer<S>(&mut self, _scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        self.collector.set_scorer(&mut self.fake)
    }

    fn collect<S>(&mut self, doc: i32, _scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        self.collector.collect(doc, &mut self.fake)
    }

    type DocIdSetIteratorRef<'a>
        = DummyDISI
    where
        Self: 'a;
}
