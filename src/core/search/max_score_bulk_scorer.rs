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
use crate::core::search::dummy::dummy_scorable::DummyScorable;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::search::scorer::Scorer;
use crate::core::util::CoreHelper;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::math_util::MathUtil;

const INNER_WINDOW_SIZE: i32 = 1 << 12;
pub struct MaxScoreBulkScorer<S>
where
    S: Scorer,
{
    max_doc: i32,
    all_scorers: Vec<DisiWrapper<S>>,
    // All scorers, sorted by increasing max score.
    pub(crate) all_scorers_idx: Vec<usize>,
    scratch: Vec<usize>,
    // These are the last scorers from `allScorers` that are "essential", ie. required for a match to
    // have a competitive score.
    essential_queue: DisiPriorityQueue,
    // Index of the first essential scorer, ie. essentialQueue contains all scorers from
    // allScorers[firstEssentialScorer:]. All scorers below this index are non-essential.
    pub(crate) first_essential_scorer: usize,
    pub(crate) first_required_scorer: usize,
    // The minimum value of minCompetitiveScore that would produce a more favorable partitioning.
    pub(crate) next_min_competitive_score: f32,
    cost: i64,
    pub(crate) min_competitive_score: f32,
    pub(crate) scorable: Score,
    pub(crate) max_score_sums: Vec<f64>,
    filter: Option<DisiWrapper<S>>,
    window_matches: Vec<u64>,
    window_scores: Vec<f64>,
    // Number of outer windows that have been evaluated
    num_outer_windows: usize,
    // Number of candidate matches so far
    num_candidates: usize,
    // Minimum window size. See #computeOuterWindowMax where we have heuristics that adjust the
    // minimum window size based on the average number of candidate matches per outer window, to keep
    // the per-window overhead under control.
    min_window_size: usize,
}
impl<S> MaxScoreBulkScorer<S>
where
    S: Scorer,
{
    pub fn new(max_doc: i32, scorers: Vec<S>, filter: Option<S>) -> Result<Self> {
        let filter = match filter {
            None => None,
            Some(f) => Some(DisiWrapper::new(f)?),
        };
        let mut all_scorers: Vec<DisiWrapper<S>> = Vec::with_capacity(scorers.len());
        let mut all_scorers_idx = Vec::with_capacity(scorers.len());
        let mut cost: i64 = 0;

        for (i, scorer) in scorers.into_iter().enumerate() {
            let w = DisiWrapper::new(scorer)?;
            cost += w.cost;
            all_scorers.push(w);
            all_scorers_idx.push(i);
        }
        let scratch = vec![0usize; all_scorers_idx.len()];
        let essential_queue = DisiPriorityQueue::new(all_scorers_idx.len() as i32);
        let max_score_sums = vec![0f64; all_scorers_idx.len()];
        let window_matches = vec![0u64; FixedBitSet::bits2words(INNER_WINDOW_SIZE) as usize];
        let window_scores = vec![0f64; INNER_WINDOW_SIZE as usize];

        Ok(Self {
            max_doc,
            all_scorers,
            all_scorers_idx,
            scratch,
            essential_queue,
            first_essential_scorer: 0,
            first_required_scorer: 0,
            next_min_competitive_score: 0.0,
            cost,
            min_competitive_score: 0.0,
            scorable: Score::new(),
            max_score_sums,
            filter,
            window_matches,
            window_scores,

            num_outer_windows: 0,
            num_candidates: 0,
            min_window_size: 1,
        })
    }
    /// allScorers = [ w0, w1, w2, ..., w(n-3), w(n-2), w(n-1) ]
    ///                                   ^       ^       ^
    ///                                   |       |       |
    ///                                block B  lead2   lead1
    /// ```
    fn score_inner_window_as_conjunction<LC, B>(
        &mut self,
        collector: &mut LC,
        accept_docs: Option<&B>,
        max: i32,
    ) -> Result<()>
    where
        LC: LeafCollector,
        B: Bits,
    {
        debug_assert!(self.first_essential_scorer == self.all_scorers_idx.len() - 1);
        debug_assert!(self.first_required_scorer <= self.all_scorers_idx.len() - 2);

        let n = self.all_scorers.len();
        let i1 = self.all_scorers_idx[n - 1];
        let i2 = self.all_scorers_idx[n - 2];

        let last = n - 1;
        let second_last = n - 2;

        let (other_and_lead2, lead1_slice) = self.all_scorers.split_at_mut(last);
        let lead1 = &mut lead1_slice[0];

        let (other, lead2_slice) = other_and_lead2.split_at_mut(second_last);
        let lead2 = &mut lead2_slice[0];

        debug_assert!(self.essential_queue.size() == 1);
        debug_assert!(self.essential_queue.top() == i1);

        if lead1.doc < lead2.doc {
            let v = lead1.iterator().advance(lead2.doc.min(max))?;
            lead1.doc = v;
        }

        let max_score_sum_at_lead2 = self.max_score_sums[n - 2];

        'outer: while lead1.doc < max {
            let accepted = match accept_docs {
                None => true,
                Some(bits) => bits.get(lead1.doc),
            };

            if !accepted {
                let v = lead1.iterator().next_doc()?;
                lead1.doc = v;
                continue;
            }

            let mut score = lead1.score()? as f64;

            if (MathUtil::sum_upper_bound(score + max_score_sum_at_lead2, n as i32) as f32)
                < self.min_competitive_score
            {
                let v = lead1.iterator().next_doc()?;
                lead1.doc = v;
                continue;
            }

            if lead2.doc < lead1.doc {
                let v = lead2.iterator().advance(lead1.doc)?;;
                lead2.doc = v;
            }
            if lead2.doc != lead1.doc {
                let v = lead1.iterator().advance(lead2.doc.min(max))?;
                lead1.doc = v;
                continue;
            }

            score += lead2.score()? as f64;

            for j in (self.all_scorers_idx.len() - 3..=self.first_required_scorer).rev() {

                if (MathUtil::sum_upper_bound(score + self.max_score_sums[j], n as i32) as f32)
                    < self.min_competitive_score
                {
                    let v = lead1.iterator().next_doc()?;
                    lead1.doc = v;
                    continue 'outer;
                }

                let w_index = self.all_scorers_idx[j];
                let w = &mut other[w_index];

                if w.doc < lead1.doc {
                    let v =  w.iterator().advance(lead1.doc)?;
                    w.doc = v;
                }
                if w.doc != lead1.doc {
                    let v = lead1.iterator().advance(w.doc.min(max))?;
                    lead1.doc = v;
                    continue 'outer;
                }

                score += w.score()? as f64;
            }
            let v = lead1.doc;
            self.score_non_essential_clauses(
                collector,
                v,
                score,
                self.first_required_scorer,
            )?;
            let v =  lead1.iterator().next_doc()?;
            lead1.doc = v;
        }

        Ok(())
    }


    fn score_inner_window_multiple_essential_clauses<LC, B>(
        &mut self,
        collector: &mut LC,
        accept_docs: Option<&B>,
        max: i32,
    ) -> Result<()>
    where
        LC: LeafCollector,
        B: Bits,
    {
        let top_index = self.essential_queue.top();
        let mut top = &mut self.all_scorers[top_index];

        let inner_window_min = top.doc;
        let inner_window_max = std::cmp::min(max, inner_window_min + INNER_WINDOW_SIZE);
        // Collect matches of essential clauses into a bitset
        loop {
            let mut doc = top.doc;
            while doc < inner_window_max {
                let accepted = match accept_docs {
                    None => true,
                    Some(bits) => bits.get(doc),
                };

                if accepted {
                    let i = (doc - inner_window_min) as usize;
                    self.window_matches[i >> 6] |= 1u64 << i;
                    self.window_scores[i] += top.score()? as f64;
                }
                doc = top.iterator().next_doc()?;
            }

            let doc_id = top.iterator().doc_id();
            top.doc = doc_id;
            let next_index = self.essential_queue.update_top(&self.all_scorers);
            top = &mut self.all_scorers[next_index];

            if top.doc >= inner_window_max {
                break;
            }
        }

        for word_index in 0..self.window_matches.len() {
            let mut bits = self.window_matches[word_index];
            self.window_matches[word_index] = 0;

            while bits != 0 {
                let ntz = bits.trailing_zeros() as usize;
                bits ^= 1u64 << ntz;

                let index = (word_index << 6) | ntz;
                let v:i32 = index.try_into()?;
                let doc = inner_window_min +v ;

                let score = self.window_scores[index];
                self.window_scores[index] = 0.0;

                self.score_non_essential_clauses(
                    collector,
                    doc,
                    score,
                    self.first_essential_scorer,
                )?;
            }
        }

        Ok(())
    }

    /// Only use essential scorers to compute the window's max doc ID, in order to avoid constantly
    /// recomputing max scores over small windows
    fn compute_outer_window_max(&mut self, window_min: i32) -> Result<i32> {
        let n = self.all_scorers_idx.len();
        let first_window_lead = self.first_essential_scorer.min(n - 1);

        let mut window_max = NO_MORE_DOCS;

        for i in first_window_lead..n {
            let index = self.all_scorers_idx[i];
            let scorer = &mut self.all_scorers[index];

            if self.filter.is_none() || scorer.cost >= self.filter.as_ref().unwrap().cost {
                let up_to = scorer.advance_shallow(scorer.doc.max(window_min))?;
                window_max = window_max.min(up_to + 1); // upTo is inclusive
            }
        }

        if n - first_window_lead > 1 {
            // The more clauses we consider to compute outer windows, the higher chances that one of these
            // clauses has a block boundary in the next few doc IDs. This situation can result in more
            // time spent computing maximum scores per outer window than evaluating hits. To avoid such
            // situations, we target at least 32 candidate matches per clause per outer window on average,
            // to make sure we amortize the cost of computing maximum scores.
            let threshold = self.num_outer_windows * 32 * n;
            if (self.num_candidates) < threshold {
                self.min_window_size = (self.min_window_size << 1).min(INNER_WINDOW_SIZE as usize);
            } else {
                self.min_window_size = 1;
            }
            let v: i32 = self.min_window_size.try_into()?;
            let min_window_max = (window_min + v).min(i32::MAX);
            window_max = window_max.max(min_window_max);
        }
        Ok(window_max)
    }
    fn update_max_window_scores(&mut self, window_min: i32, window_max: i32) -> Result<()> {
        for &idx in &self.all_scorers_idx {
            let w = &mut self.all_scorers[idx];

            if w.doc < window_max {
                if w.doc < window_min {
                    // Make sure to advance shallow if necessary to get as good score upper bounds as
                    // possible.
                    w.advance_shallow(window_min)?;
                }
                w.max_window_score = w.get_max_score(window_max - 1)?;
            } else {
                // This scorer has no documents in the considered window.
                w.max_window_score = 0.0;
            }
        }
        Ok(())
    }
    fn score_non_essential_clauses<LC>(
        &mut self,
        collector: &mut LC,
        doc: i32,
        essential_score: f64,
        num_non_essential_clauses: usize,
    ) -> Result<()>
    where
        LC: LeafCollector,
    {
        self.num_candidates += 1;

        let mut score = essential_score;
        for i in (0..num_non_essential_clauses).rev() {
            let max_possible_score = MathUtil::sum_upper_bound(
                score + self.max_score_sums[i],
                self.all_scorers_idx.len() as i32,
            ) as f32;

            if max_possible_score < self.min_competitive_score {
                // Hit is not competitive.
                return Ok(());
            }

            let index = self.all_scorers_idx[i];
            let w = &mut self.all_scorers[index];

            if w.doc < doc {
                let v = w.iterator().advance(doc)?;
                w.doc = v;
            }
            if w.doc == doc {
                score += w.score()? as f64;
            }
        }

        self.scorable.score = score as f32;
        collector.collect(doc, &mut self.scorable)?;
        Ok(())
    }

    /// Partitioning scorers is an optimization problem: the optimal set of non-essential scorers is
    /// the subset of scorers whose sum of max window scores is less than the minimum competitive
    /// score that maximizes the sum of costs.
    /// Computing the optimal solution to this problem would take O(2^num_clauses). As a first
    /// approximation, we take the first scorers sorted by max_window_score / cost whose sum of max
    /// scores is less than the minimum competitive scores. In the common case, maximum scores are
    /// inversely correlated with document frequency so this is the same as only sorting by maximum
    /// score, as described in the MAXSCORE paper and gives the optimal solution. However, this can
    /// make a difference when using custom scores (like FuzzyQuery), high query-time boosts, or
    /// scoring based on wacky weights.
    fn partition_scorers(&mut self) -> Result<bool> {
        for i in 0..self.all_scorers_idx.len() {
            self.scratch[i] = i;
        }

        self.scratch.sort_by(|&i1, &i2| {
            let w1 = &self.all_scorers[i1];
            let w2 = &self.all_scorers[i2];
            let s1 = w1.max_window_score as f64 / (w1.cost.max(1) as f64);
            let s2 = w2.max_window_score as f64 / (w2.cost.max(1) as f64);
            // s2 never be zero  so we could use `total_cmp` directly on the division result.
            s1.total_cmp(&s2)
        });

        let mut max_score_sum: f64 = 0.0;
        self.first_essential_scorer = 0;
        self.next_min_competitive_score = f32::INFINITY;

        let n = self.all_scorers_idx.len();

        for idx in 0..n {
            let index = self.all_scorers_idx[self.scratch[idx]];
            let w = &self.all_scorers[index];
            let new_max_score_sum = max_score_sum + w.max_window_score as f64;
            let v: i32 = self.first_essential_scorer.try_into()?;
            let max_score_sum_float = MathUtil::sum_upper_bound(new_max_score_sum, v + 1) as f32;

            if max_score_sum_float < self.min_competitive_score {
                max_score_sum = new_max_score_sum;
                self.all_scorers_idx[self.first_essential_scorer] = index;
                self.max_score_sums[self.first_essential_scorer] = max_score_sum;
                self.first_essential_scorer += 1;
            } else {
                let pos = n - 1 - (idx - self.first_essential_scorer);
                self.all_scorers_idx[pos] = index;
                self.next_min_competitive_score =
                    self.next_min_competitive_score.min(max_score_sum_float);
            }
        }

        self.first_required_scorer = n;

        if self.first_essential_scorer == n {
            return Ok(false);
        }

        self.essential_queue.clear();
        for i in self.first_essential_scorer..n {
            self.essential_queue
                .add(self.all_scorers_idx[i], &self.all_scorers);
        }

        if self.first_essential_scorer == n - 1 {
            // single essential clause
            // If there is a single essential clause and matching it plus all non-essential clauses but
            // the best one is not enough to yield a competitive match, the we know that hits must match
            // both the essential clause and the best non-essential clause. Here are some examples when
            // this optimization would kick in:
            //   `quick fox`  when maxscore(quick) = 1, maxscore(fox) = 1, minCompetitiveScore = 1.5
            //   `the quick fox` when maxscore (the) = 0.1, maxscore(quick) = 1, maxscore(fox) = 1,
            //       minCompetitiveScore = 1.5
            self.first_required_scorer = n - 1;
            let mut max_required_score = self.all_scorers
                [self.all_scorers_idx[self.first_essential_scorer]]
                .max_window_score as f64;

            while self.first_required_scorer > 0 {
                let mut max_possible_score_without_previous = max_required_score;

                if self.first_required_scorer > 1 {
                    max_possible_score_without_previous +=
                        self.max_score_sums[self.first_required_scorer - 2];
                }

                if (max_possible_score_without_previous as f32) >= self.min_competitive_score {
                    break;
                }
                // The sum of maximum scores ignoring the previous clause is less than the minimum
                // competitive
                self.first_required_scorer -= 1;
                max_required_score += self.all_scorers
                    [self.all_scorers_idx[self.first_required_scorer]]
                    .max_window_score as f64;
            }
        }

        Ok(true)
    }

    /// Return the next candidate on or after `rangeEnd`.
    fn next_candidate(&self, range_end: i32) -> i32 {
        if range_end >= self.max_doc {
            return NO_MORE_DOCS;
        }

        let mut next = NO_MORE_DOCS;
        for w in &self.all_scorers_idx {
            let w = &self.all_scorers[*w];
            if w.doc < range_end {
                return range_end;
            } else {
                next = next.min(w.doc);
            }
        }
        next
    }
}

pub struct Score {
    score: f32,
}
impl Score {
    fn new() -> Self {
        Self { score: 0.0 }
    }
}
impl Scorable for Score {
    fn score(&mut self) -> Result<f32> {
        todo!()
    }

    type Scorable = DummyScorable;
}
