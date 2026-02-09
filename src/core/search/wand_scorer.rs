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
use crate::core::search::disi_priority_queue::DisiPriorityQueue;
use crate::core::search::disi_wrapper::DisiWrapper;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::scorer_util::ScorerUtil;
use crate::core::search::two_phase_iterator::{
    TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator,
};
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::math_util::MathUtil;

/// Implements the WAND (Weak AND) algorithm for dynamic pruning as described in
/// “Efficient Query Evaluation using a Two-Level Retrieval Process”
/// by Broder, Carmel, Herscovici, Soffer and Zien.
///
/// It is enhanced with techniques from
/// “Faster Top-k Document Retrieval Using Block-Max Indexes” by Ding and Suel.
///
/// For [`ScoreMode::TopScores`], this scorer maintains a feedback loop with the
/// collector in order to know, at any time, the minimum score that is required
/// for a hit to be competitive.
///
///
/// The implementation supports both:
/// - `minCompetitiveScore` by enforcing `∑ max_score >= minCompetitiveScore`
/// - `minShouldMatch` by enforcing `freq >= minShouldMatch`
///
/// It keeps sub-scorers in three different places:
/// - **tail**: a heap that contains scorers that are *behind* the desired doc ID,
///   ordered by cost so that the least costly ones can be advanced first.
/// - **lead**: a linked list of scorers that are positioned on the desired doc ID
/// - **head**: a heap that contains scorers which are *beyond* the desired doc ID,
///   ordered by doc ID in order to move quickly to the next candidate.
///
///
/// When `score_mode == ScoreMode::TopScores`, it leverages
/// [`Scorer::get_max_score`] from each scorer in order to know when it may call
/// [`DocIdSetIterator::advance`] rather than [`DocIdSetIterator::next_doc`] to move
/// to the next competitive hit.
///
/// When `score_mode != ScoreMode::TopScores`, block-max scoring related logic is skipped.
///
/// Finding the next match consists of:
/// 1. Setting the desired doc ID to the least entry in `head`.
/// 2. Advancing `tail` until there is a match, by meeting the configured
///    constraints:
///    - `freq >= minShouldMatch`
///    - and/or `∑ max_score >= minCompetitiveScore`.
pub struct WANDScorer<S>
where
    S: Scorer,
{
    disi: TwoPhaseIteratorAsDocIdSetIterator<TwoPhaseIteratorImpl<S>>,
}

impl<S> WANDScorer<S>
where
    S: Scorer,
{
    pub(crate) fn new(
        scorers: Vec<S>,
        min_should_match: i32,
        score_mode: ScoreMode,
        lead_cost: i64,
    ) -> Result<WANDScorer<S>> {
        let v = DocIdSetIteratorImpl::new(scorers, min_should_match, score_mode, lead_cost)?;
        let tpi = TwoPhaseIteratorImpl::new(v);
        Ok(WANDScorer {
            disi: TwoPhaseIteratorAsDocIdSetIterator::new(tpi),
        })
    }
}

impl<S> Scorable for WANDScorer<S>
where
    S: Scorer,
{
    fn score(&mut self) -> Result<f32> {
        let disi = &mut self.disi.two_phase_iterator.approximation;
        // we need to know about all matches
        disi.advance_all_tail()?;
        let mut lead_score = disi.lead_score;

        if disi.score_mode != ScoreMode::TopScores {
            // With TOP_SCORES, the score was already computed on the fly.
            let mut cur = disi.lead;
            while let Some(idx) = cur {
                lead_score += disi.all_scorers[idx].scorer.score()? as f64;
                cur = disi.all_scorers[idx].next;
            }
        }

        Ok(lead_score as f32)
    }

    fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
        // Let this disjunction know about the new min score so that it can skip
        // over clauses that produce low scores.
        let disi = &mut self.disi.two_phase_iterator.approximation;
        debug_assert_eq!(
            disi.score_mode,
            ScoreMode::TopScores,
            "minCompetitiveScore can only be set for ScoreMode.TOP_SCORES, but got: {:?}",
            disi.score_mode
        );
        debug_assert!(min_score >= 0f32);
        let scaled_min_score = scale_min_score(min_score, disi.scaling_factor);
        debug_assert!(scaled_min_score >= disi.min_competitive_score);
        disi.min_competitive_score = scaled_min_score;
        Ok(())
    }
}

impl<S> Scorer for WANDScorer<S>
where
    S: Scorer + 'static,
{
    fn doc_id(&mut self) -> Result<i32> {
        Ok(self.disi.two_phase_iterator.approximation.doc)
    }

    fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&self.disi)
    }

    fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&mut self.disi)
    }

    fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
        let WANDScorer { disi, .. } = *self;
        Box::new(disi)
    }

    fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
        Some(Box::new(&self.disi.two_phase_iterator))
    }

    fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
        Some(Box::new(&mut self.disi.two_phase_iterator))
    }

    fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>>
    where
        Self: Sized,
    {
        let WANDScorer { disi, .. } = *self;
        Some(Box::new(disi.two_phase_iterator))
    }

    fn get_max_score(&mut self, upto: i32) -> Result<f32> {
        let all_scorers = self
            .disi
            .two_phase_iterator
            .approximation
            .all_scorers
            .as_mut_slice();

        let mut max_score_sum = 0f64;
        let len = all_scorers.len();
        for w in all_scorers {
            let scorer = &mut w.scorer;

            if scorer.doc_id()? <= upto {
                max_score_sum += scorer.get_max_score(upto)? as f64;
            }
        }
        Ok(MathUtil::sum_upper_bound(max_score_sum, len.try_convert()?) as f32)
    }

    fn has_two_phase_iterator(&self) -> TwoPhaseState {
        TwoPhaseState::Yes
    }
}

pub struct DocIdSetIteratorImpl<S>
where
    S: Scorer,
{
    all_scorers: Vec<DisiWrapper<S>>,
    doc: i32,
    score_mode: ScoreMode,
    upto: i32,
    /// priority queue of scorers that are too advanced compared to the current
    /// doc. Ordered by doc ID.
    pub(crate) head: DisiPriorityQueue,
    /// list of scorers which 'lead' the iteration and are currently
    /// positioned on 'doc'. This is sometimes called the 'pivot' in
    /// some descriptions of WAND (Weak AND).
    pub(crate) lead: Option<usize>,
    pub(crate) freq: i32,
    pub(crate) lead_cost: i64,
    /// score of the leads
    pub(crate) lead_score: f64,
    /// priority queue of scorers which are behind the current doc.
    /// Ordered by maxScore.
    pub(crate) tail: Vec<usize>,
    /// sum of max scores of scorers in tail
    pub(crate) tail_max_score: i64,
    pub(crate) tail_size: i32,
    /// scaled min competitive score
    min_competitive_score: i64,
    pub(crate) min_should_match: i32,
    /// cost from Lucene
    pub(crate) cost: i64,
    /// scalingFactor in Lucene
    scaling_factor: i32,
}
impl<S> DocIdSetIteratorImpl<S>
where
    S: Scorer,
{
    fn new(
        mut scorers: Vec<S>,
        min_should_match: i32,
        score_mode: ScoreMode,
        lead_cost: i64,
    ) -> Result<DocIdSetIteratorImpl<S>> {
        let num_scorers = scorers.len();

        if min_should_match as usize >= num_scorers {
            return Err(LuceneError::illegal_argument(
                "minShouldMatch should be < the number of scorers",
            ));
        }

        debug_assert!(
            min_should_match >= 0,
            "minShouldMatch should not be negative, but got {}",
            min_should_match
        );

        let scaling_factor = if score_mode == ScoreMode::TopScores {
            // To avoid accuracy issues with floating-point numbers, this scorer operates on scaled longs.
            // How do you choose the scaling factor? The thing is that we want to retain as many
            // significant bits as possible, but not too many, otherwise operations on longs would be more
            // precise than the equivalent operations on their unscaled counterparts and we might skip too
            // many hits. So we compute the maximum possible score produced by this scorer, which is the
            // sum of the maximum scores of each clause, and compute a scaling factor that would preserve
            // 24 bits of accuracy - the number of mantissa bits of single-precision floating-point
            // numbers.
            let mut max_score_sum_double = 0f64;
            for scorer in scorers.iter_mut() {
                scorer.advance_shallow(0)?;
                let max_score = scorer.get_max_score(NO_MORE_DOCS)?;
                max_score_sum_double += max_score as f64;
            }
            let max_score_sum = MathUtil::sum_upper_bound(max_score_sum_double, num_scorers as i32);
            scaling_factor(max_score_sum as f32)?
        } else {
            0
        };

        let mut all_scorers = Vec::with_capacity(num_scorers);
        let mut cost = Vec::with_capacity(num_scorers);
        for scorer in scorers {
            cost.push(scorer.iterator().cost()?);
            all_scorers.push(DisiWrapper::new(scorer)?);
        }

        let head = DisiPriorityQueue::new(num_scorers as i32);
        let tail = vec![0usize; num_scorers];

        let mut this = Self {
            all_scorers,
            doc: -1,
            score_mode,
            upto: -1,
            head,
            lead: None,
            freq: 0,
            lead_cost,
            lead_score: 0.0,
            tail,
            tail_max_score: 0,
            tail_size: 0,
            min_competitive_score: 0,
            min_should_match,
            cost: 0,
            scaling_factor,
        };

        for idx in 0..num_scorers {
            // Ideally we would pass true when scoreMode == TOP_SCORES and false otherwise, but this would
            // break the optimization as there could then be 3 different impls of DocIdSetIterator
            // (ImpactsEnum, PostingsEnum and <Else>). So we pass true to favor disjunctions sorted by
            // descending score as opposed to non-scoring disjunctions whose minShouldMatch is greater
            // than 1.
            this.add_unpositioned_lead(idx);
        }

        this.cost = ScorerUtil::cost_with_min_should_match(
            cost,
            num_scorers,
            min_should_match.try_convert()?,
        )?;

        Ok(this)
    }
    /// Add a disi to the linked list of leads.
    fn add_lead(&mut self, lead_idx: usize) -> Result<()> {
        self.all_scorers[lead_idx].next = self.lead;
        self.lead = Some(lead_idx);
        self.freq += 1;
        if self.score_mode == ScoreMode::TopScores {
            let scorer = self.all_scorers[lead_idx].scorer.score()?;
            self.lead_score += scorer as f64;
        }
        Ok(())
    }
    /// Add a disi to the linked list of leads.
    fn add_unpositioned_lead(&mut self, lead_idx: usize) {
        debug_assert!(self.all_scorers[lead_idx].doc == -1);
        self.all_scorers[lead_idx].next = self.lead;
        self.lead = Some(lead_idx);
        self.freq += 1;
    }
    /// Make sure all disis in 'head' are on or after 'target'.
    fn push_back_leads(&mut self, target: i32) -> Result<()> {
        let mut cur = self.lead;
        while let Some(idx) = cur {
            let evicted = self.insert_tail_with_overflow(idx);

            if let Some(evicted_idx) = evicted {
                let new_doc = self.all_scorers[evicted_idx]
                    .scorer
                    .iterator_mut()
                    .advance(target)?;
                self.all_scorers[evicted_idx].doc = new_doc;
                self.head.add(evicted_idx, &self.all_scorers);
            }
            cur = self.all_scorers[idx].next;
        }
        self.lead = None;
        Ok(())
    }

    /// Make sure all disis in 'head' are on or after 'target'.
    fn advance_head(&mut self, target: i32) -> Result<Option<usize>> {
        let mut head_top = self.head.top();

        while let Some(ref top_idx) = head_top {
            let top_idx = *top_idx;
            if self.all_scorers[top_idx].doc >= target {
                break;
            }
            let evicted = self.insert_tail_with_overflow(top_idx);

            if let Some(evicted_idx) = evicted {
                let new_doc = self.all_scorers[evicted_idx]
                    .scorer
                    .iterator_mut()
                    .advance(target)?;
                self.all_scorers[evicted_idx].doc = new_doc;
                head_top = Some(self.head.update_top_with(evicted_idx, &self.all_scorers));
            } else {
                self.head.pop(&self.all_scorers);
                head_top = self.head.top();
            }
        }

        Ok(head_top)
    }

    fn advance_tail(&mut self, idx: usize) -> Result<()> {
        let new_doc = self.all_scorers[idx]
            .scorer
            .iterator_mut()
            .advance(self.doc)?;
        self.all_scorers[idx].doc = new_doc;

        if new_doc == self.doc {
            self.add_lead(idx)?;
        } else {
            self.head.add(idx, &self.all_scorers);
        }

        Ok(())
    }
    /// Pop the entry from the 'tail' that has the greatest score contribution,
    /// advance it to the current doc and then add it to 'lead' or 'head' depending on whether it matches.
    fn advance_tail_top(&mut self) -> Result<()> {
        let top = self.pop_tail();
        self.advance_tail(top)
    }
    fn update_max_scores(&mut self, target: i32) -> Result<()> {
        let mut new_upto = NO_MORE_DOCS;
        // If we have entries in 'head', we treat them all as leads and take the minimum of their next
        // block boundaries as a next boundary.
        // We don't take entries in 'tail' into account on purpose: 'tail' is supposed to contain the
        // least score contributors, and taking them into account might not move the boundary fast
        // enough, so we'll waste CPU re-computing the next boundary all the time.
        // Likewise, we ignore clauses whose cost is greater than the lead cost to avoid recomputing
        // per-window max scores over and over again. In the event when this makes us compute upTo as
        // NO_MORE_DOCS, this scorer will effectively implement WAND rather than block-max WAND.
        {
            let iter = self.head.iter();
            for idx in iter {
                let w = &mut self.all_scorers[idx];

                if w.doc <= new_upto && w.cost <= self.lead_cost {
                    let shallow = w.scorer.advance_shallow(w.doc)?;
                    new_upto = new_upto.min(shallow);
                }
            }
        }
        // Only look at the tail if none of the `head` clauses had a block we could reuse and if its
        // cost is less than or equal to the lead cost.
        if new_upto == NO_MORE_DOCS
            && self.tail_size > 0
            && self.all_scorers[self.tail[0]].cost <= self.lead_cost
        {
            let top_idx = self.tail[0];

            new_upto = self.all_scorers[top_idx].scorer.advance_shallow(target)?;
            // upTo must be on or after the least `head` doc
            if let Some(ht) = self.head.top() {
                new_upto = new_upto.max(self.all_scorers[ht].doc);
            }
        }

        self.upto = new_upto;

        {
            // Now update the max scores of clauses that are before upTo.
            let iter = self.head.iter();
            for idx in iter {
                if self.all_scorers[idx].doc <= self.upto {
                    let v = self.all_scorers[idx].scorer.get_max_score(new_upto)?;
                    self.all_scorers[idx].scaled_max_score =
                        scale_max_score(v, self.scaling_factor);
                }
            }
        }

        self.tail_max_score = 0;

        for i in 0..(self.tail_size as usize) {
            let idx = self.tail[i];
            self.all_scorers[idx].scorer.advance_shallow(target)?;
            let v = self.all_scorers[idx].scorer.get_max_score(self.upto)?;

            self.all_scorers[idx].scaled_max_score = scale_max_score(v, self.scaling_factor);

            Self::up_heap_max_score(&mut self.tail, i, &self.all_scorers); // the heap might need to be reordered

            self.tail_max_score += self.all_scorers[idx].scaled_max_score;
        }
        // We need to make sure that entries in 'tail' alone cannot match
        // a competitive hit.
        while self.tail_size > 0 && self.tail_max_score >= self.min_competitive_score {
            let idx = self.pop_tail();

            let new_doc = self.all_scorers[idx]
                .scorer
                .iterator_mut()
                .advance(target)?;
            self.all_scorers[idx].doc = new_doc;
            self.head.add(idx, &self.all_scorers);
        }

        Ok(())
    }

    /// Update upTo and maximum scores of sub scorers so that upTo is greater than or equal to the next candidate after target,
    /// i.e. the top of `head`.
    #[allow(clippy::never_loop)]
    fn move_to_next_block(&mut self, mut target: i32) -> Result<()> {
        debug_assert!(self.lead.is_none());

        while self.upto < NO_MORE_DOCS {
            if self.head.size() == 0 {
                // All clauses could fit in the tail, which means that the sum of the
                // maximum scores of sub clauses is less than the minimum competitive score.
                // Move to the next block until this condition becomes false.
                target = target.max(self.upto + 1);
                self.update_max_scores(target)?;
            } else {
                let top_idx = self.head.top().expect("top is empty");

                let top_doc = self.all_scorers[top_idx].doc;
                // We have a next candidate but it's not in the current block. We need to
                // move to the next block in order to not miss any potential hits between
                // `target` and `head.top().doc`.
                if top_doc > self.upto {
                    debug_assert!(top_doc >= target);
                    self.update_max_scores(target)?;
                    break;
                } else {
                    break;
                }
            }
            break;
        }

        debug_assert!(
            self.head.size() == 0 || self.all_scorers[self.head.top().unwrap()].doc <= self.upto
        );
        debug_assert!(self.upto >= target);

        Ok(())
    }
    /// Set 'doc' to the next potential match, and move all disis of 'head' that are on this doc into 'lead'.
    fn move_to_next_candidate(&mut self) -> Result<()> {
        // The top of `head` defines the next potential match
        // pop all documents which are on this doc
        let lead_idx = self.head.pop(&self.all_scorers);
        self.lead = Some(lead_idx);
        debug_assert!(self.doc == self.all_scorers[lead_idx].doc);
        self.all_scorers[lead_idx].next = None;
        self.freq = 1;

        if self.score_mode == ScoreMode::TopScores {
            self.lead_score = self.all_scorers[lead_idx].scorer.score()? as f64;
        }
        while self.head.size() > 0 {
            let top_idx = match self.head.top() {
                Some(idx) => idx,
                None => return Err(LuceneError::illegal_state("head top is empty")),
            };

            if self.all_scorers[top_idx].doc != self.doc {
                break;
            }

            let popped = self.head.pop(&self.all_scorers);
            self.add_lead(popped)?;
        }

        Ok(())
    }

    /// Advance all entries from the tail to know about all matches on the current doc.
    fn advance_all_tail(&mut self) -> Result<()> {
        // we return the next doc when the sum of the scores of the potential
        // matching clauses is high enough but some of the clauses in 'tail' might
        // match as well
        // since we are advancing all clauses in tail, we just iterate the array
        // without reorganizing the PQ
        for i in (0..self.tail_size as usize).rev() {
            self.advance_tail(self.tail[i])?;
        }
        self.tail_size = 0;
        self.tail_max_score = 0;
        debug_assert!(self.ensure_consistent()?);
        Ok(())
    }

    /// Insert an entry in 'tail' and evict the least-costly scorer if full.
    fn insert_tail_with_overflow(&mut self, s: usize) -> Option<usize> {
        let s_score = self.all_scorers[s].scaled_max_score;

        if self.tail_max_score + s_score < self.min_competitive_score
            || self.tail_size + 1 < self.min_should_match
        {
            // we have free room for this new entry
            self.add_tail(s);
            self.tail_max_score += s_score;
            None
        } else if self.tail_size == 0 {
            Some(s)
        } else {
            let top = self.tail[0];
            if !Self::greater_max_score(&self.all_scorers[top], &self.all_scorers[s]) {
                Some(s)
            } else {
                // swap top and s
                self.tail[0] = s;
                Self::down_heap_max_score(
                    &mut self.tail,
                    self.tail_size as usize,
                    &self.all_scorers,
                );
                self.tail_max_score =
                    self.tail_max_score - self.all_scorers[top].scaled_max_score + s_score;
                Some(top)
            }
        }
    }

    /// Add an entry to 'tail'. Fails if over capacity.
    fn add_tail(&mut self, idx: usize) {
        self.tail[self.tail_size as usize] = idx;
        Self::up_heap_max_score(&mut self.tail, self.tail_size as usize, &self.all_scorers);
        self.tail_size += 1;
    }

    /// Pop the least-costly scorer from 'tail'.
    fn pop_tail(&mut self) -> usize {
        debug_assert!(self.tail_size > 0);
        let result = self.tail[0];
        self.tail_size -= 1;
        self.tail[0] = self.tail[self.tail_size as usize];
        Self::down_heap_max_score(&mut self.tail, self.tail_size as usize, &self.all_scorers);
        self.tail_max_score -= self.all_scorers[result].scaled_max_score;

        result
    }

    /// Heap helpers
    fn up_heap_max_score(heap: &mut [usize], mut i: usize, all: &[DisiWrapper<S>]) {
        let node = heap[i];
        let mut j = DisiPriorityQueue::parent_node(i);

        while j < heap.len() && Self::greater_max_score(&all[node], &all[heap[j]]) {
            heap[i] = heap[j];
            i = j;
            j = DisiPriorityQueue::parent_node(j);
        }

        heap[i] = node;
    }

    fn down_heap_max_score(heap: &mut [usize], size: usize, all: &[DisiWrapper<S>]) {
        let mut i = 0;
        let node = heap[0];
        let mut j = DisiPriorityQueue::left_node(i);

        if j < size {
            let mut k = DisiPriorityQueue::right_node(j);

            if k < size && Self::greater_max_score(&all[heap[k]], &all[heap[j]]) {
                j = k;
            }

            if Self::greater_max_score(&all[heap[j]], &all[node]) {
                loop {
                    heap[i] = heap[j];
                    i = j;
                    j = DisiPriorityQueue::left_node(i);
                    k = DisiPriorityQueue::right_node(j);

                    if k < size && Self::greater_max_score(&all[heap[k]], &all[heap[j]]) {
                        j = k;
                    }

                    if j >= size || !Self::greater_max_score(&all[heap[j]], &all[node]) {
                        break;
                    }
                }
                heap[i] = node;
            }
        }
    }

    /// In the tail, we want to get first entries that produce the maximum scores and in case of ties
    /// (eg. constant-score queries), those that have the least cost so that they are likely to advance further.
    fn greater_max_score(w1: &DisiWrapper<S>, w2: &DisiWrapper<S>) -> bool {
        if w1.scaled_max_score > w2.scaled_max_score {
            true
        } else if w1.scaled_max_score < w2.scaled_max_score {
            false
        } else {
            w1.cost < w2.cost
        }
    }
    // returns a boolean so that it can be called from assert
    // the return value is useless: it always returns true
    fn ensure_consistent(&mut self) -> Result<bool> {
        if self.score_mode == ScoreMode::TopScores {
            let mut max_score_sum: i64 = 0;

            for i in 0..(self.tail_size as usize) {
                let idx = self.tail[i];
                let w = &self.all_scorers[idx];

                debug_assert!(w.doc < self.doc);
                max_score_sum = max_score_sum.checked_add(w.scaled_max_score).unwrap();
            }

            debug_assert!(
                max_score_sum == self.tail_max_score,
                "tailMaxScore mismatch: {max_score_sum} vs {}",
                self.tail_max_score
            );

            let mut lead_scores: Vec<f32> = Vec::new();

            let mut cur = self.lead;
            while let Some(idx) = cur {
                let w = &mut self.all_scorers[idx];
                debug_assert!(w.doc == self.doc);
                lead_scores.push(w.scorer.score()?);
                cur = w.next;
            }
            // Make sure to recompute the sum in the same order to get the same floating point rounding
            // errors.
            lead_scores.reverse();

            let mut recomputed_lead_score = 0f64;
            for score in &lead_scores {
                recomputed_lead_score += *score as f64;
            }

            debug_assert!(
                (recomputed_lead_score == self.lead_score),
                "leadScore mismatch: recomputed={recomputed_lead_score} stored={}",
                self.lead_score
            );

            debug_assert!(
                self.min_competitive_score == 0
                    || self.tail_max_score < self.min_competitive_score
                    || self.tail_size < self.min_should_match,
            );

            debug_assert!(self.doc <= self.upto);
        }

        let head_iter = self.head.iter();
        for idx in head_iter {
            let w = &self.all_scorers[idx];

            if self.lead.is_none() {
                debug_assert!(w.doc >= self.doc);
            } else {
                debug_assert!(w.doc > self.doc);
            }
        }

        Ok(true)
    }
}
impl<S> DocIdSetIterator for DocIdSetIteratorImpl<S>
where
    S: Scorer,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc + 1)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        // Move 'lead' iterators back to the tail
        self.push_back_leads(target)?;
        // Make sure `head` is also on or beyond `target`
        let mut head_top = self.advance_head(target)?;

        if self.score_mode == ScoreMode::TopScores
            && (head_top.is_none() || self.all_scorers[head_top.unwrap()].doc > self.upto)
        {
            // Update score bounds if necessary
            self.move_to_next_block(target)?;
            debug_assert!(self.upto >= target);

            head_top = self.head.top();
        }
        if let Some(idx) = head_top {
            self.doc = self.all_scorers[idx].doc;
            Ok(self.doc)
        } else {
            self.doc = NO_MORE_DOCS;
            Ok(self.doc)
        }
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.cost)
    }
}

pub struct TwoPhaseIteratorImpl<S>
where
    S: Scorer,
{
    approximation: DocIdSetIteratorImpl<S>,
}
impl<S> TwoPhaseIteratorImpl<S>
where
    S: Scorer,
{
    pub fn new(approximation: DocIdSetIteratorImpl<S>) -> Self {
        Self { approximation }
    }
}
impl<S> TwoPhaseIterator for TwoPhaseIteratorImpl<S>
where
    S: Scorer,
{
    fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&mut self.approximation)
    }

    fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&self.approximation)
    }

    fn matches(&mut self) -> Result<bool> {
        let approx = &mut self.approximation;

        debug_assert!(approx.lead.is_none());

        approx.move_to_next_candidate()?;

        let mut scaled_lead_score: i64 = 0;

        if approx.score_mode == ScoreMode::TopScores {
            scaled_lead_score = scale_max_score(
                MathUtil::sum_upper_bound(approx.lead_score, FLOAT_MANTISSA_BITS) as f32,
                approx.scaling_factor,
            );
        }

        while scaled_lead_score < approx.min_competitive_score
            || approx.freq < approx.min_should_match
        {
            debug_assert!(approx.ensure_consistent()?);

            if scaled_lead_score + approx.tail_max_score < approx.min_competitive_score
                || approx.freq + approx.tail_size < approx.min_should_match
            {
                return Ok(false);
            } else {
                // a match on doc is still possible, try to
                // advance scorers from the tail
                let prev_lead = approx.lead;
                approx.advance_tail_top()?;

                if approx.score_mode == ScoreMode::TopScores && approx.lead != prev_lead {
                    debug_assert!(prev_lead == approx.all_scorers[approx.lead.unwrap()].next);

                    scaled_lead_score = scale_max_score(
                        MathUtil::sum_upper_bound(approx.lead_score, FLOAT_MANTISSA_BITS) as f32,
                        approx.scaling_factor,
                    );
                }
            }
        }
        debug_assert!(approx.ensure_consistent()?);
        Ok(true)
    }

    fn match_cost(&self) -> f32 {
        self.approximation.tail.len() as f32
    }
}

pub(crate) const FLOAT_MANTISSA_BITS: i32 = 24;
const MAX_SCALED_SCORE: i64 = (1_i64 << 24) - 1;

#[inline]
fn get_exponent_f32(f: f32) -> i32 {
    let bits = f.to_bits();
    let exp = ((bits >> 23) & 0xFF) as i32;
    exp - 127
}
/// Return a scaling factor for the given float such that
/// `f * 2^scaling_factor` falls within the interval `[2^23, 2^24)`.
///
/// Special cases:
/// - `scalingFactor(0) = scalingFactor(MIN_VALUE) + 1`
/// - `scalingFactor(+∞) = scalingFactor(MAX_VALUE) - 1`
///
/// This ensures that values are scaled so that their significant bits fit within
/// the 24-bit mantissa of a single-precision floating-point number.
fn scaling_factor(f: f32) -> Result<i32> {
    if f < 0.0 {
        Err(LuceneError::illegal_argument(
            "Scores must be positive or null",
        ))
    } else if f == 0.0 {
        Ok(scaling_factor(f32::MIN_POSITIVE)? + 1)
    } else if f.is_infinite() {
        Ok(scaling_factor(f32::MAX)? - 1)
    } else {
        let exp = get_exponent_f32(f);
        Ok(FLOAT_MANTISSA_BITS - 1 - exp)
    }
}
/// Scale maximum scores into an unsigned integer in order to avoid overflows
/// (only the lower 32 bits of the `u64` are used) and to prevent
/// floating-point arithmetic errors.
///
/// Values are rounded **up** to ensure that no competitive matches are missed.
fn scale_max_score(max_score: f32, scaling_factor: i32) -> i64 {
    assert!(!max_score.is_nan());
    assert!(max_score >= 0.0);
    let scaled = (max_score as f64) * (2f64.powi(scaling_factor));

    if scaled > MAX_SCALED_SCORE as f64 {
        // This happens if one scorer returns +Infty as a max score, or if the scorer returns greater
        // max scores locally than globally - which shouldn't happen with well-behaved scorers
        return MAX_SCALED_SCORE;
    }
    scaled.ceil() as i64
}
/// Scale minimum competitive scores in the same way as maximum scores,
/// except values are rounded **down** in order to ensure that no matches
/// are missed during pruning.
fn scale_min_score(min_score: f32, scaling_factor: i32) -> i64 {
    assert!(min_score.is_finite());
    assert!(min_score >= 0.0);
    let scaled = (min_score as f64) * (2f64.powi(scaling_factor));
    scaled.floor() as i64
}
#[cfg(test)]
pub(crate) mod tests {
    use crate::core::document::document::Document;
    use crate::core::document::field::Store;
    use crate::core::document::string_field::StringField;
    use crate::core::index::directory_reader::directory_reader_util;
    use crate::core::index::index_reader::Identity;
    use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
    use crate::core::index::index_writer::IndexWriter;
    use crate::core::index::index_writer_config::IndexWriterConfig;
    use crate::core::index::leaf_reader::{LRTermState, LeafReader};
    use crate::core::index::leaf_reader_context::LeafReaderContext;
    use crate::core::index::term::Term;
    use crate::core::index::term_states::TermStates;
    use crate::core::search::boolean_clause::Occur;
    use crate::core::search::boolean_query::{BooleanQuery, Builder};
    use crate::core::search::boolean_weight::BooleanWeight;
    use crate::core::search::boost_query::BoostQuery;
    use crate::core::search::constant_score_query::ConstantScoreQuery;
    use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::core::search::explanation::Explanation;
    use crate::core::search::index_searcher::IndexSearcher;
    use crate::core::search::matches_utils::MatchWithNoTerms;
    use crate::core::search::query::{
        Query, QueryBase, QueryWeight, QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer,
    };
    use crate::core::search::query_visitor::QueryVisitor;
    use crate::core::search::scorable::Scorable;
    use crate::core::search::score_mode::ScoreMode;
    use crate::core::search::scorer::{Scorer, ScorerEnum2, TwoPhaseState};
    use crate::core::search::scorer_supplier::ScorerSupplier;
    use crate::core::search::segment_cacheable::SegmentCacheable;
    use crate::core::search::term_query::TermQuery;
    use crate::core::search::two_phase_iterator::TwoPhaseIterator;
    use crate::core::search::wand_scorer::{
        FLOAT_MANTISSA_BITS, WANDScorer, scale_max_score, scaling_factor,
    };
    use crate::core::search::weight::{DefaultScorerSupplier, Weight};
    use crate::core::util::error::lucene_error::Result;
    use crate::core::util::{HasIdentity, ToInt};
    use crate::test::search::check_hits::CheckHits;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        at_least, new_directory_shared, new_index_writer_config, new_searcher_with_reader,
        new_searcher_with_threads, random,
    };
    use rand::Rng;
    use std::fmt::Debug;
    use std::hash::{Hash, Hasher};
    use std::rc::Rc;
    use std::sync::Arc;

    #[allow(dead_code)] // for quick search
    struct TestWANDScorer;
    #[test]
    fn test_scaling_factor() -> Result<()> {
        use std::f32;

        do_test_scaling_factor(1.0)?;
        do_test_scaling_factor(2.0)?;
        do_test_scaling_factor(1.0f32.next_down())?;
        do_test_scaling_factor(1.0f32.next_up())?;
        do_test_scaling_factor(f32::MIN_POSITIVE)?;
        do_test_scaling_factor(f32::MIN_POSITIVE.next_up())?;
        do_test_scaling_factor(f32::MAX)?;
        do_test_scaling_factor(f32::MAX.next_down())?;

        assert_eq!(scaling_factor(f32::MIN_POSITIVE)? + 1, scaling_factor(0.0)?);

        assert_eq!(
            scaling_factor(f32::MAX)? - 1,
            scaling_factor(f32::INFINITY)?
        );

        assert!(scaling_factor(1.0)? > scaling_factor(10.0)?);
        assert!(scaling_factor(f32::MAX)? > scaling_factor(f32::INFINITY)?);
        assert!(scaling_factor(0.0)? > scaling_factor(f32::MIN_POSITIVE)?);
        Ok(())
    }
    fn do_test_scaling_factor(v: f32) -> Result<()> {
        let sf = scaling_factor(v)?;
        let scaled = (v as f64) * (2f64.powi(sf));
        assert!(
            scaled >= (1u64 << 23) as f64 && scaled < (1u64 << 24) as f64,
            "v={v}, sf={sf}, scaled={scaled}"
        );
        Ok(())
    }
    #[test]
    fn test_scale_max_score() -> Result<()> {
        let expected = 1i64 << (FLOAT_MANTISSA_BITS - 1);
        let sf = scaling_factor(32.0)?;
        let scaled = scale_max_score(32.0, sf);
        assert_eq!(expected, scaled);

        let v = (1.0f32 as f64 * 2f64.powi(60)) as f32;
        let sf2 = scaling_factor(v)?;
        let scaled2 = scale_max_score(32.0, sf2);
        assert_eq!(1, scaled2);

        let sf3 = scaling_factor(f32::INFINITY)?;
        let scaled3 = scale_max_score(32.0, sf3);
        assert_eq!(1, scaled3);

        Ok(())
    }
    #[test]
    fn test_basics() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        // TODO newLogMergePolicy 未实现
        let conf = IndexWriterConfig::new();
        let w = IndexWriter::new(dir.clone(), conf)?;

        let docs: &[&[&str]] = &[
            &["A", "B"],      // 0
            &["A"],           // 1
            &[],              // 2
            &["A", "B", "C"], // 3
            &["B"],           // 4
            &["B", "C"],      // 5
        ];

        for values in docs {
            let mut doc = Document::new();
            for value in *values {
                doc.add(StringField::from_string("foo", *value, Store::No)?);
            }
            w.add_document(doc)?;
        }

        // TODO force_merge未实现
        // w.force_merge(1)?;
        w.close()?;

        let reader = directory_reader_util::open(dir)?;
        let searcher = new_searcher_with_reader(reader)?;
        let mut builder = Builder::new();
        builder
            .add(
                BoostQuery::new(
                    Box::new(
                        ConstantScoreQuery::new(Box::new(
                            TermQuery::new(Term::from_text("foo", "A")).into(),
                        ))
                        .into(),
                    ),
                    2.0,
                )?,
                Occur::Should,
            )?
            .add(
                ConstantScoreQuery::new(Box::new(
                    TermQuery::new(Term::from_text("foo", "B")).into(),
                )),
                Occur::Should,
            )?
            .add(
                BoostQuery::new(
                    Box::new(
                        ConstantScoreQuery::new(Box::new(
                            TermQuery::new(Term::from_text("foo", "C")).into(),
                        ))
                        .into(),
                    ),
                    3.0,
                )?,
                Occur::Should,
            )?;
        let mut query = Query::WANDScorer(WANDScorerQuery::new(
            builder.build(),
            random.random_bool(0.5),
        ));

        let weight =
            searcher.create_weight(searcher.rewrite(query)?, ScoreMode::TopScores, 1.0, None)?;
        let context = &searcher.get_leaf_contexts()?[0];

        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;

        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        assert_eq!(3.0, scorer.score()?);

        assert_eq!(1, scorer.iterator_mut().next_doc()?);
        assert_eq!(2.0, scorer.score()?);

        assert_eq!(3, scorer.iterator_mut().next_doc()?);
        assert_eq!(6.0, scorer.score()?);

        assert_eq!(4, scorer.iterator_mut().next_doc()?);
        assert_eq!(1.0, scorer.score()?);

        assert_eq!(5, scorer.iterator_mut().next_doc()?);
        assert_eq!(4.0, scorer.score()?);

        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;
        scorer.set_min_competitive_score(4.0)?;

        assert_eq!(3, scorer.iterator_mut().next_doc()?);
        assert_eq!(6.0, scorer.score()?);

        assert_eq!(5, scorer.iterator_mut().next_doc()?);
        assert_eq!(4.0, scorer.score()?);

        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;

        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        assert_eq!(3.0, scorer.score()?);

        scorer.set_min_competitive_score(10.0)?;

        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);
        //  test a filtered disjunction
        builder = Builder::new();
        builder
            .add(
                Query::WANDScorer(WANDScorerQuery::new(
                    {
                        let mut v = Builder::new();
                        v.add(
                            BoostQuery::new(
                                Box::new(
                                    ConstantScoreQuery::new(Box::new(
                                        TermQuery::new(Term::from_text("foo", "A")).into(),
                                    ))
                                    .into(),
                                ),
                                2.0,
                            )?,
                            Occur::Should,
                        )?
                        .add(
                            ConstantScoreQuery::new(Box::new(
                                TermQuery::new(Term::from_text("foo", "B")).into(),
                            )),
                            Occur::Should,
                        )?;
                        v.build()
                    },
                    random.random_bool(0.5),
                )),
                Occur::Must,
            )?
            .add(TermQuery::new(Term::from_text("foo", "C")), Occur::Filter)?;
        query = builder.build().into();

        let weight =
            searcher.create_weight(searcher.rewrite(query)?, ScoreMode::TopScores, 1.0, None)?;
        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;

        assert_eq!(3, scorer.iterator_mut().next_doc()?);
        assert_eq!(3.0, scorer.score()?);

        assert_eq!(5, scorer.iterator_mut().next_doc()?);
        assert_eq!(1.0, scorer.score()?);

        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;
        scorer.set_min_competitive_score(2.0)?;

        assert_eq!(3, scorer.iterator_mut().next_doc()?);
        assert_eq!(3.0, scorer.score()?);

        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        builder = Builder::new();
        builder
            .add(
                Query::WANDScorer(WANDScorerQuery::new(
                    {
                        let mut v = Builder::new();
                        v.add(
                            BoostQuery::new(
                                Box::new(
                                    ConstantScoreQuery::new(Box::new(
                                        TermQuery::new(Term::from_text("foo", "A")).into(),
                                    ))
                                    .into(),
                                ),
                                2.0,
                            )?,
                            Occur::Should,
                        )?
                        .add(
                            ConstantScoreQuery::new(Box::new(
                                TermQuery::new(Term::from_text("foo", "B")).into(),
                            )),
                            Occur::Should,
                        )?;
                        v.build()
                    },
                    random.random_bool(0.5),
                )),
                Occur::Must,
            )?
            .add(TermQuery::new(Term::from_text("foo", "C")), Occur::MustNot)?;
        query = builder.build().into();

        let weight =
            searcher.create_weight(searcher.rewrite(query)?, ScoreMode::TopScores, 1.0, None)?;
        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;

        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        assert_eq!(3.0, scorer.score()?);

        assert_eq!(1, scorer.iterator_mut().next_doc()?);
        assert_eq!(2.0, scorer.score()?);

        assert_eq!(4, scorer.iterator_mut().next_doc()?);
        assert_eq!(1.0, scorer.score()?);

        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;
        scorer.set_min_competitive_score(3.0)?;

        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        assert_eq!(3.0, scorer.score()?);

        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        Ok(())
    }

    #[test]
    fn test_basics_with_disjunction_and_min_should_match() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        // TODO newLogMergePolicy 未实现
        let conf = IndexWriterConfig::new();
        let w = IndexWriter::new(dir.clone(), conf)?;

        let docs: &[&[&str]] = &[
            &["A", "B"],      // 0
            &["A"],           // 1
            &[],              // 2
            &["A", "B", "C"], // 3
            &["B"],           // 4
            &["B", "C"],      // 5
        ];

        for values in docs {
            let mut doc = Document::new();
            for value in *values {
                doc.add(StringField::from_string("foo", *value, Store::No)?);
            }
            w.add_document(doc)?;
        }

        // TODO force_merge未实现
        // w.force_merge(1)?;
        w.close()?;

        let reader = directory_reader_util::open(dir)?;
        let searcher = new_searcher_with_reader(reader)?;
        let context = &searcher.get_leaf_contexts()?[0];

        let mut builder = Builder::new();
        builder
            .add(
                BoostQuery::new(
                    Box::new(
                        ConstantScoreQuery::new(Box::new(
                            TermQuery::new(Term::from_text("foo", "A")).into(),
                        ))
                        .into(),
                    ),
                    2.0,
                )?,
                Occur::Should,
            )?
            .add(
                ConstantScoreQuery::new(Box::new(
                    TermQuery::new(Term::from_text("foo", "B")).into(),
                )),
                Occur::Should,
            )?
            .add(
                BoostQuery::new(
                    Box::new(
                        ConstantScoreQuery::new(Box::new(
                            TermQuery::new(Term::from_text("foo", "C")).into(),
                        ))
                        .into(),
                    ),
                    3.0,
                )?,
                Occur::Should,
            )?;
        builder.set_minimum_number_should_match(2);

        let query: Query = Query::WANDScorer(WANDScorerQuery::new(
            builder.build(),
            random.random_bool(0.5),
        ));

        let weight =
            searcher.create_weight(searcher.rewrite(query)?, ScoreMode::TopScores, 1.0, None)?;
        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;

        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        assert_eq!(3.0, scorer.score()?);

        assert_eq!(3, scorer.iterator_mut().next_doc()?);
        assert_eq!(6.0, scorer.score()?);

        assert_eq!(5, scorer.iterator_mut().next_doc()?);
        assert_eq!(4.0, scorer.score()?);

        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;
        scorer.set_min_competitive_score(4.0)?;

        assert_eq!(3, scorer.iterator_mut().next_doc()?);
        assert_eq!(6.0, scorer.score()?);

        assert_eq!(5, scorer.iterator_mut().next_doc()?);
        assert_eq!(4.0, scorer.score()?);

        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;

        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        assert_eq!(3.0, scorer.score()?);

        scorer.set_min_competitive_score(10.0)?;

        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        Ok(())
    }

    #[test]
    fn test_basics_with_disjunction_and_min_should_match_and_tail_size_condition() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        // TODO newLogMergePolicy 未实现
        let conf = IndexWriterConfig::new();
        let w = IndexWriter::new(dir.clone(), conf)?;

        let docs: &[&[&str]] = &[
            &["A", "B"],      // 0
            &["A"],           // 1
            &[],              // 2
            &["A", "B", "C"], // 3
            // 2 "B"s here and the non constant score term query below forces the
            // tailMaxScore >= minCompetitiveScore && tailSize < minShouldMatch condition
            &["B", "B"], // 4
            &["B", "C"], // 5
        ];

        for values in docs {
            let mut doc = Document::new();
            for value in *values {
                doc.add(StringField::from_string("foo", *value, Store::No)?);
            }
            w.add_document(doc)?;
        }

        // TODO force_merge未实现
        // w.force_merge(1)?;
        w.close()?;

        let reader = directory_reader_util::open(dir)?;
        let searcher = new_searcher_with_reader(reader)?;
        let context = &searcher.get_leaf_contexts()?[0];

        let mut builder = Builder::new();
        builder
            .add(TermQuery::new(Term::from_text("foo", "A")), Occur::Should)?
            .add(TermQuery::new(Term::from_text("foo", "B")), Occur::Should)?
            .add(TermQuery::new(Term::from_text("foo", "C")), Occur::Should)?;
        builder.set_minimum_number_should_match(2);

        let query: Query = Query::WANDScorer(WANDScorerQuery::new(
            builder.build(),
            random.random_bool(0.5),
        ));

        let weight =
            searcher.create_weight(searcher.rewrite(query)?, ScoreMode::TopScores, 1.0, None)?;
        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;

        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        let score = scorer.score()?;
        scorer.set_min_competitive_score(score)?;

        assert_eq!(3, scorer.iterator_mut().next_doc()?);

        Ok(())
    }

    #[test]
    fn test_basics_with_disjunction_and_min_should_match_and_non_scoring_mode() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        // TODO newLogMergePolicy 未实现
        let conf = IndexWriterConfig::new();
        let w = IndexWriter::new(dir.clone(), conf)?;

        let docs: &[&[&str]] = &[
            &["A", "B"],      // 0
            &["A"],           // 1
            &[],              // 2
            &["A", "B", "C"], // 3
            &["B"],           // 4
            &["B", "C"],      // 5
        ];

        for values in docs {
            let mut doc = Document::new();
            for value in *values {
                doc.add(StringField::from_string("foo", *value, Store::No)?);
            }
            w.add_document(doc)?;
        }

        // TODO force_merge未实现
        // w.force_merge(1)?;
        w.close()?;

        let reader = directory_reader_util::open(dir)?;
        let searcher = new_searcher_with_reader(reader)?;
        let context = &searcher.get_leaf_contexts()?[0];

        let mut builder = Builder::new();
        builder
            .add(
                BoostQuery::new(
                    Box::new(
                        ConstantScoreQuery::new(Box::new(
                            TermQuery::new(Term::from_text("foo", "A")).into(),
                        ))
                        .into(),
                    ),
                    2.0,
                )?,
                Occur::Should,
            )?
            .add(
                ConstantScoreQuery::new(Box::new(
                    TermQuery::new(Term::from_text("foo", "B")).into(),
                )),
                Occur::Should,
            )?
            .add(
                BoostQuery::new(
                    Box::new(
                        ConstantScoreQuery::new(Box::new(
                            TermQuery::new(Term::from_text("foo", "C")).into(),
                        ))
                        .into(),
                    ),
                    3.0,
                )?,
                Occur::Should,
            )?;
        builder.set_minimum_number_should_match(2);

        let query: Query = Query::WANDScorer(WANDScorerQuery::new(
            builder.build(),
            random.random_bool(0.5),
        ));

        let weight = searcher.create_weight(
            searcher.rewrite(query)?,
            ScoreMode::CompleteNoScores,
            1.0,
            None,
        )?;
        let mut scorer = weight
            .scorer(context)?
            .expect("expected scorer to be present");

        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        assert_eq!(3, scorer.iterator_mut().next_doc()?);
        assert_eq!(5, scorer.iterator_mut().next_doc()?);
        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        Ok(())
    }
    #[test]
    fn test_basics_with_filtered_disjunction_and_min_should_match() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        // TODO newLogMergePolicy 未实现
        let conf = IndexWriterConfig::new();
        let w = IndexWriter::new(dir.clone(), conf)?;

        let docs: &[&[&str]] = &[
            &["A", "B"],           // 0
            &["A", "C", "D"],      // 1
            &[],                   // 2
            &["A", "B", "C", "D"], // 3
            &["B"],                // 4
            &["C", "D"],           // 5
        ];

        for values in docs {
            let mut doc = Document::new();
            for value in *values {
                doc.add(StringField::from_string("foo", *value, Store::No)?);
            }
            w.add_document(doc)?;
        }

        // TODO force_merge未实现
        // w.force_merge(1)?;
        w.close()?;

        let reader = directory_reader_util::open(dir)?;
        let searcher = new_searcher_with_reader(reader)?;
        let context = &searcher.get_leaf_contexts()?[0];

        let query: Query = {
            let mut inner = Builder::new();
            inner
                .add(
                    BoostQuery::new(
                        Box::new(
                            ConstantScoreQuery::new(Box::new(
                                TermQuery::new(Term::from_text("foo", "A")).into(),
                            ))
                            .into(),
                        ),
                        2.0,
                    )?,
                    Occur::Should,
                )?
                .add(
                    ConstantScoreQuery::new(Box::new(
                        TermQuery::new(Term::from_text("foo", "B")).into(),
                    )),
                    Occur::Should,
                )?
                .add(
                    BoostQuery::new(
                        Box::new(
                            ConstantScoreQuery::new(Box::new(
                                TermQuery::new(Term::from_text("foo", "D")).into(),
                            ))
                            .into(),
                        ),
                        4.0,
                    )?,
                    Occur::Should,
                )?;
            inner.set_minimum_number_should_match(2);

            let inner_query =
                Query::WANDScorer(WANDScorerQuery::new(inner.build(), random.random_bool(0.5)));

            let mut outer = Builder::new();
            outer
                .add(inner_query, Occur::Must)?
                .add(TermQuery::new(Term::from_text("foo", "C")), Occur::Filter)?;
            outer.build().into()
        };

        let weight =
            searcher.create_weight(searcher.rewrite(query)?, ScoreMode::TopScores, 1.0, None)?;
        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;

        assert_eq!(1, scorer.iterator_mut().next_doc()?);
        assert_eq!(6.0, scorer.score()?); // 2 + 4

        assert_eq!(3, scorer.iterator_mut().next_doc()?);
        assert_eq!(7.0, scorer.score()?); // 2 + 1 + 4

        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;
        scorer.set_min_competitive_score(7.0)?; // 2 + 1 + 4

        assert_eq!(3, scorer.iterator_mut().next_doc()?);
        assert_eq!(7.0, scorer.score()?);

        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        Ok(())
    }

    #[test]
    fn test_basics_with_filtered_disjunction_and_min_should_match_and_non_scoring_mode()
    -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        // TODO newLogMergePolicy 未实现
        let conf = IndexWriterConfig::new();
        let w = IndexWriter::new(dir.clone(), conf)?;

        let docs: &[&[&str]] = &[
            &["A", "B"],           // 0
            &["A", "C", "D"],      // 1
            &[],                   // 2
            &["A", "B", "C", "D"], // 3
            &["B"],                // 4
            &["C", "D"],           // 5
        ];

        for values in docs {
            let mut doc = Document::new();
            for value in *values {
                doc.add(StringField::from_string("foo", *value, Store::No)?);
            }
            w.add_document(doc)?;
        }

        // TODO force_merge未实现
        // w.force_merge(1)?;
        w.close()?;

        let reader = directory_reader_util::open(dir)?;
        let searcher = new_searcher_with_reader(reader)?;
        let context = &searcher.get_leaf_contexts()?[0];

        let query: Query = {
            let mut inner = Builder::new();
            inner
                .add(
                    BoostQuery::new(
                        Box::new(
                            ConstantScoreQuery::new(Box::new(
                                TermQuery::new(Term::from_text("foo", "A")).into(),
                            ))
                            .into(),
                        ),
                        2.0,
                    )?,
                    Occur::Should,
                )?
                .add(
                    ConstantScoreQuery::new(Box::new(
                        TermQuery::new(Term::from_text("foo", "B")).into(),
                    )),
                    Occur::Should,
                )?
                .add(
                    BoostQuery::new(
                        Box::new(
                            ConstantScoreQuery::new(Box::new(
                                TermQuery::new(Term::from_text("foo", "D")).into(),
                            ))
                            .into(),
                        ),
                        4.0,
                    )?,
                    Occur::Should,
                )?;
            inner.set_minimum_number_should_match(2);

            let inner_query =
                Query::WANDScorer(WANDScorerQuery::new(inner.build(), random.random_bool(0.5)));

            let mut outer = Builder::new();
            outer
                .add(inner_query, Occur::Must)?
                .add(TermQuery::new(Term::from_text("foo", "C")), Occur::Filter)?;
            outer.build().into()
        };

        let weight =
            searcher.create_weight(searcher.rewrite(query)?, ScoreMode::TopDocs, 1.0, None)?;
        let mut scorer = weight
            .scorer(context)?
            .expect("expected scorer to be present");

        assert_eq!(1, scorer.iterator_mut().next_doc()?);
        assert_eq!(3, scorer.iterator_mut().next_doc()?);
        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        Ok(())
    }

    #[test]
    fn test_basics_with_filtered_disjunction_and_must_not_and_min_should_match() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        // TODO newLogMergePolicy 未实现
        let conf = IndexWriterConfig::new();
        let w = IndexWriter::new(dir.clone(), conf)?;

        let docs: &[&[&str]] = &[
            &["A", "B"],           // 0
            &["A", "C", "D"],      // 1
            &[],                   // 2
            &["A", "B", "C", "D"], // 3
            &["B", "D"],           // 4
            &["C", "D"],           // 5
        ];

        for values in docs {
            let mut doc = Document::new();
            for value in *values {
                doc.add(StringField::from_string("foo", *value, Store::No)?);
            }
            w.add_document(doc)?;
        }

        // TODO force_merge未实现
        // w.force_merge(1)?;
        w.close()?;

        let reader = directory_reader_util::open(dir)?;
        let searcher = new_searcher_with_reader(reader)?;
        let context = &searcher.get_leaf_contexts()?[0];

        let query: Query = {
            let mut inner = Builder::new();
            inner
                .add(
                    BoostQuery::new(
                        Box::new(
                            ConstantScoreQuery::new(Box::new(
                                TermQuery::new(Term::from_text("foo", "A")).into(),
                            ))
                            .into(),
                        ),
                        2.0,
                    )?,
                    Occur::Should,
                )?
                .add(
                    ConstantScoreQuery::new(Box::new(
                        TermQuery::new(Term::from_text("foo", "B")).into(),
                    )),
                    Occur::Should,
                )?
                .add(
                    BoostQuery::new(
                        Box::new(
                            ConstantScoreQuery::new(Box::new(
                                TermQuery::new(Term::from_text("foo", "D")).into(),
                            ))
                            .into(),
                        ),
                        4.0,
                    )?,
                    Occur::Should,
                )?;
            inner.set_minimum_number_should_match(2);

            let inner_query =
                Query::WANDScorer(WANDScorerQuery::new(inner.build(), random.random_bool(0.5)));

            let mut outer = Builder::new();
            outer
                .add(inner_query, Occur::Must)?
                .add(TermQuery::new(Term::from_text("foo", "C")), Occur::MustNot)?;
            outer.build().into()
        };

        let weight =
            searcher.create_weight(searcher.rewrite(query)?, ScoreMode::TopScores, 1.0, None)?;
        let mut scorer = weight
            .scorer(context)?
            .expect("expected scorer to be present");

        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        assert_eq!(3.0, scorer.score()?); // 2 + 1

        assert_eq!(4, scorer.iterator_mut().next_doc()?);
        assert_eq!(5.0, scorer.score()?); // 1 + 4

        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        let mut ss = weight
            .scorer_supplier(context)?
            .expect("expected scorer supplier");
        ss.set_top_level_scoring_clause()?;
        let mut scorer = ss.get(i64::MAX, context)?;
        scorer.set_min_competitive_score(4.0)?;

        assert_eq!(4, scorer.iterator_mut().next_doc()?);
        assert_eq!(5.0, scorer.score()?);

        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        Ok(())
    }

    #[test]
    fn test_basics_with_filtered_disjunction_and_must_not_and_min_should_match_and_non_scoring_mode()
    -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        // TODO newLogMergePolicy 未实现
        let conf = IndexWriterConfig::new();
        let w = IndexWriter::new(dir.clone(), conf)?;

        let docs: &[&[&str]] = &[
            &["A", "B"],           // 0
            &["A", "C", "D"],      // 1
            &[],                   // 2
            &["A", "B", "C", "D"], // 3
            &["B", "D"],           // 4
            &["C", "D"],           // 5
        ];

        for values in docs {
            let mut doc = Document::new();
            for value in *values {
                doc.add(StringField::from_string("foo", *value, Store::No)?);
            }
            w.add_document(doc)?;
        }

        // TODO force_merge未实现
        // w.force_merge(1)?;
        w.close()?;

        let reader = directory_reader_util::open(dir)?;
        let searcher = new_searcher_with_reader(reader)?;
        let context = &searcher.get_leaf_contexts()?[0];

        let query: Query = {
            let mut inner = Builder::new();
            inner
                .add(
                    BoostQuery::new(
                        Box::new(
                            ConstantScoreQuery::new(Box::new(
                                TermQuery::new(Term::from_text("foo", "A")).into(),
                            ))
                            .into(),
                        ),
                        2.0,
                    )?,
                    Occur::Should,
                )?
                .add(
                    ConstantScoreQuery::new(Box::new(
                        TermQuery::new(Term::from_text("foo", "B")).into(),
                    )),
                    Occur::Should,
                )?
                .add(
                    BoostQuery::new(
                        Box::new(
                            ConstantScoreQuery::new(Box::new(
                                TermQuery::new(Term::from_text("foo", "D")).into(),
                            ))
                            .into(),
                        ),
                        4.0,
                    )?,
                    Occur::Should,
                )?;
            inner.set_minimum_number_should_match(2);

            let inner_query =
                Query::WANDScorer(WANDScorerQuery::new(inner.build(), random.random_bool(0.5)));

            let mut outer = Builder::new();
            outer
                .add(inner_query, Occur::Must)?
                .add(TermQuery::new(Term::from_text("foo", "C")), Occur::MustNot)?;
            outer.build().into()
        };

        let weight = searcher.create_weight(
            searcher.rewrite(query)?,
            ScoreMode::CompleteNoScores,
            1.0,
            None,
        )?;
        let mut scorer = weight
            .scorer(context)?
            .expect("expected scorer to be present");

        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        assert_eq!(4, scorer.iterator_mut().next_doc()?);
        assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().next_doc()?);

        Ok(())
    }

    #[test]
    fn test_random() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
        let num_docs = at_least(&mut random, 1000);
        for _ in 0..num_docs {
            let mut doc = Document::new();
            let v = random.random_range(0..5);
            let num_values = random.random_range(0..1 << v);
            let start = random.random_range(0..10);
            for j in 0..num_values {
                doc.add(StringField::from_string(
                    "foo",
                    (start + j).to_string(),
                    Store::No,
                )?);
            }
            w.add_document(doc)?;
        }

        let reader = directory_reader_util::open_with_writer(&w)?;
        w.close()?;

        // turn off concurrent search to avoid Random object used across threads resulting into
        // RuntimeException, as WANDScorerQuery#createWeight has reference to this searcher,
        // but will be called during searching
        let searcher = new_searcher_with_threads(&reader, true, true, false)?;

        for _ in 0..100 {
            let start = random.random_range(0..10);
            let v = random.random_range(0..5);
            let num_clauses = random.random_range(0..1 << v);

            let mut builder = Builder::new();
            for i in 0..num_clauses {
                let tq = TermQuery::new(Term::from_text("foo", (start + i).to_string()));
                // TODO IMPORTANT 这里没有调用maybeWrap方法
                builder.add(tq, Occur::Should)?;
            }

            let query = Query::WANDScorer(WANDScorerQuery::new(
                builder.build(),
                random.random_bool(0.5),
            ));

            CheckHits::check_top_scores(&mut random, &query, &searcher)?;

            let filter_term = random.random_range(0..30);
            let filtered_query: Query = {
                let mut b = Builder::new();
                b.add(query, Occur::Must)?.add(
                    TermQuery::new(Term::from_text("foo", filter_term.to_string())),
                    Occur::Filter,
                )?;
                b.build().into()
            };

            CheckHits::check_top_scores(&mut random, &filtered_query, &searcher)?;
        }

        Ok(())
    }

    /// Degenerate case: all clauses produce a score of 0.
    #[test]
    fn test_random_with_zero_scores() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
        let num_docs = at_least(&mut random, 1000);
        for _ in 0..num_docs {
            let mut doc = Document::new();
            let v = random.random_range(0..5);
            let num_values = random.random_range(0..1 << v);
            let start = random.random_range(0..10);
            for j in 0..num_values {
                doc.add(StringField::from_string(
                    "foo",
                    (start + j).to_string(),
                    Store::No,
                )?);
            }
            w.add_document(doc)?;
        }

        let reader = directory_reader_util::open_with_writer(&w)?;
        w.close()?;

        // turn off concurrent search to avoid Random object used across threads resulting into
        // RuntimeException, as WANDScorerQuery#createWeight has reference to this searcher,
        // but will be called during searching
        let searcher = new_searcher_with_threads(&reader, true, true, false)?;

        for _ in 0..100 {
            let start = random.random_range(0..10);
            let v = random.random_range(0..5);
            let num_clauses = random.random_range(0..1 << v);

            let mut builder = Builder::new();
            for i in 0..num_clauses {
                let tq = TermQuery::new(Term::from_text("foo", (start + i).to_string()));
                let q: Query = BoostQuery::new(
                    Box::new(ConstantScoreQuery::new(Box::new(tq.into())).into()),
                    0.0,
                )?
                .into();
                // TODO IMPORTANT 这里没有调用maybeWrap方法
                builder.add(q, Occur::Should)?;
            }

            let query = Query::WANDScorer(WANDScorerQuery::new(
                builder.build(),
                random.random_bool(0.5),
            ));

            CheckHits::check_top_scores(&mut random, &query, &searcher)?;

            let filter_term = random.random_range(0..30);
            let filtered_query: Query = {
                let mut b = Builder::new();
                b.add(query, Occur::Must)?.add(
                    TermQuery::new(Term::from_text("foo", filter_term.to_string())),
                    Occur::Filter,
                )?;
                b.build().into()
            };

            CheckHits::check_top_scores(&mut random, &filtered_query, &searcher)?;
        }

        Ok(())
    }
    /// Test the case when some clauses produce infinite max scores.
    #[test]
    fn test_random_with_infinite_max_score() -> Result<()> {
        do_test_random_special_max_score(f32::INFINITY)
    }

    /// Test the case when some clauses produce finite max scores, but their sum overflows.
    #[test]
    fn test_random_with_max_score_overflow() -> Result<()> {
        do_test_random_special_max_score(f32::MAX)
    }

    fn do_test_random_special_max_score(max_score: f32) -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
        let num_docs = at_least(&mut random, 1000);
        for _ in 0..num_docs {
            let mut doc = Document::new();
            let v = random.random_range(0..5);
            let num_values = random.random_range(0..1 << v);
            let start = random.random_range(0..10);
            for j in 0..num_values {
                doc.add(StringField::from_string(
                    "foo",
                    (start + j).to_string(),
                    Store::No,
                )?);
            }
            w.add_document(doc)?;
        }

        let reader = directory_reader_util::open_with_writer(&w)?;
        w.close()?;

        // turn off concurrent search to avoid Random object used across threads resulting into
        // RuntimeException, as WANDScorerQuery::create_weight has reference to this searcher,
        // but will be called during searching
        let searcher = new_searcher_with_threads(&reader, true, true, false)?;

        for _ in 0..100 {
            let start = random.random_range(0..10);
            let v = random.random_range(0..5);
            let num_clauses = random.random_range(0..1 << v);

            let mut builder = Builder::new();
            for i in 0..num_clauses {
                let mut q: Query =
                    TermQuery::new(Term::from_text("foo", (start + i).to_string())).into();

                if random.random_bool(0.5) {
                    let denom = random.random_range(1..=100);
                    let max_range = (num_docs as i32) / denom;
                    q = Query::MaxScoreWrapper(MaxScoreWrapperQuery::new(q, max_range, max_score));
                }

                builder.add(q, Occur::Should)?;
            }

            let query = Query::WANDScorer(WANDScorerQuery::new(
                builder.build(),
                random.random_bool(0.5),
            ));

            CheckHits::check_top_scores(&mut random, &query, &searcher)?;

            let filter_term = random.random_range(0..30);
            let filtered_query: Query = {
                let mut b = Builder::new();
                b.add(query, Occur::Must)?.add(
                    TermQuery::new(Term::from_text("foo", filter_term.to_string())),
                    Occur::Filter,
                )?;
                b.build().into()
            };

            CheckHits::check_top_scores(&mut random, &filtered_query, &searcher)?;
        }

        Ok(())
    }

    struct MaxScoreWrapperScorer<S>
    where
        S: Scorer,
    {
        max_range: i32,
        max_score: f32,
        last_shallow_target: i32,
        scorer: S,
    }
    impl<S> MaxScoreWrapperScorer<S>
    where
        S: Scorer,
    {
        fn new(scorer: S, max_range: i32, max_score: f32) -> Self {
            Self {
                max_range,
                max_score,
                last_shallow_target: -1,
                scorer,
            }
        }
    }

    impl<S> Scorable for MaxScoreWrapperScorer<S>
    where
        S: Scorer,
    {
        fn score(&mut self) -> Result<f32> {
            self.scorer.score()
        }
    }

    impl<S> Scorer for MaxScoreWrapperScorer<S>
    where
        S: Scorer,
    {
        fn doc_id(&mut self) -> Result<i32> {
            self.scorer.doc_id()
        }

        fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
            self.scorer.iterator()
        }

        fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
            self.scorer.iterator_mut()
        }

        fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
            unreachable!()
        }

        fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
            self.scorer.two_phase_iterator()
        }

        fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
            self.scorer.two_phase_iterator_mut()
        }

        fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>>
        where
            Self: Sized,
        {
            unreachable!()
        }

        fn advance_shallow(&mut self, target: i32) -> Result<i32> {
            self.last_shallow_target = target;
            self.scorer.advance_shallow(target)
        }

        fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
            let v = self.doc_id()?.max(self.last_shallow_target);
            if up_to - v >= self.max_range {
                return Ok(self.max_score);
            }
            self.scorer.get_max_score(up_to)
        }

        fn has_two_phase_iterator(&self) -> TwoPhaseState {
            self.scorer.has_two_phase_iterator()
        }
    }

    #[derive(Clone, Debug)]
    pub struct MaxScoreWrapperQuery {
        query: Box<Query>,
        max_range: i32,
        max_score: f32,
        id: Identity,
    }
    impl MaxScoreWrapperQuery {
        fn new<T>(query: T, max_range: i32, max_score: f32) -> Self
        where
            T: Into<Box<Query>>,
        {
            let query = query.into();
            Self {
                query,
                max_range,
                max_score,
                id: Identity::new(),
            }
        }
    }

    impl HasIdentity for MaxScoreWrapperQuery {
        fn identity(&self) -> &Identity {
            &self.id
        }
    }
    impl Hash for MaxScoreWrapperQuery {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.query.hash(state);
            self.max_range.hash(state);
            self.max_score.to_bits().hash(state);
        }
    }
    impl Eq for MaxScoreWrapperQuery {}

    impl PartialEq for MaxScoreWrapperQuery {
        fn eq(&self, other: &Self) -> bool {
            self.query == other.query
                && self.max_range == other.max_range
                && self.max_score.total_cmp(&other.max_score).to_int() == 0
        }
    }

    impl QueryBase for MaxScoreWrapperQuery {
        fn as_string(&self, _field: &str) -> String {
            "MaxScoreWrapperQuery".to_string()
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
            let weight =
                self.query
                    .create_weight(searcher, score_mode, boost, per_reader_term_state)?;
            Ok(Box::new(MaxScoreWrapperQueryWeight::new(
                self.max_range,
                self.max_score,
                weight,
            )))
        }

        fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
        where
            IRC: IndexReaderContext,
            Self: Sized,
        {
            let rewritten = self.query.rewrite(searcher)?;
            Ok(Query::MaxScoreWrapper(MaxScoreWrapperQuery::new(
                rewritten,
                self.max_range,
                self.max_score,
            )))
        }

        fn visit<QV>(&self, _visitor: &QV)
        where
            QV: QueryVisitor,
        {
        }
    }
    struct MaxScoreWrapperQueryWeight<LR>
    where
        LR: LeafReader,
    {
        max_range: i32,
        max_score: f32,
        weight: QueryWeight<LR>,
    }
    impl<LR> MaxScoreWrapperQueryWeight<LR>
    where
        LR: LeafReader,
    {
        fn new(max_range: i32, max_score: f32, weight: QueryWeight<LR>) -> Self {
            Self {
                max_range,
                max_score,
                weight,
            }
        }
    }

    impl<LR> SegmentCacheable<LR> for MaxScoreWrapperQueryWeight<LR>
    where
        LR: LeafReader,
    {
        fn is_cacheable(&self, ctx: &LeafReaderContext<LR>) -> Result<bool> {
            self.weight.is_cacheable(ctx)
        }
    }

    impl<LR> Weight<LR> for MaxScoreWrapperQueryWeight<LR>
    where
        LR: LeafReader + 'static,
    {
        type Matches = MatchWithNoTerms;

        fn matches(
            &self,
            context: &LeafReaderContext<LR>,
            doc: i32,
        ) -> Result<Option<Self::Matches>> {
            self.weight.matches(context, doc)
        }

        fn explain(&self, context: &LeafReaderContext<LR>, doc: i32) -> Result<Explanation> {
            self.weight.explain(context, doc)
        }

        fn get_query(&self) -> Arc<Query> {
            self.weight.get_query()
        }

        type ScorerSupplier = QueryWeightSs<LR>;

        fn scorer_supplier(
            &self,
            context: &LeafReaderContext<LR>,
        ) -> Result<Option<Self::ScorerSupplier>> {
            match self.weight.scorer_supplier(context)? {
                Some(s) => Ok(Some(Box::new(ScorerSupplierImpl::new(
                    s,
                    self.max_range,
                    self.max_score,
                )))),
                None => Ok(None),
            }
        }
    }
    struct ScorerSupplierImpl<LR>
    where
        LR: LeafReader,
    {
        supplier: QueryWeightSs<LR>,
        max_range: i32,
        max_score: f32,
    }
    impl<LR> ScorerSupplierImpl<LR>
    where
        LR: LeafReader,
    {
        fn new(supplier: QueryWeightSs<LR>, max_range: i32, max_score: f32) -> Self {
            Self {
                supplier,
                max_range,
                max_score,
            }
        }
    }
    impl<LR> ScorerSupplier<LR> for ScorerSupplierImpl<LR>
    where
        LR: LeafReader,
    {
        type Scorer = QueryWeightSsScorer;
        type BulkScorer = QueryWeightSsBulkScorer;

        fn get(&mut self, lead_cost: i64, context: &LeafReaderContext<LR>) -> Result<Self::Scorer> {
            let v = self.supplier.get(lead_cost, context)?;
            let s = MaxScoreWrapperScorer::new(v, self.max_range, self.max_score);
            Ok(Box::new(s))
        }

        fn bulk_scorer(
            &mut self,
            context: &LeafReaderContext<LR>,
        ) -> Result<Option<Self::BulkScorer>> {
            Ok(Some(Box::new(self.default_bulk_scorer(context)?)))
        }

        fn cost(&mut self, context: &LeafReaderContext<LR>) -> Result<i64> {
            self.supplier.cost(context)
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct WANDScorerQuery {
        query: BooleanQuery,
        do_blocks: bool,
        id: Identity,
    }
    impl WANDScorerQuery {
        fn new(query: BooleanQuery, do_blocks: bool) -> Self {
            let id = Identity::new();
            assert_eq!(
                query.clauses().len(),
                query.get_clauses_idx(Occur::Should).len()
            );
            Self {
                query,
                do_blocks,
                id,
            }
        }
    }

    impl HasIdentity for WANDScorerQuery {
        fn identity(&self) -> &Identity {
            &self.id
        }
    }

    impl QueryBase for WANDScorerQuery {
        fn as_string(&self, _field: &str) -> String {
            "WANDScorerQuery".to_string()
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
            let w = self.query.clone().raw_weight(
                searcher,
                score_mode,
                boost,
                per_reader_term_state,
            )?;
            Ok(Box::new(WANDScorerQueryWeight::new(
                self.query,
                self.do_blocks,
                w,
                *score_mode,
            )))
        }

        fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
        where
            IRC: IndexReaderContext,
            Self: Sized,
        {
            Ok(Query::WANDScorer(self))
        }

        fn visit<QV>(&self, _visitor: &QV)
        where
            QV: QueryVisitor,
        {
        }
    }

    struct WANDScorerQueryWeight<LR>
    where
        LR: LeafReader + 'static,
    {
        minimum_number_should_match: i32,
        query: Arc<Query>,
        do_blocks: bool,
        weight: Rc<BooleanWeight<LR>>,
        score_mode: ScoreMode,
    }
    impl<LR> WANDScorerQueryWeight<LR>
    where
        LR: LeafReader,
    {
        fn new(
            query: BooleanQuery,
            do_blocks: bool,
            weight: BooleanWeight<LR>,
            score_mode: ScoreMode,
        ) -> Self {
            let minimum_number_should_match = query.get_minimum_number_should_match();
            let query = Arc::new(query.into());
            Self {
                minimum_number_should_match,
                query,
                do_blocks,
                weight: Rc::new(weight),
                score_mode,
            }
        }
    }

    impl<LR> SegmentCacheable<LR> for WANDScorerQueryWeight<LR>
    where
        LR: LeafReader,
    {
        fn is_cacheable(&self, _ctx: &LeafReaderContext<LR>) -> Result<bool> {
            Ok(false)
        }
    }

    impl<LR> Weight<LR> for WANDScorerQueryWeight<LR>
    where
        LR: LeafReader,
    {
        type Matches = MatchWithNoTerms;

        fn matches(
            &self,
            _context: &LeafReaderContext<LR>,
            _doc: i32,
        ) -> Result<Option<Self::Matches>> {
            unreachable!("")
        }

        fn explain(&self, _context: &LeafReaderContext<LR>, _doc: i32) -> Result<Explanation> {
            unreachable!("")
        }

        fn get_query(&self) -> Arc<Query> {
            self.query.clone()
        }

        type ScorerSupplier = QueryWeightSs<LR>;

        fn scorer_supplier(
            &self,
            context: &LeafReaderContext<LR>,
        ) -> Result<Option<Self::ScorerSupplier>> {
            let mut optional_scorers = Vec::new();
            for wc in self.weight.weighted_clauses.iter() {
                let w = &wc.weight;
                let ss = w.scorer_supplier(context)?;
                if let Some(mut ss) = ss {
                    let scorer = ss.get(i64::MAX, context)?;
                    optional_scorers.push(scorer);
                }
            }

            let scorer = if !optional_scorers.is_empty() {
                ScorerEnum2::A(WANDScorer::new(
                    optional_scorers,
                    self.minimum_number_should_match,
                    self.score_mode,
                    if self.do_blocks { i64::MAX } else { 0 },
                )?)
            } else {
                match self.weight.scorer(context)? {
                    Some(ss) => ScorerEnum2::B(ss),
                    None => return Ok(None),
                }
            };
            let v = DefaultScorerSupplier::new(scorer);
            Ok(Some(Box::new(v)))
        }
    }
}
