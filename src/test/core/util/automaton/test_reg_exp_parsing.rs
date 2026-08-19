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

use crate::test_framework::core::util::lucene_test_case::random;
use std::collections::{HashMap, HashSet};

use rand::Rng;
use rand::RngExt;
use regex::Regex;

use crate::core::index::BytesRef;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::automaton_provider::AutomatonProvider;
use crate::core::util::automation::byte_run_automaton::ByteRunAutomaton;
use crate::core::util::automation::byte_runnable::ByteRunnable;
use crate::core::util::automation::character_run_automaton::CharacterRunAutomaton;
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::util::automaton::automaton_test_util::AutomatonTestUtil;
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
    let mut matcher = ByteRunAutomaton::new(automaton.into_owned())?;

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
      let mut cs_matcher = ByteRunAutomaton::new(cs_automaton.into_owned())?;
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
/// Simple unit tests for [`RegExp`] parsing.
///
/// For each type of node:
/// - test the `to_string()` output and parse tree,
/// - test the resulting automaton's language,
/// - and whether it is deterministic.
#[allow(dead_code)] // for quick search
struct TestRegExpParsing;
#[test]
fn test_any_char() -> Result<()> {
  let re = RegExp::from_string(".")?;

  assert_eq!(".", re.to_string());
  assert_eq!("AnyChar\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_any_char()?;
  assert_same_language(&expected, &actual)?;

  Ok(())
}
#[test]
fn test_any_string() -> Result<()> {
  let re = RegExp::parse("@", RegExp::ALL, 0)?;

  assert_eq!("@", re.to_string());
  assert_eq!("AnyString\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_any_string()?;
  assert_same_language(&expected, &actual)?;

  Ok(())
}
#[test]
fn test_char() -> Result<()> {
  let re = RegExp::from_string("c")?;

  assert_eq!("\\c", re.to_string());
  assert_eq!("Char char=c\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_char('c' as i32)?;
  assert_same_language(&expected, &actual)?;

  Ok(())
}

#[test]
fn test_case_insensitive_char() -> Result<()> {
  let re = RegExp::parse("c", RegExp::NONE, RegExp::ASCII_CASE_INSENSITIVE)?;

  assert_eq!("\\c", re.to_string());
  assert_eq!("Char char=c\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let c_lower = Automata::make_char('c' as i32)?;
  let c_upper = Automata::make_char('C' as i32)?;
  let expected = Operations::union(&c_lower, &c_upper)?;

  assert_same_language(&expected, &actual)?;
  Ok(())
}

#[test]
fn test_case_insensitive_char_upper() -> Result<()> {
  let re = RegExp::parse("C", RegExp::NONE, RegExp::ASCII_CASE_INSENSITIVE)?;

  assert_eq!("\\C", re.to_string());
  assert_eq!("Char char=C\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let c_lower = Automata::make_char('c' as i32)?;
  let c_upper = Automata::make_char('C' as i32)?;
  let expected = Operations::union(&c_lower, &c_upper)?;

  assert_same_language(&expected, &actual)?;
  Ok(())
}
#[test]
fn test_case_insensitive_char_not_sensitive() -> Result<()> {
  let re = RegExp::parse("4", RegExp::NONE, RegExp::ASCII_CASE_INSENSITIVE)?;

  assert_eq!("\\4", re.to_string());
  assert_eq!("Char char=4\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_char('4' as i32)?;
  assert_same_language(&expected, &actual)?;

  Ok(())
}

#[test]
fn test_case_insensitive_char_non_ascii() -> Result<()> {
  let re = RegExp::parse("Ж", RegExp::NONE, RegExp::ASCII_CASE_INSENSITIVE)?;

  assert_eq!("\\Ж", re.to_string());
  assert_eq!("Char char=Ж\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_char('Ж' as i32)?;
  assert_same_language(&expected, &actual)?;

  Ok(())
}

#[test]
fn test_negated_char() -> Result<()> {
  let re = RegExp::from_string("[^c]")?;

  assert_eq!("(.&~(\\c))", re.to_string());
  assert_eq!(
    "Intersection\n  AnyChar\n  Complement\n    Char char=c\n",
    re.to_string_tree()
  );

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Operations::union(
    &Automata::make_char_range(0, 'b' as i32)?,
    &Automata::make_char_range('d' as i32, i32::MAX)?,
  )?;
  assert_same_language(&expected, &actual)?;

  Ok(())
}
#[test]
fn test_char_range() -> Result<()> {
  let re = RegExp::from_string("[b-d]")?;

  assert_eq!("[\\b-\\d]", re.to_string());
  assert_eq!("CharRange from=b to=d\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_char_range('b' as i32, 'd' as i32)?;
  assert_same_language(&expected, &actual)?;

  Ok(())
}

#[test]
fn test_negated_char_range() -> Result<()> {
  let re = RegExp::from_string("[^b-d]")?;
  assert_eq!("(.&~([\\b-\\d]))", re.to_string());
  assert_eq!(
    "Intersection\n  AnyChar\n  Complement\n    CharRange from=b to=d\n",
    re.to_string_tree()
  );

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Operations::union(
    &Automata::make_char_range(0, 'a' as i32)?,
    &Automata::make_char_range('e' as i32, i32::MAX)?,
  )?;

  assert_same_language(&expected, &actual)?;

  Ok(())
}
#[test]
fn test_illegal_char_range() {
  let err = RegExp::from_string("[z-a]");
  assert!(
    matches!(err, Err(LuceneError::IllegalArgument(_))),
    "Expected IllegalArgument but got: {:?}",
    err
  );
}

#[test]
fn test_char_class_digit() -> Result<()> {
  let re = RegExp::from_string("[\\d]")?;

  assert_eq!("\\d", re.to_string());
  assert_eq!("PreClass class=\\d\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_char_range('0' as i32, '9' as i32)?;
  assert_same_language(&expected, &actual)?;

  Ok(())
}

#[test]
fn test_char_class_non_digit() -> Result<()> {
  let re = RegExp::from_string("[\\D]")?;

  assert_eq!("\\D", re.to_string());
  assert_eq!("PreClass class=\\D\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let all = Automata::make_any_char()?;
  let digits = Automata::make_char_range('0' as i32, '9' as i32)?;
  let expected = Operations::minus(&all, &digits, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert_same_language(&expected, &actual)?;

  Ok(())
}
#[test]
fn test_char_class_whitespace() -> Result<()> {
  let re = RegExp::from_string("[\\s]")?;

  assert_eq!("\\s", re.to_string());
  assert_eq!("PreClass class=\\s\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let mut expected = Automata::make_char(' ' as i32)?;
  expected = Operations::union(&expected, &Automata::make_char('\n' as i32)?)?;
  expected = Operations::union(&expected, &Automata::make_char('\r' as i32)?)?;
  expected = Operations::union(&expected, &Automata::make_char('\t' as i32)?)?;

  assert_same_language(&expected, &actual)?;
  Ok(())
}

#[test]
fn test_char_class_non_whitespace() -> Result<()> {
  let re = RegExp::from_string("[\\S]")?;

  assert_eq!("\\S", re.to_string());
  assert_eq!("PreClass class=\\S\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_any_char()?;
  let v = Automata::make_char(' ' as i32)?;
  let expected = Operations::minus(&expected, &v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  let v = Automata::make_char('\n' as i32)?;
  let expected = Operations::minus(&expected, &v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  let v = Automata::make_char('\r' as i32)?;
  let expected = Operations::minus(&expected, &v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  let v = Automata::make_char('\t' as i32)?;
  let expected = Operations::minus(&expected, &v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert_same_language(&expected, &actual)?;
  Ok(())
}
#[test]
fn test_char_class_word() -> Result<()> {
  let re = RegExp::from_string("[\\w]")?;

  assert_eq!("\\w", re.to_string());
  assert_eq!("PreClass class=\\w\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let mut expected = Automata::make_char_range('a' as i32, 'z' as i32)?;
  expected = Operations::union(
    &expected,
    &Automata::make_char_range('A' as i32, 'Z' as i32)?,
  )?;
  expected = Operations::union(
    &expected,
    &Automata::make_char_range('0' as i32, '9' as i32)?,
  )?;
  expected = Operations::union(&expected, &Automata::make_char('_' as i32)?)?;

  assert_same_language(&expected, &actual)?;
  Ok(())
}
#[test]
fn test_char_class_non_word() -> Result<()> {
  let re = RegExp::from_string("[\\W]")?;

  assert_eq!("\\W", re.to_string());
  assert_eq!("PreClass class=\\W\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_any_char()?;
  let v = Automata::make_char_range('a' as i32, 'z' as i32)?;
  let expected = Operations::minus(&expected, &v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  let v = Automata::make_char_range('A' as i32, 'Z' as i32)?;
  let expected = Operations::minus(&expected, &v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  let v = Automata::make_char_range('0' as i32, '9' as i32)?;
  let expected = Operations::minus(&expected, &v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  let v = Automata::make_char('_' as i32)?;
  let expected = Operations::minus(&expected, &v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert_same_language(&expected, &actual)?;
  Ok(())
}
#[test]
fn test_truncated_char_class() {
  let err = RegExp::from_string("[b-d");
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_bogus_char_class() {
  let err = RegExp::from_string("[\\q]");
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_escaped_not_char_class() -> Result<()> {
  let re = RegExp::from_string("[\\?]")?;

  assert_eq!("\\?", re.to_string());
  assert_eq!("Char char=?\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_char('?' as i32)?;
  assert_same_language(&expected, &actual)?;

  Ok(())
}

#[test]
fn test_escaped_slash_not_char_class() -> Result<()> {
  let re = RegExp::from_string("[\\\\]")?;

  assert_eq!("\\\\", re.to_string());
  assert_eq!("Char char=\\\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_char('\\' as i32)?;
  assert_same_language(&expected, &actual)?;

  Ok(())
}
#[test]
fn test_empty() -> Result<()> {
  let re = RegExp::parse("#", RegExp::EMPTY, 0)?;

  assert_eq!("#", re.to_string());
  assert_eq!("Empty\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_empty()?;
  assert_same_language(&expected, &actual)?;

  Ok(())
}

#[test]
fn test_interval() -> Result<()> {
  let re = RegExp::from_string("<5-40>")?;

  assert_eq!("<5-40>", re.to_string());
  assert_eq!("Interval<5-40>\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  let expected = Automata::make_decimal_interval(5, 40, 0)?;

  assert_same_language(&expected, &actual)?;
  Ok(())
}

#[test]
fn test_backwards_interval() -> Result<()> {
  let re = RegExp::from_string("<40-5>")?;

  assert_eq!("<5-40>", re.to_string());
  assert_eq!("Interval<5-40>\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  let expected = Automata::make_decimal_interval(5, 40, 0)?;

  assert_same_language(&expected, &actual)?;
  Ok(())
}
#[test]
fn test_truncated_interval() {
  let err = RegExp::from_string("<1-");
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_truncated_interval2() {
  let err = RegExp::from_string("<1");
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_empty_interval() {
  let err = RegExp::from_string("<->");
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_optional() -> Result<()> {
  let re = RegExp::from_string("a?")?;

  assert_eq!("(\\a)?", re.to_string());
  assert_eq!("Optional\n  Char char=a\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let a = Automata::make_char('a' as i32)?;
  let expected = Operations::optional(&a)?;

  assert_same_language(&expected, &actual)?;
  Ok(())
}
#[test]
fn test_repeat_0() -> Result<()> {
  let re = RegExp::from_string("a*")?;

  assert_eq!("(\\a)*", re.to_string());
  assert_eq!("Repeat\n  Char char=a\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let a = Automata::make_char('a' as i32)?;
  let expected = Operations::repeat(&a)?;

  assert_same_language(&expected, &actual)?;
  Ok(())
}

#[test]
fn test_repeat_1() -> Result<()> {
  let re = RegExp::from_string("a+")?;

  assert_eq!("(\\a){1,}", re.to_string());
  assert_eq!("RepeatMin min=1\n  Char char=a\n", re.to_string_tree());

  let a = Automata::make_char('a' as i32)?;
  let expected = Operations::repeat_count(&a, 1)?;
  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  assert_same_language(&expected, &actual)?;
  Ok(())
}

#[test]
fn test_repeat_n() -> Result<()> {
  let re = RegExp::from_string("a{5}")?;

  assert_eq!("(\\a){5,5}", re.to_string());
  assert_eq!(
    "RepeatMinMax min=5 max=5\n  Char char=a\n",
    re.to_string_tree()
  );

  let a = Automata::make_char('a' as i32)?;
  let expected = Operations::repeat_min_max(&a, 5, 5)?;
  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  assert_same_language(&expected, &actual)?;
  Ok(())
}
#[test]
fn test_repeat_n_plus() -> Result<()> {
  let re = RegExp::from_string("a{5,}")?;

  assert_eq!("(\\a){5,}", re.to_string());
  assert_eq!("RepeatMin min=5\n  Char char=a\n", re.to_string_tree());

  let a = Automata::make_char('a' as i32)?;
  let expected = Operations::repeat_count(&a, 5)?;
  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  assert_same_language(&expected, &actual)?;
  Ok(())
}

#[test]
fn test_repeat_mn() -> Result<()> {
  let re = RegExp::from_string("a{5,8}")?;

  assert_eq!("(\\a){5,8}", re.to_string());
  assert_eq!(
    "RepeatMinMax min=5 max=8\n  Char char=a\n",
    re.to_string_tree()
  );

  let a = Automata::make_char('a' as i32)?;
  let expected = Operations::repeat_min_max(&a, 5, 8)?;
  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  assert_same_language(&expected, &actual)?;
  Ok(())
}

#[test]
fn test_truncated_repeat() {
  let err = RegExp::from_string("a{5,8");
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_bogus_repeat() {
  let err = RegExp::from_string("a{Z}");
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_string() -> Result<()> {
  let re = RegExp::from_string("boo")?;

  assert_eq!("\"boo\"", re.to_string());
  assert_eq!("String string=boo\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_string("boo")?;
  assert_same_language(&expected, &actual)?;
  Ok(())
}

#[test]
fn test_case_insensitive_string() -> Result<()> {
  let re = RegExp::parse("boo", RegExp::NONE, RegExp::ASCII_CASE_INSENSITIVE)?;

  assert_eq!("\"boo\"", re.to_string());
  assert_eq!("String string=boo\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let b = Operations::union(
    &Automata::make_char('b' as i32)?,
    &Automata::make_char('B' as i32)?,
  )?;
  let o = Operations::union(
    &Automata::make_char('o' as i32)?,
    &Automata::make_char('O' as i32)?,
  )?;

  let expected = Operations::concatenate(&b, &o)?;
  let expected = Operations::concatenate(&expected, &o)?;

  assert_same_language(&expected, &actual)?;
  Ok(())
}
#[test]
fn test_explicit_string() -> Result<()> {
  let re = RegExp::from_string("\"boo\"")?;

  assert_eq!("\"boo\"", re.to_string());
  assert_eq!("String string=boo\n", re.to_string_tree());

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_string("boo")?;
  assert_same_language(&expected, &actual)?;
  Ok(())
}

#[test]
fn test_not_terminated_string() {
  let err = RegExp::from_string("\"boo");
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_concatenation() -> Result<()> {
  let re = RegExp::from_string("[b-c][e-f]")?;

  assert_eq!("[\\b-\\c][\\e-\\f]", re.to_string());
  assert_eq!(
    "Concatenation\n  CharRange from=b to=c\n  CharRange from=e to=f\n",
    re.to_string_tree()
  );

  let r1 = Automata::make_char_range('b' as i32, 'c' as i32)?;
  let r2 = Automata::make_char_range('e' as i32, 'f' as i32)?;
  let expected = Operations::concatenate(&r1, &r2)?;

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  assert_same_language(&expected, &actual)?;
  Ok(())
}
#[test]
fn test_intersection() -> Result<()> {
  let re = RegExp::from_string("[b-f]&[e-f]")?;

  assert_eq!("([\\b-\\f]&[\\e-\\f])", re.to_string());
  assert_eq!(
    "Intersection\n  CharRange from=b to=f\n  CharRange from=e to=f\n",
    re.to_string_tree()
  );

  let r1 = Automata::make_char_range('b' as i32, 'f' as i32)?;
  let r2 = Automata::make_char_range('e' as i32, 'f' as i32)?;
  let expected = Operations::intersection(&r1, &r2)?;

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  assert_same_language(&expected, &actual)?;
  Ok(())
}

#[test]
fn test_truncated_intersection() {
  let err = RegExp::from_string("a&");
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_truncated_intersection_parens() {
  let err = RegExp::from_string("(a)&(");
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_union() -> Result<()> {
  let re = RegExp::from_string("[b-c]|[e-f]")?;

  assert_eq!("([\\b-\\c]|[\\e-\\f])", re.to_string());
  assert_eq!(
    "Union\n  CharRange from=b to=c\n  CharRange from=e to=f\n",
    re.to_string_tree()
  );

  let r1 = Automata::make_char_range('b' as i32, 'c' as i32)?;
  let r2 = Automata::make_char_range('e' as i32, 'f' as i32)?;
  let expected = Operations::union(&r1, &r2)?;

  let actual = re.to_automaton()?;
  assert!(actual.is_deterministic());

  assert_same_language(&expected, &actual)?;
  Ok(())
}

#[test]
fn test_truncated_union() {
  let err = RegExp::from_string("a|");
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_truncated_union_parens() {
  let err = RegExp::from_string("(a)|(");
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_automaton() -> Result<()> {
  struct MyProvider;
  impl AutomatonProvider for MyProvider {
    fn get_automaton(&self, name: &str) -> Result<Option<Automaton>> {
      assert_eq!(name, "myletter");
      Ok(Some(Automata::make_char('z' as i32)?))
    }
  }

  let re = RegExp::parse("<myletter>", RegExp::ALL, 0)?;
  assert_eq!("<myletter>", re.to_string());
  assert_eq!("Automaton\n", re.to_string_tree());
  assert_eq!(
    re.get_identifiers_set(),
    HashSet::from(["myletter".to_string()])
  );

  let actual = re.to_automaton_from_provider(&MyProvider)?;
  assert!(actual.is_deterministic());

  let expected = Automata::make_char('z' as i32)?;
  assert_same_language(&expected, &actual)?;
  Ok(())
}
#[test]
fn test_automaton_map() -> Result<()> {
  let re = RegExp::parse("<myletter>", RegExp::ALL, 0)?;
  assert_eq!("<myletter>", re.to_string());
  assert_eq!("Automaton\n", re.to_string_tree());
  assert_eq!(
    re.get_identifiers_set(),
    HashSet::from(["myletter".to_string()])
  );

  let actual = re.to_automaton_from_map(&HashMap::from([(
    "myletter".to_string(),
    Automata::make_char('z' as i32)?,
  )]))?;

  assert!(actual.is_deterministic());

  let expected = Automata::make_char('z' as i32)?;
  assert_same_language(&expected, &actual)?;
  Ok(())
}

#[test]
fn test_automaton_io_exception() {
  struct MyProvider;
  impl AutomatonProvider for MyProvider {
    fn get_automaton(&self, _name: &str) -> Result<Option<Automaton>> {
      Err(LuceneError::illegal_argument("fake error"))
    }
  }

  let re = RegExp::parse("<myletter>", RegExp::ALL, 0).unwrap();
  assert_eq!("<myletter>", re.to_string());
  assert_eq!("Automaton\n", re.to_string_tree());
  assert_eq!(
    re.get_identifiers_set(),
    HashSet::from(["myletter".to_string()])
  );

  let err = re.to_automaton_from_provider(&MyProvider);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_automaton_not_found() {
  let re = RegExp::parse("<bogus>", RegExp::ALL, 0).unwrap();
  assert_eq!("<bogus>", re.to_string());
  assert_eq!("Automaton\n", re.to_string_tree());

  let err = re.to_automaton_from_map(&HashMap::from([(
    "myletter".to_string(),
    Automata::make_char('z' as i32).unwrap(),
  )]));
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_illegal_syntax_flags() {
  let err = RegExp::parse("bogus", i32::MAX, 0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_illegal_match_flags() {
  let err = RegExp::parse("bogus", RegExp::ALL, 1);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

fn assert_same_language(expected: &Automaton, actual: &Automaton) -> Result<()> {
  let expected = Operations::determinize(expected, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  let actual = Operations::determinize(actual, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  let result = AutomatonTestUtil::same_language(&expected, &actual)?;
  if !result {
    // println!("{}", expected.to_dot()?);
    // println!("{}", actual.to_dot()?);
  }
  assert!(result);
  Ok(())
}
