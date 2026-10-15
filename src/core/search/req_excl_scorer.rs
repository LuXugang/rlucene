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

use crate::core::search::approximation_disi::ApproximationBitSetAccess;
use crate::core::search::approximation_disi::ApproximationDISI;
use crate::core::search::doc_id_set_iterator::BitSetIteratorAccess;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::scorable::{ChildScorable, Scorable};
#[cfg(test)]
use crate::core::search::scorer::ScorerKind;
use crate::core::search::scorer::TwoPhaseState::Yes;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::{
  TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator, TwoPhaseIteratorEnum2,
};
use crate::core::util::error::lucene_error::Result;

/// A scorer whose required/excluded matching order is selected at construction.
pub struct ReqExclScorer<T> {
  disi: TwoPhaseIteratorAsDocIdSetIterator<T>,
}
impl<T> ReqExclScorer<T> {
  pub(crate) fn new(two_phase_iterator: T) -> Self {
    Self {
      disi: TwoPhaseIteratorAsDocIdSetIterator::new(two_phase_iterator),
    }
  }
}
impl<S1: Scorer, S2: Scorer> ReqExclScorer<TwoPhaseIteratorImpl1<S1, S2>> {
  pub(crate) fn prepare(mut req_scorer: S1, mut excl_scorer: S2) -> Result<Tpi<S1, S2>> {
    let match_cost = match_cost(&mut req_scorer, &mut excl_scorer)?;

    let check_req = match req_scorer.has_two_phase_iterator() {
      TwoPhaseState::Yes => match excl_scorer.has_two_phase_iterator() {
        TwoPhaseState::Yes => req_scorer.match_cost() <= excl_scorer.match_cost(),
        TwoPhaseState::No => false,
      },
      TwoPhaseState::No => true,
    };

    let two_phase_iterator = if check_req {
      TwoPhaseIteratorEnum2::A(TwoPhaseIteratorImpl1::new(
        req_scorer,
        excl_scorer,
        match_cost,
      ))
    } else {
      TwoPhaseIteratorEnum2::B(TwoPhaseIteratorImpl2::new(
        req_scorer,
        excl_scorer,
        match_cost,
      ))
    };
    Ok(two_phase_iterator)
  }
}

/// Access to the required scorer owned by either concrete matching order.
#[doc(hidden)]
pub trait ReqExclState: TwoPhaseIterator + 'static {
  type Required: Scorer + 'static;
  fn req_scorer_mut(&mut self) -> &mut Self::Required;
}
impl<S1: Scorer + 'static, S2: Scorer + 'static> ReqExclState for TwoPhaseIteratorImpl1<S1, S2> {
  type Required = S1;
  fn req_scorer_mut(&mut self) -> &mut S1 {
    &mut self.req_scorer
  }
}
impl<S1: Scorer + 'static, S2: Scorer + 'static> ReqExclState for TwoPhaseIteratorImpl2<S1, S2> {
  type Required = S1;
  fn req_scorer_mut(&mut self) -> &mut S1 {
    &mut self.req_scorer
  }
}
impl<T: ReqExclState> Scorable for ReqExclScorer<T> {
  type Child<'a>
    = &'a mut T::Required
  where
    Self: 'a;
  fn score(&mut self) -> Result<f32> {
    self.disi.two_phase_iterator.req_scorer_mut().score()
  }
  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    self
      .disi
      .two_phase_iterator
      .req_scorer_mut()
      .set_min_competitive_score(min_score)
  }
  fn get_children(&mut self) -> Result<Vec<ChildScorable<Self::Child<'_>>>> {
    Ok(vec![ChildScorable::new(
      self.disi.two_phase_iterator.req_scorer_mut(),
      "MUST",
    )])
  }
  fn cost(&self) -> Result<i64> {
    self.disi.cost()
  }
}
impl<T> crate::core::search::scorable::FixedScore for ReqExclScorer<T> {}
impl<T: TwoPhaseIterator> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for ReqExclScorer<T>
{
}
impl<T: TwoPhaseIterator> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for ReqExclScorer<T>
{
}
impl<T: TwoPhaseIterator> DocIdSetIterator for ReqExclScorer<T> {
  fn doc_id(&self) -> i32 {
    self.disi.doc_id()
  }
  fn next_doc(&mut self) -> Result<i32> {
    self.disi.next_doc()
  }
  fn advance(&mut self, target: i32) -> Result<i32> {
    self.disi.advance(target)
  }
  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    self.disi.slow_advance(target)
  }
  fn cost(&self) -> Result<i64> {
    self.disi.cost()
  }
}
impl<T: ReqExclState> ApproximationDISI for ReqExclScorer<T> {
  fn approx_doc_id(&self) -> i32 {
    self.disi.two_phase_iterator.approx_doc_id()
  }
  fn approx_next_doc(&mut self) -> Result<i32> {
    self.disi.two_phase_iterator.approx_next_doc()
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    self.disi.two_phase_iterator.approx_advance(target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.disi.two_phase_iterator.approx_slow_advance(target)
  }
  fn approx_cost(&self) -> Result<i64> {
    self.disi.two_phase_iterator.approx_cost()
  }
}
impl<T: ReqExclState> ApproximationBitSetAccess for ReqExclScorer<T> {
  fn approx_is_bit_iter(&self) -> bool {
    self.disi.two_phase_iterator.approx_is_bit_iter()
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    self.disi.two_phase_iterator.approx_get(index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.disi.two_phase_iterator.approx_set_doc_id(doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    self.disi.two_phase_iterator.approx_bit_set_length()
  }
}

impl<T: ReqExclState> TwoPhaseIterator for ReqExclScorer<T> {
  fn matches(&mut self) -> Result<bool> {
    self.disi.two_phase_iterator.matches()
  }
  fn match_cost(&self) -> f32 {
    self.disi.two_phase_iterator.match_cost()
  }
}

impl<T: ReqExclState> Scorer for ReqExclScorer<T> {
  fn scoring_doc_id(&mut self) -> Result<i32> {
    self
      .disi
      .two_phase_iterator
      .req_scorer_mut()
      .scoring_doc_id()
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    self
      .disi
      .two_phase_iterator
      .req_scorer_mut()
      .advance_shallow(target)
  }
  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    self
      .disi
      .two_phase_iterator
      .req_scorer_mut()
      .get_max_score(upto)
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    Yes
  }

  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    ScorerKind::ReqExcl
  }
}

pub struct TwoPhaseIteratorImpl1<S1, S2> {
  req_scorer: S1,
  excl_scorer: S2,
  match_cost: f32,
}
impl<S1, S2> TwoPhaseIteratorImpl1<S1, S2> {
  fn new(req_scorer: S1, excl_scorer: S2, match_cost: f32) -> Self {
    Self {
      req_scorer,
      excl_scorer,
      match_cost,
    }
  }
}
impl<S1, S2> ApproximationDISI for TwoPhaseIteratorImpl1<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  fn approx_doc_id(&self) -> i32 {
    ({ crate::core::search::approximation_disi::ApproximationRef(&self.req_scorer) }).doc_id()
  }

  fn approx_next_doc(&mut self) -> Result<i32> {
    ({ crate::core::search::approximation_disi::ApproximationMut(&mut self.req_scorer) }).next_doc()
  }

  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    ({ crate::core::search::approximation_disi::ApproximationMut(&mut self.req_scorer) })
      .advance(target)
  }

  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    ({ crate::core::search::approximation_disi::ApproximationMut(&mut self.req_scorer) })
      .slow_advance(target)
  }

  fn approx_cost(&self) -> Result<i64> {
    ({ crate::core::search::approximation_disi::ApproximationRef(&self.req_scorer) }).cost()
  }
}
impl<S1, S2> ApproximationBitSetAccess for TwoPhaseIteratorImpl1<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  fn approx_is_bit_iter(&self) -> bool {
    ({ crate::core::search::approximation_disi::ApproximationRef(&self.req_scorer) }).is_bit_iter()
  }

  fn approx_get(&self, index: usize) -> Result<bool> {
    ({ crate::core::search::approximation_disi::ApproximationRef(&self.req_scorer) }).get(index)
  }

  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    ({ crate::core::search::approximation_disi::ApproximationMut(&mut self.req_scorer) })
      .set_doc_id(doc)
  }

  fn approx_bit_set_length(&self) -> Result<usize> {
    ({ crate::core::search::approximation_disi::ApproximationRef(&self.req_scorer) })
      .bit_set_length()
  }
}

impl<S1, S2> TwoPhaseIterator for TwoPhaseIteratorImpl1<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  fn matches(&mut self) -> Result<bool> {
    let doc = self.req_scorer.approx_doc_id();
    // check if the doc is not excluded
    {
      let mut excl_doc = self.excl_scorer.approx_doc_id();
      if excl_doc < doc {
        excl_doc = self.excl_scorer.approx_advance(doc)?;
      }
      if excl_doc != doc {
        return self.req_scorer.matches();
      }
    }
    let req_match = self.req_scorer.matches()?;
    match req_match {
      true => {
        let v = self.excl_scorer.matches()?;
        Ok(!v)
      },
      false => Ok(false),
    }
  }

  fn match_cost(&self) -> f32 {
    self.match_cost
  }
}

pub struct TwoPhaseIteratorImpl2<S1, S2> {
  req_scorer: S1,
  excl_scorer: S2,
  match_cost: f32,
}
impl<S1, S2> TwoPhaseIteratorImpl2<S1, S2> {
  fn new(req_scorer: S1, excl_scorer: S2, match_cost: f32) -> Self {
    Self {
      req_scorer,
      excl_scorer,
      match_cost,
    }
  }
}
impl<S1, S2> ApproximationDISI for TwoPhaseIteratorImpl2<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  fn approx_doc_id(&self) -> i32 {
    ({ crate::core::search::approximation_disi::ApproximationRef(&self.req_scorer) }).doc_id()
  }

  fn approx_next_doc(&mut self) -> Result<i32> {
    ({ crate::core::search::approximation_disi::ApproximationMut(&mut self.req_scorer) }).next_doc()
  }

  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    ({ crate::core::search::approximation_disi::ApproximationMut(&mut self.req_scorer) })
      .advance(target)
  }

  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    ({ crate::core::search::approximation_disi::ApproximationMut(&mut self.req_scorer) })
      .slow_advance(target)
  }

  fn approx_cost(&self) -> Result<i64> {
    ({ crate::core::search::approximation_disi::ApproximationRef(&self.req_scorer) }).cost()
  }
}
impl<S1, S2> ApproximationBitSetAccess for TwoPhaseIteratorImpl2<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  fn approx_is_bit_iter(&self) -> bool {
    ({ crate::core::search::approximation_disi::ApproximationRef(&self.req_scorer) }).is_bit_iter()
  }

  fn approx_get(&self, index: usize) -> Result<bool> {
    ({ crate::core::search::approximation_disi::ApproximationRef(&self.req_scorer) }).get(index)
  }

  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    ({ crate::core::search::approximation_disi::ApproximationMut(&mut self.req_scorer) })
      .set_doc_id(doc)
  }

  fn approx_bit_set_length(&self) -> Result<usize> {
    ({ crate::core::search::approximation_disi::ApproximationRef(&self.req_scorer) })
      .bit_set_length()
  }
}

impl<S1, S2> TwoPhaseIterator for TwoPhaseIteratorImpl2<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  fn matches(&mut self) -> Result<bool> {
    let doc = self.req_scorer.approx_doc_id();

    // check if doc is excluded
    {
      let excl_approximation = &mut self.excl_scorer;
      let mut excl_doc = excl_approximation.approx_doc_id();
      if excl_doc < doc {
        excl_doc = excl_approximation.approx_advance(doc)?;
      }

      if excl_doc != doc {
        return self.req_scorer.matches();
      }
    }

    let excl_not_match = !self.excl_scorer.matches()?;

    if !excl_not_match {
      return Ok(false);
    }

    let req_match = self.req_scorer.matches()?;

    Ok(req_match)
  }

  fn match_cost(&self) -> f32 {
    self.match_cost
  }
}

pub type Tpi<S1, S2> =
  TwoPhaseIteratorEnum2<TwoPhaseIteratorImpl1<S1, S2>, TwoPhaseIteratorImpl2<S1, S2>>;
/// Estimation of the number of operations required to call DISI.advance.
/// This is likely completely wrong,
/// especially given that the cost of this method usually depends on how far you want to advance,
/// but it's probably better than nothing.
const ADVANCE_COST: i32 = 10;
fn match_cost<S1, S2>(req_scorer: &mut S1, excl_scorer: &mut S2) -> Result<f32>
where
  S1: Scorer,
  S2: Scorer,
{
  let mut match_cost: f32 = 2.0;

  if req_scorer.has_two_phase_iterator() == TwoPhaseState::Yes {
    // this two-phase iterator must always be matched
    match_cost += req_scorer.match_cost();
  }
  // match cost of the prohibited clause: we need to advance the approximation
  // and match the two-phased iterator
  let excl_match_cost = {
    let extra = excl_scorer.match_cost();
    (ADVANCE_COST as f32) + extra
  };
  // upper value for the ratio of documents that reqApproximation matches that
  // exclApproximation also matches
  let req_cost = req_scorer.approx_cost()?;
  let excl_cost = excl_scorer.approx_cost()?;

  let ratio = if req_cost <= 0 {
    1.0
  } else if excl_cost <= 0 {
    0.0
  } else {
    (req_cost.min(excl_cost) as f32) / (req_cost as f32)
  };

  match_cost += ratio * excl_match_cost;

  Ok(match_cost)
}
