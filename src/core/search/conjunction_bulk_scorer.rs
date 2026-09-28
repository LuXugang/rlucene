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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::search::scorer::Scorer;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// BulkScorer implementation of [`ConjunctionScorer`](crate::core::search::conjunction_scorer::ConjunctionScorer).
///
/// For simplicity, it focuses on scorers that produce regular
/// [`DocIdSetIterator`]s rather than [`TwoPhaseIterator`](crate::core::search::two_phase_iterator::TwoPhaseIterator)s.
pub struct ConjunctionBulkScorer<S> {
  // lead1: all_scores[0]
  // lead2: all_scores[1]
  all_scores: Vec<S>,
  required_scoring_idx: Vec<usize>,
  all_scores_input_idx: Box<[usize]>,
}
impl<S> ConjunctionBulkScorer<S>
where
  S: Scorer,
{
  pub(crate) fn new(required_scoring: Vec<S>, required_no_scoring: Vec<S>) -> Result<Self> {
    let required_scoring_len = required_scoring.len();
    let num_clauses = required_scoring_len + required_no_scoring.len();
    if num_clauses <= 1 {
      return Err(LuceneError::illegal_argument(format!(
        "Expected 2 or more clauses, got {num_clauses}"
      )));
    }
    let mut costs = Vec::with_capacity(num_clauses);
    let mut i = 0usize;
    let mut tmp_all_scores = Vec::with_capacity(num_clauses);
    for scorer in required_scoring.into_iter() {
      costs.push((DocIdSetIterator::cost(&scorer)?, true, i));
      tmp_all_scores.push(Some(scorer));
      i += 1;
    }

    for scorer in required_no_scoring.into_iter() {
      costs.push((DocIdSetIterator::cost(&scorer)?, false, i));
      tmp_all_scores.push(Some(scorer));
      i += 1;
    }

    costs.sort_by_key(|a| a.0);

    let mut all_scores = Vec::with_capacity(num_clauses);
    let mut all_scores_input_idx = vec![0; num_clauses].into_boxed_slice();
    let mut required_scoring_idx = Vec::with_capacity(required_scoring_len);
    for (_, is_required_score, idx) in costs {
      let scorer = tmp_all_scores[idx]
        .take()
        .ok_or_else(|| LuceneError::illegal_state("scorer is missing"))?;
      all_scores_input_idx[idx] = all_scores.len();
      all_scores.push(scorer);
      if is_required_score {
        required_scoring_idx.push(all_scores.len() - 1);
      }
    }

    Ok(Self {
      all_scores,
      required_scoring_idx,
      all_scores_input_idx,
    })
  }
}

impl<S> BulkScorer for ConjunctionBulkScorer<S>
where
  S: Scorer,
{
  fn score(
    &mut self,
    collector: &mut dyn LeafCollector,
    accept_docs: Option<&dyn Bits>,
    min: i32,
    max: i32,
  ) -> Result<i32> {
    let (mut lead1_doc_id, lead2_doc_id) = {
      let (first, rest) = self.all_scores.split_at_mut(1);
      let lead1 = &mut first[0];

      let (second, _) = rest.split_at_mut(1);
      let lead2 = &mut second[0];
      debug_assert!({ DocIdSetIterator::doc_id(&lead1) >= DocIdSetIterator::doc_id(&lead2) });

      if DocIdSetIterator::doc_id(&lead1) < min {
        lead1.advance(min)?;
      }
      if DocIdSetIterator::doc_id(&lead1) >= max {
        return Ok(DocIdSetIterator::doc_id(&lead1));
      }
      (
        DocIdSetIterator::doc_id(&lead1),
        DocIdSetIterator::doc_id(&lead2),
      )
    };
    collector.set_scorer(&mut ScorableImpl::new(self))?;

    // In the main loop, we rely on the invariant that `DocIdSetIterator::doc_id(&lead1)` is greater than
    // lead2.doc(). However it's possible that these two are equal on the first document in a
    // scoring window. So we treat this case separately here.
    if lead1_doc_id == lead2_doc_id {
      let doc = lead1_doc_id;
      if match accept_docs {
        None => true,
        Some(bits) => bits.get(doc as usize)?,
      } {
        let mut matched = true;
        {
          let (first, rest) = self.all_scores.split_at_mut(1);
          let lead1 = &mut first[0];
          let (_, other_scorers) = rest.split_at_mut(1);

          let competitive_iterator = collector.competitive_iterator()?;
          let others = other_scorers
            .iter_mut()
            .map(|scorer| scorer as &mut dyn DocIdSetIterator)
            .chain(competitive_iterator);

          for it in others {
            if DocIdSetIterator::doc_id(&it) < doc {
              let next = it.advance(doc)?;
              if next != doc {
                lead1.advance(next)?;
                matched = false;
                break;
              }
            }
            debug_assert!(DocIdSetIterator::doc_id(&it) == doc);
          }
          lead1_doc_id = DocIdSetIterator::doc_id(&lead1);
        }

        if matched {
          collector.collect(doc, &mut ScorableImpl::new(self))?;
          let (first, _) = self.all_scores.split_at_mut(1);
          let lead1 = &mut first[0];
          lead1.next_doc()?;
          lead1_doc_id = DocIdSetIterator::doc_id(&lead1);
        }
      } else {
        let (first, _) = self.all_scores.split_at_mut(1);
        let lead1 = &mut first[0];
        lead1.next_doc()?;
        lead1_doc_id = DocIdSetIterator::doc_id(&lead1);
      }
    }

    let mut doc = lead1_doc_id;

    'advance_head: while doc < max {
      {
        let (first, rest) = self.all_scores.split_at_mut(1);
        let lead1 = &mut first[0];
        let (second, other_scorers) = rest.split_at_mut(1);
        let lead2 = &mut second[0];

        debug_assert!(DocIdSetIterator::doc_id(&lead2) < doc);

        if match accept_docs {
          None => false,
          Some(bits) => !bits.get(doc as usize)?,
        } {
          doc = lead1.next_doc()?;
          continue;
        }
        // We maintain `DocIdSetIterator::doc_id(&lead2) < DocIdSetIterator::doc_id(&lead1)` so that we do not need to check
        // if lead2 is already on the same doc as lead1 here.
        let next2 = lead2.advance(doc)?;
        if next2 != doc {
          doc = lead1.advance(next2)?;
          if doc != next2 {
            continue;
          } else if doc >= max {
            break;
          } else if match accept_docs {
            None => false,
            Some(bits) => !bits.get(doc as usize)?,
          } {
            doc = lead1.next_doc()?;
            continue;
          }
        }
        debug_assert!(DocIdSetIterator::doc_id(&lead2) == doc);

        let competitive_iterator = collector.competitive_iterator()?;
        let others = other_scorers
          .iter_mut()
          .map(|scorer| scorer as &mut dyn DocIdSetIterator)
          .chain(competitive_iterator);

        for it in others {
          if DocIdSetIterator::doc_id(&it) < doc {
            let next = it.advance(doc)?;
            if next != doc {
              doc = lead1.advance(next)?;
              continue 'advance_head;
            }
          }
          debug_assert!(DocIdSetIterator::doc_id(&it) == doc);
        }
      }
      collector.collect(doc, &mut ScorableImpl::new(self))?;
      let (first, _) = self.all_scores.split_at_mut(1);
      let lead1 = &mut first[0];
      doc = lead1.next_doc()?;
    }
    let (first, _) = self.all_scores.split_at_mut(1);
    let lead1 = &mut first[0];
    Ok(DocIdSetIterator::doc_id(&lead1))
  }

  fn cost(&mut self) -> Result<i64> {
    DocIdSetIterator::cost(&self.all_scores[0])
  }
}

struct ScorableImpl<'a, S> {
  base: &'a mut ConjunctionBulkScorer<S>,
}
impl<'a, S> ScorableImpl<'a, S> {
  fn new(base: &'a mut ConjunctionBulkScorer<S>) -> Self {
    Self { base }
  }
}
impl<S> Scorable for ScorableImpl<'_, S>
where
  S: Scorer,
{
  fn score(&mut self) -> Result<f32> {
    let mut score = 0f64;
    for scorer in self.base.required_scoring_idx.iter() {
      score += self.base.all_scores[*scorer].score()? as f64;
    }
    Ok(score as f32)
  }

  fn get_children(&mut self) -> Result<Vec<ChildScorable<&mut dyn Scorable>>> {
    let mut scorers: Vec<_> = self.base.all_scores.iter_mut().map(Some).collect();
    self
      .base
      .all_scores_input_idx
      .iter()
      .map(|&idx| {
        let scorer = scorers[idx]
          .take()
          .ok_or_else(|| LuceneError::illegal_state("duplicate conjunction child"))?;
        Ok(ChildScorable::new(scorer as &mut dyn Scorable, "MUST"))
      })
      .collect()
  }

  fn cost(&self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl<S> crate::core::search::scorable::FixedScore for ScorableImpl<'_, S> {}
