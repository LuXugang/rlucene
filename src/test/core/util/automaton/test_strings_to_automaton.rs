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
use std::borrow::Cow;
use std::collections::HashSet;

use rand::Rng;
use rand::RngExt;

use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::byte_runnable::ByteRunnable;
use crate::core::util::automation::compiled_automaton::CompiledAutomaton;
use crate::core::util::automation::finite_strings_iterator::{
  FiniteStringsIterator, FiniteStringsIteratorBase,
};
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::strings_to_automaton::StringsToAutomaton;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fst_impl::util::Util;
use crate::test::core::util::automaton::automaton_test_util::AutomatonTestUtil;
use crate::test::core::util::automaton::minimization_operation::MinimizationOperations;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  is_night_mode, new_bytes_ref_from_bytes_ref, new_bytes_ref_from_string, random,
};
use crate::test::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestStringsToAutomaton;
#[test]
fn test_basic() -> Result<()> {
  let mut random = random();
  let mut terms = basic_terms(&mut random)?;
  terms.sort();

  let a = build(&mut random, terms.clone(), false)?;
  check_automaton(&terms, a.clone(), false)?;
  check_minimized(&a)?;

  Ok(())
}
#[test]
fn test_basic_binary() -> Result<()> {
  let mut random = random();
  let mut terms = basic_terms(&mut random)?;
  terms.sort();

  let a = build(&mut random, terms.clone(), true)?;
  check_automaton(&terms, a.clone(), true)?;
  check_minimized(&a)?;

  Ok(())
}

#[test]
fn test_random_minimized() -> Result<()> {
  let mut random = random();
  let iters = if is_night_mode() { 20 } else { 5 };

  for _ in 0..iters {
    let build_binary = false;
    let size = 2;

    let mut terms = Vec::new();
    let mut automaton_list = vec![];

    for _ in 0..size {
      if build_binary {
        let t = TestUtil::random_binary_term_with_len(&mut random, 8);
        automaton_list.push(Automata::make_binary(&t)?);
        terms.push(t);
      } else {
        let s = TestUtil::random_realistic_unicode_string_with_len(&mut random, 8);
        let t = new_bytes_ref_from_string(&mut random, &s)?;
        automaton_list.push(Automata::make_string(&s)?);
        terms.push(t);
      }
    }

    let a = Operations::union_list(&automaton_list.iter().collect::<Vec<_>>())?;
    let expected =
      MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

    terms.sort_unstable();
    let actual = build(&mut random, terms, build_binary)?;

    assert_same_automaton(&expected, &actual)?;
  }

  Ok(())
}
#[test]
fn test_random_unicode_only() -> Result<()> {
  let mut random = random();
  test_random(&mut random, false)
}

#[test]
fn test_random_binary() -> Result<()> {
  let mut random = random();
  test_random(&mut random, true)
}
#[test]
fn test_large_terms() -> Result<()> {
  let mut random = random();
  let b10k = vec![b'a'; 10_000];

  let result = build(&mut random, vec![BytesRef::from_bytes(b10k.clone())], false);
  assert!(
    matches!(result, Err(LuceneError::IllegalArgument(msg)) if msg.message.starts_with(
        &format!(
            "This builder doesn't allow terms that are larger than {} UTF-8 bytes",
            Automata::MAX_STRING_UNION_TERM_LENGTH
        )
    ))
  );

  let b1k = ArrayUtil::copy_of_sub_array(&b10k, 0, 1000);
  build(&mut random, vec![BytesRef::from_bytes(b1k)], false)?; // should not panic

  Ok(())
}

fn test_random<R>(random: &mut R, allow_binary: bool) -> Result<()>
where
  R: Rng + ?Sized,
{
  let iters = if is_night_mode() { 50 } else { 10 };

  for _ in 0..iters {
    let size = random.random_range(500..2000);
    let mut terms = HashSet::with_capacity(size);

    let mut j = 0;
    while j < size {
      if allow_binary && random.random_range(0..10) < 2 {
        // Sometimes random bytes term that isn't necessarily valid unicode
        let v = TestUtil::random_binary_term(random);
        terms.insert(new_bytes_ref_from_bytes_ref(random, &v)?);
      } else {
        let s = TestUtil::random_realistic_unicode_string(random);
        terms.insert(new_bytes_ref_from_string(random, &s)?);
      }
      j += 1;
    }

    let mut sorted: Vec<_> = terms.into_iter().collect();
    sorted.sort_unstable();

    let a = build(random, sorted.clone(), allow_binary)?;
    check_automaton(&sorted, a, allow_binary)?;
  }

  Ok(())
}

fn check_automaton(expected: &[BytesRef<Vec<u8>>], a: Automaton, is_binary: bool) -> Result<()> {
  let mut c = CompiledAutomaton::with_binary(a, true, false, is_binary)?;
  let run_automaton = c.run_automaton.as_mut().unwrap();

  // Make sure every expected term is accepted
  for t in expected {
    let readable = if is_binary {
      format!("{:?}", t.bytes)
    } else {
      t.utf8_to_string()?
    };

    assert!(
      run_automaton.run(&t.bytes, t.offset, t.length)?,
      "{} should be found but wasn't",
      readable
    );
  }

  // Make sure every term produced by the automaton is expected
  let mut scratch = BytesRefBuilder::new();
  let mut it = FiniteStringsIterator::new(&c.run_automaton.as_ref().unwrap().base.automaton);
  while let Some(r) = it.next()? {
    let t = Util::to_bytes_ref(&r, &mut scratch)?;
    assert!(
      expected.iter().any(|x| x == &t),
      "Unexpected term found: {:?}",
      t.utf8_to_string()?
    );
  }

  Ok(())
}

fn check_minimized(a: &Automaton) -> Result<()> {
  let minimized = MinimizationOperations::minimize(a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  assert_same_automaton(&minimized, a)?;
  Ok(())
}
fn assert_same_automaton(a: &Automaton, b: &Automaton) -> Result<()> {
  assert_eq!(a.get_num_states(), b.get_num_states());
  assert_eq!(a.get_num_transitions(), b.get_num_transitions());
  assert!(AutomatonTestUtil::same_language(a, b)?);
  Ok(())
}

fn basic_terms<R>(random: &mut R) -> Result<Vec<BytesRef<Vec<u8>>>>
where
  R: Rng + ?Sized,
{
  Ok(vec![
    new_bytes_ref_from_string(random, "dog")?,
    new_bytes_ref_from_string(random, "day")?,
    new_bytes_ref_from_string(random, "dad")?,
    new_bytes_ref_from_string(random, "cats")?,
    new_bytes_ref_from_string(random, "cat")?,
  ])
}

fn build<R>(random: &mut R, terms: Vec<BytesRef<Vec<u8>>>, as_binary: bool) -> Result<Automaton>
where
  R: Rng + ?Sized,
{
  if random.random_bool(0.5) {
    StringsToAutomaton::build(terms.as_slice(), as_binary)
  } else {
    StringsToAutomaton::build_from_iterator(
      &mut TermIterator {
        it: terms.into_iter(),
      },
      as_binary,
    )
  }
}

struct TermIterator {
  it: std::vec::IntoIter<BytesRef<Vec<u8>>>,
}
impl BytesRefIterator for TermIterator {
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    match self.it.next() {
      Some(b) => Ok(Some(Cow::Owned(b))),
      None => Ok(None),
    }
  }
}
