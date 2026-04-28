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
use crate::core::util::automation::lev1_parametric_description::Lev1ParametricDescription;
use crate::core::util::automation::lev1t_parametric_description::Lev1TParametricDescription;
use crate::core::util::automation::lev2_parametric_description::Lev2ParametricDescription;
use crate::core::util::automation::lev2t_parametric_description::Lev2TParametricDescription;
use crate::impl_from_for_enum;

pub struct LevenshteinAutomata;

/// A `ParametricDescription` describes the structure of a Levenshtein DFA for some degree `n`.
///
/// There are four components of a parametric description, all parameterized on the length of
/// the word `w`:
///
/// 1. The number of states: [`ParametricDescription::size`]
/// 2. The set of final states: [`ParametricDescription::is_accept`]
/// 3. The transition function: [`ParametricDescription::transition`]
/// 4. Minimal boundary function: [`ParametricDescription::get_position`]
pub(crate) struct ParametricDescription {
  pub(crate) w: i32,
  n: i32,
  min_errors: Vec<i32>,
  sub: ParametricDescriptionBaseEnum,
}

impl ParametricDescription {
  pub(crate) fn new<T>(w: i32, n: i32, min_errors: Vec<i32>, sub: T) -> Self
  where
    T: Into<ParametricDescriptionBaseEnum>,
  {
    Self {
      w,
      n,
      min_errors,
      sub: sub.into(),
    }
  }
  /// Return the number of states needed to compute a Levenshtein DFA.
  pub(crate) fn size(&self) -> i32 {
    self.min_errors.len() as i32 * (self.w + 1)
  }

  /// Returns `true` if the `state` in any Levenshtein DFA is an accept state (final state).
  pub(crate) fn is_accept(&self, abs_state: i32) -> bool {
    // decode absState -> state, offset
    let state = abs_state / (self.w + 1);
    let offset = abs_state % (self.w + 1);
    debug_assert!(offset >= 0);

    self.w - offset + self.min_errors[state as usize] <= self.n
  }

  /// Returns the position in the input word for a given `state`. This is the minimal boundary for
  /// the state.
  pub(crate) fn get_position(&self, abs_state: i32) -> i32 {
    abs_state % (self.w + 1)
  }
}
pub trait ParametricDescriptionBase {
  /// Returns the state number for a transition from the given `state`, assuming `position` and
  /// characteristic vector `vector`.
  fn transition(&self, state: i32, position: i32, vector: i32, base: &ParametricDescription)
  -> i32;
}
pub enum ParametricDescriptionBaseEnum {
  Lev1(Lev1ParametricDescription),
  Lev1T(Lev1TParametricDescription),
  Lev2(Lev2ParametricDescription),
  Lev2T(Lev2TParametricDescription),
}
impl_from_for_enum!(
ParametricDescriptionBaseEnum,
Lev1ParametricDescription=> Lev1,
Lev1TParametricDescription=> Lev1T,
Lev2ParametricDescription=> Lev2,
Lev2TParametricDescription=> Lev2T,
);
impl ParametricDescriptionBase for ParametricDescriptionBaseEnum {
  fn transition(
    &self,
    state: i32,
    position: i32,
    vector: i32,
    base: &ParametricDescription,
  ) -> i32 {
    match self {
      ParametricDescriptionBaseEnum::Lev1(lev1) => lev1.transition(state, position, vector, base),
      ParametricDescriptionBaseEnum::Lev1T(lev1t) => {
        lev1t.transition(state, position, vector, base)
      },
      ParametricDescriptionBaseEnum::Lev2(lev2) => lev2.transition(state, position, vector, base),
      ParametricDescriptionBaseEnum::Lev2T(lev2t) => {
        lev2t.transition(state, position, vector, base)
      },
    }
  }
}
const MASKS: [i64; 63] = [
  0x1,
  0x3,
  0x7,
  0xf,
  0x1f,
  0x3f,
  0x7f,
  0xff,
  0x1ff,
  0x3ff,
  0x7ff,
  0xfff,
  0x1fff,
  0x3fff,
  0x7fff,
  0xffff,
  0x1ffff,
  0x3ffff,
  0x7ffff,
  0xfffff,
  0x1fffff,
  0x3fffff,
  0x7fffff,
  0xffffff,
  0x1ffffff,
  0x3ffffff,
  0x7ffffff,
  0xfffffff,
  0x1fffffff,
  0x3fffffff,
  0x7fffffff,
  0xffffffff,
  0x1ffffffff,
  0x3ffffffff,
  0x7ffffffff,
  0xfffffffff,
  0x1fffffffff,
  0x3fffffffff,
  0x7fffffffff,
  0xffffffffff,
  0x1ffffffffff,
  0x3ffffffffff,
  0x7ffffffffff,
  0xfffffffffff,
  0x1fffffffffff,
  0x3fffffffffff,
  0x7fffffffffff,
  0xffffffffffff,
  0x1ffffffffffff,
  0x3ffffffffffff,
  0x7ffffffffffff,
  0xfffffffffffff,
  0x1fffffffffffff,
  0x3fffffffffffff,
  0x7fffffffffffff,
  0xffffffffffffff,
  0x1ffffffffffffff,
  0x3ffffffffffffff,
  0x7ffffffffffffff,
  0xfffffffffffffff,
  0x1fffffffffffffff,
  0x3fffffffffffffff,
  0x7fffffffffffffff,
];
pub(crate) fn unpack(data: &[i64], index: i32, bits_per_value: i32) -> i32 {
  let bit_loc = bits_per_value as i64 * index as i64;
  let data_loc = (bit_loc >> 6) as usize;
  let bit_start = (bit_loc & 63) as i32;

  if bit_start + bits_per_value <= 64 {
    ((data[data_loc] >> bit_start) & MASKS[(bits_per_value - 1) as usize]) as i32
  } else {
    let part = 64 - bit_start;
    (((data[data_loc] >> bit_start) & MASKS[(part - 1) as usize])
      + ((data[1 + data_loc] & MASKS[(bits_per_value - part - 1) as usize]) << part)) as i32
  }
}
