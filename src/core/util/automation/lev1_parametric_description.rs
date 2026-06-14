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

// The following code was generated with the moman/finenight pkg
// This package is available under the MIT License, see NOTICE.txt
// for more details.
// This source file is auto-generated, Please do not modify it directly.
// You should modify the gradle/generation/moman/createAutomata.py instead.

/*
 Parametric transitions for LEV1.
 ┏━━━━━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┓
 ┃ char vector ┃ State 0 ┃ State 1 ┃ State 2 ┃ State 3 ┃ State 4 ┃
 ┡━━━━━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━┩
 │ (0,0)       │ (2, 0)  │ (-1, 0) │ (-1, 0) │ (-1, 0) │ (-1, 0) │
 │ (0,1)       │ (3, 0)  │ (-1, 0) │ (1, 2)  │ (1, 2)  │ (-1, 0) │
 │ (1,0)       │ (0, 1)  │ (1, 1)  │ (1, 1)  │ (1, 1)  │ (1, 1)  │
 │ (1,1)       │ (0, 1)  │ (1, 1)  │ (2, 1)  │ (2, 1)  │ (1, 1)  │
 └─────────────┴─────────┴─────────┴─────────┴─────────┴─────────┘

 `char vector` is the characteristic vector in the paper.

 Entry `(i, j)` in the table means that the next transition state is `i`, and the
 next offset is `j + current_offset` if we meet the corresponding char vector.

 When `i = -1`, it means an empty state.

 We store this table in `to_state` and `offset_incrs`.

 `to_state = [i + 1 | for entry in entries]`.

 `offset_incrs = [j | for entry in entries]`.
*/

/// Parametric description for generating a Levenshtein automaton of degree 1.
pub(crate) struct Lev1ParametricDescription;
// state map
//   0 -> [(0, 0)]
//   1 -> [(0, 1)]
//   2 -> [(0, 1), (1, 1)]
//   3 -> [(0, 1), (1, 1), (2, 1)]
//   4 -> [(0, 1), (2, 1)]
pub(crate) fn new(w: i32) -> ParametricDescription {
  ParametricDescription::new(w, 1, vec![0, 1, 0, -1, -1], Lev1ParametricDescription)
}
impl ParametricDescriptionBase for Lev1ParametricDescription {
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
      if state < 5 {
        let loc = vector * 5 + state;
        offset += levenshtein_automata::unpack(&OFFSET_INCRS_2, loc, 2);
        state = levenshtein_automata::unpack(&TO_STATES_2, loc, 3) - 1;
      }
    } else if state < 5 {
      let loc = vector * 5 + state;
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

/// 1 vector; 2 states per vector; array length = 2
/// Parametric transitions for LEV1  (position = w)
/// ┏━━━━━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┓
/// ┃ char vector ┃ State 0 ┃ State 1 ┃
/// ┡━━━━━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━┩
/// │ ()          │ (1, 0)  │ (-1, 0) │
/// └─────────────┴─────────┴─────────┘
const TO_STATES_0: [i64; 1] = [0x2];

/// 1 bit per value.
const OFFSET_INCRS_0: [i64; 1] = [0x0];

/// 2 vectors; 3 states per vector; array length = 6
/// Parametric transitions for LEV1 (position = w-1)
/// ┏━━━━━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┓
/// ┃ char vector ┃ State 0 ┃ State 1 ┃ State 2 ┃
/// ┡━━━━━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━┩
/// │ (0)         │ (2, 0)  │ (-1, 0) │ (-1, 0) │
/// │ (1)         │ (0, 1)  │ (1, 1)  │ (1, 1)  │
/// └─────────────┴─────────┴─────────┴─────────┘
const TO_STATES_1: [i64; 1] = [0xa43];

/// 1 bit per value.
const OFFSET_INCRS_1: [i64; 1] = [0x38];

/// 4 vectors; 5 states per vector; array length = 20
/// Parametric transitions for LEV1 ( position == w-2 )
/// ┏━━━━━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┓
/// ┃ char vector ┃ State 0 ┃ State 1 ┃ State 2 ┃ State 3 ┃ State 4 ┃
/// ┡━━━━━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━┩
/// │ (0,0)       │ (2, 0)  │ (-1, 0) │ (-1, 0) │ (-1, 0) │ (-1, 0) │
/// │ (0,1)       │ (3, 0)  │ (-1, 0) │ (1, 2)  │ (1, 2)  │ (-1, 0) │
/// │ (1,0)       │ (0, 1)  │ (1, 1)  │ (1, 1)  │ (1, 1)  │ (1, 1)  │
/// │ (1,1)       │ (0, 1)  │ (1, 1)  │ (2, 1)  │ (2, 1)  │ (1, 1)  │
/// └─────────────┴─────────┴─────────┴─────────┴─────────┴─────────┘
const TO_STATES_2: [i64; 1] = [0x4da292442420003];

/// 2 bits per value.
const OFFSET_INCRS_2: [i64; 1] = [0x5555528000];

/// 8 vectors; 5 states per vector; array length = 40
/// Parametric transitions for LEV1 (0 <= position <= w-3 )
/// ┏━━━━━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┓
/// ┃ char vector ┃ State 0 ┃ State 1 ┃ State 2 ┃ State 3 ┃ State 4 ┃
/// ┡━━━━━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━╇━━━━━━━━━┩
/// │ (0,0,0)     │ (2, 0)  │ (-1, 0) │ (-1, 0) │ (-1, 0) │ (-1, 0) │
/// │ (0,0,1)     │ (2, 0)  │ (-1, 0) │ (-1, 0) │ (1, 3)  │ (1, 3)  │
/// │ (0,1,0)     │ (3, 0)  │ (-1, 0) │ (1, 2)  │ (1, 2)  │ (-1, 0) │
/// │ (0,1,1)     │ (3, 0)  │ (-1, 0) │ (1, 2)  │ (2, 2)  │ (1, 3)  │
/// │ (1,0,0)     │ (0, 1)  │ (1, 1)  │ (1, 1)  │ (1, 1)  │ (1, 1)  │
/// │ (1,0,1)     │ (0, 1)  │ (1, 1)  │ (1, 1)  │ (4, 1)  │ (4, 1)  │
/// │ (1,1,0)     │ (0, 1)  │ (1, 1)  │ (2, 1)  │ (2, 1)  │ (1, 1)  │
/// │ (1,1,1)     │ (0, 1)  │ (1, 1)  │ (2, 1)  │ (3, 1)  │ (4, 1)  │
/// └─────────────┴─────────┴─────────┴─────────┴─────────┴─────────┘
const TO_STATES_3: [i64; 2] = [0x14d0812112018003, 0xb1a29b46d48a49];

/// 2 bits per value.
const OFFSET_INCRS_3: [i64; 2] = [0x555555e80a0f0000, 0x5555];
