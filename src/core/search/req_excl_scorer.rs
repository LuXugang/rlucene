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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::scorable::Scorable;
#[cfg(test)]
use crate::core::search::scorer::ScorerKind;
use crate::core::search::scorer::TwoPhaseState::Yes;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::scorer_util::ScorerUtil;
use crate::core::search::two_phase_iterator::{
  TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator, TwoPhaseIteratorEnum2,
};
use crate::core::util::error::lucene_error::Result;

/// A Scorer for queries with a required subscorer and an excluding (prohibited) sub `Scorer`.
pub struct ReqExclScorer<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  disi: TwoPhaseIteratorAsDocIdSetIterator<Tpi<S1, S2>>,
}
impl<S1, S2> ReqExclScorer<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  pub(crate) fn new(mut req_scorer: S1, mut excl_scorer: S2) -> Result<Self> {
    let match_cost = match_cost(&mut req_scorer, &mut excl_scorer)?;

    let check_req = match req_scorer.two_phase_iterator_mut() {
      Some(ref mut req_scorer_tpi) => match excl_scorer.two_phase_iterator_mut() {
        Some(excl_scorer_tpi) => req_scorer_tpi.match_cost() <= excl_scorer_tpi.match_cost(),
        None => false,
      },
      None => true,
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
    let disi = TwoPhaseIteratorAsDocIdSetIterator::new(two_phase_iterator);
    Ok(Self { disi })
  }
}

impl<S1, S2> Scorable for ReqExclScorer<S1, S2>
where
  S1: Scorer + 'static,
  S2: Scorer + 'static,
{
  fn score(&mut self) -> Result<f32> {
    match self.disi.two_phase_iterator {
      TwoPhaseIteratorEnum2::A(ref mut tpi) => tpi.req_scorer.score(),
      TwoPhaseIteratorEnum2::B(ref mut tpi) => tpi.req_scorer.score(),
    }
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    match self.disi.two_phase_iterator {
      TwoPhaseIteratorEnum2::A(ref mut tpi) => tpi.req_scorer.set_min_competitive_score(min_score),
      TwoPhaseIteratorEnum2::B(ref mut tpi) => tpi.req_scorer.set_min_competitive_score(min_score),
    }
  }

  fn cost(&self) -> Result<i64> {
    self.iterator().cost()
  }
}

impl<S1, S2> Scorer for ReqExclScorer<S1, S2>
where
  S1: Scorer + 'static,
  S2: Scorer + 'static,
{
  fn doc_id(&mut self) -> Result<i32> {
    match self.disi.two_phase_iterator {
      TwoPhaseIteratorEnum2::A(ref mut tpi) => tpi.req_scorer.doc_id(),
      TwoPhaseIteratorEnum2::B(ref mut tpi) => tpi.req_scorer.doc_id(),
    }
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.disi)
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.disi)
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let ReqExclScorer { disi, .. } = *self;
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
    let ReqExclScorer { disi, .. } = *self;
    Some(Box::new(disi.two_phase_iterator))
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    match self.disi.two_phase_iterator {
      TwoPhaseIteratorEnum2::A(ref mut tpi) => Ok(tpi.req_scorer.advance_shallow(target)?),
      TwoPhaseIteratorEnum2::B(ref mut tpi) => Ok(tpi.req_scorer.advance_shallow(target)?),
    }
  }

  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    match self.disi.two_phase_iterator {
      TwoPhaseIteratorEnum2::A(ref mut tpi) => Ok(tpi.req_scorer.get_max_score(upto)?),
      TwoPhaseIteratorEnum2::B(ref mut tpi) => Ok(tpi.req_scorer.get_max_score(upto)?),
    }
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    Yes
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    self.disi.two_phase_iterator.approximation_mut()
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    self.disi.two_phase_iterator.approximation()
  }
  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    ScorerKind::ReqExcl
  }
}

pub struct TwoPhaseIteratorImpl1<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  req_scorer: S1,
  excl_scorer: S2,
  match_cost: f32,
}
impl<S1, S2> TwoPhaseIteratorImpl1<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  fn new(req_scorer: S1, excl_scorer: S2, match_cost: f32) -> Self {
    Self {
      req_scorer,
      excl_scorer,
      match_cost,
    }
  }
}
impl<S1, S2> TwoPhaseIterator for TwoPhaseIteratorImpl1<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    self.req_scorer.approximation_mut()
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    self.req_scorer.approximation()
  }

  fn matches(&mut self) -> Result<bool> {
    let doc = ScorerUtil::doc_id(&self.req_scorer);
    // check if the doc is not excluded
    {
      let mut excl_doc = ScorerUtil::doc_id(&self.excl_scorer);
      if excl_doc < doc {
        excl_doc = ScorerUtil::advance(&mut self.excl_scorer, doc)?;
      }
      if excl_doc != doc {
        return match self.req_scorer.two_phase_iterator_mut() {
          Some(mut req_tpi) => req_tpi.matches(),
          None => Ok(true),
        };
      }
    }
    let req_match = match self.req_scorer.two_phase_iterator_mut() {
      Some(mut req_tpi) => req_tpi.matches()?,
      None => true,
    };
    match req_match {
      true => {
        let v = match self.excl_scorer.two_phase_iterator_mut() {
          Some(mut excl_tpi) => excl_tpi.matches()?,
          None => true,
        };
        Ok(!v)
      },
      false => Ok(false),
    }
  }

  fn match_cost(&self) -> f32 {
    self.match_cost
  }
}

pub struct TwoPhaseIteratorImpl2<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  req_scorer: S1,
  excl_scorer: S2,
  match_cost: f32,
}
impl<S1, S2> TwoPhaseIteratorImpl2<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  fn new(req_scorer: S1, excl_scorer: S2, match_cost: f32) -> Self {
    Self {
      req_scorer,
      excl_scorer,
      match_cost,
    }
  }
}
impl<S1, S2> TwoPhaseIterator for TwoPhaseIteratorImpl2<S1, S2>
where
  S1: Scorer,
  S2: Scorer,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    self.req_scorer.approximation_mut()
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    self.req_scorer.approximation()
  }

  fn matches(&mut self) -> Result<bool> {
    let doc = ScorerUtil::doc_id(&self.req_scorer);

    // check if doc is excluded
    {
      let mut excl_doc = ScorerUtil::doc_id(&self.excl_scorer);
      if excl_doc < doc {
        excl_doc = ScorerUtil::advance(&mut self.excl_scorer, doc)?;
      }

      if excl_doc != doc {
        return match self.req_scorer.two_phase_iterator_mut() {
          Some(mut req_tpi) => req_tpi.matches(),
          None => Ok(true),
        };
      }
    }

    let excl_not_match = match self.excl_scorer.two_phase_iterator_mut() {
      Some(mut excl_tpi) => !excl_tpi.matches()?,
      None => false,
    };

    if !excl_not_match {
      return Ok(false);
    }

    let req_match = match self.req_scorer.two_phase_iterator_mut() {
      Some(mut req_tpi) => req_tpi.matches()?,
      None => true,
    };

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

  if let Some(req_tpi) = req_scorer.two_phase_iterator_mut() {
    // this two-phase iterator must always be matched
    match_cost += req_tpi.match_cost();
  }
  // match cost of the prohibited clause: we need to advance the approximation
  // and match the two-phased iterator
  let excl_match_cost = {
    let extra = match excl_scorer.two_phase_iterator_mut() {
      Some(excl_tpi) => excl_tpi.match_cost(),
      None => 0.0,
    };
    (ADVANCE_COST as f32) + extra
  };
  // upper value for the ratio of documents that reqApproximation matches that
  // exclApproximation also matches
  let req_cost = ScorerUtil::cost(req_scorer)?;
  let excl_cost = ScorerUtil::cost(excl_scorer)?;

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
