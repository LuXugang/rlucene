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
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum2};
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::score_mode::ScoreMode::TopScores;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::{
    TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub type ReqOptSumScorerDisi<S1, S2> = DocIdSetIteratorEnum2<
    DocIdSetIteratorImpl<S1, S2>,
    TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<S1, S2>>,
>;
/// A scorer for queries with a required part and an optional part.
/// Delays advance on the optional part until a score is needed.
pub struct ReqOptSumScorer<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    disi: ReqOptSumScorerDisi<S1, S2>,
    tpi_state: TwoPhaseState,
}
impl<S1, S2> ReqOptSumScorer<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    /// Construct a `ReqOptScorer`.
    ///
    /// * `req_scorer` — the required scorer, which must match
    /// * `opt_scorer` — the optional scorer, used only for scoring
    /// * `score_mode` — how the produced scorers will be consumed
    pub(crate) fn new(
        mut req_scorer: S1,
        mut opt_scorer: S2,
        score_mode: ScoreMode,
    ) -> Result<Self> {
        let req_max_score = if score_mode != TopScores {
            f32::MAX
        } else {
            req_scorer.advance_shallow(0)?;
            opt_scorer.advance_shallow(0)?;
            req_scorer.get_max_score(NO_MORE_DOCS)?
        };
        let has_tpi = (req_scorer.has_two_phase_iterator() == TwoPhaseState::Yes
            || req_scorer.two_phase_iterator()?.is_some())
            && (opt_scorer.has_two_phase_iterator() == TwoPhaseState::Yes
                || opt_scorer.two_phase_iterator()?.is_some());
        let approximation = DocIdSetIteratorImpl::new(req_scorer, opt_scorer, req_max_score)?;
        match has_tpi {
            true => Ok(Self {
                disi: DocIdSetIteratorEnum2::B(TwoPhaseIteratorAsDocIdSetIterator::new(
                    TwoPhaseIteratorImpl::new(approximation),
                )),
                tpi_state: TwoPhaseState::Yes,
            }),
            false => Ok(Self {
                disi: DocIdSetIteratorEnum2::A(approximation),
                tpi_state: TwoPhaseState::No,
            }),
        }
    }
    #[cfg(test)]
    pub(crate) fn with_fixed_max_score(
        req_scorer: S1,
        opt_scorer: S2,
        score_mode: ScoreMode,
    ) -> Result<Self> {
        let mut v = Self::new(req_scorer, opt_scorer, score_mode)?;
        match v.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => disi.fixed_max_score = true,
            DocIdSetIteratorEnum2::B(ref mut wrapper) => {
                wrapper.two_phase_iterator.disi.fixed_max_score = true
            },
        }
        Ok(v)
    }
}

impl<S1, S2> Scorable for ReqOptSumScorer<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    fn score(&mut self) -> Result<f32> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => disi.score(),
            DocIdSetIteratorEnum2::B(ref mut wrapper) => wrapper.two_phase_iterator.disi.score(),
        }
    }

    fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => disi.set_min_competitive_score(min_score),
            DocIdSetIteratorEnum2::B(ref mut wrapper) => wrapper
                .two_phase_iterator
                .disi
                .set_min_competitive_score(min_score),
        }
    }
}

impl<S1, S2> Scorer for ReqOptSumScorer<S1, S2>
where
    S1: Scorer + 'static,
    S2: Scorer + 'static,
{
    fn doc_id(&mut self) -> Result<i32> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => disi.req_scorer.doc_id(),
            DocIdSetIteratorEnum2::B(ref mut wrapper) => {
                wrapper.two_phase_iterator.disi.req_scorer.doc_id()
            },
        }
    }

    fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&self.disi)
    }

    fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&mut self.disi)
    }

    fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
        let ReqOptSumScorer { disi, .. } = *self;
        Box::new(disi)
    }

    fn two_phase_iterator(&self) -> Result<Option<Box<dyn TwoPhaseIterator + '_>>> {
        match self.tpi_state {
            TwoPhaseState::No => Ok(None),
            _ => match &self.disi {
                DocIdSetIteratorEnum2::A(_) => Err(LuceneError::illegal_state(
                    "No two-phase iterator available",
                )),
                DocIdSetIteratorEnum2::B(wrapper) => {
                    Ok(Some(Box::new(&wrapper.two_phase_iterator)))
                },
            },
        }
    }

    fn two_phase_iterator_mut(&mut self) -> Result<Option<Box<dyn TwoPhaseIterator + '_>>> {
        match self.tpi_state {
            TwoPhaseState::No => Ok(None),
            _ => match &mut self.disi {
                DocIdSetIteratorEnum2::A(_) => Err(LuceneError::illegal_state(
                    "No two-phase iterator available",
                )),
                DocIdSetIteratorEnum2::B(wrapper) => {
                    Ok(Some(Box::new(&mut wrapper.two_phase_iterator)))
                },
            },
        }
    }

    fn take_two_phase_iterator(self: Box<Self>) -> Result<Option<Box<dyn TwoPhaseIterator>>>
    where
        Self: Sized,
    {
        let ReqOptSumScorer {
            disi, tpi_state, ..
        } = *self;
        match tpi_state {
            TwoPhaseState::No => Ok(None),
            _ => match disi {
                DocIdSetIteratorEnum2::A(_) => Err(LuceneError::illegal_state(
                    "No two-phase iterator available",
                )),
                DocIdSetIteratorEnum2::B(wrapper) => Ok(Some(Box::new(wrapper.two_phase_iterator))),
            },
        }
    }

    fn advance_shallow(&mut self, target: i32) -> Result<i32> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => disi.advance_shallow(target),
            DocIdSetIteratorEnum2::B(ref mut wrapper) => {
                wrapper.two_phase_iterator.disi.advance_shallow(target)
            },
        }
    }

    fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
        match self.disi {
            DocIdSetIteratorEnum2::A(ref mut disi) => disi.get_max_score(up_to),
            DocIdSetIteratorEnum2::B(ref mut wrapper) => {
                wrapper.two_phase_iterator.disi.get_max_score(up_to)
            },
        }
    }

    fn has_two_phase_iterator(&self) -> TwoPhaseState {
        self.tpi_state
    }
}

pub struct DocIdSetIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    upto: i32,
    max_score: f32,
    opt_is_required: bool,
    min_score: f32,
    req_scorer: S1,
    opt_scorer: S2,
    req_max_score: f32,
    #[cfg(test)]
    fixed_max_score: bool,
}
impl<S1, S2> DocIdSetIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    fn new(req_scorer: S1, opt_scorer: S2, req_max_score: f32) -> Result<Self> {
        let disi = Self {
            upto: -1,
            max_score: 0.0,
            opt_is_required: false,
            min_score: 0.0,
            req_scorer,
            opt_scorer,
            req_max_score,
            #[cfg(test)]
            fixed_max_score: false,
        };
        Ok(disi)
    }

    fn move_to_next_block(&mut self, target: i32) -> Result<()> {
        self.upto = self.advance_shallow(target)?;
        let req_max_score_block = self.req_scorer.get_max_score(self.upto)?;
        self.max_score = self.get_max_score(self.upto)?;
        self.opt_is_required = req_max_score_block < self.min_score;
        Ok(())
    }
    fn advance_shallow(&mut self, target: i32) -> Result<i32> {
        let mut up_to = self.req_scorer.advance_shallow(target)?;

        let opt_doc = {
            let it = self.opt_scorer.iterator();
            it.doc_id()
        };

        if opt_doc <= target {
            let v = self.opt_scorer.advance_shallow(target)?;
            up_to = up_to.min(v);
        } else if opt_doc != NO_MORE_DOCS {
            up_to = up_to.min(opt_doc - 1);
        }

        Ok(up_to)
    }
    fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
        #[cfg(test)]
        {
            if self.fixed_max_score {
                return Ok(f32::INFINITY);
            }
        }
        let mut max_score = self.req_scorer.get_max_score(up_to)?;

        let opt_doc = {
            let it = self.opt_scorer.iterator();
            it.doc_id()
        };

        if opt_doc <= up_to {
            max_score += self.opt_scorer.get_max_score(up_to)?;
        }

        Ok(max_score)
    }
    fn advance_impacts(&mut self, mut target: i32) -> Result<i32> {
        if target > self.upto {
            self.move_to_next_block(target)?;
        }

        loop {
            if self.max_score >= self.min_score {
                return Ok(target);
            }

            if self.upto == NO_MORE_DOCS {
                return Ok(NO_MORE_DOCS);
            }

            target = self.upto + 1;

            self.move_to_next_block(target)?;
        }
    }
    fn advance_internal(&mut self, target: i32) -> Result<i32> {
        if target == NO_MORE_DOCS {
            self.req_scorer.iterator_mut().advance(target)?;
            return Ok(NO_MORE_DOCS);
        }

        let mut req_doc = target;

        'advance_head: loop {
            if self.min_score != 0.0 {
                req_doc = self.advance_impacts(req_doc)?;
            }

            {
                let mut req_it = self.req_scorer.iterator_mut();
                if req_it.doc_id() < req_doc {
                    req_doc = req_it.advance(req_doc)?;
                }
            }

            if req_doc == NO_MORE_DOCS || !self.opt_is_required {
                return Ok(req_doc);
            }

            let upper_bound = if self.req_max_score < self.min_score {
                NO_MORE_DOCS
            } else {
                self.upto
            };

            if req_doc > upper_bound {
                continue;
            }
            // Find the next common doc within the current block
            let mut opt_it = self.opt_scorer.iterator_mut();
            let mut req_it = self.req_scorer.iterator_mut();
            loop {
                let mut opt_doc = opt_it.doc_id();

                if opt_doc < req_doc {
                    opt_doc = opt_it.advance(req_doc)?;
                }

                if opt_doc > upper_bound {
                    req_doc = upper_bound + 1;
                    continue 'advance_head;
                }

                if opt_doc != req_doc {
                    req_doc = req_it.advance(opt_doc)?;
                    if req_doc > upper_bound {
                        continue 'advance_head;
                    }
                }

                if req_doc == NO_MORE_DOCS || opt_doc == req_doc {
                    return Ok(req_doc);
                }
            }
        }
    }
    fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
        self.min_score = min_score;
        // Potentially move to a conjunction
        if self.req_max_score < self.min_score {
            self.opt_is_required = true;
            if self.req_max_score == 0.0 {
                // If the required clause doesn't contribute scores, we can propagate the minimum
                // competitive score to the optional clause. This happens when the required clause is a
                // FILTER clause.
                // In theory we could generalize this and set minScore - reqMaxScore as a minimum
                // competitive score, but it's unlikely to help in practice unless reqMaxScore is much
                // smaller than typical scores of the optional clause.
                self.opt_scorer.set_min_competitive_score(self.min_score)?;
            }
        }
        Ok(())
    }
    fn score(&mut self) -> Result<f32> {
        let (mut opt_doc, cur_doc, mut score) = {
            let cur_doc = {
                let req_it = self.req_scorer.iterator();
                req_it.doc_id()
            };
            let score = self.req_scorer.score()?;

            let opt_it = self.opt_scorer.iterator();
            let opt_doc = opt_it.doc_id();
            (opt_doc, cur_doc, score)
        };

        if opt_doc < cur_doc {
            opt_doc = self.opt_scorer.iterator_mut().advance(cur_doc)?;

            let should_skip = {
                if let Some(mut opt_tpi) = self.opt_scorer.two_phase_iterator()? {
                    opt_doc == cur_doc && !opt_tpi.matches()?
                } else {
                    false
                }
            };
            if should_skip {
                opt_doc = self.opt_scorer.iterator_mut().next_doc()?;
            }
        }

        if opt_doc == cur_doc {
            score += self.opt_scorer.score()?;
        }

        Ok(score)
    }
}
impl<S1, S2> DocIdSetIterator for DocIdSetIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    fn doc_id(&self) -> i32 {
        self.req_scorer.iterator().doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let next = self.req_scorer.iterator().doc_id() + 1;
        self.advance_internal(next)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.advance_internal(target)
    }

    fn cost(&self) -> Result<i64> {
        self.req_scorer.iterator().cost()
    }
}

pub struct TwoPhaseIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    disi: DocIdSetIteratorImpl<S1, S2>,
}
impl<S1, S2> TwoPhaseIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    fn new(disi: DocIdSetIteratorImpl<S1, S2>) -> Self {
        Self { disi }
    }
}
impl<S1, S2> TwoPhaseIterator for TwoPhaseIteratorImpl<S1, S2>
where
    S1: Scorer,
    S2: Scorer,
{
    fn approximation_mut(&mut self) -> Result<Box<dyn DocIdSetIterator + '_>> {
        Ok(Box::new(&mut self.disi))
    }

    fn approximation(&self) -> Result<Box<dyn DocIdSetIterator + '_>> {
        Ok(Box::new(&self.disi))
    }

    fn matches(&mut self) -> Result<bool> {
        if let Some(mut req_tpi) = self.disi.req_scorer.two_phase_iterator()?
            && !req_tpi.matches()?
        {
            return Ok(false);
        }

        // optional scorer logic
        if let Some(mut opt_tpi) = self.disi.opt_scorer.two_phase_iterator()? {
            // The below condition is rare and can only happen if we transitioned to
            // optIsRequired=true
            // after the opt approximation was advanced and before it was confirmed.
            let (opt_doc, req_doc) = {
                let opt_disi = opt_tpi.approximation_mut()?;
                let req_it = self.disi.req_scorer.iterator();
                (opt_disi.doc_id(), req_it.doc_id())
            };

            if self.disi.opt_is_required {
                if req_doc != opt_doc {
                    let mut d = opt_doc;
                    if d < req_doc {
                        let mut opt_disi = opt_tpi.approximation_mut()?;
                        d = opt_disi.advance(req_doc)?;
                    }
                    if d != req_doc {
                        return Ok(false);
                    }
                }

                if !opt_tpi.matches()? {
                    let mut opt_disi = opt_tpi.approximation_mut()?;
                    opt_disi.next_doc()?;
                    return Ok(false);
                }
            } else if opt_doc == req_doc && !opt_tpi.matches()? {
                let mut opt_disi = opt_tpi.approximation_mut()?;
                // Advance the iterator to make it clear it doesn't match the current doc id
                opt_disi.next_doc()?;
            }
        }

        Ok(true)
    }

    fn match_cost(&self) -> f32 {
        let mut cost = 1.0;

        if let Ok(Some(req_tpi)) = self.disi.req_scorer.two_phase_iterator() {
            cost += req_tpi.match_cost();
        }

        if let Ok(Some(opt_tpi)) = self.disi.opt_scorer.two_phase_iterator() {
            cost += opt_tpi.match_cost();
        }

        cost
    }
}
#[cfg(test)]
mod tests {
    use crate::core::document::document::Document;
    use crate::core::document::field::Store;
    use crate::core::document::float_point::FloatPoint;
    use crate::core::document::string_field::StringField;
    use crate::core::index::directory_reader::directory_reader_util;
    use crate::core::index::index_reader_context::IndexReaderContext;
    use crate::core::index::index_writer::IndexWriter;
    use crate::core::index::term::Term;
    use crate::core::search::boolean_clause::Occur;
    use crate::core::search::boolean_query::Builder;
    use crate::core::search::constant_score_query::ConstantScoreQuery;
    use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::core::search::index_searcher::IndexSearcher;
    use crate::core::search::query::{Query, QueryWeightSsScorer};
    use crate::core::search::req_opt_sum_scorer::ReqOptSumScorer;
    use crate::core::search::scorable::Scorable;
    use crate::core::search::score_mode::ScoreMode;
    use crate::core::search::scorer::{Scorer, TwoPhaseState};
    use crate::core::search::term_query::TermQuery;
    use crate::core::search::two_phase_iterator::TwoPhaseIterator;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        new_directory_shared, new_index_writer_config, new_searcher_with_reader, random,
    };
    use rand::Rng;

    #[allow(dead_code)]
    struct TestReqOptSumScorer;
    #[test]
    fn test_basics_must() -> Result<()> {
        let mut random = random();
        do_test_basics(&mut random, Occur::Must)
    }

    #[test]
    fn test_basics_filter() -> Result<()> {
        let mut random = random();
        do_test_basics(&mut random, Occur::Filter)
    }

    fn do_test_basics<R: Rng + ?Sized>(random: &mut R, req_occur: Occur) -> Result<()> {
        let dir = new_directory_shared(random)?;

        // TODO RandomIndexWriter / newLogMergePolicy 未实现：用当前 IndexWriter 路径代替
        let w = IndexWriter::new(dir.clone(), new_index_writer_config(random))?;

        {
            let mut doc = Document::new();
            doc.add(StringField::from_string("f", "foo".to_string(), Store::No)?);
            w.add_document(doc)?;
        }
        {
            let mut doc = Document::new();
            doc.add(StringField::from_string("f", "foo".to_string(), Store::No)?);
            doc.add(StringField::from_string("f", "bar".to_string(), Store::No)?);
            w.add_document(doc)?;
        }
        {
            let mut doc = Document::new();
            doc.add(StringField::from_string("f", "foo".to_string(), Store::No)?);
            w.add_document(doc)?;
        }
        {
            let mut doc = Document::new();
            doc.add(StringField::from_string("f", "bar".to_string(), Store::No)?);
            w.add_document(doc)?;
        }
        {
            let mut doc = Document::new();
            doc.add(StringField::from_string("f", "foo".to_string(), Store::No)?);
            doc.add(StringField::from_string("f", "bar".to_string(), Store::No)?);
            w.add_document(doc)?;
        }

        // TODO force_merge未实现
        // w.force_merge(1)?;

        let reader = directory_reader_util::open_with_writer(&w)?;
        w.close()?;

        let searcher = new_searcher_with_reader(reader)?;
        let query: Query = {
            let mut b = Builder::new();
            b.add(
                ConstantScoreQuery::new(Box::new(
                    TermQuery::new(Term::from_text("f", "foo")).into(),
                )),
                req_occur,
            )?
            .add(
                ConstantScoreQuery::new(Box::new(
                    TermQuery::new(Term::from_text("f", "bar")).into(),
                )),
                Occur::Should,
            )?;
            b.build().into()
        };

        let weight =
            searcher.create_weight(searcher.rewrite(query)?, ScoreMode::TopScores, 1.0, None)?;
        let context = &searcher.get_leaf_contexts()?[0];

        let mut scorer = weight.scorer(context)?.expect("expected scorer");
        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        assert_eq!(1, scorer.iterator_mut().next_doc()?);
        assert_eq!(2, scorer.iterator_mut().next_doc()?);
        assert_eq!(4, scorer.iterator_mut().next_doc()?);
        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;
        scorer.set_min_competitive_score(FloatPoint::next_down(1.0))?;

        if req_occur == Occur::Must {
            assert_eq!(0, scorer.iterator_mut().next_doc()?);
        }
        assert_eq!(1, scorer.iterator_mut().next_doc()?);
        if req_occur == Occur::Must {
            assert_eq!(2, scorer.iterator_mut().next_doc()?);
        }
        assert_eq!(4, scorer.iterator_mut().next_doc()?);
        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;
        scorer.set_min_competitive_score(FloatPoint::next_up(1.0))?;

        if req_occur == Occur::Must {
            assert_eq!(1, scorer.iterator_mut().next_doc()?);
            assert_eq!(4, scorer.iterator_mut().next_doc()?);
        }
        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;

        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        scorer.set_min_competitive_score(FloatPoint::next_up(1.0))?;
        if req_occur == Occur::Must {
            assert_eq!(1, scorer.iterator_mut().next_doc()?);
            assert_eq!(4, scorer.iterator_mut().next_doc()?);
        }
        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        Ok(())
    }
    #[test]
    fn test_max_block() -> Result<()> {
        // TODO TermFreqTokenStream未实现
        Ok(())
    }
    #[test]
    fn test_max_score_segment() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let conf = new_index_writer_config(&mut random);
        let w = IndexWriter::new(dir.clone(), conf)?;

        let docs: &[&[&str]] = &[
            &["A"],      // 0
            &["A"],      // 1
            &[],         // 2
            &["A", "B"], // 3
            &["A"],      // 4
            &["B"],      // 5
            &["A", "B"], // 6
            &["B"],      // 7
        ];

        for values in docs {
            let mut doc = Document::new();
            for v in *values {
                doc.add(StringField::from_string(
                    "foo",
                    (*v).to_string(),
                    Store::No,
                )?);
            }
            w.add_document(doc)?;
        }

        // TODO force_merge
        // w.force_merge(1)?;
        w.close()?;

        let reader = directory_reader_util::open(dir)?;
        let searcher = new_searcher_with_reader(reader)?;
        let _ctx = &searcher.get_leaf_contexts()?[0];

        let req_q =
            ConstantScoreQuery::new(Box::new(TermQuery::new(Term::from_text("foo", "A")).into()));
        let opt_q =
            ConstantScoreQuery::new(Box::new(TermQuery::new(Term::from_text("foo", "B")).into()));

        let mut scorer = req_opt_scorer(&searcher, req_q.clone(), opt_q.clone(), false)?;

        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        assert_eq!(1.0, scorer.score()?);
        assert_eq!(1, scorer.iterator_mut().next_doc()?);
        assert_eq!(1.0, scorer.score()?);
        assert_eq!(3, scorer.iterator_mut().next_doc()?);
        assert_eq!(2.0, scorer.score()?);
        assert_eq!(4, scorer.iterator_mut().next_doc()?);
        assert_eq!(1.0, scorer.score()?);
        assert_eq!(6, scorer.iterator_mut().next_doc()?);
        assert_eq!(2.0, scorer.score()?);
        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        let mut scorer = req_opt_scorer(&searcher, req_q.clone(), opt_q.clone(), false)?;
        scorer.set_min_competitive_score(f32::from_bits(1.0f32.to_bits() - 1))?;
        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        assert_eq!(1.0, scorer.score()?);
        assert_eq!(1, scorer.iterator_mut().next_doc()?);
        assert_eq!(1.0, scorer.score()?);
        assert_eq!(3, scorer.iterator_mut().next_doc()?);
        assert_eq!(2.0, scorer.score()?);
        assert_eq!(4, scorer.iterator_mut().next_doc()?);
        assert_eq!(1.0, scorer.score()?);
        assert_eq!(6, scorer.iterator_mut().next_doc()?);
        assert_eq!(2.0, scorer.score()?);
        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        let mut scorer = req_opt_scorer(&searcher, req_q.clone(), opt_q.clone(), false)?;
        scorer.set_min_competitive_score(f32::from_bits(1.0f32.to_bits() + 1))?;
        assert_eq!(3, scorer.iterator_mut().next_doc()?);
        assert_eq!(2.0, scorer.score()?);
        assert_eq!(6, scorer.iterator_mut().next_doc()?);
        assert_eq!(2.0, scorer.score()?);
        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        let mut scorer = req_opt_scorer(&searcher, req_q, opt_q, true)?;
        scorer.set_min_competitive_score(f32::from_bits(2.0f32.to_bits() + 1))?;
        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        Ok(())
    }
    #[test]
    fn test_must_random_frequent_opt() -> Result<()> {
        let mut random = random();
        do_test_random(&mut random, Occur::Must, 0.5)
    }

    #[test]
    fn test_must_random_rare_opt() -> Result<()> {
        let mut random = random();
        do_test_random(&mut random, Occur::Must, 0.05)
    }

    #[test]
    fn test_filter_random_frequent_opt() -> Result<()> {
        let mut random = random();
        do_test_random(&mut random, Occur::Filter, 0.5)
    }

    #[test]
    fn test_filter_random_rare_opt() -> Result<()> {
        let mut random = random();
        do_test_random(&mut random, Occur::Filter, 0.05)
    }

    fn do_test_random<R>(_random: &mut R, _req_occur: Occur, _opt_freq: f64) -> Result<()>
    where
        R: Rng + ?Sized,
    {
        // TODO RandomApproximationQuery未实现
        Ok(())
    }

    fn req_opt_scorer<IRC, Q>(
        searcher: &IndexSearcher<IRC>,
        req_q: Q,
        opt_q: Q,
        with_block_score: bool,
    ) -> Result<ReqOptSumScorer<QueryWeightSsScorer, QueryWeightSsScorer>>
    where
        Q: Into<Query>,
        IRC: IndexReaderContext,
        <IRC as IndexReaderContext>::LeafReader: 'static,
    {
        let req_q = req_q.into();
        let opt_q = opt_q.into();
        let ctx = &searcher.get_leaf_contexts()?[0];

        let req_scorer = searcher
            .create_weight(req_q, ScoreMode::TopScores, 1.0, None)?
            .scorer(ctx)?
            .expect("required scorer");

        let opt_scorer = searcher
            .create_weight(opt_q, ScoreMode::TopScores, 1.0, None)?
            .scorer(ctx)?
            .expect("optional scorer");
        let v = match with_block_score {
            true => ReqOptSumScorer::new(req_scorer, opt_scorer, ScoreMode::TopScores)?,
            false => {
                ReqOptSumScorer::with_fixed_max_score(req_scorer, opt_scorer, ScoreMode::TopScores)?
            },
        };
        Ok(v)
    }

    struct ReqOptSumScorerWrapper<S1, S2>
    where
        S1: Scorer,
        S2: Scorer,
    {
        base: ReqOptSumScorer<S1, S2>,
    }
    impl<S1, S2> ReqOptSumScorerWrapper<S1, S2>
    where
        S1: Scorer,
        S2: Scorer,
    {
        fn new(base: ReqOptSumScorer<S1, S2>) -> Self {
            Self { base }
        }
    }

    impl<S1, S2> Scorable for ReqOptSumScorerWrapper<S1, S2>
    where
        S1: Scorer,
        S2: Scorer,
    {
        fn score(&mut self) -> Result<f32> {
            self.base.score()
        }
    }

    impl<S1, S2> Scorer for ReqOptSumScorerWrapper<S1, S2>
    where
        S1: Scorer + 'static,
        S2: Scorer + 'static,
    {
        fn doc_id(&mut self) -> Result<i32> {
            self.base.doc_id()
        }

        fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
            self.base.iterator()
        }

        fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
            self.base.iterator_mut()
        }

        fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
            let ReqOptSumScorerWrapper { base } = *self;
            Box::new(base).take_iterator()
        }

        fn two_phase_iterator(&self) -> Result<Option<Box<dyn TwoPhaseIterator + '_>>> {
            self.base.two_phase_iterator()
        }

        fn two_phase_iterator_mut(&mut self) -> Result<Option<Box<dyn TwoPhaseIterator + '_>>> {
            self.base.two_phase_iterator_mut()
        }

        fn take_two_phase_iterator(self: Box<Self>) -> Result<Option<Box<dyn TwoPhaseIterator>>>
        where
            Self: Sized,
        {
            let ReqOptSumScorerWrapper { base } = *self;
            Box::new(base).take_two_phase_iterator()
        }

        fn advance_shallow(&mut self, target: i32) -> Result<i32> {
            self.base.advance_shallow(target)
        }

        fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
            Ok(f32::MAX)
        }

        fn default_cost(&mut self) -> Result<i64> {
            self.base.default_cost()
        }

        fn has_two_phase_iterator(&self) -> TwoPhaseState {
            self.base.has_two_phase_iterator()
        }
    }
}
