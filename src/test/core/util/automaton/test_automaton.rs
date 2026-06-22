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
use crate::test::core::util::lucene_test_case::{
  at_least, new_bytes_ref, new_bytes_ref_empty, new_bytes_ref_from_string, random, random_from_seed,
};
use std::borrow::Cow;
use std::collections::{BTreeSet, HashSet};

use rand::Rng;
use rand::RngExt;
use rand::prelude::SliceRandom;

use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::ToInt;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::{Automaton, Builder};
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::automation::transition::Transition;
use crate::core::util::automation::transition_accessor::TransitionAccessor;
use crate::core::util::automation::utf32_to_utf8::UTF32ToUTF8;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fst_impl::util::Util;
use crate::core::util::ints_ref::IntsRef;
use crate::core::util::ints_ref_builder::IntsRefBuilder;
use crate::core::util::unicode_util::UnicodeUtil;
use crate::test::core::util::automaton::automaton_test_util::{
  AutomatonTestUtil, RandomAcceptedStrings,
};
use crate::test::core::util::automaton::minimization_operation::MinimizationOperations;
use crate::test::core::util::automaton::test_operations::TestOperations;
use crate::test::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestAutomaton;

#[test]
fn test_basic() -> Result<()> {
  let mut a = Automaton::new();
  let start = a.create_state();
  let x = a.create_state();
  let y = a.create_state();
  let end = a.create_state();
  a.set_accept(end, true);

  a.add_transition(start, x, 'a' as i32, 'a' as i32)?;
  a.add_transition(start, end, 'd' as i32, 'd' as i32)?;
  a.add_transition(x, y, 'b' as i32, 'b' as i32)?;
  a.add_transition(y, end, 'c' as i32, 'c' as i32)?;

  a.finish_state()?;
  Ok(())
}
#[test]
fn test_reduce_basic() -> Result<()> {
  let mut a = Automaton::new();
  let start = a.create_state();
  let end = a.create_state();
  a.set_accept(end, true);

  // Should collapse to a-b:
  a.add_transition(start, end, 'a' as i32, 'a' as i32)?;
  a.add_transition(start, end, 'b' as i32, 'b' as i32)?;
  // Should collapse to m-m:
  a.add_transition(start, end, 'm' as i32, 'm' as i32)?;
  // Should collapse to x-y:
  a.add_transition(start, end, 'x' as i32, 'x' as i32)?;
  a.add_transition(start, end, 'y' as i32, 'y' as i32)?;

  a.finish_state()?;

  assert_eq!(3, a.get_num_transitions_with_state(start));

  let mut scratch = Transition::default();
  a.init_transition(start, &mut scratch);
  a.get_next_transition(&mut scratch);
  assert_eq!('a' as i32, scratch.min);
  assert_eq!('b' as i32, scratch.max);

  a.get_next_transition(&mut scratch);
  assert_eq!('m' as i32, scratch.min);
  assert_eq!('m' as i32, scratch.max);

  a.get_next_transition(&mut scratch);
  assert_eq!('x' as i32, scratch.min);
  assert_eq!('y' as i32, scratch.max);

  Ok(())
}
#[test]
fn test_same_language() -> Result<()> {
  let a1 = Automata::make_string("foobar")?;
  let v = Operations::concatenate(
    &Automata::make_string("foo")?,
    &Automata::make_string("bar")?,
  )?;
  let a2 = Operations::remove_dead_states(&v)?;
  assert!(AutomatonTestUtil::same_language(&a1, &a2)?);
  Ok(())
}
#[test]
fn test_common_prefix_string() -> Result<()> {
  let a = Operations::concatenate(
    &Automata::make_string("foobar")?,
    &Automata::make_any_string()?,
  )?;

  let prefix = Operations::get_common_prefix(&a)?;
  assert_eq!(prefix, "foobar");

  Ok(())
}
#[test]
fn test_common_prefix_empty() -> Result<()> {
  let a = Automata::make_empty()?;
  let prefix = Operations::get_common_prefix(&a)?;
  assert_eq!(prefix, "");
  Ok(())
}

#[test]
fn test_common_prefix_empty_string() -> Result<()> {
  let a = Automata::make_empty_string()?;
  let prefix = Operations::get_common_prefix(&a)?;
  assert_eq!(prefix, "");
  Ok(())
}

#[test]
fn test_common_prefix_any() -> Result<()> {
  let a = Automata::make_any_string()?;
  let prefix = Operations::get_common_prefix(&a)?;
  assert_eq!(prefix, "");
  Ok(())
}

#[test]
fn test_common_prefix_range() -> Result<()> {
  let a = Automata::make_char_range('a' as i32, 'b' as i32)?;
  let prefix = Operations::get_common_prefix(&a)?;
  assert_eq!(prefix, "");
  Ok(())
}
#[test]
fn test_alternatives() -> Result<()> {
  let a = Automata::make_char('a' as i32)?;
  let c = Automata::make_char('c' as i32)?;
  let union = Operations::union(&a, &c)?;
  let prefix = Operations::get_common_prefix(&union)?;
  assert_eq!(prefix, "");
  Ok(())
}

#[test]
fn test_common_prefix_leading_wildcard() -> Result<()> {
  let a = Operations::concatenate(&Automata::make_any_char()?, &Automata::make_string("boo")?)?;
  let prefix = Operations::get_common_prefix(&a)?;
  assert_eq!(prefix, "");
  Ok(())
}

#[test]
fn test_common_prefix_trailing_wildcard() -> Result<()> {
  let a = Operations::concatenate(&Automata::make_string("boo")?, &Automata::make_any_char()?)?;
  let prefix = Operations::get_common_prefix(&a)?;
  assert_eq!(prefix, "boo");
  Ok(())
}

#[test]
fn test_common_prefix_leading_kleen_star() -> Result<()> {
  let a = Operations::concatenate(
    &Automata::make_any_string()?,
    &Automata::make_string("boo")?,
  )?;
  let prefix = Operations::get_common_prefix(&a)?;
  assert_eq!(prefix, "");
  Ok(())
}

#[test]
fn test_common_prefix_trailing_kleen_star() -> Result<()> {
  let a = Operations::concatenate(
    &Automata::make_string("boo")?,
    &Automata::make_any_string()?,
  )?;
  let prefix = Operations::get_common_prefix(&a)?;
  assert_eq!(prefix, "boo");
  Ok(())
}
#[test]
fn test_common_prefix_dead_states() -> Result<()> {
  let a = Operations::concatenate(
    &Automata::make_any_string()?,
    &Automata::make_string("boo")?,
  )?;

  // reverse twice to create dead states
  let with_dead_states = Operations::reverse(&Operations::reverse(&a)?)?;

  let result = Operations::get_common_prefix(&with_dead_states);
  assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
  assert!(
    result
      .unwrap_err()
      .to_string()
      .eq("input automaton has dead states")
  );

  Ok(())
}
#[test]
fn test_common_prefix_remove_dead_states() -> Result<()> {
  let a = Operations::concatenate(
    &Automata::make_any_string()?,
    &Automata::make_string("boo")?,
  )?;

  // reverse twice to create dead states
  let with_dead_states = Operations::reverse(&Operations::reverse(&a)?)?;

  // now remove the dead states
  let without_dead_states = Operations::remove_dead_states(&with_dead_states)?;

  let prefix = Operations::get_common_prefix(&without_dead_states)?;
  assert_eq!(prefix, "");

  Ok(())
}
#[test]
fn test_common_prefix_optional() -> Result<()> {
  let mut a = Automaton::new();
  let init = a.create_state();
  let fini = a.create_state();
  a.set_accept(init, true);
  a.set_accept(fini, true);
  a.add_transition(init, fini, 'm' as i32, 'm' as i32)?;
  a.add_transition(fini, fini, 'm' as i32, 'm' as i32)?;
  a.finish_state()?;

  let prefix = Operations::get_common_prefix(&a)?;
  assert_eq!(prefix, "");

  Ok(())
}

#[test]
fn test_common_prefix_nfa() -> Result<()> {
  let mut a = Automaton::new();
  let init = a.create_state();
  let medial = a.create_state();
  let fini = a.create_state();
  a.set_accept(fini, true);
  a.add_transition(init, medial, 'm' as i32, 'm' as i32)?;
  a.add_transition(init, fini, 'm' as i32, 'm' as i32)?;
  a.add_transition(medial, fini, 'o' as i32, 'o' as i32)?;
  a.finish_state()?;

  let prefix = Operations::get_common_prefix(&a)?;
  assert_eq!(prefix, "m");

  Ok(())
}

#[test]
fn test_common_prefix_nfa_infinite() -> Result<()> {
  let mut a = Automaton::new();
  let init = a.create_state();
  let medial = a.create_state();
  let fini = a.create_state();
  a.set_accept(fini, true);
  a.add_transition(init, medial, 'm' as i32, 'm' as i32)?;
  a.add_transition(init, fini, 'm' as i32, 'm' as i32)?;
  a.add_transition(medial, fini, 'm' as i32, 'm' as i32)?;
  a.add_transition(fini, fini, 'm' as i32, 'm' as i32)?;
  a.finish_state()?;

  let prefix = Operations::get_common_prefix(&a)?;
  assert_eq!(prefix, "m");

  Ok(())
}
#[test]
fn test_common_prefix_unicode() -> Result<()> {
  let a = Operations::concatenate(
    &Automata::make_string("boo😂😂😂")?,
    &Automata::make_any_char()?,
  )?;
  let prefix = Operations::get_common_prefix(&a)?;
  assert_eq!(prefix, "boo😂😂😂");
  Ok(())
}

#[test]
fn test_concatenate1() -> Result<()> {
  let a = Operations::concatenate(&Automata::make_string("m")?, &Automata::make_any_string()?)?;
  assert!(Operations::run_str(&a, "m"));
  assert!(Operations::run_str(&a, "me"));
  assert!(Operations::run_str(&a, "me too"));
  Ok(())
}

#[test]
fn test_concatenate2() -> Result<()> {
  let a = Operations::concatenate_with_list(&[
    &Automata::make_string("m")?,
    &Automata::make_any_string()?,
    &Automata::make_string("n")?,
    &Automata::make_any_string()?,
  ])?;
  let a = Operations::determinize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(Operations::run_str(&a, "mn"));
  assert!(Operations::run_str(&a, "mone"));
  assert!(!Operations::run_str(&a, "m"));
  assert!(!AutomatonTestUtil::is_finite(&a)?);

  Ok(())
}
#[test]
fn test_union1() -> Result<()> {
  let a1 = Automata::make_string("foobar")?;
  let a2 = Automata::make_string("barbaz")?;

  let union = Operations::union_list(&[&a1, &a2])?;
  let det = Operations::determinize(&union, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(Operations::run_str(&det, "foobar"));
  assert!(Operations::run_str(&det, "barbaz"));

  assert_matches(&det, &["foobar", "barbaz"])?;
  Ok(())
}
#[test]
fn test_union2() -> Result<()> {
  let a1 = Automata::make_string("foobar")?;
  let a2 = Automata::make_string("")?;
  let a3 = Automata::make_string("barbaz")?;

  let union = Operations::union_list(&[&a1, &a2, &a3])?;
  let det = Operations::determinize(&union, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(Operations::run_str(&det, "foobar"));
  assert!(Operations::run_str(&det, "barbaz"));
  assert!(Operations::run_str(&det, ""));

  assert_matches(&det, &["", "foobar", "barbaz"])?;
  Ok(())
}
#[test]
fn test_minimize_simple() -> Result<()> {
  let a = Automata::make_string("foobar")?;
  let a_min = MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(AutomatonTestUtil::same_language(&a, &a_min)?);
  Ok(())
}
#[test]
fn test_minimize2() -> Result<()> {
  let a1 = Automata::make_string("foobar")?;
  let a2 = Automata::make_string("boobar")?;

  let union = Operations::union_list(&[&a1, &a2])?;
  let a_min = MinimizationOperations::minimize(&union, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  let removed = Operations::remove_dead_states(&union)?;
  let det = Operations::determinize(&removed, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(AutomatonTestUtil::same_language(&det, &a_min)?);
  Ok(())
}

#[test]
fn test_reverse() -> Result<()> {
  let a = Automata::make_string("foobar")?;
  let ra = Operations::reverse(&a)?;
  let ra_rev = Operations::reverse(&ra)?;
  let a2 = Operations::determinize(&ra_rev, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(AutomatonTestUtil::same_language(&a, &a2)?);
  Ok(())
}

#[test]
fn test_optional() -> Result<()> {
  let a = Automata::make_string("foobar")?;
  let a2 = Operations::optional(&a)?;
  let a2 = Operations::determinize(&a2, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(Operations::run_str(&a, "foobar"));
  assert!(!Operations::run_str(&a, ""));
  assert!(Operations::run_str(&a2, "foobar"));
  assert!(Operations::run_str(&a2, ""));
  Ok(())
}

#[test]
fn test_repeat_any() -> Result<()> {
  let a = Automata::make_string("zee")?;
  let repeated = Operations::repeat(&a)?;
  let a2 = Operations::determinize(&repeated, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(Operations::run_str(&a2, ""));
  assert!(Operations::run_str(&a2, "zee"));
  assert!(Operations::run_str(&a2, "zeezee"));
  assert!(Operations::run_str(&a2, "zeezeezee"));
  Ok(())
}
#[test]
fn test_repeat_min() -> Result<()> {
  let a = Automata::make_string("zee")?;
  let repeated = Operations::repeat_count(&a, 2)?;
  let a2 = Operations::determinize(&repeated, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(!Operations::run_str(&a2, ""));
  assert!(!Operations::run_str(&a2, "zee"));
  assert!(Operations::run_str(&a2, "zeezee"));
  assert!(Operations::run_str(&a2, "zeezeezee"));
  Ok(())
}

#[test]
fn test_repeat_min_max1() -> Result<()> {
  let a = Automata::make_string("zee")?;
  let repeated = Operations::repeat_min_max(&a, 0, 2)?;
  let a2 = Operations::determinize(&repeated, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(Operations::run_str(&a2, ""));
  assert!(Operations::run_str(&a2, "zee"));
  assert!(Operations::run_str(&a2, "zeezee"));
  assert!(!Operations::run_str(&a2, "zeezeezee"));
  Ok(())
}

#[test]
fn test_repeat_min_max2() -> Result<()> {
  let a = Automata::make_string("zee")?;
  let repeated = Operations::repeat_min_max(&a, 2, 4)?;
  let a2 = Operations::determinize(&repeated, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(!Operations::run_str(&a2, ""));
  assert!(!Operations::run_str(&a2, "zee"));
  assert!(Operations::run_str(&a2, "zeezee"));
  assert!(Operations::run_str(&a2, "zeezeezee"));
  assert!(Operations::run_str(&a2, "zeezeezeezee"));
  assert!(!Operations::run_str(&a2, "zeezeezeezeezee"));
  Ok(())
}
#[test]
fn test_complement() -> Result<()> {
  let a = Automata::make_string("zee")?;
  let comp = Operations::complement(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  let a2 = Operations::determinize(&comp, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(Operations::run_str(&a2, ""));
  assert!(!Operations::run_str(&a2, "zee"));
  assert!(Operations::run_str(&a2, "zeezee"));
  assert!(Operations::run_str(&a2, "zeezeezee"));
  Ok(())
}

#[test]
fn test_interval() -> Result<()> {
  let interval = Automata::make_decimal_interval(17, 100, 3)?;
  let a = Operations::determinize(&interval, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(!Operations::run_str(&a, ""));
  assert!(Operations::run_str(&a, "017"));
  assert!(Operations::run_str(&a, "100"));
  assert!(Operations::run_str(&a, "073"));
  Ok(())
}

#[test]
fn test_common_suffix() -> Result<()> {
  let mut a = Automaton::new();
  let init = a.create_state();
  let fini = a.create_state();
  a.set_accept(init, true);
  a.set_accept(fini, true);
  a.add_transition_label(init, fini, 'm' as i32)?;
  a.add_transition_label(fini, fini, 'm' as i32)?;
  a.finish_state()?;

  let suffix = Operations::get_common_suffix_bytes_ref(&a)?;
  assert_eq!(suffix.length, 0);
  Ok(())
}
#[test]
fn test_common_suffix_empty() -> Result<()> {
  let a = Automata::make_empty()?;
  let suffix = Operations::get_common_suffix_bytes_ref(&a)?;
  assert_eq!(suffix, BytesRef::new());
  Ok(())
}

#[test]
fn test_common_suffix_empty_string() -> Result<()> {
  let a = Automata::make_empty_string()?;
  let suffix = Operations::get_common_suffix_bytes_ref(&a)?;
  assert_eq!(suffix, BytesRef::new());
  Ok(())
}

#[test]
fn test_common_suffix_trailing_wildcard() -> Result<()> {
  let a = Operations::concatenate(&Automata::make_string("boo")?, &Automata::make_any_char()?)?;
  let suffix = Operations::get_common_suffix_bytes_ref(&a)?;
  assert_eq!(suffix, BytesRef::new());
  Ok(())
}

#[test]
fn test_common_suffix_leading_kleen_star() -> Result<()> {
  let mut random = random();
  let a = Operations::concatenate(
    &Automata::make_any_string()?,
    &Automata::make_string("boo")?,
  )?;
  let suffix = Operations::get_common_suffix_bytes_ref(&a)?;
  assert_eq!(suffix, new_bytes_ref_from_string(&mut random, "boo")?);
  Ok(())
}

#[test]
fn test_common_suffix_trailing_kleen_star() -> Result<()> {
  let a = Operations::concatenate(
    &Automata::make_string("boo")?,
    &Automata::make_any_string()?,
  )?;
  let suffix = Operations::get_common_suffix_bytes_ref(&a)?;
  assert_eq!(suffix, BytesRef::new());
  Ok(())
}

#[test]
fn test_common_suffix_unicode() -> Result<()> {
  let mut random = random();
  let a = Operations::concatenate(
    &Automata::make_any_string()?,
    &Automata::make_string("boo😂😂😂")?,
  )?;

  let binary = UTF32ToUTF8::default().convert(&a)?;
  let suffix = Operations::get_common_suffix_bytes_ref(&binary)?;

  assert_eq!(new_bytes_ref_from_string(&mut random, "boo😂😂😂")?, suffix);
  Ok(())
}
#[test]
fn test_reverse_random1() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 100);

  for _ in 0..iters {
    let a = AutomatonTestUtil::random_automaton(&mut random)?;
    let ra = Operations::reverse(&a)?;
    let rra = Operations::reverse(&ra)?;

    let v = Operations::remove_dead_states(&a)?;
    let orig = Operations::determinize(&v, i32::MAX as usize)?;
    let v = Operations::remove_dead_states(&rra)?;
    let reversed = Operations::determinize(&v, i32::MAX as usize)?;

    assert!(AutomatonTestUtil::same_language(&orig, &reversed)?);
  }

  Ok(())
}
#[test]
fn test_reverse_random2() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 100);

  for _ in 0..iters {
    let bool = random.random_bool(0.5);
    let seed: u64 = random.random();
    let mut a = AutomatonTestUtil::random_automaton(&mut random)?;
    if bool && let Cow::Owned(o) = Operations::remove_dead_states(&a)? {
      a = Cow::Owned(o)
    }

    let ra = Operations::reverse(&a)?;
    let rda = Operations::determinize(&ra, i32::MAX as usize)?;

    if Operations::is_empty(&a) {
      assert!(Operations::is_empty(&rda));
      continue;
    }

    let ras = RandomAcceptedStrings::new(&a)?;
    for _ in 0..20 {
      let mut random1 = random_from_seed(seed);
      // Find string accepted by original automaton
      let s = ras.get_random_accepted_string(&mut random1)?;
      let reversed: Vec<i32> = s.iter().copied().rev().collect();
      let len = reversed.len();
      let ints_ref = IntsRef::from_slice(reversed, 0, len);
      assert!(Operations::run_ints_ref(&rda, &ints_ref));
    }
  }

  Ok(())
}
#[test]
fn test_any_string_empty_string() -> Result<()> {
  let any = Automata::make_any_string()?;
  let a = Operations::determinize(&any, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  assert!(Operations::run_str(&a, ""));
  Ok(())
}

#[test]
fn test_basic_is_empty() -> Result<()> {
  let mut a = Automaton::new();
  a.create_state();
  assert!(Operations::is_empty(&a));
  Ok(())
}

#[test]
fn test_remove_dead_transitions_empty() -> Result<()> {
  let a = Automata::make_empty()?;
  let a2 = Operations::remove_dead_states(&a)?;
  assert!(Operations::is_empty(&a2));
  Ok(())
}
#[test]
#[should_panic(expected = "from state")]
fn test_invalid_add_transition() {
  let mut a = Automaton::new();
  let s1 = a.create_state();
  let s2 = a.create_state();
  a.add_transition(s1, s2, 'a' as i32, 'a' as i32).unwrap();
  a.add_transition(s2, s2, 'a' as i32, 'a' as i32).unwrap();
  // This should panic because transitions on s1 were already added
  a.add_transition(s1, s2, 'b' as i32, 'b' as i32).unwrap();
}
#[test]
fn test_builder_random() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 100);

  for _ in 0..iters {
    let seed: u64 = random.random();
    let mut random1 = random_from_seed(seed);
    let a = AutomatonTestUtil::random_automaton(&mut random)?;

    let mut all_trans = vec![];
    let num_states = a.get_num_states();
    for s in 0..num_states {
      let count = a.get_num_transitions_with_state(s);
      for i in 0..count {
        let mut t = Transition::default();
        a.get_transition(s, i, &mut t);
        all_trans.push(t);
      }
    }

    let mut builder = Builder::new();
    for i in 0..num_states {
      let s = builder.create_state();
      builder.set_accept(s, a.is_accept(i));
    }

    all_trans.shuffle(&mut random1);
    for t in all_trans {
      builder.add_transition(t.source, t.dest, t.min, t.max);
    }

    let v1 = Operations::remove_dead_states(&a)?;
    let a1 = Operations::determinize(&v1, i32::MAX as usize)?;
    let b = builder.finish()?;
    let v2 = Operations::remove_dead_states(&b)?;
    let a2 = Operations::determinize(&v2, i32::MAX as usize)?;
    assert!(AutomatonTestUtil::same_language(&a1, &a2)?);
  }

  Ok(())
}
#[test]
fn test_is_total() -> Result<()> {
  assert!(!Operations::is_total(&Automaton::new())?);

  let mut a = Automaton::new();
  let init = a.create_state();
  let fini = a.create_state();
  a.set_accept(fini, true);
  a.add_transition(init, fini, char::MIN as i32, char::MAX as i32)?;
  a.finish_state()?;

  assert!(!Operations::is_total(&a)?);

  a.add_transition(fini, fini, char::MIN as i32, char::MAX as i32)?;
  a.finish_state()?;

  assert!(!Operations::is_total(&a)?);

  a.set_accept(init, true);
  let minimized = MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  assert!(Operations::is_total(&minimized)?);

  Ok(())
}

#[test]
fn test_minimize_empty() -> Result<()> {
  let mut a = Automaton::new();
  let init = a.create_state();
  let fini = a.create_state();
  a.add_transition_label(init, fini, 'a' as i32)?;
  a.finish_state()?;

  let a = MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  assert_eq!(a.get_num_states(), 0);
  Ok(())
}
#[test]
fn test_minus() -> Result<()> {
  let mut random = random();
  let a1 = Automata::make_string("foobar")?;
  let a2 = Automata::make_string("boobar")?;
  let a3 = Automata::make_string("beebar")?;

  let a = Operations::union_list(&[&a1, &a2, &a3])?;

  let a = if random.random_bool(0.5) {
    Operations::determinize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
  } else if random.random_bool(0.5) {
    MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
  } else {
    Cow::Owned(a)
  };

  assert_matches(&a, &["foobar", "beebar", "boobar"])?;

  let a2 = Automata::make_string("boobar")?;
  let a4 = Operations::minus(&a, &a2, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  let a4 = Operations::determinize(&a4, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(Operations::run_str(&a4, "foobar"));
  assert!(!Operations::run_str(&a4, "boobar"));
  assert!(Operations::run_str(&a4, "beebar"));
  assert_matches(&a4, &["foobar", "beebar"])?;
  let a1 = Automata::make_string("foobar")?;
  let a4 = Operations::minus(&a4, &a1, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  let a4 = Operations::determinize(&a4, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(!Operations::run_str(&a4, "foobar"));
  assert!(!Operations::run_str(&a4, "boobar"));
  assert!(Operations::run_str(&a4, "beebar"));
  assert_matches(&a4, &["beebar"])?;
  let a3 = Automata::make_string("beebar")?;
  let a4 = Operations::minus(&a4, &a3, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  let a4 = Operations::determinize(&a4, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(!Operations::run_str(&a4, "foobar"));
  assert!(!Operations::run_str(&a4, "boobar"));
  assert!(!Operations::run_str(&a4, "beebar"));
  assert_matches(&a4, &[])?;

  Ok(())
}
#[test]
fn test_one_interval() -> Result<()> {
  let a = Automata::make_decimal_interval(999, 1032, 0)?;
  let a = Operations::determinize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(Operations::run_str(&a, "0999"));
  assert!(Operations::run_str(&a, "00999"));
  assert!(Operations::run_str(&a, "000999"));
  Ok(())
}

#[test]
fn test_another_interval() -> Result<()> {
  let a = Automata::make_decimal_interval(1, 2, 0)?;
  let a = Operations::determinize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  assert!(Operations::run_str(&a, "01"));
  Ok(())
}
#[test]
fn test_interval_random() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 100);

  for _ in 0..iters {
    let min = TestUtil::next_int(&mut random, 0, 100_000);
    let max = TestUtil::next_int(&mut random, min, min + 100_000);

    let digits = if random.random_bool(0.5) {
      0
    } else {
      let max_str = max.to_string();
      TestUtil::next_int(
        &mut random,
        max_str.len() as i32,
        (2 * max_str.len()) as i32,
      )
    };

    let prefix = "0".repeat(digits as usize);

    let a = Automata::make_decimal_interval(min, max, digits)?;
    let a = Operations::determinize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
    let a = if random.random_bool(0.5) {
      MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
    } else {
      a
    };

    let mut mins = min.to_string();
    let mut maxs = max.to_string();
    if digits > 0 {
      mins = format!("{}{}", &prefix[mins.len()..], mins);
      maxs = format!("{}{}", &prefix[maxs.len()..], maxs);
    }

    assert!(Operations::run_str(&a, &mins));
    assert!(Operations::run_str(&a, &maxs));

    for _ in 0..100 {
      let x = random.random_range(0..2 * max);
      let expected = x >= min && x <= max;
      let mut sx = x.to_string();

      if sx.len() < digits as usize {
        sx = format!("{}{}", &prefix[sx.len()..], sx);
      } else if digits == 0 {
        let num_zeros = random.random_range(0..10);
        sx = format!("{}{}", "0".repeat(num_zeros), sx);
      }

      assert_eq!(Operations::run_str(&a, &sx), expected);
    }
  }

  Ok(())
}

fn assert_matches(a: &Automaton, strings: &[&str]) -> Result<()> {
  let mut expected = HashSet::new();
  let mut scratch = IntsRefBuilder::new();

  for s in strings {
    Util::to_utf32(s, &mut scratch);
    let v = scratch.get_owner();
    expected.insert(v);
  }
  let det = Operations::determinize(a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  let actual = TestOperations::get_finite_strings(&det).expect("Failed to get finite strings");

  assert_eq!(expected, actual);
  Ok(())
}
#[test]
fn test_concatenate_preserves_det() -> Result<()> {
  let a1 = Automata::make_string("foobar")?;
  assert!(a1.is_deterministic());

  let a2 = Automata::make_string("baz")?;
  assert!(a2.is_deterministic());

  let concat = Operations::concatenate_with_list(&[&a1, &a2])?;
  assert!(concat.is_deterministic());

  Ok(())
}
#[test]
fn test_remove_dead_states() -> Result<()> {
  let a1 = Automata::make_string("x")?;
  let a2 = Automata::make_string("y")?;

  let a = Operations::concatenate_with_list(&[&a1, &a2])?;
  assert_eq!(a.get_num_states(), 4);

  let a = Operations::remove_dead_states(&a)?;
  assert_eq!(a.get_num_states(), 3);

  Ok(())
}

#[test]
fn test_remove_dead_states_empty1() -> Result<()> {
  let mut a = Automaton::new();
  a.finish_state()?;
  assert!(Operations::is_empty(&a));

  let a2 = Operations::remove_dead_states(&a)?;
  assert!(Operations::is_empty(&a2));

  Ok(())
}

#[test]
fn test_remove_dead_states_empty2() -> Result<()> {
  let mut a = Automaton::new();
  a.finish_state()?;
  assert!(Operations::is_empty(&a));

  let a2 = Operations::remove_dead_states(&a)?;
  assert!(Operations::is_empty(&a2));

  Ok(())
}

#[test]
fn test_remove_dead_states_empty3() -> Result<()> {
  let mut a = Automaton::new();
  let init = a.create_state();
  let fini = a.create_state();
  a.add_transition_label(init, fini, 'a' as i32)?;

  let a2 = Operations::remove_dead_states(&a)?;
  assert_eq!(a2.get_num_states(), 0);

  Ok(())
}
#[test]
fn test_concat_empty() -> Result<()> {
  let a = Operations::concatenate(&Automata::make_empty()?, &Automata::make_string("foo")?)?;
  let strings = TestOperations::get_finite_strings(&a)?;
  assert!(strings.is_empty());

  let a = Operations::concatenate(&Automata::make_string("foo")?, &Automata::make_empty()?)?;
  let strings = TestOperations::get_finite_strings(&a)?;
  assert!(strings.is_empty());

  Ok(())
}

#[test]
fn test_seems_non_empty_but_is_not1() -> Result<()> {
  let mut a = Automaton::new();
  let init = a.create_state();
  let s = a.create_state();
  a.add_transition_label(init, s, 'a' as i32)?;
  a.finish_state()?;
  assert!(Operations::is_empty(&a));
  Ok(())
}

#[test]
fn test_seems_non_empty_but_is_not2() -> Result<()> {
  let mut a = Automaton::new();
  let init = a.create_state();
  let s = a.create_state();
  a.add_transition_label(init, s, 'a' as i32)?;
  let orphan = a.create_state();
  a.set_accept(orphan, true);
  a.finish_state()?;
  assert!(Operations::is_empty(&a));
  Ok(())
}
#[test]
fn test_same_language1() -> Result<()> {
  let a = Automata::make_empty_string()?;
  let mut a2 = Automata::make_empty_string()?;
  let state = a2.create_state();
  a2.add_transition_label(0, state, 'a' as i32)?;
  a2.finish_state()?;

  let a_removed = Operations::remove_dead_states(&a)?;
  let a2_removed = Operations::remove_dead_states(&a2)?;

  assert!(AutomatonTestUtil::same_language(&a_removed, &a2_removed)?);
  Ok(())
}

fn random_no_op<'a, R>(a: &'a Automaton, random: &mut R) -> Result<Cow<'a, Automaton>>
where
  R: Rng + ?Sized,
{
  match random.random_range(0..7) {
    0 => Ok(Operations::determinize(a, i32::MAX as usize)?),
    1 => {
      if a.get_num_states() < 100 {
        Ok(MinimizationOperations::minimize(
          a,
          Operations::DEFAULT_DETERMINIZE_WORK_LIMIT,
        )?)
      } else {
        Ok(Cow::Borrowed(a))
      }
    },
    2 => Ok(Operations::remove_dead_states(a)?),
    3 => {
      // reverse -> randomNoOp -> reverse
      let a0 = Operations::reverse(a)?;
      let a1 = random_no_op(&a0, random)?;
      Ok(Cow::Owned(Operations::reverse(&a1)?))
    },
    4 => Ok(Cow::Owned(Operations::concatenate(
      a,
      &Automata::make_empty_string()?,
    )?)),
    5 => {
      // union with empty automaton
      Ok(Cow::Owned(Operations::union(a, &Automata::make_empty()?)?))
    },
    6 => Ok(Cow::Borrowed(a)),
    _ => unreachable!(),
  }
}
fn has_massive_term(terms: &[BytesRef<Vec<u8>>]) -> bool {
  for term in terms {
    if term.length > Automata::MAX_STRING_UNION_TERM_LENGTH as usize {
      return true;
    }
  }
  false
}
fn union_terms<R>(terms: &[BytesRef<Vec<u8>>], rng: &mut R) -> Result<Automaton>
where
  R: Rng + ?Sized,
{
  let a = if rng.random_bool(0.5) || has_massive_term(terms) {
    let owned_automata: Vec<Automaton> = terms
      .iter()
      .map(|term| Automata::make_string(&term.utf8_to_string()?))
      .collect::<Result<Vec<_>>>()?;
    let refs: Vec<&Automaton> = owned_automata.iter().collect();
    Operations::union_list(&refs)?
  } else {
    let mut terms_list = terms.to_vec();
    terms_list.sort();
    Automata::make_string_union(&terms_list)?
  };
  Ok(random_no_op(&a, rng)?.into_owned())
}
fn get_random_string<R>(random: &mut R) -> String
where
  R: Rng + ?Sized,
{
  TestUtil::random_realistic_unicode_string(random)
}
#[test]
fn test_random_finite() -> Result<()> {
  let mut random = random();
  let num_terms = at_least(&mut random, 10);
  let iters = at_least(&mut random, 100);

  let mut terms: BTreeSet<BytesRef<Vec<u8>>> = BTreeSet::new();
  while terms.len() < num_terms as usize {
    let s = get_random_string(&mut random);
    terms.insert(new_bytes_ref_from_string(&mut random, &s)?);
  }

  let mut a = Cow::Owned(union_terms(
    &terms.iter().cloned().collect::<Vec<_>>(),
    &mut random,
  )?);
  assert_same(&terms.iter().cloned().collect::<Vec<_>>(), &a, &mut random)?;

  for _ in 0..iters {
    match random.random_range(0..15) {
      0 => {
        let string = get_random_string(&mut random);
        let prefix = new_bytes_ref_from_string(&mut random, &string)?;
        let mut new_terms = BTreeSet::new();
        let mut new_term = BytesRefBuilder::new();
        for term in &terms {
          new_term.copy_bytes_from_ref(&prefix);
          new_term.append(term);
          new_terms.insert(new_term.get_bytes_ref_copy());
        }
        terms = new_terms;
        let was_deterministic1 = a.is_deterministic();
        a = Cow::Owned(Operations::concatenate(
          &Automata::make_string(&prefix.utf8_to_string()?)?,
          &a,
        )?);
        assert_eq!(was_deterministic1, a.is_deterministic());
      },
      1 => {
        let v = get_random_string(&mut random);
        let suffix = new_bytes_ref_from_string(&mut random, &v)?;
        let mut new_terms = BTreeSet::new();
        let mut b = BytesRefBuilder::new();
        for term in &terms {
          b.copy_bytes_from_ref(term);
          b.append(&suffix);
          new_terms.insert(b.get_bytes_ref_copy());
        }
        terms = new_terms;
        a = Cow::Owned(Operations::concatenate(
          &a,
          &Automata::make_string(&suffix.utf8_to_string()?)?,
        )?);
      },
      2 => {
        if let Cow::Owned(a2) = Operations::determinize(&a, i32::MAX as usize)? {
          a = Cow::Owned(a2);
        }
        assert!(a.is_deterministic());
      },
      3 => {
        if a.get_num_states() < 100
          && let Cow::Owned(a2) =
            MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
        {
          a = Cow::Owned(a2);
          assert!(a.is_deterministic());
        }
      },
      4 => {
        let mut new_terms = BTreeSet::new();
        let num_new = random.random_range(0..5);
        while new_terms.len() < num_new {
          let s = get_random_string(&mut random);
          new_terms.insert(new_bytes_ref_from_string(&mut random, &s)?);
        }
        let mut combined = terms.clone();
        combined.extend(new_terms.iter().cloned());
        let a2 = union_terms(&new_terms.iter().cloned().collect::<Vec<_>>(), &mut random)?;
        terms = combined;
        a = Cow::Owned(Operations::union(&a, &a2)?);
      },
      5 => {
        if let Cow::Owned(a2) = Operations::optional(&a)? {
          a = Cow::Owned(a2);
        }
        terms.insert(new_bytes_ref_empty(&mut random)?);
      },
      6 if !terms.is_empty() => {
        let v = Operations::remove_dead_states(&a)?;
        let ras = RandomAcceptedStrings::new(&v)?;
        let mut to_remove = BTreeSet::new();
        let num_to_remove = TestUtil::next_int(&mut random, 1, terms.len().div_ceil(2) as i32);
        while to_remove.len() < num_to_remove as usize {
          let ints = ras.get_random_accepted_string(&mut random)?;
          let len = ints.len();
          let s = new_bytes_ref_from_string(&mut random, &UnicodeUtil::new_string(&ints, 0, len)?)?;
          if !to_remove.contains(&s) {
            to_remove.insert(s);
          }
        }
        for t in &to_remove {
          let removed = terms.remove(t);
          assert!(removed)
        }
        let a2 = union_terms(&to_remove.iter().cloned().collect::<Vec<_>>(), &mut random)?;
        if let Cow::Owned(o) = Operations::minus(&a, &a2, i32::MAX as usize)? {
          a = Cow::Owned(o);
        }
      },
      7 => {
        // minus infinite
        let count = TestUtil::next_int(&mut random, 1, 5);
        let mut prefixes = HashSet::new();
        while prefixes.len() < count as usize {
          let prefix = random.random_range(0..128);
          prefixes.insert(prefix);
        }

        if cfg!(feature = "test_log_verbose") {
          println!("  op=minus infinite prefixes={:?}", prefixes);
        }

        let mut as_ = vec![];

        for &prefix in &prefixes {
          let mut a2 = Automaton::new();
          let init = a2.create_state();
          let state = a2.create_state();
          a2.add_transition_label(init, state, prefix)?;
          a2.set_accept(state, true);
          a2.add_transition(state, state, char::MIN as i32, char::MAX as i32)?;
          a2.finish_state()?;
          as_.push(a2);
          terms.retain(|t| {
            if t.length > 0 {
              let first_byte = t.bytes[t.offset] as i32;
              first_byte != prefix
            } else {
              true
            }
          });
        }

        let refs: Vec<&Automaton> = as_.iter().collect();
        let v = Operations::union_list(&refs)?;
        let a2 = random_no_op(&v, &mut random)?;
        if let Cow::Owned(o) =
          Operations::minus(&a, &a2, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
        {
          a = Cow::Owned(o);
        }
      },
      8 => {
        let count = TestUtil::next_int(&mut random, 10, 20);
        if cfg!(feature = "test_log_verbose") {
          println!("  op=intersect infinite count={}", count);
        }

        let mut prefixes = HashSet::new();
        while prefixes.len() < count as usize {
          let prefix = random.random_range(0..128);
          prefixes.insert(prefix);
        }

        if cfg!(feature = "test_log_verbose") {
          println!("  prefixes={:?}", prefixes);
        }

        let mut as_ = vec![];

        for &prefix in &prefixes {
          let mut a2 = Automaton::new();
          let init = a2.create_state();
          let state = a2.create_state();
          a2.add_transition_label(init, state, prefix)?;
          a2.set_accept(state, true);
          a2.add_transition(state, state, char::MIN as i32, char::MAX as i32)?;
          a2.finish_state()?;
          as_.push(a2);
        }

        let refs: Vec<&Automaton> = as_.iter().collect();
        let mut a2 = Cow::Owned(Operations::union_list(&refs)?);
        if random.random_bool(0.5) {
          if let Cow::Owned(o) =
            Operations::determinize(&a2, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
          {
            a2 = Cow::Owned(o);
          }
        } else if random.random_bool(0.5)
          && let Cow::Owned(o) =
            MinimizationOperations::minimize(&a2, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
        {
          a2 = Cow::Owned(o);
        }

        if let Cow::Owned(o) = Operations::intersection(&a, &a2)? {
          a = Cow::Owned(o);
        }

        terms.retain(|t| {
          if t.length == 0 {
            false
          } else {
            let first_byte = t.bytes[t.offset] as i32;
            prefixes.contains(&first_byte)
          }
        });
      },

      9 => {
        a = Cow::Owned(Operations::reverse(&a)?);
        let mut reversed_terms = BTreeSet::new();
        for t in &terms {
          let rev = t.utf8_to_string()?.chars().rev().collect::<String>();
          reversed_terms.insert(new_bytes_ref_from_string(&mut random, &rev)?);
        }
        terms = reversed_terms;
      },
      10 => {
        if let Cow::Owned(o) = random_no_op(&a, &mut random)? {
          a = Cow::Owned(o);
        }
      },
      11 => {
        let min = random.random_range(0..1000);
        let max = min + random.random_range(0..50);
        let digits = max.to_string().len();

        if cfg!(feature = "test_log_verbose") {
          println!(
            "  op=union interval min={} max={} digits={}",
            min, max, digits
          );
        }

        let interval_automaton = Automata::make_decimal_interval(min, max, digits as i32)?;
        a = Cow::Owned(Operations::union(&a, &interval_automaton)?);

        let prefix = "0".repeat(digits);
        for i in min..=max {
          let mut s = i.to_string();
          if s.len() < digits {
            s = format!("{}{}", &prefix[s.len()..], s);
          }
          terms.insert(new_bytes_ref_from_string(&mut random, &s)?);
        }
      },
      12 => {
        let v = Automata::make_empty_string()?;
        if let Cow::Owned(o) =
          Operations::minus(&a, &v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
        {
          a = Cow::Owned(o)
        }
        terms.remove(&new_bytes_ref_empty(&mut random)?);
      },
      13 => {
        a = Cow::Owned(Operations::union(&a, &Automata::make_empty_string()?)?);
        terms.insert(new_bytes_ref_empty(&mut random)?);
      },
      14 if terms.len() <= (num_terms * 3) as usize => {
        if cfg!(feature = "test_log_verbose") {
          println!("  op=concat finite automaton");
        }

        let count = if random.random_bool(0.5) { 2 } else { 3 };
        let mut add_terms = BTreeSet::new();
        while add_terms.len() < count {
          let s = get_random_string(&mut random);
          add_terms.insert(new_bytes_ref_from_string(&mut random, &s)?);
        }

        if cfg!(feature = "test_log_verbose") {
          for term in &add_terms {
            println!("    term={:?}", term);
          }
        }

        let add_vec: Vec<_> = add_terms.iter().cloned().collect();
        let a2 = union_terms(&add_vec, &mut random)?;

        let mut new_terms = BTreeSet::new();

        if random.random_bool(0.5) {
          // suffix
          if cfg!(feature = "test_log_verbose") {
            println!("  do suffix");
          }
          let a2 = random_no_op(&a2, &mut random)?;
          a = Cow::Owned(Operations::concatenate(&a, &a2)?);

          let mut new_term = BytesRefBuilder::new();
          for term in &terms {
            for suffix in &add_terms {
              new_term.copy_bytes_from_ref(term);
              new_term.append(suffix);
              new_terms.insert(new_term.get_bytes_ref_copy());
            }
          }
        } else {
          // prefix
          if cfg!(feature = "test_log_verbose") {
            println!("  do prefix");
          }
          let a2 = random_no_op(&a2, &mut random)?;
          a = Cow::Owned(Operations::concatenate(&a2, &a)?);

          let mut new_term = BytesRefBuilder::new();
          for term in &terms {
            for prefix in &add_terms {
              new_term.copy_bytes_from_ref(prefix);
              new_term.append(term);
              new_terms.insert(new_term.get_bytes_ref_copy());
            }
          }
        }

        terms = new_terms;
      },

      _ => {}, // others omitted for brevity
    }
    assert_same(&terms.iter().cloned().collect::<Vec<_>>(), &a, &mut random)?;
    let left = AutomatonTestUtil::is_deterministic_slow(&a);
    let right = a.is_deterministic();
    assert_eq!(left, right);
    if random.random_range(0..10) == 7 {
      a = Cow::Owned(verify_topo_sort(&a)?)
    }
  }
  assert_same(&terms.iter().cloned().collect::<Vec<_>>(), &a, &mut random)?;

  Ok(())
}
/// Runs topo sort, verifies transitions then only "go forwards", and builds
/// and returns new automaton with those remapped toposorted states.
pub fn verify_topo_sort(a: &Automaton) -> Result<Automaton> {
  let sorted = Operations::topo_sort_states(a)?;
  // This can be < if we removed dead states:
  assert!(sorted.len() <= a.get_num_states() as usize);

  let mut a2 = Automaton::new();
  let mut state_map = vec![-1; a.get_num_states() as usize];
  let mut t = Transition::default();

  for &state in &sorted {
    let new_state = a2.create_state();
    let accept = a.is_accept(state);
    a2.set_accept(new_state, accept);
    assert_eq!(state_map[state as usize], -1);
    state_map[state as usize] = new_state;
  }
  // 2nd pass: add new transitions
  for &state in &sorted {
    let count = a.init_transition(state, &mut t);
    for _ in 0..count {
      a.get_next_transition(&mut t);
      assert!(state_map[t.dest as usize] > state_map[state as usize]);
      a2.add_transition(
        state_map[state as usize],
        state_map[t.dest as usize],
        t.min,
        t.max,
      )?;
    }
  }

  a2.finish_state()?;
  Ok(a2)
}

pub fn assert_same<R>(terms: &[BytesRef<Vec<u8>>], a: &Automaton, random: &mut R) -> Result<()>
where
  R: Rng + ?Sized,
{
  assert!(AutomatonTestUtil::is_finite(a)?);
  assert!(!Operations::is_total(a)?);

  let det_a = Operations::determinize(a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  // Make sure all terms are accepted:
  let mut scratch: IntsRefBuilder<Vec<i32>> = IntsRefBuilder::new();
  for term in terms {
    Util::to_ints_ref(term, &mut scratch);
    let s = term.utf8_to_string()?;
    assert!(
      Operations::run_str(&det_a, &s),
      "failed to accept term={}",
      s
    );
  }

  // Use getFiniteStrings:
  let mut expected = HashSet::new();
  for term in terms {
    let mut ints_ref = IntsRefBuilder::new();
    let s = term.utf8_to_string()?;
    Util::to_utf32(&s, &mut ints_ref);
    expected.insert(ints_ref.to_ints_ref());
  }
  let actual = TestOperations::get_finite_strings(a)?;

  if expected != actual {
    println!("FAILED: ");
    for term in &expected {
      if !actual.contains(term) {
        println!("  term={:?} should be accepted but isn't", term);
      }
    }
    for term in &actual {
      if !expected.contains(term) {
        println!("  term={:?} is accepted but should not be", term);
      }
    }
    unreachable!("mismatch");
  }
  // check same language via determinized unionTerms
  let v0 = &union_terms(terms, random)?;
  let v1 = Operations::determinize(v0, i32::MAX as usize)?;
  let a2 = Operations::remove_dead_states(&v1)?;
  let v0 = Operations::determinize(a, i32::MAX as usize)?;
  let a3 = Operations::remove_dead_states(&v0)?;
  assert!(AutomatonTestUtil::same_language(&a2, &a3)?);

  // check in UTF8 space
  let v = UTF32ToUTF8::default().convert(a)?;
  let utf8 = random_no_op(&v, random)?;

  let mut expected2 = HashSet::new();
  for term in terms {
    let mut ints_ref = IntsRefBuilder::new();
    Util::to_ints_ref(term, &mut ints_ref);
    expected2.insert(ints_ref.to_ints_ref());
  }

  assert_eq!(expected2, TestOperations::get_finite_strings(&utf8)?);

  Ok(())
}
fn accepts(a: &Automaton, b: &BytesRef<Vec<u8>>) -> Result<bool> {
  let mut builder = IntsRefBuilder::new();
  Util::to_ints_ref(b, &mut builder);
  Ok(Operations::run_ints_ref(a, builder.get()))
}
fn make_binary_interval(
  min_term: Option<BytesRef<Vec<u8>>>,
  min_inclusive: bool,
  max_term: Option<BytesRef<Vec<u8>>>,
  max_inclusive: bool,
) -> Result<Automaton> {
  let a = Automata::make_binary_interval(
    min_term.as_ref(),
    min_inclusive,
    max_term.as_ref(),
    max_inclusive,
  )?;
  let min_a = MinimizationOperations::minimize(&a, i32::MAX as usize)?;

  if min_a.get_num_states() != a.get_num_states() {
    assert!(min_a.get_num_states() < a.get_num_states());
    return Err(LuceneError::illegal_state("automaton was not minimal"));
  }
  Ok(a)
}
#[test]
fn test_make_binary_interval_finite_cases_basic() -> Result<()> {
  let zeros = vec![0u8; 3];
  let mut random = random();

  // 0 (incl) - 00 (incl)
  let a = make_binary_interval(
    Some(new_bytes_ref(&mut random, zeros.as_slice(), 0, 1)?),
    true,
    Some(new_bytes_ref(&mut random, zeros.as_slice(), 0, 2)?),
    true,
  )?;
  assert!(AutomatonTestUtil::is_finite(&a)?);
  assert!(!accepts(&a, &new_bytes_ref_empty(&mut random)?)?);
  assert!(accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 1)?
  )?);
  assert!(accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 2)?
  )?);
  assert!(!accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 3)?
  )?);

  // '' (incl) - 00 (incl)
  let a = make_binary_interval(
    Some(new_bytes_ref_empty(&mut random)?),
    true,
    Some(new_bytes_ref(&mut random, zeros.as_slice(), 0, 2)?),
    true,
  )?;
  assert!(AutomatonTestUtil::is_finite(&a)?);
  assert!(accepts(&a, &new_bytes_ref_empty(&mut random)?)?);
  assert!(accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 1)?
  )?);
  assert!(accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 2)?
  )?);
  assert!(!accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 3)?
  )?);

  // '' (excl) - 00 (incl)
  let a = make_binary_interval(
    Some(new_bytes_ref_empty(&mut random)?),
    false,
    Some(new_bytes_ref(&mut random, zeros.as_slice(), 0, 2)?),
    true,
  )?;
  assert!(AutomatonTestUtil::is_finite(&a)?);
  assert!(!accepts(&a, &new_bytes_ref_empty(&mut random)?)?);
  assert!(accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 1)?
  )?);
  assert!(accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 2)?
  )?);
  assert!(!accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 3)?
  )?);

  // 0 (excl) - 00 (incl)
  let a = make_binary_interval(
    Some(new_bytes_ref(&mut random, zeros.as_slice(), 0, 1)?),
    false,
    Some(new_bytes_ref(&mut random, zeros.as_slice(), 0, 2)?),
    true,
  )?;
  assert!(AutomatonTestUtil::is_finite(&a)?);
  assert!(!accepts(&a, &new_bytes_ref_empty(&mut random)?)?);
  assert!(!accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 1)?
  )?);
  assert!(accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 2)?
  )?);
  assert!(!accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 3)?
  )?);

  // 0 (excl) - 00 (excl)
  let a = make_binary_interval(
    Some(new_bytes_ref(&mut random, zeros.as_slice(), 0, 1)?),
    false,
    Some(new_bytes_ref(&mut random, zeros.as_slice(), 0, 2)?),
    false,
  )?;
  assert!(AutomatonTestUtil::is_finite(&a)?);
  assert!(!accepts(&a, &new_bytes_ref_empty(&mut random)?)?);
  assert!(!accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 1)?
  )?);
  assert!(!accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 2)?
  )?);
  assert!(!accepts(
    &a,
    &new_bytes_ref(&mut random, zeros.as_slice(), 0, 3)?
  )?);

  Ok(())
}

#[test]
fn test_make_binary_interval_finite_cases_random() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 100);

  for _ in 0..iters {
    let s = TestUtil::random_realistic_unicode_string(&mut random);
    let prefix = new_bytes_ref_from_string(&mut random, &s)?;

    let mut b = BytesRefBuilder::new();
    b.append(&prefix);
    let num_zeros = random.random_range(0..10);
    for _ in 0..num_zeros {
      b.append_byte(0);
    }
    let min_term = b.get_bytes_ref_copy();

    let mut b = BytesRefBuilder::new();
    b.append(&min_term);
    let num_zeros = random.random_range(0..10);
    for _ in 0..num_zeros {
      b.append_byte(0);
    }
    let max_term = b.get_bytes_ref_copy();

    let min_inclusive = random.random_bool(0.5);
    let max_inclusive = random.random_bool(0.5);

    let a = make_binary_interval(
      Some(min_term.clone()),
      min_inclusive,
      Some(max_term.clone()),
      max_inclusive,
    )?;
    assert!(AutomatonTestUtil::is_finite(&a)?);

    let mut expected_count = max_term.length as i32 - min_term.length as i32 + 1;
    if !min_inclusive {
      expected_count -= 1;
    }
    if !max_inclusive {
      expected_count -= 1;
    }

    if expected_count <= 0 {
      assert!(Operations::is_empty(&a));
      continue;
    } else {
      // Enumerate all finite strings and verify the count matches what we expect:
      let actual = TestOperations::get_finite_strings_with_limit(&a, expected_count)?;
      assert_eq!(expected_count as usize, actual.len());
    }

    let mut b = BytesRefBuilder::new();
    b.append(&min_term);

    if !min_inclusive {
      assert!(!accepts(&a, &b.get_bytes_ref_copy())?);
      b.append_byte(0);
    }

    while b.length() < max_term.length {
      b.append_byte(0);

      let expected = if b.length() == max_term.length {
        max_inclusive
      } else {
        true
      };

      assert_eq!(expected, accepts(&a, &b.get_bytes_ref_copy())?);
    }
  }
  Ok(())
}

#[test]
fn test_make_binary_interval_random() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 100);

  for _ in 0..iters {
    let min_term = TestUtil::random_binary_term(&mut random);
    let min_inclusive = random.random_bool(0.5);
    let max_term = TestUtil::random_binary_term(&mut random);
    let max_inclusive = random.random_bool(0.5);

    let a = make_binary_interval(
      Some(min_term.clone()),
      min_inclusive,
      Some(max_term.clone()),
      max_inclusive,
    )?;

    for _ in 0..500 {
      let term = TestUtil::random_binary_term(&mut random);

      let min_cmp = min_term.cmp(&term).to_int();
      let max_cmp = max_term.cmp(&term).to_int();

      let expected = if min_cmp > 0 || max_cmp < 0 {
        false
      } else if min_cmp == 0 && max_cmp == 0 {
        min_inclusive && max_inclusive
      } else if min_cmp == 0 {
        min_inclusive
      } else if max_cmp == 0 {
        max_inclusive
      } else {
        true
      };

      let mut ints_builder = IntsRefBuilder::new();
      Util::to_ints_ref(&term, &mut ints_builder);
      let actual = Operations::run_ints_ref(&a, &ints_builder.to_ints_ref());
      assert_eq!(expected, actual,);
    }
  }

  Ok(())
}
fn ints_ref<R>(s: &str, random: &mut R) -> Result<IntsRef<Vec<i32>>>
where
  R: Rng + ?Sized,
{
  let mut builder = IntsRefBuilder::new();
  let b: BytesRef<Vec<u8>> = new_bytes_ref_from_string(random, s)?;
  Util::to_ints_ref(&b, &mut builder);
  Ok(builder.get().clone())
}

#[test]
fn test_make_binary_interval_basic() -> Result<()> {
  let mut random = random();

  let a = Automata::make_binary_interval(
    Some(&new_bytes_ref_from_string(&mut random, "bar")?),
    true,
    Some(&new_bytes_ref_from_string(&mut random, "foo")?),
    true,
  )?;
  assert!(Operations::run_ints_ref(&a, &ints_ref("bar", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("foo", &mut random)?));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("beep", &mut random)?
  ));
  assert!(!Operations::run_ints_ref(
    &a,
    &ints_ref("baq", &mut random)?
  ));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("bara", &mut random)?
  ));

  Ok(())
}

#[test]
fn test_make_binary_interval_lower_bound_empty_string() -> Result<()> {
  let mut random = random();

  let a = Automata::make_binary_interval(
    Some(&new_bytes_ref_from_string(&mut random, "")?),
    true,
    Some(&new_bytes_ref_from_string(&mut random, "bar")?),
    true,
  )?;
  assert!(Operations::run_ints_ref(&a, &ints_ref("", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("a", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("bar", &mut random)?));
  assert!(!Operations::run_ints_ref(
    &a,
    &ints_ref("bara", &mut random)?
  ));
  assert!(!Operations::run_ints_ref(
    &a,
    &ints_ref("baz", &mut random)?
  ));

  let a = Automata::make_binary_interval(
    Some(&new_bytes_ref_from_string(&mut random, "")?),
    false,
    Some(&new_bytes_ref_from_string(&mut random, "bar")?),
    true,
  )?;
  assert!(!Operations::run_ints_ref(&a, &ints_ref("", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("a", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("bar", &mut random)?));
  assert!(!Operations::run_ints_ref(
    &a,
    &ints_ref("bara", &mut random)?
  ));
  assert!(!Operations::run_ints_ref(
    &a,
    &ints_ref("baz", &mut random)?
  ));

  Ok(())
}
#[test]
fn test_make_binary_interval_equal() -> Result<()> {
  let mut random = random();

  let a = Automata::make_binary_interval(
    Some(&new_bytes_ref_from_string(&mut random, "bar")?),
    true,
    Some(&new_bytes_ref_from_string(&mut random, "bar")?),
    true,
  )?;
  assert!(Operations::run_ints_ref(&a, &ints_ref("bar", &mut random)?));
  assert!(AutomatonTestUtil::is_finite(&a)?);
  let strings = TestOperations::get_finite_strings(&a)?;
  assert_eq!(1, strings.len());

  Ok(())
}
#[test]
fn test_make_binary_interval_common_prefix() -> Result<()> {
  let mut random = random();

  let a = Automata::make_binary_interval(
    Some(&new_bytes_ref_from_string(&mut random, "bar")?),
    true,
    Some(&new_bytes_ref_from_string(&mut random, "barfoo")?),
    true,
  )?;
  assert!(!Operations::run_ints_ref(
    &a,
    &ints_ref("bam", &mut random)?
  ));
  assert!(Operations::run_ints_ref(&a, &ints_ref("bar", &mut random)?));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("bara", &mut random)?
  ));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("barf", &mut random)?
  ));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("barfo", &mut random)?
  ));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("barfoo", &mut random)?
  ));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("barfonz", &mut random)?
  ));
  assert!(!Operations::run_ints_ref(
    &a,
    &ints_ref("barfop", &mut random)?
  ));
  assert!(!Operations::run_ints_ref(
    &a,
    &ints_ref("barfoop", &mut random)?
  ));

  Ok(())
}
#[test]
fn test_make_binary_except_empty() -> Result<()> {
  let mut random = random();

  let a = Automata::make_non_empty_binary()?;
  assert!(!Operations::run_ints_ref(&a, &ints_ref("", &mut random)?));

  let s = TestUtil::random_realistic_unicode_string_range(&mut random, 1, 10);
  assert!(Operations::run_ints_ref(&a, &ints_ref(&s, &mut random)?));

  Ok(())
}
#[test]
fn test_make_binary_interval_open_max() -> Result<()> {
  let mut random = random();

  let a = Automata::make_binary_interval(
    Some(&new_bytes_ref_from_string(&mut random, "bar")?),
    true,
    None,
    true,
  )?;

  assert!(!Operations::run_ints_ref(
    &a,
    &ints_ref("bam", &mut random)?
  ));
  assert!(Operations::run_ints_ref(&a, &ints_ref("bar", &mut random)?));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("bara", &mut random)?
  ));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("barf", &mut random)?
  ));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("barfo", &mut random)?
  ));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("barfoo", &mut random)?
  ));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("barfonz", &mut random)?
  ));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("barfop", &mut random)?
  ));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("barfoop", &mut random)?
  ));
  assert!(Operations::run_ints_ref(&a, &ints_ref("zzz", &mut random)?));

  Ok(())
}
#[test]
fn test_make_binary_interval_open_max_zero_length_min() -> Result<()> {
  let mut random = random();

  let a = Automata::make_binary_interval(
    Some(&new_bytes_ref_from_string(&mut random, "")?),
    true,
    None,
    true,
  )?;

  assert!(Operations::run_ints_ref(&a, &ints_ref("", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("a", &mut random)?));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("aaaaaa", &mut random)?
  ));

  let a = Automata::make_binary_interval(
    Some(&new_bytes_ref_from_string(&mut random, "")?),
    false,
    None,
    true,
  )?;

  assert!(!Operations::run_ints_ref(&a, &ints_ref("", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("a", &mut random)?));
  assert!(Operations::run_ints_ref(
    &a,
    &ints_ref("aaaaaa", &mut random)?
  ));

  Ok(())
}
#[test]
fn test_make_binary_interval_open_min() -> Result<()> {
  let mut random = random();

  let a = Automata::make_binary_interval(
    None,
    true,
    Some(&new_bytes_ref_from_string(&mut random, "foo")?),
    true,
  )?;

  assert!(!Operations::run_ints_ref(
    &a,
    &ints_ref("foz", &mut random)?
  ));
  assert!(!Operations::run_ints_ref(
    &a,
    &ints_ref("zzz", &mut random)?
  ));
  assert!(Operations::run_ints_ref(&a, &ints_ref("foo", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("a", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("aaa", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("bz", &mut random)?));

  Ok(())
}
#[test]
fn test_make_binary_interval_open_both() -> Result<()> {
  let mut random = random();

  let a = Automata::make_binary_interval(None, true, None, true)?;

  assert!(Operations::run_ints_ref(&a, &ints_ref("foz", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("zzz", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("foo", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("a", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("aaa", &mut random)?));
  assert!(Operations::run_ints_ref(&a, &ints_ref("bz", &mut random)?));

  Ok(())
}
#[test]
fn test_accept_all_empty_string_min() -> Result<()> {
  let mut random = random();

  let a = Automata::make_binary_interval(
    Some(&new_bytes_ref_from_string(&mut random, "")?),
    true,
    None,
    true,
  )?;
  let any = Automata::make_any_binary()?;
  assert!(AutomatonTestUtil::same_language(&any, &a)?);

  Ok(())
}
fn to_ints_ref(s: &str) -> IntsRef<Vec<i32>> {
  let mut builder = IntsRefBuilder::new();
  for ch in s.chars() {
    builder.append(ch as i32);
  }
  builder.get().clone()
}
#[test]
fn test_get_singleton() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 10_000);

  for _ in 0..iters {
    let s = TestUtil::random_realistic_unicode_string(&mut random);
    let a = Automata::make_string(&s)?;
    assert_eq!(to_ints_ref(&s), Operations::get_singleton(&a)?.unwrap());
  }

  Ok(())
}
#[test]
fn test_get_singleton_empty_string() -> Result<()> {
  let mut a = Automaton::new();
  let s = a.create_state();
  a.set_accept(s, true);
  a.finish_state()?;
  assert_eq!(IntsRef::new(), Operations::get_singleton(&a)?.unwrap());
  Ok(())
}

#[test]
fn test_get_singleton_nothing() -> Result<()> {
  let mut a = Automaton::new();
  a.create_state();
  a.finish_state()?;
  assert!(Operations::get_singleton(&a)?.is_none());
  Ok(())
}

#[test]
fn test_get_singleton_two() -> Result<()> {
  let mut a = Automaton::new();
  let s = a.create_state();
  let x = a.create_state();
  a.set_accept(x, true);
  a.add_transition_label(s, x, 55)?;
  let y = a.create_state();
  a.set_accept(y, true);
  a.add_transition_label(s, y, 58)?;
  a.finish_state()?;
  assert!(Operations::get_singleton(&a)?.is_none());
  Ok(())
}
// LUCENE-9981
#[test]
fn test_determinize_too_much_effort() {
  // make sure determinize properly aborts, relatively quickly, for this regexp:
  let result = (|| {
    let a = RegExp::from_string("(.*a){2000}")?.to_automaton()?;
    Operations::determinize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
    Ok::<(), LuceneError>(())
  })();
  assert!(matches!(
    result,
    Err(LuceneError::TooComplexToDeterminize(_))
  ));

  let result = (|| {
    let a = RegExp::from_string("(.*a){2000}")?.to_automaton()?;
    let rev = Operations::reverse(&a)?;
    Operations::determinize(&rev, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
    Ok::<(), LuceneError>(())
  })();
  assert!(matches!(
    result,
    Err(LuceneError::TooComplexToDeterminize(_))
  ));
}
