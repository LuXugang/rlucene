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
use std::collections::HashSet;
use std::string::FromUtf16Error;

use rand::Rng;
use rand::RngExt;

use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::byte_run_automaton::ByteRunAutomaton;
use crate::core::util::automation::byte_runnable::ByteRunnable;
use crate::core::util::automation::character_run_automaton::CharacterRunAutomaton;
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::automation::utf32_to_utf8::UTF32ToUTF8;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fst_impl::util::Util;
use crate::core::util::ints_ref_builder::IntsRefBuilder;
use crate::core::util::unicode_util::UnicodeUtil;
use crate::test::core::util::automaton::automaton_test_util::{
  AutomatonTestUtil, RandomAcceptedStrings,
};
use crate::test::core::util::automaton::test_operations::TestOperations;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, at_least_usize, new_bytes_ref_from_string, random,
};
use crate::test::core::util::test_util::TestUtil;
#[allow(dead_code)] // for quick search
struct TestUTF32ToUTF8;
const MAX_UNICODE: i32 = 0x10FFFF;
fn matches(a: &mut ByteRunAutomaton, code: i32) -> Result<bool> {
  let ch = std::char::from_u32(code as u32)
    .ok_or_else(|| LuceneError::illegal_argument("Invalid Unicode code point"))?;
  let len = UnicodeUtil::max_utf8_length(code)?;
  let mut buf = vec![0; len];
  let _ = ch.encode_utf8(&mut buf);
  a.run(buf.as_slice(), 0, len)
}
fn test_one<R>(
  random: &mut R,
  a: &mut ByteRunAutomaton,
  start_code: i32,
  end_code: i32,
  iters: usize,
) -> Result<()>
where
  R: Rng + ?Sized,
{
  let sur_start = UnicodeUtil::UNI_SUR_HIGH_START;
  let sur_end = UnicodeUtil::UNI_SUR_LOW_END;

  let (non_surrogate_count, ov_sur_start) = if end_code < sur_start || start_code > sur_end {
    (end_code - start_code + 1, false)
  } else if is_surrogate(start_code) {
    (
      end_code - start_code + 1 - (sur_end - start_code + 1),
      false,
    )
  } else if is_surrogate(end_code) {
    (end_code - start_code + 1 - (end_code - sur_start + 1), true)
  } else {
    (end_code - start_code + 1 - (sur_end - sur_start + 1), true)
  };

  assert!(non_surrogate_count > 0);

  for _ in 0..iters {
    let mut code = start_code + random.random_range(0..non_surrogate_count);
    if is_surrogate(code) {
      if ov_sur_start {
        code = sur_end + 1 + (code - sur_start);
      } else {
        code = sur_end + 1 + (code - start_code);
      }
    }

    assert!(
      code >= start_code && code <= end_code,
      "code={} start={} end={}",
      code,
      start_code,
      end_code
    );
    assert!(!is_surrogate(code));

    assert!(
      matches(a, code)?,
      "DFA for range {}-{} failed to match code={}",
      start_code,
      end_code,
      code
    );
  }

  // check out-of-range values are NOT accepted
  let invalid_range = MAX_UNICODE - (end_code - start_code + 1);
  if invalid_range > 0 {
    for _ in 0..iters {
      let x = random.random_range(0..invalid_range);
      let code = if x >= start_code {
        end_code + 1 + x - start_code
      } else {
        x
      };

      if is_surrogate(code) {
        continue;
      }

      assert!(
        !matches(a, code)?,
        "DFA for range {}-{} matched invalid code={}",
        start_code,
        end_code,
        code
      );
    }
  }
  Ok(())
}
fn get_code_start<R>(random: &mut R) -> i32
where
  R: Rng + ?Sized,
{
  match random.random_range(0..4) {
    0 => random.random_range(0..128),
    1 => random.random_range(128..2048),
    2 => random.random_range(2048..65536),
    _ => random.random_range(65536..=MAX_UNICODE),
  }
}
fn is_surrogate(code: i32) -> bool {
  (UnicodeUtil::UNI_SUR_HIGH_START..=UnicodeUtil::UNI_SUR_HIGH_END).contains(&code)
    || (UnicodeUtil::UNI_SUR_LOW_START..=UnicodeUtil::UNI_SUR_LOW_END).contains(&code)
}

#[test]
fn test_random_ranges() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 10);
  let iters_per_dfa = at_least_usize(&mut random, 100);

  for _ in 0..iters {
    let x1 = get_code_start(&mut random);
    let x2 = get_code_start(&mut random);
    let (start_code, end_code) = if x1 < x2 { (x1, x2) } else { (x2, x1) };

    if is_surrogate(start_code) && is_surrogate(end_code) {
      continue;
    }

    let a = Automata::make_char_range(start_code, end_code)?;
    let mut dfa = ByteRunAutomaton::new(a)?;
    test_one(&mut random, &mut dfa, start_code, end_code, iters_per_dfa)?;
  }

  Ok(())
}
#[test]
fn test_special_case() -> Result<()> {
  let re = RegExp::from_string(".?")?;
  let automaton = re.to_automaton()?;

  let cra = CharacterRunAutomaton::new(automaton.clone())?;
  let mut bra = ByteRunAutomaton::new(automaton)?;

  // make sure character dfa accepts empty string
  assert!(cra.base.is_accept(0)?);
  assert!(cra.run_str("")?);
  assert!(cra.run_chars(&[], 0, 0)?);

  // make sure byte dfa accepts empty string
  assert!(bra.is_accept(0)?);
  assert!(bra.run(&[], 0, 0)?);

  Ok(())
}
#[test]
fn test_special_case2() -> Result<()> {
  let utf16: [u16; 12] = [
    0xfadc, 0xfffd, 0xb80b, 0xda5a, 0xdc68, 0xf234, 0x0056, 0xda5b, 0xdcc1, 0xfffd, 0xfffd, 0x0775,
  ];

  let input = String::from_utf16(&utf16).map_err(|e: FromUtf16Error| {
    LuceneError::illegal_argument(format!("invalid UTF-16 input: {e}"))
  })?;

  let re = RegExp::from_string(".+\u{0775}")?;
  let mut automaton = re.to_automaton()?;
  automaton =
    Operations::determinize(&automaton, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?.into_owned();

  let cra = CharacterRunAutomaton::new(automaton.clone())?;
  let mut bra = ByteRunAutomaton::new(automaton)?;

  assert!(cra.run_str(&input)?);

  let bytes = input.as_bytes();
  assert!(bra.run(bytes, 0, bytes.len())?);

  Ok(())
}
#[test]
fn test_special_case3() -> Result<()> {
  let utf16_input: [u16; 15] = [
    0x5cfd, 0xfffd, 0xb2f7, 0x0033, 0xe304, 0x51d7, 0x3692, 0xdb50, 0xdfb3, 0x0576, 0xdae2, 0xdc62,
    0x0053, 0x0449, 0x04d4,
  ];
  let input = String::from_utf16(&utf16_input)
    .map_err(|e| LuceneError::illegal_argument(format!("invalid UTF-16 input: {e}")))?;

  let utf16_regex: [u16; 11] = [
    0x0028, 0x005c, 0x9bfa, 0x0029, 0x002a, 0x0028, 0x002e, 0x0029, 0x002a, 0x005c, 0x04d4,
  ];
  let regex_str = String::from_utf16(&utf16_regex)
    .map_err(|e| LuceneError::illegal_argument(format!("invalid UTF-16 regex: {e}")))?;

  let re = RegExp::from_string(&regex_str)?;
  let mut automaton = re.to_automaton()?;
  automaton =
    Operations::determinize(&automaton, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?.into_owned();

  let cra = CharacterRunAutomaton::new(automaton.clone())?;
  let mut bra = ByteRunAutomaton::new(automaton)?;

  assert!(cra.run_str(&input)?);

  let bytes = input.as_bytes();
  assert!(bra.run(bytes, 0, bytes.len())?);

  Ok(())
}

#[test]
fn test_random_regexes() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 50);

  for _ in 0..num {
    let s = AutomatonTestUtil::random_regexp(&mut random)?;
    let mut automaton = RegExp::from_str_with_flags(&s, RegExp::NONE)?.to_automaton()?;
    automaton =
      Operations::determinize(&automaton, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?.into_owned();
    assert_automaton(&mut random, &automaton)?;
  }

  Ok(())
}

#[test]
fn test_singleton() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 100);

  for _ in 0..iters {
    let s = TestUtil::random_realistic_unicode_string(&mut random);
    let a = Automata::make_string(&s)?;
    let utf8 = UTF32ToUTF8::new().convert(&a)?.into_owned();

    let mut ints = IntsRefBuilder::new();
    Util::to_ints_ref(
      &new_bytes_ref_from_string::<_, Vec<u8>>(&mut random, &s)?,
      &mut ints,
    );
    let mut set = HashSet::new();
    set.insert(ints.get_owner());

    let actual = TestOperations::get_finite_strings(&utf8)?;
    assert_eq!(set, actual, "Failed for input string: {:?}", s);
  }

  Ok(())
}

fn assert_automaton<R>(random: &mut R, a: &Automaton) -> Result<()>
where
  R: Rng + ?Sized,
{
  let cra = CharacterRunAutomaton::new(a.clone())?;
  let mut bra = ByteRunAutomaton::new(a.clone())?;
  let ras = RandomAcceptedStrings::new(a)?;

  let num = at_least(random, 1000);

  for _ in 0..num {
    let string = if random.random_bool(0.5) {
      // likely not accepted
      TestUtil::random_unicode_string(random)
    } else {
      // will be accepted
      let codepoints = ras.get_random_accepted_string(random)?;
      UnicodeUtil::new_string(&codepoints, 0, codepoints.len())?
    };

    let bytes = string.as_bytes();
    let cra_result = cra.run_str(&string)?;
    let bra_result = bra.run(bytes, 0, bytes.len())?;

    assert_eq!(
      cra_result, bra_result,
      "Mismatch on input: {:?} (UTF-8: {:?})",
      string, bytes
    );
  }

  Ok(())
}
