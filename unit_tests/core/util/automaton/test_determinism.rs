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
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::automaton::automaton_test_util::AutomatonTestUtil;
use crate::test_framework::core::util::lucene_test_case::{at_least, random};
/// Not completely thorough, but tries to test determinism correctness somewhat
/// randomly.
#[allow(dead_code)] // for quick search
pub struct TestDeterminism;
/// test a bunch of random regular expressions
#[test]
fn test_regexps() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 500);
  for _ in 0..num {
    let pattern = AutomatonTestUtil::random_regexp(&mut random)?;
    let re = RegExp::parse(&pattern, RegExp::NONE, 0)?;
    let a = re.to_automaton()?;
    assert_automaton(&a)?;
  }
  Ok(())
}
/// test against a simple, unoptimized det
#[test]
fn test_against_simple() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 200);

  for _ in 0..num {
    let a0 = AutomatonTestUtil::random_automaton(&mut random)?;
    let a = AutomatonTestUtil::determinize_simple(&a0)?;
    let b = Operations::determinize(&a, usize::MAX)?;
    assert!(AutomatonTestUtil::same_language(&a, &b)?);
  }

  Ok(())
}
pub fn assert_automaton(a: &Automaton) -> Result<()> {
  let v = Operations::remove_dead_states(a)?;
  let a = Operations::determinize(&v, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;

  // complement(complement(a)) == a
  let equivalent = {
    let tmp = Operations::complement(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
    Operations::complement(&tmp, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
  };
  assert!(AutomatonTestUtil::same_language(&a, &equivalent)?);

  // a union a == a
  let union = Operations::union(&a, &a)?;
  let reduced = Operations::remove_dead_states(&union)?;
  let equivalent = Operations::determinize(&reduced, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  assert!(AutomatonTestUtil::same_language(&a, &equivalent)?);

  // a intersect a == a
  let inter = Operations::intersection(&a, &a)?;
  let reduced = Operations::remove_dead_states(&inter)?;
  let equivalent = Operations::determinize(&reduced, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  assert!(AutomatonTestUtil::same_language(&a, &equivalent)?);

  // a - a == empty
  let empty = Operations::minus(&a, &a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
  assert!(Operations::is_empty(&empty));

  // if a doesn't accept empty string: optional(a) - ε == a
  if !Operations::run_str(&a, "") {
    let optional = Operations::optional(&a)?;
    let epsilon = Automata::make_empty_string()?;
    let equivalent = Operations::minus(
      &optional,
      &epsilon,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT,
    )?;
    assert!(AutomatonTestUtil::same_language(&a, &equivalent)?);
  }

  Ok(())
}
