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

use rand::RngExt;

use crate::core::index::BytesRef;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::finite_strings_iterator::{
  FiniteStringsIterator, FiniteStringsIteratorBase,
};
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fst_impl::util::Util;
use crate::core::util::ints_ref::IntsRef;
use crate::core::util::ints_ref_builder::IntsRefBuilder;
use crate::test::core::util::automaton::automaton_test_util::AutomatonTestUtil;
use crate::test::core::util::automaton::minimization_operation::MinimizationOperations;
use crate::test::core::util::automaton::test_operations::TestOperations;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use crate::test::core::util::test_util::TestUtil;
/// Test for FiniteStringsIterator.
#[allow(dead_code)] // for quick search
struct TestFiniteStringsIterator;
#[test]
fn test_random_finite_strings1() -> Result<()> {
  let mut random = random();
  // let num_strings = at_least(&mut random, 100);
  let num_strings = 1;
  if cfg!(feature = "test_log_verbose") {
    println!("TEST: num_strings={}", num_strings);
  }

  let mut strings = HashSet::new();
  let mut string_list = Vec::new();
  let mut scratch = IntsRefBuilder::new();

  for _ in 0..num_strings {
    let s = TestUtil::random_simple_string_range(&mut random, 1, 200);
    Util::get_utf32_with_slice(&s, 0, s.len(), &mut scratch);
    if strings.insert(scratch.to_ints_ref()) {
      string_list.push(Automata::make_string(&s)?);
      if cfg!(feature = "test_log_verbose") {
        println!("  add string={}", s);
      }
    }
  }
  let refs: Vec<&Automaton> = string_list.iter().collect();
  let a = Operations::union_list(&refs)?;

  let a = if random.random_bool(0.5) {
    let v = MinimizationOperations::minimize(&a, 1_000_000)?;
    if cfg!(feature = "test_log_verbose") {
      println!("TEST: a.minimize numStates={}", a.get_num_states());
    }
    v
  } else if random.random_bool(0.5) {
    if cfg!(feature = "test_log_verbose") {
      println!("TEST: a.determinize");
    }
    Operations::determinize(&a, 1_000_000)?
  } else if random.random_bool(0.5) {
    if cfg!(feature = "test_log_verbose") {
      println!("TEST: a.removeDeadStates");
    }
    Operations::remove_dead_states(&a)?
  } else {
    Cow::Owned(a)
  };

  let mut iterator = FiniteStringsIterator::new(&a);
  let actual = get_finite_strings(&mut iterator)?;
  assert_finite_strings_recursive(&a, actual.clone());

  let actual_set: HashSet<_> = actual.into_iter().collect();

  if strings != actual_set {
    if cfg!(feature = "test_log_verbose") {
      println!(
        "strings.size()={} actual.size={}",
        strings.len(),
        actual_set.len()
      );
    }

    let mut x: Vec<_> = strings.into_iter().collect();
    let mut y: Vec<_> = actual_set.into_iter().collect();
    x.sort();
    y.sort();

    let end = x.len().min(y.len());
    for i in 0..end {
      if cfg!(feature = "test_log_verbose") {
        println!(
          "  i={} string={} actual={}",
          i,
          to_ascii_string(&x[i]),
          to_ascii_string(&y[i])
        );
      }
    }
    unreachable!("wrong strings found");
  }

  Ok(())
}

/// Basic test for getFiniteStrings
#[test]
fn test_finite_strings_basic() -> Result<()> {
  let a = Operations::union(
    &Automata::make_string("dog")?,
    &Automata::make_string("duck")?,
  )?;
  let a = MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  let mut iterator = FiniteStringsIterator::new(&a);
  let actual = get_finite_strings(&mut iterator)?;

  assert_finite_strings_recursive(&a, actual.clone());
  assert_eq!(actual.len(), 2);

  let mut dog = IntsRefBuilder::new();
  Util::get_ints_ref(&BytesRef::<Vec<u8>>::from_string("dog"), &mut dog);
  assert!(actual.contains(dog.get()));

  let mut duck = IntsRefBuilder::new();
  Util::get_ints_ref(&BytesRef::<Vec<u8>>::from_string("duck"), &mut duck);
  assert!(actual.contains(duck.get()));

  Ok(())
}

#[test]
fn test_finite_strings_eats_stack() -> Result<()> {
  let mut chars = vec![0u16; 50000];
  let mut random = random();
  let chars_len = chars.len();
  TestUtil::random_fixed_length_unicode_string(&mut random, &mut chars, 0, chars_len);
  let big_string1 = String::from_utf16(&chars).unwrap();
  TestUtil::random_fixed_length_unicode_string(&mut random, &mut chars, 0, chars_len);
  let big_string2 = String::from_utf16(&chars).unwrap();

  let a = Operations::union(
    &Automata::make_string(&big_string1)?,
    &Automata::make_string(&big_string2)?,
  )?;

  let mut iterator = FiniteStringsIterator::new(&a);
  let actual = get_finite_strings(&mut iterator)?;
  assert_eq!(actual.len(), 2);

  let mut scratch = IntsRefBuilder::new();
  Util::get_utf32_with_slice(&big_string1, 0, big_string1.len(), &mut scratch);
  assert!(actual.contains(scratch.get()));

  Util::get_utf32_with_slice(&big_string2, 0, big_string2.len(), &mut scratch);
  assert!(actual.contains(scratch.get()));

  Ok(())
}

#[test]
fn test_with_cycle() {
  let result = (|| {
    let a = RegExp::from_str_with_flags("abc.*", RegExp::NONE)?.to_automaton()?;
    let mut iterator = FiniteStringsIterator::new(&a);
    get_finite_strings(&mut iterator)?;
    Ok::<(), LuceneError>(())
  })();
  assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_singleton_no_limit() -> Result<()> {
  let a = Automata::make_string("foobar")?;
  let mut iterator = FiniteStringsIterator::new(&a);
  let actual = get_finite_strings(&mut iterator)?;
  assert_eq!(actual.len(), 1);

  let mut scratch = IntsRefBuilder::new();
  Util::get_utf32_with_slice("foobar", 0, 6, &mut scratch);
  assert!(actual.contains(scratch.get()));

  Ok(())
}

#[test]
fn test_short_accept() -> Result<()> {
  let a = Operations::union(&Automata::make_string("x")?, &Automata::make_string("xy")?)?;
  let a = MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  let mut iterator = FiniteStringsIterator::new(&a);
  let actual = get_finite_strings(&mut iterator)?;
  assert_eq!(actual.len(), 2);

  let mut x = IntsRefBuilder::new();
  Util::get_ints_ref(&BytesRef::<Vec<u8>>::from_string("x"), &mut x);
  assert!(actual.contains(x.get()));

  let mut xy = IntsRefBuilder::new();
  Util::get_ints_ref(&BytesRef::<Vec<u8>>::from_string("xy"), &mut xy);
  assert!(actual.contains(xy.get()));
  Ok(())
}

#[test]
fn test_single_string() -> Result<()> {
  let mut a = Automaton::new();
  let start = a.create_state();
  let end = a.create_state();
  a.set_accept(end, true);
  a.add_transition(start, end, 'a' as i32, 'a' as i32)?;
  a.finish_state()?;

  let accepted = TestOperations::get_finite_strings(&a)?;

  assert_eq!(accepted.len(), 1);

  let mut ints_ref = IntsRefBuilder::new();
  ints_ref.append('a' as i32);

  assert!(accepted.contains(&ints_ref.to_ints_ref()));
  Ok(())
}

/// All strings generated by the iterator.
pub(crate) fn get_finite_strings(
  iterator: &mut impl FiniteStringsIteratorBase,
) -> Result<Vec<IntsRef<Vec<i32>>>> {
  let mut result = Vec::new();
  while let Some(finite_string) = iterator.next()? {
    result.push(IntsRef::deep_copy_of(&finite_string));
  }
  Ok(result)
}

/// Checks that the strings returned by the automaton are as expected.
///
/// Parameters:
/// - `automaton`: The automaton
/// - `actual`: Strings generated by the automaton
fn assert_finite_strings_recursive(automaton: &Automaton, actual: Vec<IntsRef<Vec<i32>>>) {
  let expected = AutomatonTestUtil::get_finite_strings_recursive(automaton, -1);

  // Check that no string is emitted twice
  assert_eq!(
    expected.len(),
    actual.len(),
    "Expected and actual lengths differ"
  );

  let actual_set: HashSet<_> = actual.into_iter().collect();
  assert_eq!(expected, actual_set, "Expected and actual sets differ");
}

/// Only handles ASCII (for this test helper).
fn to_ascii_string(ints: &IntsRef<Vec<i32>>) -> String {
  let mut bytes = Vec::with_capacity(ints.length);
  for i in 0..ints.length {
    bytes.push(ints.ints[ints.offset + i] as u8);
  }
  String::from_utf8(bytes).expect("Only ASCII supported in intsref_to_ascii_string")
}
