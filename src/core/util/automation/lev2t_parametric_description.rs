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

/// Parametric description for generating a Levenshtein automaton of degree 2,
/// with transpositions as primitive edits. The comment in [`Lev1ParametricDescription`]
/// may be helpful for you to understand this class.
///
/// See [`Lev1ParametricDescription`].
pub(crate) struct Lev2TParametricDescription;

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
//   11 -> [t(0, 1), (0, 1), (1, 1), (2, 1)]
//   12 -> [t(0, 2), (0, 2), (1, 2), (2, 2)]
//   13 -> [(0, 2), (1, 2), (2, 2), (3, 2)]
//   14 -> [(0, 1), (1, 1), (3, 2)]
//   15 -> [(0, 1), (2, 2), (3, 2)]
//   16 -> [(0, 1), (3, 2)]
//   17 -> [(0, 1), t(1, 2), (2, 2), (3, 2)]
//   18 -> [(0, 2), (1, 2), (3, 1)]
//   19 -> [(0, 2), (1, 2), (3, 2)]
//   20 -> [(0, 2), (1, 2), t(1, 2), (2, 2), (3, 2)]
//   21 -> [(0, 2), (2, 1), (3, 1)]
//   22 -> [(0, 2), (2, 2), (3, 2)]
//   23 -> [(0, 2), (3, 1)]
//   24 -> [(0, 2), (3, 2)]
//   25 -> [(0, 2), t(1, 2), (1, 2), (2, 2), (3, 2)]
//   26 -> [t(0, 2), (0, 2), (1, 2), (2, 2), (3, 2)]
//   27 -> [t(0, 2), (0, 2), (1, 2), (3, 1)]
//   28 -> [(0, 2), (1, 2), (2, 2), (3, 2), (4, 2)]
//   29 -> [(0, 2), (1, 2), (2, 2), (4, 2)]
//   30 -> [(0, 2), (1, 2), (2, 2), t(2, 2), (3, 2), (4, 2)]
//   31 -> [(0, 2), (1, 2), (3, 2), (4, 2)]
//   32 -> [(0, 2), (1, 2), (4, 2)]
//   33 -> [(0, 2), (1, 2), t(1, 2), (2, 2), (3, 2), (4, 2)]
//   34 -> [(0, 2), (1, 2), t(2, 2), (2, 2), (3, 2), (4, 2)]
//   35 -> [(0, 2), (2, 1), (4, 2)]
//   36 -> [(0, 2), (2, 2), (3, 2), (4, 2)]
//   37 -> [(0, 2), (2, 2), (4, 2)]
//   38 -> [(0, 2), (3, 2), (4, 2)]
//   39 -> [(0, 2), (4, 2)]
//   40 -> [(0, 2), t(1, 2), (1, 2), (2, 2), (3, 2), (4, 2)]
//   41 -> [(0, 2), t(2, 2), (2, 2), (3, 2), (4, 2)]
//   42 -> [t(0, 2), (0, 2), (1, 2), (2, 2), (3, 2), (4, 2)]
//   43 -> [t(0, 2), (0, 2), (1, 2), (2, 2), (4, 2)]
//   44 -> [t(0, 2), (0, 2), (1, 2), (2, 2), t(2, 2), (3, 2), (4, 2)]

pub(crate) fn new(w: i32) -> ParametricDescription {
  ParametricDescription::new(
    w,
    2,
    vec![
      0, 1, 2, 0, 1, -1, 0, -1, 0, -1, 0, -1, 0, -1, -1, -1, -1, -1, -2, -1, -1, -2, -1, -2, -1,
      -1, -1, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2,
    ],
    Lev2TParametricDescription,
  )
}

impl ParametricDescriptionBase for Lev2TParametricDescription {
  fn transition(
    &self,
    abs_state: i32,
    position: i32,
    vector: i32,
    base: &ParametricDescription,
  ) -> i32 {
    debug_assert_ne!(abs_state, -1);

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
      if state < 13 {
        let loc = vector * 13 + state;
        offset += levenshtein_automata::unpack(&OFFSET_INCRS_2, loc, 2);
        state = levenshtein_automata::unpack(&TO_STATES_2, loc, 4) - 1;
      }
    } else if position == base.w - 3 {
      if state < 28 {
        let loc = vector * 28 + state;
        offset += levenshtein_automata::unpack(&OFFSET_INCRS_3, loc, 2);
        state = levenshtein_automata::unpack(&TO_STATES_3, loc, 5) - 1;
      }
    } else if position == base.w - 4 {
      if state < 45 {
        let loc = vector * 45 + state;
        offset += levenshtein_automata::unpack(&OFFSET_INCRS_4, loc, 3);
        state = levenshtein_automata::unpack(&TO_STATES_4, loc, 6) - 1;
      }
    } else if state < 45 {
      let loc = vector * 45 + state;
      offset += levenshtein_automata::unpack(&OFFSET_INCRS_5, loc, 3);
      state = levenshtein_automata::unpack(&TO_STATES_5, loc, 6) - 1;
    }

    if state == -1 {
      -1
    } else {
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

// 4 vectors; 13 states per vector; array length = 52
const TO_STATES_2: [i64; 4] = [
  long(0xdc0703570707054),
  long(0x2323213a03dd3a3a),
  long(0x2254543215435223),
  long(0x5435),
];
const OFFSET_INCRS_2: [i64; 2] = [long(0x5558208800080000), long(0x5555555555)];

// 8 vectors; 28 states per vector; array length = 224
const TO_STATES_3: [i64; 18] = [
  long(0x700a5701c0380a4),
  long(0x180a000ca529c0),
  long(0xc5498e60a80af180),
  long(0x8c4300e85a546398),
  long(0xd8d43501ac18c601),
  long(0x51976d6a863500ad),
  long(0xc3501ac28ca0180a),
  long(0x76dda8a5b0c5be16),
  long(0xc41294a018c4519),
  long(0x1086520ce248d231),
  long(0x13946358ce31ac42),
  long(0x6732d4942d0348c4),
  long(0xd635ad4b1ad224a5),
  long(0xce24948520c4139),
  long(0x58ce729d22110a52),
  long(0x941cc520c41394e3),
  long(0x4729d22490e732d4),
  long(0x39ce35ad),
];
const OFFSET_INCRS_3: [i64; 7] = [
  long(0xc0c83000080000),
  long(0x2200fcff300f3c30),
  long(0x3c2200a8caa00a08),
  long(0x55555555a8fea00a),
  long(0x5555555555555555),
  long(0x5555555555555555),
  long(0x5555555555555555),
];

// 16 vectors; 45 states per vector; array length = 720
const TO_STATES_4: [i64; 68] = [
  long(0x1453803801c0144),
  long(0xc000514514700038),
  long(0x1400001401),
  long(0x140000),
  long(0x6301f00700510000),
  long(0xa186178301f00d1),
  long(0xc20c30c20ca0c3),
  long(0xc00c00cd0c30030c),
  long(0x4c054014f0c00c30),
  long(0x55150c34c30944c3),
  long(0x430c014308300550),
  long(0xc30850c00050c31),
  long(0x50053c50c3143000),
  long(0x850d30c25130d301),
  long(0xc21441430a08608),
  long(0x2145003143142145),
  long(0x4c1431451400c314),
  long(0x28014d6c32832803),
  long(0x1c50c76cd34a0c3),
  long(0x430c30c31c314014),
  long(0xc30050000001431),
  long(0xd36d0e40ca00d303),
  long(0xcb2abb2c90b0e400),
  long(0x2c32ca2c70c20ca1),
  long(0x31c00c00cd2c70cb),
  long(0x558328034c2c32c),
  long(0x6cd6ca14558309b7),
  long(0x51c51401430850c7),
  long(0xc30871430c714),
  long(0xca00d3071451450),
  long(0xb9071560c26dc156),
  long(0xc70c21441cb2abb2),
  long(0x1421c70cb1c51ca1),
  long(0x30811c51c51c00c3),
  long(0xc51031c224324308),
  long(0x5c33830d70820820),
  long(0x30c30c30c33850c3),
  long(0x451450c30c30c31c),
  long(0xda0920d20c20c20),
  long(0x365961145145914f),
  long(0xd964365351965865),
  long(0x51964364365a6590),
  long(0x920b203243081505),
  long(0xd72422492c718b28),
  long(0x2cb3872c35cb28b0),
  long(0xb0c32cb2972c30d7),
  long(0xc80c90c204e1c75c),
  long(0x4504171c62ca2482),
  long(0x33976585d65d9610),
  long(0x4b5ca5d70d95cb5d),
  long(0x1030813873975c36),
  long(0x41451031c2245105),
  long(0xc35c338714e24208),
  long(0x1c51c51451453851),
  long(0x20451450c70c30c3),
  long(0x4f0da09214f1440c),
  long(0x6533944d04513d41),
  long(0xe15450551350e658),
  long(0x551938364365a50),
  long(0x2892071851030815),
  long(0x714e2422441c718b),
  long(0x4e1c73871c35cb28),
  long(0x5c70c32cb28e1c51),
  long(0x81c61440c204e1c7),
  long(0xd04503ce1c62ca24),
  long(0x39338e6585d63944),
  long(0x364b5ca38e154387),
  long(0x38739738),
];
const OFFSET_INCRS_4: [i64; 34] = [
  long(0xc0000010000000),
  long(0x40000060061),
  long(0x8001000800000000),
  long(0x8229048249248a4),
  long(0x6c360300002092),
  long(0x6db6036db61b6c30),
  long(0x361b0180000db6c0),
  long(0xdb11b71b91b72000),
  long(0x100820006db6236),
  long(0x2492490612480012),
  long(0x8041000248200049),
  long(0x4924a48924000900),
  long(0x2080012510822492),
  long(0x9241b69200048360),
  long(0x4000926806da4924),
  long(0x291b49000241b010),
  long(0x494934236d249249),
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
  long(0x249249249249),
];

// 32 vectors; 45 states per vector; array length = 1440
const TO_STATES_5: [i64; 135] = [
  long(0x1453803801c0144),
  long(0xc000514514700038),
  long(0x1400001401),
  long(0x140000),
  long(0x4e00e00700510000),
  long(0x3451451c000e0051),
  long(0x30cd00000d015000),
  long(0xc30c30d40c30c30c),
  long(0x7c01c01440c30c30),
  long(0x185e0c07c03458c0),
  long(0x830c30832830c286),
  long(0x33430c00c30030),
  long(0x70051030030c3003),
  long(0x8301f00d16301f00),
  long(0xc20ca0c30a18617),
  long(0xb1450c51431420c3),
  long(0x4f14314514314315),
  long(0x4c30944c34c05401),
  long(0x30830055055150c3),
  long(0xc00050c31430c014),
  long(0xc31430000c30850),
  long(0x25130d30150053c5),
  long(0xc03541545430d30c),
  long(0x1cb2cd0c300d0c90),
  long(0x72c30cb2c91cb0c3),
  long(0xc34c054014f1cb2c),
  long(0x8218221434c30944),
  long(0x50851430851050c2),
  long(0x30c50851400c50c),
  long(0x150053c50c51450),
  long(0x8850d30c25130d3),
  long(0x450c21441430a086),
  long(0x1c91c70c51cb1c21),
  long(0x34c1cb1c71c314b),
  long(0xc328014d6c328328),
  long(0x1401c50c76cd34a0),
  long(0x31430c30c31c3140),
  long(0x30c300500000014),
  long(0x535b0ca0ca00d3),
  long(0x514369b34d2830ca),
  long(0x5965965a0c500d01),
  long(0x6435030c30d46546),
  long(0xdb4390328034c659),
  long(0xcaaecb242c390034),
  long(0xcb28b1c30832872),
  long(0x700300334b1c32cb),
  long(0xe40ca00d30b0cb0c),
  long(0xb2c90b0e400d36d0),
  long(0xa2c70c20ca1cb2ab),
  long(0x4315b5ce6575d95c),
  long(0x28034c5d95c53831),
  long(0xa14558309b705583),
  long(0x401430850c76cd6c),
  long(0x871430c71451c51),
  long(0xd3071451450000c3),
  long(0x560c26dc1560ca00),
  long(0xc914369b35b2851),
  long(0x465939451a14500d),
  long(0x945075030cb2c939),
  long(0x9b70558328034c3),
  long(0x72caaecae41c5583),
  long(0xc71472871c308510),
  long(0x1470030c50871c32),
  long(0xc1560ca00d307147),
  long(0xabb2b9071560c26d),
  long(0x38a1c70c21441cb2),
  long(0x314b1c938e657394),
  long(0x4308308139438738),
  long(0x820c51031c22432),
  long(0x50c35c33830d7082),
  long(0xc31c30c30c30c338),
  long(0xc20451450c30c30),
  long(0x31440c70890c90c2),
  long(0xea0df0c3a8208208),
  long(0xa28a28a28a231430),
  long(0x1861868a28a28a1e),
  long(0xc368248348308308),
  long(0x4d96584514516453),
  long(0x36590d94d4659619),
  long(0x546590d90d969964),
  long(0x920d20c20c20541),
  long(0x961145145914f0da),
  long(0xe89d351965865365),
  long(0x9e89e89e99e7a279),
  long(0xb203243081821827),
  long(0x422492c718b28920),
  long(0x3872c35cb28b0d72),
  long(0x32cb2972c30d72cb),
  long(0xc90c204e1c75cb0c),
  long(0x24b1c62ca2482c80),
  long(0xb0ea2e42c3a89089),
  long(0xa4966a289669a31c),
  long(0x8175e7a59a8a269),
  long(0x718b28920b203243),
  long(0x175976584114105c),
  long(0x5c36572d74ce5d96),
  long(0xe1ce5d70d92d7297),
  long(0xca2482c80c90c204),
  long(0x5d96104504171c62),
  long(0x79669533976585d6),
  long(0x659689e6964965a2),
  long(0x24510510308175e7),
  long(0xe2420841451031c2),
  long(0x453851c35c338714),
  long(0xc30c31c51c51451),
  long(0x41440c20451450c7),
  long(0x821051440c708914),
  long(0x1470ea0df1c58c90),
  long(0x8a1e85e861861863),
  long(0x30818618687a8a2),
  long(0x5053c36824853c51),
  long(0x96194ce51341144f),
  long(0x943855141544d439),
  long(0x5415464e0d90d96),
  long(0xf0da09214f1440c2),
  long(0x533944d04513d414),
  long(0x86082181350e6586),
  long(0x18277689e89e981d),
  long(0x8920718510308182),
  long(0x14e2422441c718b2),
  long(0xe1c73871c35cb287),
  long(0xc70c32cb28e1c514),
  long(0x1c61440c204e1c75),
  long(0x90891071c62ca248),
  long(0xa31c70ea2e41c58c),
  long(0xa269a475e86175e7),
  long(0x510308175e7a57a8),
  long(0xf38718b28920718),
  long(0x39961758e5134114),
  long(0x728e38550e1ce4ce),
  long(0xc204e1ce5ce0d92d),
  long(0x1c62ca2481c61440),
  long(0x85d63944d04503ce),
  long(0x5d86075e75338e65),
  long(0x75e7657689e69647),
];
const OFFSET_INCRS_5: [i64; 68] = [
  long(0xc0000010000000),
  long(0x40000060061),
  long(0x6000000800000000),
  long(0xdb6ab6db6b003080),
  long(0x80040000002db6),
  long(0x1148241249245240),
  long(0x4002000000104904),
  long(0xa4b2592492292000),
  long(0xd80c00009649658),
  long(0x80db6d86db0c001b),
  long(0xc06000036db01b6d),
  long(0x6db6c36d86000d86),
  long(0x300001b6ddadb6ed),
  long(0xe37236e40006c360),
  long(0xdb6c46db6236),
  long(0xb91b72000361b018),
  long(0x6db7636dbb1b71),
  long(0x6124800120100820),
  long(0x2482000492492490),
  long(0x9240009008041000),
  long(0x555b6a4924924830),
  long(0x2000480402080012),
  long(0x8411249249252449),
  long(0x24020104000928),
  long(0x5892492492922490),
  long(0x120d808200049456),
  long(0x6924924906da4800),
  long(0x6c041000249a01b),
  long(0x924924836d240009),
  long(0x6020800124d5adb4),
  long(0x2492523692000483),
  long(0x104000926846da49),
  long(0x49291b49000241b0),
  long(0x92494935636d2492),
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
  long(0x24924924),
];
