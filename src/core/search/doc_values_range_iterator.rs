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
use crate::core::index::doc_values_skipper::DocValuesSkipper;
use crate::core::search::approximation_disi::ApproximationBitSetAccess;
use crate::core::search::approximation_disi::ApproximationDISI;
use crate::core::search::doc_id_set_iterator::BitSetIteratorAccess;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// Wrapper around a [`TwoPhaseIterator`] used by doc-values range queries to
/// accelerate matching by leveraging a [`DocValuesSkipper`].
pub struct DocValuesRangeIterator<TPI, DVS> {
  pub(crate) approximation: Approximation<TPI, DVS>,
}
impl<TPI, DVS> DocValuesRangeIterator<TPI, DVS> {
  pub fn new(
    inner_approximation: TPI,
    skipper: DVS,
    lower_value: i64,
    upper_value: i64,
    has_gaps: bool,
  ) -> Self {
    let sub = if has_gaps {
      ApproximationBaseEnum::RangeWithGaps(RangeWithGapsApproximation)
    } else {
      ApproximationBaseEnum::RangeNoGaps(RangeNoGapsApproximation)
    };
    let approximation =
      Approximation::new(inner_approximation, skipper, lower_value, upper_value, sub);
    Self { approximation }
  }
}
impl<TPI, DVS> ApproximationDISI for DocValuesRangeIterator<TPI, DVS>
where
  TPI: TwoPhaseIterator,
  DVS: DocValuesSkipper,
{
  fn approx_doc_id(&self) -> i32 {
    self.approximation.doc_id()
  }

  fn approx_next_doc(&mut self) -> Result<i32> {
    self.approximation.next_doc()
  }

  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    self.approximation.advance(target)
  }

  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    self.approximation.slow_advance(target)
  }

  fn approx_cost(&self) -> Result<i64> {
    self.approximation.cost()
  }
}
impl<TPI, DVS> ApproximationBitSetAccess for DocValuesRangeIterator<TPI, DVS>
where
  TPI: TwoPhaseIterator,
  DVS: DocValuesSkipper,
{
  fn approx_is_bit_iter(&self) -> bool {
    self.approximation.is_bit_iter()
  }

  fn approx_get(&self, index: usize) -> Result<bool> {
    self.approximation.get(index)
  }

  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    self.approximation.set_doc_id(doc)
  }

  fn approx_bit_set_length(&self) -> Result<usize> {
    self.approximation.bit_set_length()
  }
}

impl<TPI, DVS> TwoPhaseIterator for DocValuesRangeIterator<TPI, DVS>
where
  TPI: TwoPhaseIterator,
  DVS: DocValuesSkipper,
{
  fn matches(&mut self) -> Result<bool> {
    match self.approximation.match_ {
      Match::YES => Ok(true),
      Match::IfDocHasValue => Ok(true),
      Match::MAYBE => self.approximation.inner_approximation.matches(),
      Match::NO => Err(LuceneError::illegal_state("Unpositioned approximation")),
    }
  }

  fn match_cost(&self) -> f32 {
    self.approximation.inner_approximation.match_cost()
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Match {
  /// None of the documents in the range match
  NO,

  /// Document values need to be checked to verify matches
  MAYBE,

  /// All documents in the range that have a value match
  IfDocHasValue,

  /// All documents in this range match unconditionally.
  YES,
}
pub struct Approximation<TPI, DVS> {
  pub(crate) inner_approximation: TPI,
  skipper: DVS,
  lower_value: i64,
  upper_value: i64,
  pub(crate) doc: i32,
  pub(crate) match_: Match,
  pub(crate) upto: i32,
  sub: ApproximationBaseEnum,
}
impl<TPI, DVS> Approximation<TPI, DVS> {
  pub(crate) fn new(
    inner_approximation: TPI,
    skipper: DVS,
    lower_value: i64,
    upper_value: i64,
    sub: ApproximationBaseEnum,
  ) -> Self {
    Self {
      inner_approximation,
      skipper,
      lower_value,
      upper_value,
      doc: -1,
      match_: Match::MAYBE,
      upto: -1,
      sub,
    }
  }
}
impl<TPI, DVS> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for Approximation<TPI, DVS>
where
  TPI: TwoPhaseIterator,
  DVS: DocValuesSkipper,
{
}
impl<TPI, DVS> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for Approximation<TPI, DVS>
where
  TPI: TwoPhaseIterator,
  DVS: DocValuesSkipper,
{
}

impl<TPI, DVS> DocIdSetIterator for Approximation<TPI, DVS>
where
  TPI: TwoPhaseIterator,
  DVS: DocValuesSkipper,
{
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.advance(self.doc_id() + 1)
  }

  fn advance(&mut self, mut target: i32) -> Result<i32> {
    loop {
      if target > self.upto {
        self.skipper.advance(target)?;
        // If target doesn't have a value and is between two blocks, advance()
        // might have moved to a block that doesn't contain `target`.
        target = target.max(self.skipper.min_doc_id_with_level(0));
        if target == NO_MORE_DOCS {
          self.doc = NO_MORE_DOCS;
          return Ok(self.doc);
        }
        self.upto = self.skipper.max_doc_id_with_level(0);
        self.match_ = self.sub.match_(0, self)?;

        // If we have a YES or NO decision, see if we still have the same decision on a higher
        // level (= on a wider range of doc IDs)
        let mut next_level = 1;
        while self.match_ != Match::MAYBE
          && next_level < self.skipper.num_levels()
          && self.match_ == self.sub.match_(next_level, self)?
        {
          self.upto = self.skipper.max_doc_id_with_level(next_level);
          next_level += 1;
        }
      }

      match self.match_ {
        Match::YES => {
          self.doc = target;
          return Ok(self.doc);
        },
        Match::MAYBE | Match::IfDocHasValue => {
          let inner_approximation = &mut self.inner_approximation;
          if target > inner_approximation.approx_doc_id() {
            target = inner_approximation.approx_advance(target)?;
          }
          if target <= self.upto {
            self.doc = target;
            return Ok(self.doc);
          }
          // Otherwise we are breaking the invariant that `doc` must always be <= upTo, so let
          // the loop run one more iteration to advance the skipper.
        },
        Match::NO => {
          if self.upto == NO_MORE_DOCS {
            self.doc = NO_MORE_DOCS;
            return Ok(self.doc);
          }
          target = self.upto + 1;
        },
      }
    }
  }

  fn cost(&self) -> Result<i64> {
    self.inner_approximation.approx_cost()
  }
}

pub(crate) struct RangeNoGapsApproximation;
impl ApproximationBase for RangeNoGapsApproximation {
  fn match_<TPI, DVS>(&self, level: usize, base: &Approximation<TPI, DVS>) -> Result<Match>
  where
    TPI: TwoPhaseIterator,
    DVS: DocValuesSkipper,
  {
    let min_value = base.skipper.min_value_with_level(level);
    let max_value = base.skipper.max_value_with_level(level);

    if min_value > base.upper_value || max_value < base.lower_value {
      Ok(Match::NO)
    } else if min_value >= base.lower_value && max_value <= base.upper_value {
      let doc_count = base.skipper.doc_count_with_level(level);
      let max_doc_id = base.skipper.max_doc_id_with_level(level);
      let min_doc_id = base.skipper.min_doc_id_with_level(level);

      if doc_count == max_doc_id - min_doc_id + 1 {
        Ok(Match::YES)
      } else {
        Ok(Match::IfDocHasValue)
      }
    } else {
      Ok(Match::MAYBE)
    }
  }
}
pub struct RangeWithGapsApproximation;
impl ApproximationBase for RangeWithGapsApproximation {
  fn match_<TPI, DVS>(&self, level: usize, base: &Approximation<TPI, DVS>) -> Result<Match>
  where
    TPI: TwoPhaseIterator,
    DVS: DocValuesSkipper,
  {
    let min_value = base.skipper.min_value_with_level(level);
    let max_value = base.skipper.max_value_with_level(level);

    if min_value > base.upper_value || max_value < base.lower_value {
      Ok(Match::NO)
    } else {
      Ok(Match::MAYBE)
    }
  }
}
pub(crate) enum ApproximationBaseEnum {
  RangeNoGaps(RangeNoGapsApproximation),
  RangeWithGaps(RangeWithGapsApproximation),
}
impl ApproximationBase for ApproximationBaseEnum {
  fn match_<TPI, DVS>(&self, level: usize, base: &Approximation<TPI, DVS>) -> Result<Match>
  where
    TPI: TwoPhaseIterator,
    DVS: DocValuesSkipper,
  {
    match self {
      ApproximationBaseEnum::RangeNoGaps(inner) => inner.match_(level, base),
      ApproximationBaseEnum::RangeWithGaps(inner) => inner.match_(level, base),
    }
  }
}
pub trait ApproximationBase {
  fn match_<TPI, DVS>(&self, level: usize, base: &Approximation<TPI, DVS>) -> Result<Match>
  where
    TPI: TwoPhaseIterator,
    DVS: DocValuesSkipper;
}
