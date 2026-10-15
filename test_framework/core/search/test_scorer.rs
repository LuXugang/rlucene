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

//! Heterogeneous scorer fixtures, available only in unit-test builds.
//! Concrete execution uses `Scorer` and `TwoPhaseIterator` directly.
use crate::core::search::approximation_disi::ApproximationBitSetAccess;
use crate::core::search::approximation_disi::ApproximationDISI;
use crate::core::search::doc_id_set_iterator::{
  BitSetIteratorAccess, DocIdSetIterator, DocIdSetIteratorExtensions,
};
use crate::core::search::scorable::Scorable;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub trait TestScorer: TestScorable + DocIdSetIterator {
  fn erased_scoring_doc_id(&mut self) -> Result<i32>;

  fn erased_advance_shallow(&mut self, target: i32) -> Result<i32>;
  fn erased_default_advance_shallow(&mut self, target: i32) -> Result<i32>;
  fn erased_get_max_score(&mut self, upto: i32) -> Result<f32>;
  fn erased_default_cost(&mut self) -> Result<i64>;
  fn erased_two_phase_state(&self) -> TwoPhaseState;

  fn erased_matches(&mut self) -> Result<bool>;
  fn erased_match_cost(&self) -> f32;
  fn erased_approximation_doc_id(&self) -> i32;
  fn erased_approximation_next_doc(&mut self) -> Result<i32>;
  fn erased_approximation_advance(&mut self, target: i32) -> Result<i32>;
  fn erased_approximation_slow_advance(&mut self, target: i32) -> Result<i32>;
  fn erased_approximation_cost(&self) -> Result<i64>;
  fn erased_approximation_is_bit_iter(&self) -> bool;
  fn erased_approximation_get(&self, index: usize) -> Result<bool>;
  fn erased_approximation_set_doc_id(&mut self, doc: i32) -> Result<()>;
  fn erased_approximation_bit_set_length(&self) -> Result<usize>;
  #[cfg(test)]
  fn erased_kind(&self) -> crate::core::search::scorer::ScorerKind;
}

impl<S: Scorer> TestScorer for S {
  fn erased_scoring_doc_id(&mut self) -> Result<i32> {
    self.scoring_doc_id()
  }

  fn erased_advance_shallow(&mut self, target: i32) -> Result<i32> {
    self.advance_shallow(target)
  }
  fn erased_default_advance_shallow(&mut self, target: i32) -> Result<i32> {
    self.default_advance_shallow(target)
  }
  fn erased_get_max_score(&mut self, upto: i32) -> Result<f32> {
    self.get_max_score(upto)
  }
  fn erased_default_cost(&mut self) -> Result<i64> {
    self.default_cost()
  }
  fn erased_two_phase_state(&self) -> TwoPhaseState {
    self.has_two_phase_iterator()
  }

  fn erased_matches(&mut self) -> Result<bool> {
    TwoPhaseIterator::matches(self)
  }
  fn erased_match_cost(&self) -> f32 {
    TwoPhaseIterator::match_cost(self)
  }
  fn erased_approximation_doc_id(&self) -> i32 {
    self.approx_doc_id()
  }
  fn erased_approximation_next_doc(&mut self) -> Result<i32> {
    self.approx_next_doc()
  }
  fn erased_approximation_advance(&mut self, target: i32) -> Result<i32> {
    self.approx_advance(target)
  }
  fn erased_approximation_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.approx_slow_advance(target)
  }
  fn erased_approximation_cost(&self) -> Result<i64> {
    self.approx_cost()
  }
  fn erased_approximation_is_bit_iter(&self) -> bool {
    self.approx_is_bit_iter()
  }
  fn erased_approximation_get(&self, index: usize) -> Result<bool> {
    self.approx_get(index)
  }
  fn erased_approximation_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.approx_set_doc_id(doc)
  }
  fn erased_approximation_bit_set_length(&self) -> Result<usize> {
    self.approx_bit_set_length()
  }
  #[cfg(test)]
  fn erased_kind(&self) -> crate::core::search::scorer::ScorerKind {
    self.kind()
  }
}

pub trait TestTwoPhaseIterator {
  fn erased_approximation_doc_id(&self) -> i32;
  fn erased_approximation_next_doc(&mut self) -> Result<i32>;
  fn erased_approximation_advance(&mut self, target: i32) -> Result<i32>;
  fn erased_approximation_slow_advance(&mut self, target: i32) -> Result<i32>;
  fn erased_approximation_cost(&self) -> Result<i64>;
  fn erased_approximation_is_bit_iter(&self) -> bool;
  fn erased_approximation_get(&self, index: usize) -> Result<bool>;
  fn erased_approximation_set_doc_id(&mut self, doc: i32) -> Result<()>;
  fn erased_approximation_bit_set_length(&self) -> Result<usize>;
  fn erased_matches(&mut self) -> Result<bool>;
  fn erased_match_cost(&self) -> f32;
}
impl<T: TwoPhaseIterator> TestTwoPhaseIterator for T {
  fn erased_approximation_doc_id(&self) -> i32 {
    self.approx_doc_id()
  }
  fn erased_approximation_next_doc(&mut self) -> Result<i32> {
    self.approx_next_doc()
  }
  fn erased_approximation_advance(&mut self, target: i32) -> Result<i32> {
    self.approx_advance(target)
  }
  fn erased_approximation_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.approx_slow_advance(target)
  }
  fn erased_approximation_cost(&self) -> Result<i64> {
    self.approx_cost()
  }
  fn erased_approximation_is_bit_iter(&self) -> bool {
    self.approx_is_bit_iter()
  }
  fn erased_approximation_get(&self, index: usize) -> Result<bool> {
    self.approx_get(index)
  }
  fn erased_approximation_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.approx_set_doc_id(doc)
  }
  fn erased_approximation_bit_set_length(&self) -> Result<usize> {
    self.approx_bit_set_length()
  }
  fn erased_matches(&mut self) -> Result<bool> {
    self.matches()
  }
  fn erased_match_cost(&self) -> f32 {
    self.match_cost()
  }
}

macro_rules! erased_iterator_views {
  ($ref:ident, $mut:ident, $owner:ident) => {
    pub struct $ref<'a, T: $owner + ?Sized>(pub(crate) &'a T);
    pub struct $mut<'a, T: $owner + ?Sized>(pub(crate) &'a mut T);
    impl<T: $owner + ?Sized> DocIdSetIteratorExtensions for $ref<'_, T> {}
    impl<T: $owner + ?Sized> BitSetIteratorAccess for $ref<'_, T> {
      fn is_bit_iter(&self) -> bool {
        self.0.erased_approximation_is_bit_iter()
      }
      fn get(&self, index: usize) -> Result<bool> {
        self.0.erased_approximation_get(index)
      }
      fn bit_set_length(&self) -> Result<usize> {
        self.0.erased_approximation_bit_set_length()
      }
    }
    impl<T: $owner + ?Sized> DocIdSetIteratorExtensions for $mut<'_, T> {}
    impl<T: $owner + ?Sized> BitSetIteratorAccess for $mut<'_, T> {
      fn is_bit_iter(&self) -> bool {
        self.0.erased_approximation_is_bit_iter()
      }
      fn get(&self, index: usize) -> Result<bool> {
        self.0.erased_approximation_get(index)
      }
      fn set_doc_id(&mut self, doc: i32) -> Result<()> {
        self.0.erased_approximation_set_doc_id(doc)
      }
      fn bit_set_length(&self) -> Result<usize> {
        self.0.erased_approximation_bit_set_length()
      }
    }
    impl<T: $owner + ?Sized> DocIdSetIterator for $ref<'_, T> {
      fn doc_id(&self) -> i32 {
        self.0.erased_approximation_doc_id()
      }
      fn next_doc(&mut self) -> Result<i32> {
        Err(LuceneError::unsupported_operation(
          "immutable iterator view",
        ))
      }
      fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation(
          "immutable iterator view",
        ))
      }
      fn cost(&self) -> Result<i64> {
        self.0.erased_approximation_cost()
      }
    }
    impl<T: $owner + ?Sized> DocIdSetIterator for $mut<'_, T> {
      fn doc_id(&self) -> i32 {
        self.0.erased_approximation_doc_id()
      }
      fn next_doc(&mut self) -> Result<i32> {
        self.0.erased_approximation_next_doc()
      }
      fn advance(&mut self, target: i32) -> Result<i32> {
        self.0.erased_approximation_advance(target)
      }
      fn slow_advance(&mut self, target: i32) -> Result<i32> {
        self.0.erased_approximation_slow_advance(target)
      }
      fn cost(&self) -> Result<i64> {
        self.0.erased_approximation_cost()
      }
    }
  };
}
erased_iterator_views!(
  TestScorerApproximationRef,
  TestScorerApproximationMut,
  TestScorer
);
erased_iterator_views!(
  DynTwoPhaseApproximationRef,
  DynTwoPhaseApproximationMut,
  TestTwoPhaseIterator
);

impl ApproximationDISI for dyn TestTwoPhaseIterator + '_ {
  fn approx_doc_id(&self) -> i32 {
    ({ DynTwoPhaseApproximationRef(self) }).doc_id()
  }

  fn approx_next_doc(&mut self) -> Result<i32> {
    ({ DynTwoPhaseApproximationMut(self) }).next_doc()
  }

  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    ({ DynTwoPhaseApproximationMut(self) }).advance(target)
  }

  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    ({ DynTwoPhaseApproximationMut(self) }).slow_advance(target)
  }

  fn approx_cost(&self) -> Result<i64> {
    ({ DynTwoPhaseApproximationRef(self) }).cost()
  }
}

impl ApproximationBitSetAccess for dyn TestTwoPhaseIterator + '_ {
  fn approx_is_bit_iter(&self) -> bool {
    ({ DynTwoPhaseApproximationRef(self) }).is_bit_iter()
  }

  fn approx_get(&self, index: usize) -> Result<bool> {
    ({ DynTwoPhaseApproximationRef(self) }).get(index)
  }

  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    ({ DynTwoPhaseApproximationMut(self) }).set_doc_id(doc)
  }

  fn approx_bit_set_length(&self) -> Result<usize> {
    ({ DynTwoPhaseApproximationRef(self) }).bit_set_length()
  }
}

impl TwoPhaseIterator for dyn TestTwoPhaseIterator + '_ {
  fn matches(&mut self) -> Result<bool> {
    self.erased_matches()
  }
  fn match_cost(&self) -> f32 {
    self.erased_match_cost()
  }
}

impl<'owner> ApproximationDISI for Box<dyn TestScorer + 'owner> {
  fn approx_doc_id(&self) -> i32 {
    DocIdSetIterator::doc_id(&{ TestScorerApproximationRef(self.as_ref()) })
  }
  fn approx_next_doc(&mut self) -> Result<i32> {
    DocIdSetIterator::next_doc(&mut { TestScorerApproximationMut(self.as_mut()) })
  }
  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    DocIdSetIterator::advance(&mut { TestScorerApproximationMut(self.as_mut()) }, target)
  }
  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    DocIdSetIterator::slow_advance(&mut { TestScorerApproximationMut(self.as_mut()) }, target)
  }
  fn approx_cost(&self) -> Result<i64> {
    DocIdSetIterator::cost(&{ TestScorerApproximationRef(self.as_ref()) })
  }
}

impl<'owner> ApproximationBitSetAccess for Box<dyn TestScorer + 'owner> {
  fn approx_is_bit_iter(&self) -> bool {
    BitSetIteratorAccess::is_bit_iter(&{ TestScorerApproximationRef(self.as_ref()) })
  }
  fn approx_get(&self, index: usize) -> Result<bool> {
    BitSetIteratorAccess::get(&{ TestScorerApproximationRef(self.as_ref()) }, index)
  }
  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    BitSetIteratorAccess::set_doc_id(&mut { TestScorerApproximationMut(self.as_mut()) }, doc)
  }
  fn approx_bit_set_length(&self) -> Result<usize> {
    BitSetIteratorAccess::bit_set_length(&{ TestScorerApproximationRef(self.as_ref()) })
  }
}

impl<'owner> TwoPhaseIterator for Box<dyn TestScorer + 'owner> {
  fn matches(&mut self) -> Result<bool> {
    (**self).erased_matches()
  }
  fn match_cost(&self) -> f32 {
    (**self).erased_match_cost()
  }
}

impl<'owner> Scorer for Box<dyn TestScorer + 'owner> {
  fn scoring_doc_id(&mut self) -> Result<i32> {
    (**self).erased_scoring_doc_id()
  }

  fn advance_shallow(&mut self, target: i32) -> Result<i32> {
    (**self).erased_advance_shallow(target)
  }
  fn default_advance_shallow(&mut self, target: i32) -> Result<i32> {
    (**self).erased_default_advance_shallow(target)
  }
  fn get_max_score(&mut self, upto: i32) -> Result<f32> {
    (**self).erased_get_max_score(upto)
  }
  fn default_cost(&mut self) -> Result<i64> {
    (**self).erased_default_cost()
  }
  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    (**self).erased_two_phase_state()
  }
  #[cfg(test)]
  fn kind(&self) -> crate::core::search::scorer::ScorerKind {
    (**self).erased_kind()
  }
}

pub trait TestScorable {
  fn erased_score(&mut self) -> Result<f32>;
  fn erased_smoothing_score(&mut self, doc: i32) -> Result<f32>;
  fn erased_min_score(&mut self, score: f32) -> Result<()>;
  fn erased_set_score(&mut self, score: f32) -> Result<()>;
  fn erased_cost(&self) -> Result<i64>;
  fn erased_children(
    &mut self,
  ) -> Result<Vec<crate::core::search::scorable::ChildScorable<Box<dyn TestScorable + '_>>>>;
  fn erased_any(&mut self) -> &mut dyn std::any::Any;
  fn erased_type_name(&self) -> &'static str;
}
impl<S: Scorable> TestScorable for S {
  fn erased_score(&mut self) -> Result<f32> {
    self.score()
  }
  fn erased_smoothing_score(&mut self, doc: i32) -> Result<f32> {
    self.smoothing_score(doc)
  }
  fn erased_min_score(&mut self, score: f32) -> Result<()> {
    self.set_min_competitive_score(score)
  }
  fn erased_set_score(&mut self, score: f32) -> Result<()> {
    self.set_score(score)
  }
  fn erased_cost(&self) -> Result<i64> {
    Scorable::cost(self)
  }
  fn erased_children(
    &mut self,
  ) -> Result<Vec<crate::core::search::scorable::ChildScorable<Box<dyn TestScorable + '_>>>> {
    Ok(
      self
        .get_children()?
        .into_iter()
        .map(|c| crate::core::search::scorable::ChildScorable {
          child: Box::new(c.child) as Box<dyn TestScorable>,
          relationship: c.relationship,
        })
        .collect(),
    )
  }
  fn erased_any(&mut self) -> &mut dyn std::any::Any {
    self.as_any()
  }
  fn erased_type_name(&self) -> &'static str {
    self.scorable_test_type_name()
  }
}
macro_rules! fixture_scorable {
  ($object:ty) => {
    impl Scorable for $object {
      type Child<'a>
        = Box<dyn TestScorable + 'a>
      where
        Self: 'a;
      fn score(&mut self) -> Result<f32> {
        self.erased_score()
      }
      fn smoothing_score(&mut self, doc: i32) -> Result<f32> {
        self.erased_smoothing_score(doc)
      }
      fn set_min_competitive_score(&mut self, score: f32) -> Result<()> {
        self.erased_min_score(score)
      }
      fn cost(&self) -> Result<i64> {
        self.erased_cost()
      }
      fn get_children(
        &mut self,
      ) -> Result<Vec<crate::core::search::scorable::ChildScorable<Self::Child<'_>>>> {
        self.erased_children()
      }
      fn as_any(&mut self) -> &mut dyn std::any::Any {
        self.erased_any()
      }
      fn scorable_test_type_name(&self) -> &'static str {
        self.erased_type_name()
      }
    }
    impl crate::core::search::scorable::FixedScore for $object {
      fn set_score(&mut self, score: f32) -> Result<()> {
        self.erased_set_score(score)
      }
    }
  };
}
fixture_scorable!(dyn TestScorable + '_);
fixture_scorable!(dyn TestScorer + '_);
