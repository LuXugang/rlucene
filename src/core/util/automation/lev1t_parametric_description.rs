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
use crate::core::util::automation::levenshtein_automata;
use crate::core::util::automation::levenshtein_automata::{
  ParametricDescription, ParametricDescriptionBase,
};
/// Parametric description for generating a Levenshtein automaton of degree 1, with transpositions as
/// primitive edits. The comment in [`Lev1ParametricDescription`] may be helpful for you to
/// understand this type.
///
/// See [`Lev1ParametricDescription`].
pub(crate) struct Lev1TParametricDescription;
// state map
//   0 -> [(0, 0)]
//   1 -> [(0, 1)]
//   2 -> [(0, 1), (1, 1)]
//   3 -> [(0, 1), (1, 1), (2, 1)]
//   4 -> [(0, 1), (2, 1)]
//   5 -> [t(0, 1), (0, 1), (1, 1), (2, 1)]

pub(crate) fn new(w: i32) -> ParametricDescription {
  ParametricDescription::new(w, 1, vec![0, 1, 0, -1, -1, -1], Lev1TParametricDescription)
}

impl ParametricDescriptionBase for Lev1TParametricDescription {
  fn transition(
    &self,
    abs_state: i32,
    position: i32,
    vector: i32,
    base: &ParametricDescription,
  ) -> i32 {
    // None absState should never be passed in
    debug_assert_ne!(abs_state, -1);

    // decode absState -> state, offset
    let mut state = abs_state / (base.w + 1);
    let mut offset = abs_state % (base.w + 1);
    debug_assert!(offset >= 0);

    if position == base.w {
      if state < 2 {
        let loc = vector * 2 + state;
        offset += levenshtein_automata::unpack(&OFFSET_INCRS_0, loc, 1);
        state = levenshtein_automata::unpack(&TO_STATES_0, loc, 2) - 1;
      }
    } else if position == base.w - 1 {
      if state < 3 {
        let loc = vector * 3 + state;
        offset += levenshtein_automata::unpack(&OFFSET_INCRS_1, loc, 1);
        state = levenshtein_automata::unpack(&TO_STATES_1, loc, 2) - 1;
      }
    } else if position == base.w - 2 {
      if state < 6 {
        let loc = vector * 6 + state;
        offset += levenshtein_automata::unpack(&OFFSET_INCRS_2, loc, 2);
        state = levenshtein_automata::unpack(&TO_STATES_2, loc, 3) - 1;
      }
    } else if state < 6 {
      let loc = vector * 6 + state;
      offset += levenshtein_automata::unpack(&OFFSET_INCRS_3, loc, 2);
      state = levenshtein_automata::unpack(&TO_STATES_3, loc, 3) - 1;
    }

    if state == -1 {
      // None state
      -1
    } else {
      // translate back to abs
      state * (base.w + 1) + offset
    }
  }
}

// 1 vectors; 2 states per vector; array length = 2
const TO_STATES_0: [i64; 1] = [0x2];
const OFFSET_INCRS_0: [i64; 1] = [0x0];

// 2 vectors; 3 states per vector; array length = 6
const TO_STATES_1: [i64; 1] = [0xa43];
const OFFSET_INCRS_1: [i64; 1] = [0x38];

// 4 vectors; 6 states per vector; array length = 24
const TO_STATES_2: [i64; 2] = [0xb45a491412180003u64 as i64, 0x69];
const OFFSET_INCRS_2: [i64; 1] = [0x5555558a0000];

// 8 vectors; 6 states per vector; array length = 48
const TO_STATES_3: [i64; 3] = [0xa1904864900c0003u64 as i64, 0x5a6d196a45a49169, 0x9634];
const OFFSET_INCRS_3: [i64; 2] = [0x5555ba08a0fc0000, 0x55555555];
