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
use std::ptr;

use rand::Rng;
use rand::RngExt;

use crate::core::index::BytesRef;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::{Automaton, Builder};
use crate::core::util::automation::finite_strings_iterator::{
  FiniteStringsIterator, FiniteStringsIteratorBase,
};
use crate::core::util::automation::limited_finite_strings_iterator::LimitedFiniteStringsIterator;
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::automation::transition::Transition;
use crate::core::util::automation::transition_accessor::TransitionAccessor;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::ints_ref::IntsRef;
use crate::core::util::unicode_util::UnicodeUtil;
use crate::test::core::util::automaton::automaton_test_util::{
  AutomatonTestUtil, RandomAcceptedStrings,
};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{at_least, random};
use crate::test::core::util::test_util::TestUtil;

pub(crate) struct TestOperations;

impl TestOperations {
  /// Returns the set of all accepted strings.
  ///
  /// This method exists primarily to ease testing.
  /// For production code, directly use [`FiniteStringsIterator`] instead.
  ///
  /// See also:
  /// - [`FiniteStringsIterator`]
  pub fn get_finite_strings(a: &Automaton) -> Result<HashSet<IntsRef<Vec<i32>>>> {
    let iter = FiniteStringsIterator::new(a);
    Self::get_finite_strings_impl(iter)
  }
  /// Returns the set of accepted strings, up to at most `limit` strings.
  ///
  /// This method exists primarily to ease testing.
  /// For production code, directly use [`LimitedFiniteStringsIterator`]
  /// instead.
  ///
  /// See also:
  /// - [`LimitedFiniteStringsIterator`]
  pub fn get_finite_strings_with_limit(
    a: &Automaton,
    limit: i32,
  ) -> Result<HashSet<IntsRef<Vec<i32>>>> {
    let iter = LimitedFiniteStringsIterator::new(a, limit)?;
    Self::get_finite_strings_impl(iter)
  }

  /// Get all finite strings of an iterator.
  pub fn get_finite_strings_impl(
    mut iterator: impl FiniteStringsIteratorBase,
  ) -> Result<HashSet<IntsRef<Vec<i32>>>> {
    let mut result = HashSet::new();
    while let Some(finite_string) = iterator.next()? {
      result.insert(IntsRef::deep_copy_of(&finite_string));
    }
    Ok(result)
  }
}
#[test]
fn test_string_union() -> Result<()> {
  let mut random = random();
  let count = random.random_range(1..1000);
  // let count = 21;
  let mut strings = Vec::with_capacity(count);
  for _ in 0..count {
    let s = TestUtil::random_unicode_string(&mut random);
    strings.push(BytesRef::from_string(&s));
  }
  strings.sort();

  let union = Automata::make_string_union(&strings)?;
  assert!(union.is_deterministic());
  assert!(!Operations::has_dead_states_from_initial(&union)?);

  let naive_union = naive_union(strings.as_slice())?;
  assert!(naive_union.is_deterministic());
  assert!(!Operations::has_dead_states_from_initial(&naive_union)?);

  assert!(AutomatonTestUtil::same_language(&union, &naive_union)?);

  Ok(())
}
fn naive_union(strings: &[BytesRef<Vec<u8>>]) -> Result<Automaton> {
  let mut string_list = vec![];

  for bref in strings {
    let s = bref.utf8_to_string()?;
    string_list.push(Automata::make_string(&s)?);
  }
  let automata: Vec<&Automaton> = string_list.iter().collect();
  let union = Operations::union_list(&automata)?;
  let det = Operations::determinize(&union, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  Ok(det.into_owned())
}
///  Test concatenation with empty language returns empty
#[test]
fn test_empty_language_concatenate() -> Result<()> {
  let a = Automata::make_string("a")?;
  let empty = Automata::make_empty()?;
  let concat = Operations::concatenate(&a, &empty)?;
  assert!(Operations::is_empty(&concat));
  Ok(())
}
/// Test case for the topoSortStates method when the input Automaton
/// contains a cycle. This test case constructs an Automaton with two
/// disjoint sets of states—one without a cycle and one with
/// a cycle. The topoSortStates method should detect the presence of a cycle
/// and throw an IllegalArgumentException.
#[test]
fn test_cycled_automaton() -> Result<()> {
  let mut random = random();
  let a = generate_random_automaton(true, &mut random)?;
  let result = Operations::topo_sort_states(&a);
  assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("input automaton has a cycle")
  );
  Ok(())
}
#[test]
fn test_topo_sort_states() -> Result<()> {
  let mut random = random();
  let a = generate_random_automaton(false, &mut random)?;

  let sorted = Operations::topo_sort_states(&a)?;
  let mut state_map = vec![-1; a.get_num_states() as usize];

  for (order, &state) in sorted.iter().enumerate() {
    assert_eq!(state_map[state as usize], -1);
    state_map[state as usize] = order as i32;
  }

  let mut transition = Transition::default();

  for &state in &sorted {
    let count = a.init_transition(state, &mut transition);
    for _ in 0..count {
      a.get_next_transition(&mut transition);
      assert!(state_map[transition.dest as usize] > state_map[state as usize]);
    }
  }

  Ok(())
}
///  Test optimization to concatenate() with empty String to an NFA
#[test]
fn test_empty_singleton_nfa_concatenate() -> Result<()> {
  let singleton = Automata::make_string("")?;
  let expanded_singleton = singleton.clone();

  // An NFA (two transitions for 't' from initial state)
  let nfa = Operations::union(
    &Automata::make_string("this")?,
    &Automata::make_string("three")?,
  )?;

  let concat1 = Operations::concatenate(&expanded_singleton, &nfa)?;
  let concat2 = Operations::concatenate(&singleton, &nfa)?;

  assert!(!concat2.is_deterministic());

  let det1 = Operations::determinize(&concat1, 100)?;
  let det2 = Operations::determinize(&concat2, 100)?;
  let det_nfa = Operations::determinize(&nfa, 100)?;

  assert!(AutomatonTestUtil::same_language(&det1, &det2)?);
  assert!(AutomatonTestUtil::same_language(&det_nfa, &det1)?);
  assert!(AutomatonTestUtil::same_language(&det_nfa, &det2)?);

  Ok(())
}
#[test]
fn test_get_random_accepted_string() -> Result<()> {
  let mut random = random();

  for _ in 0..at_least(&mut random, 100) {
    let pattern = AutomatonTestUtil::random_regexp(&mut random)?;
    let re = RegExp::from_str_with_flags(&pattern, RegExp::NONE)?;
    let v = re.to_automaton()?;
    let a = Operations::determinize(&v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
    assert!(!Operations::is_empty(&a));

    let rx = RandomAcceptedStrings::new(&a)?;
    for _ in 0..at_least(&mut random, 100) {
      let acc = rx.get_random_accepted_string(&mut random)?;
      let s = UnicodeUtil::new_string(acc.as_ref(), 0, acc.len())?;
      assert!(
        Operations::run_str(&a, &s),
        "Automaton failed to accept string generated from: {pattern}"
      );
    }
  }

  Ok(())
}
#[test]
fn test_is_finite_eats_stack() -> Result<()> {
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

  let result = AutomatonTestUtil::is_finite(&a);
  assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("input automaton is too large")
  );

  Ok(())
}

#[test]
fn test_is_total() -> Result<()> {
  // minimal
  assert!(!Operations::is_total(&Automata::make_empty()?)?);
  assert!(!Operations::is_total(&Automata::make_empty_string()?)?);
  assert!(Operations::is_total(&Automata::make_any_string()?)?);
  assert!(Operations::is_total_with_range(
    &Automata::make_any_binary()?,
    0,
    255
  )?);
  assert!(!Operations::is_total_with_range(
    &Automata::make_non_empty_binary()?,
    0,
    255
  )?);

  // deterministic, but not minimal
  let v = Automata::make_any_char()?;
  let v1 = Operations::repeat(&v)?;
  assert!(Operations::is_total(&v1)?);

  let v = Operations::union(
    &Automata::make_char_range(char::MIN as i32, 100)?,
    &Automata::make_char_range(101, char::MAX as i32)?,
  )?;
  let tricky = Operations::repeat(&v)?;
  assert!(Operations::is_total(&tricky)?);

  // not total, but close
  let v = Operations::union(
    &Automata::make_char_range((char::MIN as i32) + 1, 100)?,
    &Automata::make_char_range(101, char::MAX as i32)?,
  )?;
  let tricky2 = Operations::repeat(&v)?;
  assert!(!Operations::is_total(&tricky2)?);

  let v = Operations::union(
    &Automata::make_char_range(char::MIN as i32, 99)?,
    &Automata::make_char_range(101, char::MAX as i32)?,
  )?;
  let tricky3 = Operations::repeat(&v)?;
  assert!(!Operations::is_total(&tricky3)?);

  let v = Operations::union(
    &Automata::make_char_range(char::MIN as i32, 100)?,
    &Automata::make_char_range(101, (char::MAX as i32) - 1)?,
  )?;
  let tricky4 = Operations::repeat(&v)?;
  assert!(!Operations::is_total(&tricky4)?);

  Ok(())
}

/// This method creates a random [`Automaton`] by generating states at
/// multiple levels. At each level, a random number of states are
/// created, and transitions are added between the states of the current
/// and the previous level randomly. If the `has_cycle` parameter is
/// `true`, a transition is added from the first state of the last level
/// back to the initial state to create a cycle in the automaton.
///
/// Parameters:
/// - `has_cycle`: If `true`, the generated automaton will contain a cycle;
///   if `false`, it won't.
///
/// Returns:
/// - A randomly generated [`Automaton`] instance.
pub(crate) fn generate_random_automaton<R>(has_cycle: bool, random: &mut R) -> Result<Automaton>
where
  R: Rng + ?Sized,
{
  let mut a = Automaton::new();
  let mut last_level_states = vec![];
  let initial_state = a.create_state();
  let max_level = random.random_range(4..10);
  last_level_states.push(initial_state);

  for _level in 1..max_level {
    let num_states = random.random_range(3..10);
    let mut next_level_states = vec![];

    for _ in 0..num_states {
      let next_state = a.create_state();
      next_level_states.push(next_state);
    }

    for last_state in last_level_states {
      for &next_state in &next_level_states {
        // if hasCycle is enabled, we will always add a transition, so we could make
        // sure the generated Automaton has a cycle.
        if has_cycle || random.random_range(0..7) >= 1 {
          a.add_transition_label(last_state, next_state, random.random_range(0..10))?;
        }
      }
    }

    last_level_states = next_level_states;
  }

  if has_cycle {
    let last_state = last_level_states[0];
    a.add_transition_label(last_state, initial_state, random.random_range(0..10))?;
  }

  a.finish_state()?;
  Ok(a)
}
fn assert_same<'a>(cow: Cow<'a, Automaton>, expected: &'a Automaton) {
  match cow {
    Cow::Borrowed(b) => assert!(ptr::eq(b, expected)),
    Cow::Owned(_) => unreachable!(),
  }
}
#[test]
fn test_repeat() -> Result<()> {
  let empty_language = Automata::make_empty()?;
  let r = Operations::repeat(&empty_language)?;
  assert_same(r, &empty_language);

  let empty_string = Automata::make_empty_string()?;
  let r = Operations::repeat(&empty_string)?;
  assert_same(r, &empty_string);

  let a = Automata::make_char('a' as i32)?;
  let mut as_ = Automaton::new();
  as_.create_state();
  as_.set_accept(0, true);
  as_.add_transition_label(0, 0, 'a' as i32)?;
  as_.finish_state()?;
  let r = Operations::repeat(&a)?;
  assert!(AutomatonTestUtil::same_language(&as_, &r)?);
  let r = Operations::repeat(&as_)?;
  assert_same(r, &as_);

  let mut a_or_empty = Automaton::new();
  a_or_empty.create_state();
  a_or_empty.set_accept(0, true);
  a_or_empty.create_state();
  a_or_empty.set_accept(1, true);
  a_or_empty.add_transition_label(0, 1, 'a' as i32)?;
  let r = Operations::repeat(&a_or_empty)?;
  assert!(AutomatonTestUtil::same_language(&as_, &r)?);

  let ab = Automata::make_string("ab")?;
  let mut abs = Automaton::new();
  abs.create_state();
  abs.create_state();
  abs.set_accept(0, true);
  abs.add_transition_label(0, 1, 'a' as i32)?;
  abs.finish_state()?;
  abs.add_transition_label(1, 0, 'b' as i32)?;
  abs.finish_state()?;
  let r = Operations::repeat(&ab)?;
  assert!(AutomatonTestUtil::same_language(&abs, &r)?);
  let r = Operations::repeat(&abs)?;
  assert_same(r, &abs);

  let abs_then_c = Operations::concatenate(&abs, &Automata::make_char('c' as i32)?)?;
  let mut abs_then_cs = Automaton::new();
  abs_then_cs.create_state();
  abs_then_cs.create_state();
  abs_then_cs.create_state();
  abs_then_cs.set_accept(0, true);
  abs_then_cs.add_transition_label(0, 1, 'a' as i32)?;
  abs_then_cs.add_transition_label(0, 0, 'c' as i32)?;
  abs_then_cs.finish_state()?;
  abs_then_cs.add_transition_label(1, 2, 'b' as i32)?;
  abs_then_cs.finish_state()?;
  abs_then_cs.add_transition_label(2, 1, 'a' as i32)?;
  abs_then_cs.add_transition_label(2, 0, 'c' as i32)?;
  abs_then_cs.finish_state()?;
  let r = Operations::repeat(&abs_then_c)?;
  assert!(AutomatonTestUtil::same_language(&abs_then_cs, &r)?);
  let r = Operations::repeat(&abs_then_cs)?;
  assert_same(r, &abs_then_cs);

  let mut a_or_ab = Automaton::new();
  a_or_ab.create_state();
  a_or_ab.create_state();
  a_or_ab.create_state();
  a_or_ab.set_accept(1, true);
  a_or_ab.set_accept(2, true);
  a_or_ab.add_transition_label(0, 1, 'a' as i32)?;
  a_or_ab.finish_state()?;
  a_or_ab.add_transition_label(1, 2, 'b' as i32)?;
  a_or_ab.finish_state()?;

  let mut a_or_abs = Automaton::new();
  a_or_abs.create_state();
  a_or_abs.create_state();
  a_or_abs.set_accept(0, true);
  a_or_abs.add_transition_label(0, 0, 'a' as i32)?;
  a_or_abs.add_transition_label(0, 1, 'a' as i32)?;
  a_or_abs.finish_state()?;
  a_or_abs.add_transition_label(1, 0, 'b' as i32)?;
  a_or_abs.finish_state()?;

  let expected = Operations::determinize(&a_or_abs, i32::MAX as usize)?;
  let v = Operations::repeat(&a_or_ab)?;
  let actual = Operations::determinize(&v, i32::MAX as usize)?;
  assert!(AutomatonTestUtil::same_language(&expected, &actual)?);

  Ok(())
}
#[test]
fn test_duel_repeat() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 1000);

  for _ in 0..iters {
    let a = AutomatonTestUtil::random_automaton(&mut random)?;
    let v = Operations::repeat(&a)?;
    let repeat1 = Operations::determinize(&v, i32::MAX as usize)?;
    let v = naive_repeat(&a)?;
    let repeat2 = Operations::determinize(&v, i32::MAX as usize)?;
    assert!(AutomatonTestUtil::same_language(&repeat1, &repeat2)?);
  }

  Ok(())
}

fn naive_repeat(a: &Automaton) -> Result<Cow<'_, Automaton>> {
  if a.get_num_states() == 0 {
    return Ok(Cow::Borrowed(a));
  }

  let mut builder = Builder::default();
  // Create the initial state, which is accepted
  builder.create_state();
  builder.set_accept(0, true);
  builder.copy(a);

  let mut t = Transition::default();
  let count = a.init_transition(0, &mut t);
  for _ in 0..count {
    a.get_next_transition(&mut t);
    builder.add_transition(0, t.dest + 1, t.min, t.max);
  }

  let num_states = a.get_num_states();
  for s in 0..num_states {
    if a.is_accept(s) {
      let count = a.init_transition(0, &mut t);
      for _ in 0..count {
        a.get_next_transition(&mut t);
        builder.add_transition(s + 1, t.dest + 1, t.min, t.max);
      }
    }
  }

  Ok(Cow::Owned(builder.finish()?))
}
#[test]
fn test_optional() -> Result<()> {
  let a = Automata::make_char('a' as i32)?;
  let mut optional_a = Automaton::new();
  optional_a.create_state();
  optional_a.set_accept(0, true);
  optional_a.finish_state()?;
  optional_a.create_state();
  optional_a.set_accept(1, true);
  optional_a.add_transition_label(0, 1, 'a' as i32)?;
  optional_a.finish_state()?;

  let r = Operations::optional(&a)?;
  assert!(AutomatonTestUtil::same_language(&r, &optional_a)?);

  let r = Operations::optional(&optional_a)?;
  assert_same(r, &optional_a);

  // Now test an automaton that has a transition to state 0. a(ba)*
  let mut a = Automaton::new();
  a.create_state();
  a.create_state();
  a.set_accept(1, true);
  a.add_transition_label(0, 1, 'a' as i32)?;
  a.finish_state()?;
  a.add_transition_label(1, 0, 'b' as i32)?;
  a.finish_state()?;

  let mut optional_a = Automaton::new();
  optional_a.create_state();
  optional_a.set_accept(0, true);
  optional_a.create_state();
  optional_a.create_state();
  optional_a.set_accept(2, true);
  optional_a.add_transition_label(0, 2, 'a' as i32)?;
  optional_a.finish_state()?;
  optional_a.add_transition_label(1, 2, 'a' as i32)?;
  optional_a.finish_state()?;
  optional_a.add_transition_label(2, 1, 'b' as i32)?;
  optional_a.finish_state()?;

  let r = Operations::optional(&a)?;
  assert!(AutomatonTestUtil::same_language(&r, &optional_a)?);

  let r = Operations::optional(&optional_a)?;
  assert_same(r, &optional_a);

  Ok(())
}
#[test]
fn test_duel_optional() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 1000);

  for _ in 0..iters {
    let a = AutomatonTestUtil::random_automaton(&mut random)?;
    let r1 = Operations::optional(&a)?;
    let opt1 = Operations::determinize(&r1, i32::MAX as usize)?;
    let r2 = naive_optional(&a)?;
    let opt2 = Operations::determinize(&r2, i32::MAX as usize)?;
    assert!(AutomatonTestUtil::same_language(&opt1, &opt2)?);
  }

  Ok(())
}
// This is the original implementation of Operations#optional, before we
// improved it to generate simpler automata in some common cases.
fn naive_optional(a: &Automaton) -> Result<Automaton> {
  let mut result = Automaton::new();
  result.create_state();
  result.set_accept(0, true);
  if a.get_num_states() > 0 {
    result.copy(a);
    result.add_epsilon(0, 1)?;
  }
  result.finish_state()?;
  Ok(result)
}
