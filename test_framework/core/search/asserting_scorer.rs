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
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::search::query::QueryWeightSsScorer;
use crate::core::search::scorable::{ChildScorable, FixedScore, Scorable};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::Result;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IteratorState {
  Approximating,
  Iterating,
  ShallowAdvancing,
  Finished,
}

#[derive(Debug)]
struct AssertingScorerState {
  state: IteratorState,
  doc: i32,
  min_competitive_score: f32,
  last_shallow_target: i32,
}

impl AssertingScorerState {
  fn new(doc: i32) -> Self {
    Self {
      state: IteratorState::Iterating,
      doc,
      min_competitive_score: 0.0,
      last_shallow_target: -1,
    }
  }
}

/// Wraps a Scorer with additional checks.
pub(crate) struct AssertingScorer {
  _random_seed: u64,
  in_: Rc<RefCell<QueryWeightSsScorer>>,
  score_mode: ScoreMode,
  can_call_min_competitive_score: bool,
  state: Rc<RefCell<AssertingScorerState>>,
}

impl AssertingScorer {
  pub(crate) fn wrap(
    random_seed: u64,
    mut in_: QueryWeightSsScorer,
    score_mode: ScoreMode,
    can_call_min_competitive_score: bool,
  ) -> Self {
    let doc = in_.doc_id().expect("doc_id should be available");
    Self {
      _random_seed: random_seed,
      in_: Rc::new(RefCell::new(in_)),
      score_mode,
      can_call_min_competitive_score,
      state: Rc::new(RefCell::new(AssertingScorerState::new(doc))),
    }
  }

  fn iterating(&mut self) -> Result<bool> {
    match self.doc_id()? {
      -1 | NO_MORE_DOCS => Ok(false),
      _ => Ok(self.state.borrow().state == IteratorState::Iterating),
    }
  }
}

impl FixedScore for AssertingScorer {
  fn set_score(&mut self, score: f32) -> Result<()> {
    self.in_.borrow_mut().set_score(score)
  }
}

impl Scorable for AssertingScorer {
  fn score(&mut self) -> Result<f32> {
    assert!(self.score_mode.needs_scores());
    assert!(self.iterating()?, "{:?}", self.state.borrow().state);
    let score = self.in_.borrow_mut().score()?;
    assert!(!score.is_nan(), "NaN score");
    if self.state.borrow().last_shallow_target != -1 {
      let doc = self.doc_id()?;
      assert!(score <= self.get_max_score(doc)?);
    }
    assert!(score >= 0.0, "{}", score);
    Ok(score)
  }

  fn set_min_competitive_score(&mut self, score: f32) -> Result<()> {
    assert_eq!(self.score_mode, ScoreMode::TopScores);
    assert!(self.can_call_min_competitive_score);
    assert!(!score.is_nan());
    assert!(score >= self.state.borrow().min_competitive_score);
    self.in_.borrow_mut().set_min_competitive_score(score)?;
    self.state.borrow_mut().min_competitive_score = score;
    Ok(())
  }

  fn get_children(&self) -> Result<Vec<ChildScorable<Box<dyn Scorable>>>> {
    todo!()
  }

  fn cost(&self) -> Result<i64> {
    self.in_.borrow().cost()
  }
}

impl Scorer for AssertingScorer {
  fn doc_id(&mut self) -> Result<i32> {
    self.in_.borrow_mut().doc_id()
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(AssertingDocIdSetIterator::new(
      self.in_.clone(),
      self.state.clone(),
      false,
    ))
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(AssertingDocIdSetIterator::new(
      self.in_.clone(),
      self.state.clone(),
      false,
    ))
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    Box::new(AssertingDocIdSetIterator::new(
      self.in_.clone(),
      self.state.clone(),
      false,
    ))
  }

  fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    if self.in_.borrow().two_phase_iterator().is_some() {
      Some(Box::new(AssertingTwoPhaseIterator::new(
        self.in_.clone(),
        self.state.clone(),
      )))
    } else {
      None
    }
  }

  fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    if self.in_.borrow_mut().two_phase_iterator_mut().is_some() {
      Some(Box::new(AssertingTwoPhaseIterator::new(
        self.in_.clone(),
        self.state.clone(),
      )))
    } else {
      None
    }
  }

  fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>> {
    if self.in_.borrow().two_phase_iterator().is_some() {
      Some(Box::new(AssertingTwoPhaseIterator::new(
        self.in_.clone(),
        self.state.clone(),
      )))
    } else {
      None
    }
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    assert!(self.score_mode.needs_scores());
    {
      let state = self.state.borrow();
      assert!(
        target >= state.last_shallow_target,
        "called on decreasing targets: target = {} < last target = {}",
        target,
        state.last_shallow_target
      );
    }
    let doc_id = self.doc_id()?;
    assert!(target >= doc_id, "target = {} < docID = {}", target, doc_id);
    let up_to = self.in_.borrow_mut().advance_shallow(target)?;
    assert!(up_to >= target, "upTo = {} < target = {}", up_to, target);
    let mut state = self.state.borrow_mut();
    state.last_shallow_target = target;
    if target != state.doc {
      state.state = IteratorState::ShallowAdvancing;
    }
    Ok(up_to)
  }

  fn get_max_score(&mut self, up_to: i32) -> Result<f32> {
    assert!(self.score_mode.needs_scores());
    let doc_id = self.in_.borrow_mut().doc_id()?;
    {
      let state = self.state.borrow();
      assert!(
        up_to >= state.last_shallow_target,
        "upTo = {} < last target = {}",
        up_to,
        state.last_shallow_target
      );
      assert!(
        doc_id >= 0 || state.last_shallow_target >= 0,
        "Cannot get max scores until the iterator is positioned or advanceShallow has been called"
      );
    }
    let max_score = self.in_.borrow_mut().get_max_score(up_to)?;
    assert!(!max_score.is_nan());
    Ok(max_score)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    self.in_.borrow().has_two_phase_iterator()
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(AssertingDocIdSetIterator::new(
      self.in_.clone(),
      self.state.clone(),
      self.in_.borrow().has_two_phase_iterator() == TwoPhaseState::Yes,
    ))
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    let has_two_phase = self.in_.borrow().has_two_phase_iterator() == TwoPhaseState::Yes;
    Box::new(AssertingDocIdSetIterator::new(
      self.in_.clone(),
      self.state.clone(),
      has_two_phase,
    ))
  }
}

struct AssertingDocIdSetIterator {
  scorer: Rc<RefCell<QueryWeightSsScorer>>,
  state: Rc<RefCell<AssertingScorerState>>,
  approximation: bool,
}

impl AssertingDocIdSetIterator {
  fn new(
    scorer: Rc<RefCell<QueryWeightSsScorer>>,
    state: Rc<RefCell<AssertingScorerState>>,
    approximation: bool,
  ) -> Self {
    Self {
      scorer,
      state,
      approximation,
    }
  }
}

impl DocIdSetIterator for AssertingDocIdSetIterator {
  fn doc_id(&self) -> i32 {
    let scorer_doc = self
      .scorer
      .borrow_mut()
      .doc_id()
      .expect("doc_id should be available");
    let iterator_doc = if self.approximation {
      self.scorer.borrow().approximation().doc_id()
    } else {
      self.scorer.borrow().iterator().doc_id()
    };
    assert_eq!(scorer_doc, iterator_doc);
    iterator_doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    {
      let state = self.state.borrow();
      assert_ne!(
        state.state,
        IteratorState::Finished,
        "nextDoc() called after NO_MORE_DOCS"
      );
      assert!(self.doc_id() + 1 >= state.last_shallow_target);
    }
    let next_doc = if self.approximation {
      self.scorer.borrow_mut().approximation_mut().next_doc()?
    } else {
      self.scorer.borrow_mut().iterator_mut().next_doc()?
    };
    {
      let mut state = self.state.borrow_mut();
      assert!(
        next_doc > state.doc,
        "backwards nextDoc from {} to {}",
        state.doc,
        next_doc
      );
      state.state = if next_doc == NO_MORE_DOCS {
        IteratorState::Finished
      } else if self.approximation {
        IteratorState::Approximating
      } else {
        IteratorState::Iterating
      };
      assert_eq!(self.scorer.borrow_mut().doc_id()?, next_doc);
      state.doc = next_doc;
    }
    Ok(next_doc)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    {
      let state = self.state.borrow();
      assert_ne!(
        state.state,
        IteratorState::Finished,
        "advance() called after NO_MORE_DOCS"
      );
      assert!(
        target > state.doc,
        "target must be > docID(), got {} <= {}",
        target,
        state.doc
      );
      assert!(target >= state.last_shallow_target);
    }
    let advanced = if self.approximation {
      self
        .scorer
        .borrow_mut()
        .approximation_mut()
        .advance(target)?
    } else {
      self.scorer.borrow_mut().iterator_mut().advance(target)?
    };
    {
      let mut state = self.state.borrow_mut();
      assert!(
        advanced >= target,
        "backwards advance from: {} to: {}",
        target,
        advanced
      );
      state.state = if advanced == NO_MORE_DOCS {
        IteratorState::Finished
      } else if self.approximation {
        IteratorState::Approximating
      } else {
        IteratorState::Iterating
      };
      assert_eq!(self.scorer.borrow_mut().doc_id()?, advanced);
      state.doc = advanced;
    }
    Ok(advanced)
  }

  fn cost(&self) -> Result<i64> {
    if self.approximation {
      self.scorer.borrow().approximation().cost()
    } else {
      self.scorer.borrow().iterator().cost()
    }
  }
}

struct AssertingTwoPhaseIterator {
  scorer: Rc<RefCell<QueryWeightSsScorer>>,
  state: Rc<RefCell<AssertingScorerState>>,
}

impl AssertingTwoPhaseIterator {
  fn new(
    scorer: Rc<RefCell<QueryWeightSsScorer>>,
    state: Rc<RefCell<AssertingScorerState>>,
  ) -> Self {
    let scorer_doc = scorer
      .borrow_mut()
      .doc_id()
      .expect("doc_id should be available");
    let approximation_doc = scorer
      .borrow()
      .two_phase_iterator()
      .expect("two_phase_iterator should be available")
      .approximation()
      .doc_id();
    assert_eq!(approximation_doc, scorer_doc);
    Self { scorer, state }
  }
}

impl TwoPhaseIterator for AssertingTwoPhaseIterator {
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(AssertingDocIdSetIterator::new(
      self.scorer.clone(),
      self.state.clone(),
      true,
    ))
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(AssertingDocIdSetIterator::new(
      self.scorer.clone(),
      self.state.clone(),
      true,
    ))
  }

  fn matches(&mut self) -> Result<bool> {
    assert_eq!(
      self.state.borrow().state,
      IteratorState::Approximating,
      "{:?}",
      self.state.borrow().state
    );
    let matches = self
      .scorer
      .borrow_mut()
      .two_phase_iterator_mut()
      .expect("two_phase_iterator should be available")
      .matches()?;
    if matches {
      let doc = self
        .scorer
        .borrow()
        .two_phase_iterator()
        .expect("two_phase_iterator should be available")
        .approximation()
        .doc_id();
      assert_eq!(self.scorer.borrow_mut().doc_id()?, doc);
      let mut state = self.state.borrow_mut();
      state.doc = doc;
      state.state = IteratorState::Iterating;
    }
    Ok(matches)
  }

  fn match_cost(&self) -> f32 {
    let match_cost = self
      .scorer
      .borrow()
      .two_phase_iterator()
      .expect("two_phase_iterator should be available")
      .match_cost();
    assert!(!match_cost.is_nan());
    assert!(match_cost >= 0.0);
    match_cost
  }
}
