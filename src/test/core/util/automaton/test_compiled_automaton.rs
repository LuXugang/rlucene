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
use crate::test::core::util::lucene_test_case::{at_least, random, random_multiplier};
use std::collections::HashSet;

use rand::Rng;
use rand::RngExt;

use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::compiled_automaton::{AutomatonType, CompiledAutomaton};
use crate::core::util::automation::operations::Operations;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::test_util::TestUtil;
#[allow(dead_code)] // for quick search
struct TestCompiledAutomaton;
fn build(_determinize_work_limit: i32, strings: &[&str]) -> Result<CompiledAutomaton> {
  let mut terms: Vec<BytesRef<Vec<u8>>> =
    strings.iter().map(|s| BytesRef::from_string(s)).collect();

  terms.sort();
  let a = Automata::make_string_union(&terms)?;
  CompiledAutomaton::with_binary(a, true, false, false)
}
fn test_floor(c: &mut CompiledAutomaton, input: &str, expected: Option<&str>) -> Result<()> {
  let b = BytesRef::from_string(input);
  let mut builder = BytesRefBuilder::default();

  let result = c.floor(&b, &mut builder)?;

  match expected {
    None => {
      assert!(result.is_none(), "Expected None, got {:?}", result);
    },
    Some(expected_str) => {
      let result = result.expect("Expected Some(BytesRef), got None");
      let expected_bytes = BytesRef::from_string(expected_str);
      assert_eq!(
        result, expected_bytes,
        "actual={:?} vs expected={} (input={})",
        result, expected_str, input
      );
    },
  }

  Ok(())
}
fn test_terms<R>(random: &mut R, determinize_work_limit: i32, terms: &[&str]) -> Result<()>
where
  R: Rng + ?Sized,
{
  let mut compiled = build(determinize_work_limit, terms)?;
  let mut term_bytes: Vec<BytesRef<Vec<u8>>> =
    terms.iter().map(|s| BytesRef::from_string(s)).collect();
  term_bytes.sort();

  if cfg!(feature = "test_log_verbose") {
    println!("\nTEST: terms in unicode order");
    for t in &term_bytes {
      println!("  {}", t.utf8_to_string()?);
    }
  }

  for _ in 0..(100 * random_multiplier() as usize) {
    let s = if random.random_range(0..10) == 1 {
      terms[random.random_range(0..terms.len())].to_string()
    } else {
      random_string(random)
    };

    if cfg!(feature = "test_log_verbose") {
      println!("\nTEST: floor({s})");
    }

    let key = BytesRef::from_string(&s);
    let mut expected: Option<String> = None;

    match term_bytes.binary_search(&key) {
      Ok(_) => {
        expected = Some(s.clone());
      },
      Err(insert_pos) => {
        if insert_pos > 0 {
          expected = Some(term_bytes[insert_pos - 1].utf8_to_string()?);
        }
      },
    }

    if cfg!(feature = "test_log_verbose") {
      println!("  expected={:?}", expected);
    }

    test_floor(&mut compiled, &s, expected.as_deref())?;
  }

  Ok(())
}
#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let num_terms = at_least(&mut random, 400);
  let mut terms = HashSet::new();

  while terms.len() < num_terms as usize {
    terms.insert(random_string(&mut random));
  }
  let term_vec: Vec<&str> = terms.iter().map(|s| s.as_str()).collect();

  test_terms(&mut random, num_terms * 100, &term_vec)?;

  Ok(())
}

fn random_string<R>(random: &mut R) -> String
where
  R: Rng + ?Sized,
{
  TestUtil::random_realistic_unicode_string(random)
}
#[test]
fn test_basic() -> Result<()> {
  let mut compiled = build(
    Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
    &["fob", "foo", "goo"],
  )?;

  test_floor(&mut compiled, "goo", Some("goo"))?;
  test_floor(&mut compiled, "ga", Some("foo"))?;
  test_floor(&mut compiled, "g", Some("foo"))?;
  test_floor(&mut compiled, "foc", Some("fob"))?;
  test_floor(&mut compiled, "foz", Some("foo"))?;
  test_floor(&mut compiled, "f", None)?;
  test_floor(&mut compiled, "", None)?;
  test_floor(&mut compiled, "aa", None)?;
  test_floor(&mut compiled, "zzz", Some("goo"))?;

  Ok(())
}
// LUCENE-6367
#[test]
fn test_binary_all() -> Result<()> {
  let mut a = Automaton::new();
  let state = a.create_state();
  a.set_accept(state, true);
  a.add_transition(state, state, 0, 0xff)?;
  a.finish_state()?;

  let ca = CompiledAutomaton::with_binary(a, false, true, true)?;

  assert_eq!(ca.type_, AutomatonType::All);
  Ok(())
}
// LUCENE-6367
#[test]
fn test_unicode_all() -> Result<()> {
  let mut a = Automaton::new();
  let state = a.create_state();
  a.set_accept(state, true);
  a.add_transition(state, state, 0, char::MAX as i32)?;
  a.finish_state()?;

  let ca = CompiledAutomaton::with_binary(a, false, true, false)?;
  assert_eq!(ca.type_, AutomatonType::All);

  Ok(())
}
// LUCENE-6367
#[test]
fn test_binary_singleton() -> Result<()> {
  let a = Automata::make_string("foobar")?;
  let ca = CompiledAutomaton::with_binary(a, true, true, true)?;
  assert_eq!(ca.type_, AutomatonType::Single);
  Ok(())
}
// LUCENE-6367
#[test]
fn test_unicode_singleton() -> Result<()> {
  let mut random = random();
  let s = TestUtil::random_realistic_unicode_string(&mut random);
  let a = Automata::make_string(&s)?;
  let ca = CompiledAutomaton::with_binary(a, true, true, false)?;
  assert_eq!(ca.type_, AutomatonType::Single);
  Ok(())
}
