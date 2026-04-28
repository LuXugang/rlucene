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

/// Parametric description for generating a Levenshtein automaton of degree 2.
/// The comment in [`Lev1ParametricDescription`] may be helpful for you to
/// understand this class.
///
/// See [`Lev1ParametricDescription`].
pub(crate) struct Lev2ParametricDescription;

pub(crate) fn new(w: i32) -> ParametricDescription {
  // state map
  //   0 -> [(0, 0)]
  //   1 -> [(0, 1)]
  //   2 -> [(0, 2)]
  //   3 -> [(0, 1), (1, 1)]
  //   4 -> [(0, 2), (1, 2)]
  //   5 -> [(0, 1), (1, 1), (2, 1)]
  //   6 -> [(0, 2), (1, 2), (2, 2)]
  //   7 -> [(0, 1), (2, 1)]
  //   8 -> [(0, 1), (2, 2)]
  //   9 -> [(0, 2), (2, 1)]
  //   10 -> [(0, 2), (2, 2)]
  //   11 -> [(0, 2), (1, 2), (2, 2), (3, 2)]
  //   12 -> [(0, 1), (1, 1), (3, 2)]
  //   13 -> [(0, 1), (2, 2), (3, 2)]
  //   14 -> [(0, 1), (3, 2)]
  //   15 -> [(0, 2), (1, 2), (3, 1)]
  //   16 -> [(0, 2), (1, 2), (3, 2)]
  //   17 -> [(0, 2), (2, 1), (3, 1)]
  //   18 -> [(0, 2), (2, 2), (3, 2)]
  //   19 -> [(0, 2), (3, 1)]
  //   20 -> [(0, 2), (3, 2)]
  //   21 -> [(0, 2), (1, 2), (2, 2), (3, 2), (4, 2)]
  //   22 -> [(0, 2), (1, 2), (2, 2), (4, 2)]
  //   23 -> [(0, 2), (1, 2), (3, 2), (4, 2)]
  //   24 -> [(0, 2), (1, 2), (4, 2)]
  //   25 -> [(0, 2), (2, 1), (4, 2)]
  //   26 -> [(0, 2), (2, 2), (3, 2), (4, 2)]
  //   27 -> [(0, 2), (2, 2), (4, 2)]
  //   28 -> [(0, 2), (3, 2), (4, 2)]
  //   29 -> [(0, 2), (4, 2)]

  ParametricDescription::new(
    w,
    2,
    vec![
      0, 1, 2, 0, 1, -1, 0, -1, 0, -1, 0, -1, -1, -1, -1, -2, -1, -2, -1, -2, -1, -2, -2, -2, -2,
      -2, -2, -2, -2, -2,
    ],
    Lev2ParametricDescription,
  )
}

impl ParametricDescriptionBase for Lev2ParametricDescription {
  fn transition(
    &self,
    abs_state: i32,
    position: i32,
    vector: i32,
    base: &ParametricDescription,
  ) -> i32 {
    // null absState should never be passed in
    debug_assert_ne!(abs_state, -1);

    // decode absState -> state, offset
    let mut state = abs_state / (base.w + 1);
    let mut offset = abs_state % (base.w + 1);
    debug_assert!(offset >= 0);

    if position == base.w {
      if state < 3 {
        let loc = vector * 3 + state;
        offset += levenshtein_automata::unpack(&OFFSET_INCRS_0, loc, 1);
        state = levenshtein_automata::unpack(&TO_STATES_0, loc, 2) - 1;
      }
    } else if position == base.w - 1 {
      if state < 5 {
        let loc = vector * 5 + state;
        offset += levenshtein_automata::unpack(&OFFSET_INCRS_1, loc, 1);
        state = levenshtein_automata::unpack(&TO_STATES_1, loc, 3) - 1;
      }
    } else if position == base.w - 2 {
      if state < 11 {
        let loc = vector * 11 + state;
        offset += levenshtein_automata::unpack(&OFFSET_INCRS_2, loc, 2);
        state = levenshtein_automata::unpack(&TO_STATES_2, loc, 4) - 1;
      }
    } else if position == base.w - 3 {
      if state < 21 {
        let loc = vector * 21 + state;
        offset += levenshtein_automata::unpack(&OFFSET_INCRS_3, loc, 2);
        state = levenshtein_automata::unpack(&TO_STATES_3, loc, 5) - 1;
      }
    } else if position == base.w - 4 {
      if state < 30 {
        let loc = vector * 30 + state;
        offset += levenshtein_automata::unpack(&OFFSET_INCRS_4, loc, 3);
        state = levenshtein_automata::unpack(&TO_STATES_4, loc, 5) - 1;
      }
    } else if state < 30 {
      let loc = vector * 30 + state;
      offset += levenshtein_automata::unpack(&OFFSET_INCRS_5, loc, 3);
      state = levenshtein_automata::unpack(&TO_STATES_5, loc, 5) - 1;
    }

    if state == -1 {
      // null state
      -1
    } else {
      // translate back to abs
      state * (base.w + 1) + offset
    }
  }
}

const fn long(value: u64) -> i64 {
  value as i64
}

// 1 vectors; 3 states per vector; array length = 3
const TO_STATES_0: [i64; 1] = [long(0xe)];
const OFFSET_INCRS_0: [i64; 1] = [long(0x0)];

// 2 vectors; 5 states per vector; array length = 10
const TO_STATES_1: [i64; 1] = [long(0x1a688a2c)];
const OFFSET_INCRS_1: [i64; 1] = [long(0x3e0)];

// 4 vectors; 11 states per vector; array length = 44
const TO_STATES_2: [i64; 3] = [
  long(0x3a07603570707054),
  long(0x522323232103773a),
  long(0x352254543213),
];
const OFFSET_INCRS_2: [i64; 2] = [long(0x5555520880080000), long(0x555555)];

// 8 vectors; 21 states per vector; array length = 168
const TO_STATES_3: [i64; 14] = [
  long(0x7000a560180380a4),
  long(0xc015a0180a0194a),
  long(0x8032c58318a301c0),
  long(0x9d8350d403980318),
  long(0x3006028ca73a8602),
  long(0xc51462640b21a807),
  long(0x2310c4100c62194e),
  long(0xce35884218ce248d),
  long(0xa9285a0691882358),
  long(0x1046b5a86b1252b5),
  long(0x2110a33892521483),
  long(0xe62906208d63394e),
  long(0xd6a29c4921d6a4a0),
  long(0x1a),
];
const OFFSET_INCRS_3: [i64; 6] = [
  long(0xf0c000c8c0080000),
  long(0xca808822003f303),
  long(0x5555553fa02f0880),
  long(0x5555555555555555),
  long(0x5555555555555555),
  long(0x5555),
];

// 16 vectors; 30 states per vector; array length = 480
const TO_STATES_4: [i64; 38] = [
  long(0x7000a560180380a4),
  long(0xa000000280e0294a),
  long(0x6c0b00e029000000),
  long(0x8c4350c59cdc6039),
  long(0x600ad00c03380601),
  long(0x2962c18c5180e00),
  long(0x18c4000c6028c4),
  long(0x8a314603801802b4),
  long(0x6328c4520c59c5),
  long(0x60d43500e600c651),
  long(0x280e339cea180a7),
  long(0x4039800000a318c6),
  long(0xd57be96039ec3d0d),
  long(0xc0338d6358c4352),
  long(0x28c4c81643500e60),
  long(0x3194a028c4339d8a),
  long(0x590d403980018c4),
  long(0xc4522d57b68e3132),
  long(0xc4100c6510d6538),
  long(0x9884218ce248d231),
  long(0x318ce318c6398d83),
  long(0xa3609c370c431046),
  long(0xea3ad6958568f7be),
  long(0x2d0348c411d47560),
  long(0x9ad43989295ad494),
  long(0x3104635ad431ad63),
  long(0x8f73a6b5250b40d2),
  long(0x57350eab9d693956),
  long(0x8ce24948520c411d),
  long(0x294a398d85608442),
  long(0x5694831046318ce5),
  long(0x958460f7b623609c),
  long(0xc411d475616258d6),
  long(0x9243ad4941cc520),
  long(0x5ad4529ce39ad456),
  long(0xb525073148310463),
  long(0x27656939460f7358),
  long(0x1d573516),
];
const OFFSET_INCRS_4: [i64; 23] = [
  long(0x610600010000000),
  long(0x2040000000001000),
  long(0x1044209245200),
  long(0x80d86d86006d80c0),
  long(0x2001b6030000006d),
  long(0x8200011b6237237),
  long(0x12490612400410),
  long(0x2449001040208000),
  long(0x4d80820001044925),
  long(0x6da4906da400),
  long(0x9252369001360208),
  long(0x24924924924911b6),
  long(0x9249249249249249),
  long(0x4924924924924924),
  long(0x2492492492492492),
  long(0x9249249249249249),
  long(0x4924924924924924),
  long(0x2492492492492492),
  long(0x9249249249249249),
  long(0x4924924924924924),
  long(0x2492492492492492),
  long(0x9249249249249249),
  long(0x24924924),
];

// 32 vectors; 30 states per vector; array length = 960
const TO_STATES_5: [i64; 75] = [
  long(0x7000a560180380a4),
  long(0xa000000280e0294a),
  long(0x580600e029000000),
  long(0x80e0600e529c0029),
  long(0x380a418c6388c631),
  long(0x316737180e5b02c0),
  long(0x300ce01806310d4),
  long(0xc60396c0b00e0290),
  long(0xca328c4350c59cd),
  long(0x80e00600ad194656),
  long(0x28c402962c18c51),
  long(0x802b40018c4000c6),
  long(0xe58b06314603801),
  long(0x8d6b48c6b580e348),
  long(0x28c5180e00600ad1),
  long(0x18ca31148316716),
  long(0x3801802b4031944),
  long(0xc4520c59c58a3146),
  long(0xe61956748cab38),
  long(0x39cea180a760d435),
  long(0xa318c60280e3),
  long(0x6029d8350d403980),
  long(0x6b5a80e060d873a8),
  long(0xf43500e618c638d),
  long(0x10d4b55efa580e7b),
  long(0x3980300ce358d63),
  long(0x57be96039ec3d0d4),
  long(0x4656567598c4352d),
  long(0x8c4c81643500e619),
  long(0x194a028c4339d8a2),
  long(0x590d403980018c43),
  long(0xe348d87628a31320),
  long(0xe618d6b4d6b1880),
  long(0x5eda38c4c8164350),
  long(0x19443594e31148b5),
  long(0x31320590d4039803),
  long(0x7160c4522d57b68e),
  long(0xd2310c41195674d6),
  long(0x8d839884218ce248),
  long(0x1046318ce318c639),
  long(0x2108633892348c43),
  long(0xdebfbdef0f63b0f6),
  long(0xd8270dc310c41f7b),
  long(0x8eb5a5615a3defa8),
  long(0x70c43104751d583a),
  long(0x58568f7bea3609c3),
  long(0x41f77ddb7bbeed69),
  long(0x9295ad4942d0348c),
  long(0xad431ad639ad4398),
  long(0x5250b40d23104635),
  long(0xce0f6bd0f624a56b),
  long(0x348c41f7b9cd7bd),
  long(0xe55a3dce9ad4942d),
  long(0x4755cd43aae75a4),
  long(0x73a6b5250b40d231),
  long(0xbd7bbcdd6939568f),
  long(0xe24948520c41f779),
  long(0x4a398d856084428c),
  long(0x14831046318ce529),
  long(0xb16c2110a3389252),
  long(0x1f7bdebe739c8f63),
  long(0xed88d82715a520c4),
  long(0x58589635a561183d),
  long(0x9c569483104751d),
  long(0xc56958460f7b6236),
  long(0x520c41f77ddb6719),
  long(0x45609243ad4941cc),
  long(0x4635ad4529ce39ad),
  long(0x90eb525073148310),
  long(0xd6737b8f6bd16c24),
  long(0x941cc520c41f7b9c),
  long(0x95a4e5183dcd62d4),
  long(0x483104755cd4589d),
  long(0x460f7358b5250731),
  long(0xf779bd6717b56939),
];
const OFFSET_INCRS_5: [i64; 45] = [
  long(0x610600010000000),
  long(0x40000000001000),
  long(0xb6d56da184180),
  long(0x824914800810000),
  long(0x2002040000000411),
  long(0xc0000b2c5659245),
  long(0x6d80d86d86006d8),
  long(0x1b61801b60300000),
  long(0x6d80c0000b5b76b6),
  long(0x46d88dc8dc800),
  long(0x6372372001b60300),
  long(0x400410082000b1b7),
  long(0x2080000012490612),
  long(0x6d49241849001040),
  long(0x912400410082000b),
  long(0x402080004112494),
  long(0xb2c49252449001),
  long(0x4906da4004d80820),
  long(0x136020800006da),
  long(0x82000b5b69241b69),
  long(0x6da4948da4004d80),
  long(0x3690013602080004),
  long(0x49249249b1b69252),
  long(0x2492492492492492),
  long(0x9249249249249249),
  long(0x4924924924924924),
  long(0x2492492492492492),
  long(0x9249249249249249),
  long(0x4924924924924924),
  long(0x2492492492492492),
  long(0x9249249249249249),
  long(0x4924924924924924),
  long(0x2492492492492492),
  long(0x9249249249249249),
  long(0x4924924924924924),
  long(0x2492492492492492),
  long(0x9249249249249249),
  long(0x4924924924924924),
  long(0x2492492492492492),
  long(0x9249249249249249),
  long(0x4924924924924924),
  long(0x2492492492492492),
  long(0x9249249249249249),
  long(0x4924924924924924),
  long(0x2492492492492492),
];
