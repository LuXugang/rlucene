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
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::disi_priority_queue::DisiPriorityQueue;
use crate::core::search::disi_wrapper::DisiWrapper;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::dummy::dummy_scorer::DummyScorer;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::scorer::Scorer;
use crate::core::search::scorer_util::ScorerUtil;
use crate::core::util::TryIntoInt;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::math_util::MathUtil;

pub(crate) const INNER_WINDOW_SIZE: i32 = 1 << 12;
pub struct MaxScoreBulkScorer<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  max_doc: i32,
  all_scorers: Vec<DisiWrapper<S1>>,
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
  pub(crate) scorable: Score,
  pub(crate) max_score_sums: Vec<f64>,
  filter: Option<DisiWrapper<S2>>,
  window_matches: Vec<u64>,
  window_scores: Vec<f64>,
  // Number of outer windows that have been evaluated
  num_outer_windows: usize,
  // Number of candidate matches so far
  num_candidates: usize,
  // Minimum window size. See `compute_outer_window_max`, which adjusts the
  // minimum window size based on the average number of candidate matches per outer window, to keep
  // the per-window overhead under control.
  min_window_size: i32,
}
impl<S1> MaxScoreBulkScorer<S1, DummyScorer>
where
  S1: Scorer,
{
  pub fn with_no_filter(max_doc: i32, scorers: Vec<S1>) -> Result<Self> {
    Self::new(max_doc, scorers, None)
  }
}

impl<S1, S2> MaxScoreBulkScorer<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  pub fn new(max_doc: i32, scorers: Vec<S1>, filter: Option<S2>) -> Result<Self> {
    let filter = match filter {
      None => None,
      Some(f) => Some(DisiWrapper::new(f)?),
    };
    let mut all_scorers: Vec<DisiWrapper<S1>> = Vec::with_capacity(scorers.len());
    let mut all_scorers_idx = Vec::with_capacity(scorers.len());
    let mut cost: i64 = 0;

    for (i, scorer) in scorers.into_iter().enumerate() {
      let w = DisiWrapper::new(scorer)?;
      cost += w.cost;
      all_scorers.push(w);
      all_scorers_idx.push(i);
    }
    let scratch = vec![0usize; all_scorers_idx.len()];
    let essential_queue = DisiPriorityQueue::new(all_scorers_idx.len());
    let max_score_sums = vec![0f64; all_scorers_idx.len()];
    let window_matches = vec![0u64; FixedBitSet::bits2words(INNER_WINDOW_SIZE as usize)];
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
      scorable: Score::new(),
      max_score_sums,
      filter,
      window_matches,
      window_scores,

      num_outer_windows: 0,
      num_candidates: 0,
      min_window_size: -1,
    })
  }
  fn score_inner_window(
    &mut self,
    collector: &mut dyn LeafCollector,
    accept_docs: Option<&dyn Bits>,
    max: i32,
  ) -> Result<()> {
    if self.filter.is_some() {
      let mut filter = self.filter.take().unwrap();
      self.score_inner_window_with_filter(collector, accept_docs, max, &mut filter)?;
      self.filter = Some(filter);
    } else if self.all_scorers_idx.len() - self.first_required_scorer >= 2 {
      self.score_inner_window_as_conjunction(collector, accept_docs, max)?;
    } else {
      let top_index = self
        .essential_queue
        .top()
        .ok_or_else(|| LuceneError::illegal_state("no top available"))?;
      let top2_index_opt = self.essential_queue.top2(&self.all_scorers);

      match top2_index_opt {
        Some(top2_index) => {
          let top = &self.all_scorers[top_index];
          let top2 = &self.all_scorers[top2_index];

          if top2.doc - (INNER_WINDOW_SIZE / 2) >= top.doc {
            self.score_inner_window_single_essential_clause(
              collector,
              accept_docs,
              max.min(top2.doc),
            )?;
          } else {
            self.score_inner_window_multiple_essential_clauses(collector, accept_docs, max)?;
          }
        },
        None => {
          self.score_inner_window_single_essential_clause(collector, accept_docs, max)?;
        },
      }
    }

    Ok(())
  }

  fn score_inner_window_with_filter(
    &mut self,
    collector: &mut dyn LeafCollector,
    accept_docs: Option<&dyn Bits>,
    max: i32,
    filter: &mut DisiWrapper<S2>,
  ) -> Result<()> {
    let mut top_index = self
      .essential_queue
      .top()
      .ok_or_else(|| LuceneError::illegal_state("no top available"))?;
    {
      let top = &self.all_scorers[top_index];
      debug_assert!(top.doc < max);
    }

    let filter_doc = filter.doc;
    {
      let top = &mut self.all_scorers[top_index];
      if top.doc < filter_doc {
        let v = ScorerUtil::advance(&mut top.scorer, filter_doc)?;
        top.doc = v;
      }
    }

    let inner_window_min = self.all_scorers[top_index].doc;
    let inner_window_max = std::cmp::min(max, inner_window_min + INNER_WINDOW_SIZE);
    while self.all_scorers[top_index].doc < inner_window_max {
      let top_doc = self.all_scorers[top_index].doc;
      debug_assert!(filter.doc <= top_doc);

      if filter.doc < top_doc {
        let v = ScorerUtil::advance(&mut filter.scorer, top_doc)?;
        filter.doc = v;
      }

      if filter.doc != self.all_scorers[top_index].doc {
        loop {
          let fdoc = filter.doc;
          {
            let top = &mut self.all_scorers[top_index];
            let v = top.scorer.iterator_mut().advance(fdoc)?;
            top.doc = v;
          }
          top_index = self.essential_queue.update_top(&self.all_scorers);
          if self.all_scorers[top_index].doc >= filter.doc {
            break;
          }
        }
      } else {
        let doc = self.all_scorers[top_index].doc;
        let match_ = {
          let accepted = match accept_docs {
            None => true,
            Some(bits) => bits.get(doc as usize)?,
          };
          accepted && filter.matches_may_none()?
        };

        let mut score = 0f64;
        loop {
          if match_ {
            let s = {
              let top = &mut self.all_scorers[top_index];
              top.scorer.score()? as f64
            };
            score += s;
          }

          {
            let top = &mut self.all_scorers[top_index];
            let v = top.scorer.iterator_mut().next_doc()?;
            top.doc = v;
          }
          top_index = self.essential_queue.update_top(&self.all_scorers);

          if self.all_scorers[top_index].doc != doc {
            break;
          }
        }

        if match_ {
          self.score_non_essential_clauses(collector, doc, score, self.first_essential_scorer)?;
        }
      }
    }

    Ok(())
  }
  fn score_inner_window_single_essential_clause(
    &mut self,
    collector: &mut dyn LeafCollector,
    accept_docs: Option<&dyn Bits>,
    upto: i32,
  ) -> Result<()> {
    let top_index = self
      .essential_queue
      .top()
      .ok_or_else(|| LuceneError::illegal_state("no top available"))?;
    let mut doc = {
      let top = &mut self.all_scorers[top_index];
      // single essential clause in this window, we can iterate it directly and skip the bitset.
      // this is a common case for 2-clauses queries
      top.doc
    };

    while doc < upto {
      let accepted = match accept_docs {
        None => true,
        Some(bits) => bits.get(doc as usize)?,
      };

      if accepted {
        let score = {
          let top = &mut self.all_scorers[top_index];
          top.scorer.score()?
        };

        self.score_non_essential_clauses(
          collector,
          doc,
          score as f64,
          self.first_essential_scorer,
        )?;
      }

      let top = &mut self.all_scorers[top_index];
      doc = top.scorer.iterator_mut().next_doc()?;
    }
    let top = &mut self.all_scorers[top_index];
    let v = top.scorer.iterator_mut().doc_id();
    top.doc = v;
    self.essential_queue.update_top(&self.all_scorers);

    Ok(())
  }

  /// allScorers = [ w0, w1, w2, ..., w(n-3), w(n-2), w(n-1) ]
  ///                                   ^       ^       ^
  ///                                   |       |       |
  ///                                block B  lead2   lead1
  fn score_inner_window_as_conjunction(
    &mut self,
    collector: &mut dyn LeafCollector,
    accept_docs: Option<&dyn Bits>,
    max: i32,
  ) -> Result<()> {
    debug_assert!(self.first_essential_scorer == self.all_scorers_idx.len() - 1);
    debug_assert!(self.first_required_scorer <= self.all_scorers_idx.len() - 2);

    let all_scorers_len = self.all_scorers_idx.len();

    let leader1_idx = self.all_scorers_idx[all_scorers_len - 1];
    let leader2_idx = self.all_scorers_idx[all_scorers_len - 2];
    let (mut doc, max_score_sum_at_lead2) = {
      debug_assert!(self.essential_queue.size() == 1);
      let essential_top = self
        .essential_queue
        .top()
        .ok_or_else(|| LuceneError::illegal_state("no top available"))?;
      debug_assert!(essential_top == leader1_idx);

      if self.all_scorers[leader1_idx].doc < self.all_scorers[leader2_idx].doc {
        let target = self.all_scorers[leader2_idx].doc.min(max);
        let v = self.all_scorers[leader1_idx]
          .scorer
          .iterator_mut()
          .advance(target)?;
        self.all_scorers[leader1_idx].doc = v;
      }
      (
        self.all_scorers[leader1_idx].doc,
        self.max_score_sums[all_scorers_len - 2],
      )
    };
    // TODO IMPORTANT能否降低iterator()方法的调用次数
    'outer: while doc < max {
      let (v, score) = {
        let accepted = match accept_docs {
          None => true,
          Some(bits) => bits.get(self.all_scorers[leader1_idx].doc as usize)?,
        };

        if !accepted {
          let v = self.all_scorers[leader1_idx]
            .scorer
            .iterator_mut()
            .next_doc()?;
          self.all_scorers[leader1_idx].doc = v;
          doc = v;
          continue;
        }

        let mut score = self.all_scorers[leader1_idx].scorer.score()? as f64;

        if (MathUtil::sum_upper_bound(score + max_score_sum_at_lead2, all_scorers_len as i32)
          as f32)
          < self.scorable.min_competitive_score
        {
          let v = self.all_scorers[leader1_idx]
            .scorer
            .iterator_mut()
            .next_doc()?;
          self.all_scorers[leader1_idx].doc = v;
          doc = v;
          continue;
        }

        if self.all_scorers[leader2_idx].doc < self.all_scorers[leader1_idx].doc {
          let target = self.all_scorers[leader1_idx].doc;
          let v = self.all_scorers[leader2_idx]
            .scorer
            .iterator_mut()
            .advance(target)?;
          self.all_scorers[leader2_idx].doc = v;
        }
        if self.all_scorers[leader2_idx].doc != self.all_scorers[leader1_idx].doc {
          let target = self.all_scorers[leader2_idx].doc.min(max);
          let v = self.all_scorers[leader1_idx]
            .scorer
            .iterator_mut()
            .advance(target)?;
          self.all_scorers[leader1_idx].doc = v;
          doc = v;
          continue;
        }

        score += self.all_scorers[leader2_idx].scorer.score()? as f64;

        if self.all_scorers_idx.len() >= 3 {
          for j in (self.first_required_scorer..=self.all_scorers_idx.len() - 3).rev() {
            if (MathUtil::sum_upper_bound(score + self.max_score_sums[j], all_scorers_len as i32)
              as f32)
              < self.scorable.min_competitive_score
            {
              let v = self.all_scorers[leader1_idx]
                .scorer
                .iterator_mut()
                .next_doc()?;
              self.all_scorers[leader1_idx].doc = v;
              doc = v;
              continue 'outer;
            }

            let leader_1_doc = self.all_scorers[leader1_idx].doc;
            let w = &mut self.all_scorers[j];
            if w.doc < leader_1_doc {
              let v = w.scorer.iterator_mut().advance(leader_1_doc)?;
              w.doc = v;
            }
            let w_doc = w.doc;
            if w_doc != leader_1_doc {
              let v = self.all_scorers[leader1_idx]
                .scorer
                .iterator_mut()
                .advance(w_doc.min(max))?;
              self.all_scorers[leader1_idx].doc = v;
              doc = v;
              continue 'outer;
            }

            score += self.all_scorers[j].scorer.score()? as f64;
          }
        }
        (self.all_scorers[leader1_idx].doc, score)
      };

      self.score_non_essential_clauses(collector, v, score, self.first_required_scorer)?;
      let lead1 = &mut self.all_scorers[leader1_idx];
      let v = lead1.scorer.iterator_mut().next_doc()?;
      doc = v;
      lead1.doc = v;
    }

    Ok(())
  }

  fn score_inner_window_multiple_essential_clauses(
    &mut self,
    collector: &mut dyn LeafCollector,
    accept_docs: Option<&dyn Bits>,
    max: i32,
  ) -> Result<()> {
    let top_index = self
      .essential_queue
      .top()
      .ok_or_else(|| LuceneError::illegal_state("no top available"))?;
    let mut top = &mut self.all_scorers[top_index];

    let inner_window_min = top.doc;
    let inner_window_max = std::cmp::min(max, inner_window_min + INNER_WINDOW_SIZE);
    // Collect matches of essential clauses into a bitset
    loop {
      let mut doc = top.doc;
      while doc < inner_window_max {
        let accepted = match accept_docs {
          None => true,
          Some(bits) => bits.get(doc as usize)?,
        };

        if accepted {
          let i = (doc - inner_window_min) as usize;
          self.window_matches[i >> 6] |= 1u64 << i;
          self.window_scores[i] += top.scorer.score()? as f64;
        }
        doc = top.scorer.iterator_mut().next_doc()?;
      }

      let doc_id = top.scorer.iterator_mut().doc_id();
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
        let v: i32 = index.try_convert()?;
        let doc = inner_window_min + v;

        let score = self.window_scores[index];
        self.window_scores[index] = 0.0;

        self.score_non_essential_clauses(collector, doc, score, self.first_essential_scorer)?;
      }
    }

    Ok(())
  }

  /// Only use essential scorers to compute the window's max doc ID, in order to avoid constantly
  /// recomputing max scores over small windows
  fn compute_outer_window_max(&mut self, window_min: i32) -> Result<i32> {
    let all_scorers_len = self.all_scorers_idx.len();
    let first_window_lead = self.first_essential_scorer.min(all_scorers_len - 1);

    let mut window_max = NO_MORE_DOCS;

    for i in first_window_lead..all_scorers_len {
      let index = self.all_scorers_idx[i];
      let scorer = &mut self.all_scorers[index];

      if self.filter.is_none() || scorer.cost >= self.filter.as_ref().unwrap().cost {
        let upto = scorer.scorer.advance_shallow(scorer.doc.max(window_min))? as i64;
        window_max = (window_max as i64).min(upto + 1) as i32; // upTo is inclusive
      }
    }

    if all_scorers_len - first_window_lead > 1 {
      // The more clauses we consider to compute outer windows, the higher chances that one of these
      // clauses has a block boundary in the next few doc IDs. This situation can result in more
      // time spent computing maximum scores per outer window than evaluating hits. To avoid such
      // situations, we target at least 32 candidate matches per clause per outer window on average,
      // to make sure we amortize the cost of computing maximum scores.
      let threshold = self.num_outer_windows * 32 * all_scorers_len;
      if (self.num_candidates) < threshold {
        self.min_window_size = (self.min_window_size << 1).min(INNER_WINDOW_SIZE);
      } else {
        self.min_window_size = 1;
      }
      let v = window_min as i64 + self.min_window_size as i64;
      let min_window_max = (i32::MAX as i64).min(v) as i32;
      window_max = window_max.max(min_window_max);
    }
    Ok(window_max)
  }
  pub(crate) fn update_max_window_scores(
    &mut self,
    window_min: i32,
    window_max: i32,
  ) -> Result<()> {
    for &idx in &self.all_scorers_idx {
      let scorer = &mut self.all_scorers[idx];

      if scorer.doc < window_max {
        if scorer.doc < window_min {
          // Make sure to advance shallow if necessary to get as good score upper bounds as
          // possible.
          scorer.scorer.advance_shallow(window_min)?;
        }
        scorer.max_window_score = scorer.scorer.get_max_score(window_max - 1)?;
      } else {
        // This scorer has no documents in the considered window.
        scorer.max_window_score = 0.0;
      }
    }
    Ok(())
  }
  fn score_non_essential_clauses(
    &mut self,
    collector: &mut dyn LeafCollector,
    doc: i32,
    essential_score: f64,
    num_non_essential_clauses: usize,
  ) -> Result<()> {
    self.num_candidates += 1;

    let mut score = essential_score;
    for i in (0..num_non_essential_clauses).rev() {
      let max_possible_score = MathUtil::sum_upper_bound(
        score + self.max_score_sums[i],
        self.all_scorers_idx.len() as i32,
      ) as f32;

      if max_possible_score < self.scorable.min_competitive_score {
        // Hit is not competitive.
        return Ok(());
      }

      let index = self.all_scorers_idx[i];
      let scorer = &mut self.all_scorers[index];

      if scorer.doc < doc {
        let v = scorer.scorer.iterator_mut().advance(doc)?;
        scorer.doc = v;
      }
      if scorer.doc == doc {
        score += scorer.scorer.score()? as f64;
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
  pub(crate) fn partition_scorers(&mut self) -> Result<bool> {
    for i in 0..self.all_scorers_idx.len() {
      self.scratch[i] = self.all_scorers_idx[i];
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
      let index = self.scratch[idx];
      let w = &self.all_scorers[index];
      let new_max_score_sum = max_score_sum + w.max_window_score as f64;
      let v: i32 = self.first_essential_scorer.try_convert()?;
      let max_score_sum_float = MathUtil::sum_upper_bound(new_max_score_sum, v + 1) as f32;

      if max_score_sum_float < self.scorable.min_competitive_score {
        max_score_sum = new_max_score_sum;
        self.all_scorers_idx[self.first_essential_scorer] = index;
        self.max_score_sums[self.first_essential_scorer] = max_score_sum;
        self.first_essential_scorer += 1;
      } else {
        let pos = n - 1 - (idx - self.first_essential_scorer);
        self.all_scorers_idx[pos] = index;
        self.next_min_competitive_score = self.next_min_competitive_score.min(max_score_sum_float);
      }
    }

    self.first_required_scorer = n;

    if self.first_essential_scorer == n {
      return Ok(false);
    }

    self.essential_queue.clear();
    for i in self.first_essential_scorer..n {
      self
        .essential_queue
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
      let mut max_required_score =
        self.all_scorers[self.all_scorers_idx[self.first_essential_scorer]].max_window_score as f64;

      while self.first_required_scorer > 0 {
        let mut max_possible_score_without_previous = max_required_score;

        if self.first_required_scorer > 1 {
          max_possible_score_without_previous +=
            self.max_score_sums[self.first_required_scorer - 2];
        }

        if (max_possible_score_without_previous as f32) >= self.scorable.min_competitive_score {
          break;
        }
        // The sum of maximum scores ignoring the previous clause is less than the minimum
        // competitive
        self.first_required_scorer -= 1;
        max_required_score += self.all_scorers[self.all_scorers_idx[self.first_required_scorer]]
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
impl<S1, S2> BulkScorer for MaxScoreBulkScorer<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  fn score(
    &mut self,
    collector: &mut dyn LeafCollector,
    accept_docs: Option<&dyn Bits>,
    min: i32,
    max: i32,
  ) -> Result<i32> {
    collector.set_scorer(&mut self.scorable)?;

    // This scorer computes outer windows based on impacts that are stored in the index. These outer
    // windows should be small enough to provide good upper bounds of scores, and big enough to make
    // sure we spend more time collecting docs than recomputing windows.
    // Then within these outer windows, it creates inner windows of size WINDOW_SIZE that help
    // collect matches into a bitset and save the overhead of rebalancing the priority queue on
    // every match.

    let mut outer_window_min = min;

    'outer: while outer_window_min < max {
      let mut outer_window_max = self.compute_outer_window_max(outer_window_min)?;
      outer_window_max = outer_window_max.min(max);

      loop {
        self.update_max_window_scores(outer_window_min, outer_window_max)?;

        if !self.partition_scorers()? {
          // No matches in this window
          outer_window_min = outer_window_max;
          continue 'outer;
        }

        // There is a dependency between windows and maximum scores, as we compute windows based on
        // maximum scores and maximum scores based on windows.
        // So the approach consists of starting by computing a window based on the set of essential
        // scorers from the _previous_ window and then iteratively recompute maximum scores and
        // windows as long as the window size decreases.
        // In general the set of essential scorers is rather stable over time so this would exit
        // after a single iteration, but there is a change that some scorers got swapped between the
        // set of essential and non-essential scorers, in which case there may be multiple
        // iterations of this loop.

        let new_outer_window_max = self.compute_outer_window_max(outer_window_min)?;
        if new_outer_window_max >= outer_window_max {
          break;
        }
        outer_window_max = new_outer_window_max;
      }

      let mut top_index = self
        .essential_queue
        .top()
        .ok_or_else(|| LuceneError::illegal_state("no top available"))?;
      {
        let mut doc = self.all_scorers[top_index].doc;
        while doc < outer_window_min {
          {
            let top = &mut self.all_scorers[top_index];
            let v = top.scorer.iterator_mut().advance(outer_window_min)?;
            top.doc = v;
          }
          top_index = self.essential_queue.update_top(&self.all_scorers);
          doc = self.all_scorers[top_index].doc;
        }
      }

      let mut top_doc = self.all_scorers[top_index].doc;

      while top_doc < outer_window_max {
        self.score_inner_window(collector, accept_docs, outer_window_max)?;
        top_index = self
          .essential_queue
          .top()
          .ok_or_else(|| LuceneError::illegal_state("no top available"))?;
        top_doc = self.all_scorers[top_index].doc;

        if self.scorable.min_competitive_score >= self.next_min_competitive_score {
          // The minimum competitive score increased substantially, so we can now partition scorers
          // in a more favorable way.
          break;
        }
      }
      outer_window_min = std::cmp::min(top_doc, outer_window_max);
      self.num_outer_windows += 1;
    }

    Ok(self.next_candidate(max))
  }

  fn cost(&mut self) -> Result<i64> {
    Ok(self.cost)
  }
}

pub struct Score {
  score: f32,
  pub(crate) min_competitive_score: f32,
}
impl Score {
  fn new() -> Self {
    Self {
      score: 0.0,
      min_competitive_score: 0.0,
    }
  }
}
impl Scorable for Score {
  fn score(&mut self) -> Result<f32> {
    Ok(self.score)
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    self.min_competitive_score = min_score;
    Ok(())
  }

  fn cost(&self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl FixedScore for Score {}
