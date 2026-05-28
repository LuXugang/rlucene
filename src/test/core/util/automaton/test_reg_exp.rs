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

use rand::Rng;
use rand::RngExt;
use regex::Regex;

use crate::core::index::BytesRef;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::byte_run_automaton::ByteRunAutomaton;
use crate::core::util::automation::byte_runnable::ByteRunnable;
use crate::core::util::automation::character_run_automaton::CharacterRunAutomaton;
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::util::automaton::automaton_test_util::AutomatonTestUtil;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
#[allow(dead_code)] // for quick search
struct TestRegExp {
  case_sensitive_query: bool,
}
impl TestRegExp {
  fn random_doc_value<R>(random: &mut R, min_length: usize) -> String
  where
    R: Rng + ?Sized,
  {
    let char_palette = "AAAaaaBbbCccc123456 \t".chars().collect::<Vec<_>>();
    (0..min_length)
      .map(|_| {
        let i = Self::random_int(random, char_palette.len());
        char_palette[i]
      })
      .collect()
  }
  fn random_int<R>(random: &mut R, bound: usize) -> usize
  where
    R: Rng + ?Sized,
  {
    if bound == 0 {
      0
    } else {
      random.random_range(0..bound)
    }
  }
  fn check_random_expression<R>(&mut self, random: &mut R, doc_value: &str) -> Result<String>
  where
    R: Rng + ?Sized,
  {
    use std::fmt::Write;
    // Generate and test a random regular expression which should match the given
    // docValue
    let mut result = String::new();
    let len = doc_value.len();
    // Pick a part of the string to change
    let substitution_point = random.random_range(0..len);
    let substitution_length =
      1 + random.random_range(0..(std::cmp::min(10, len - substitution_point)));

    let head = &doc_value[..substitution_point];
    result.push_str(head);

    let replacement_part = &doc_value[substitution_point..substitution_point + substitution_length];
    let mutation = random.random_range(0..15);

    match mutation {
      0 => {
        let rand_str = Self::random_doc_value(random, replacement_part.len());
        write!(result, "({}|d{})", replacement_part, rand_str)?;
      },
      1 => {
        write!(result, "({}|doesnotexist)", replacement_part)?;
      },
      2 => {
        let inner = self.check_random_expression(random, replacement_part)?;
        write!(result, "({}|doesnotexist)", inner)?;
      },
      3 => {
        result.push_str(&replacement_part.replace("ab", ".*"));
      },
      4 => {
        result.push_str(&replacement_part.replace("b", "."));
      },
      5 => {
        write!(result, ".{{1,{}}}", replacement_part.len())?;
      },
      6 => {
        result.push_str(&".".repeat(replacement_part.len()));
      },
      7 => {
        for c in replacement_part.chars() {
          write!(result, "[{}{}]", c, c.to_ascii_uppercase())?;
        }
      },
      8 => {
        result.push_str(&replacement_part.replace("b", "[^a]"));
      },
      9 => {
        write!(result, "({})+", replacement_part)?;
      },
      10 => {
        write!(result, "({})?", replacement_part)?;
      },
      11 => {
        let re = Regex::new(r"\d").unwrap();
        result.push_str(&re.replace_all(replacement_part, r"\d"));
      },
      12 => {
        let re = Regex::new(r"\s").unwrap();
        result.push_str(&re.replace_all(replacement_part, r"\W"));
      },
      13 => {
        let re = Regex::new(r"\s").unwrap();
        result.push_str(&re.replace_all(replacement_part, r"\s"));
      },
      14 => {
        let mut switched = String::new();
        for p in replacement_part.chars() {
          let new_p = if p.is_lowercase() {
            p.to_ascii_uppercase()
          } else {
            p.to_ascii_lowercase()
          };
          switched.push(new_p);
          if p != new_p {
            self.case_sensitive_query = false;
          }
        }
        result.push_str(&switched);
      },
      _ => {},
    }
    // add any remaining tail, unchanged
    if substitution_point + substitution_length < len {
      result.push_str(&doc_value[substitution_point + substitution_length..]);
    }

    let regex_pattern = result;
    // Assert our randomly generated regex actually matches the provided raw input
    // using java's expression matcher
    let re = if self.case_sensitive_query {
      Regex::new(&regex_pattern).unwrap()
    } else {
      Regex::new(&format!("(?i){}", regex_pattern)).unwrap()
    };
    assert!(
      re.is_match(doc_value),
      "Regex `{}` did not match `{}`",
      regex_pattern,
      doc_value
    );

    let match_flags = if self.case_sensitive_query {
      0
    } else {
      RegExp::ASCII_CASE_INSENSITIVE
    };
    let regex = RegExp::parse(&regex_pattern, RegExp::ALL, match_flags)?;
    let v = regex.to_automaton()?;
    let automaton = Operations::determinize(&v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
    let matcher = ByteRunAutomaton::new(automaton.into_owned())?;

    let br: BytesRef<Vec<u8>> = BytesRef::from_string(doc_value);
    assert!(
      matcher.run(&br.bytes, br.offset, br.length)?,
      "[{}] should match [{}] {}-{}/{}",
      regex_pattern,
      doc_value,
      substitution_point,
      substitution_length,
      len
    );

    if !self.case_sensitive_query {
      let cs_regex = RegExp::parse(&regex_pattern, RegExp::ALL, 0)?;
      let v = cs_regex.to_automaton()?;
      let cs_automaton = Operations::determinize(&v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
      let cs_matcher = ByteRunAutomaton::new(cs_automaton.into_owned())?;
      assert!(
        !cs_matcher.run(&br.bytes, br.offset, br.length)?,
        "[{}] (case-sensitive) should not match [{}]",
        regex_pattern,
        doc_value
      );
    }

    Ok(regex_pattern)
  }
}

/// Simple smoke test for regular expression.
#[test]
fn test_smoke() -> Result<()> {
  let r = RegExp::from_str_with_flags("a(b+|c+)d", 0)?;
  let a = r.to_automaton()?;
  assert!(a.is_deterministic());

  let run = CharacterRunAutomaton::new(a)?;
  assert!(run.run_str("abbbbbd")?);
  assert!(run.run_str("acd")?);
  assert!(!run.run_str("ad")?);

  Ok(())
}
// LUCENE-6046
#[test]
fn test_repeat_with_empty_string() -> Result<()> {
  let a = RegExp::from_str_with_flags("[^y]*{1,2}", 0)?.to_automaton()?;

  // paranoia
  let s = format!("{:?}", a);
  assert!(!s.is_empty());

  Ok(())
}
#[test]
fn test_repeat_with_empty_language() -> Result<()> {
  let patterns = ["#*", "#+", "#{2,10}", "#?"];

  for pat in patterns {
    let a = RegExp::from_str_with_flags(pat, 0)?.to_automaton()?;
    let s = format!("{:?}", a);
    assert!(
      !s.is_empty(),
      "Automaton is unexpectedly empty for pattern: {}",
      pat
    );
  }

  Ok(())
}
#[test]
fn test_core_java_parity() -> Result<()> {
  let mut random = random();
  let mut test = TestRegExp {
    case_sensitive_query: true,
  };

  for _ in 0..1000 {
    test.case_sensitive_query = true;
    let min_length = random.random_range(0..30);
    let doc_value = TestRegExp::random_doc_value(&mut random, 1 + min_length);
    test.check_random_expression(&mut random, &doc_value)?;
  }
  Ok(())
}

#[test]
fn test_illegal_backslash_chars() {
  let illegal_chars = "abcefghijklmnopqrtuvxyzABCEFGHIJKLMNOPQRTUVXYZ";

  for ch in illegal_chars.chars() {
    let expr = format!("\\{}", ch);
    let err = RegExp::from_string(&expr);
    assert!(
      matches!(err, Err(LuceneError::IllegalArgument(_))),
      "Expected IllegalArgument for `\\{}` but got: {:?}",
      ch,
      err
    );
    assert!(
      err
        .unwrap_err()
        .to_string()
        .contains("invalid character class")
    );
  }
}

#[test]
fn test_legal_backslash_chars() -> Result<()> {
  let legal_chars = "dDsSWw0123456789[]*&^$@!{}\\/";

  for ch in legal_chars.chars() {
    let expr = format!("\\{}", ch);
    RegExp::from_string(&expr)?;
  }

  Ok(())
}

#[test]
fn test_parse_illegal_repeat_exp() -> Result<()> {
  let err = RegExp::parse("a{99,11}", RegExp::ALL, 0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert!(err.unwrap_err().to_string().contains("out of order"));

  Ok(())
}

#[test]
fn test_regexp_no_stack_overflow() -> Result<()> {
  let mut pattern = "(a)|".repeat(50_000);
  pattern.push_str("(a)");
  let _ = RegExp::from_string(&pattern)?;
  Ok(())
}
/// Tests the deprecated complement flag.
/// Keep the simple test only—no random tests to avoid instability.
///
/// @deprecated Remove in Lucene 11
#[test]
fn test_deprecated_complement() -> Result<()> {
  let expected = {
    let a = Automata::make_string("abcd")?;
    Operations::complement(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
  };
  #[allow(deprecated)]
  let actual = RegExp::parse("~(abcd)", RegExp::DEPRECATED_COMPLEMENT, 0)?.to_automaton()?;
  assert!(
    AutomatonTestUtil::same_language(&expected, &actual)?,
    "Automaton language differs between expected and actual"
  );

  Ok(())
}
