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
use crate::core::util::automation::operations::Operations;
#[cfg(feature = "nightly")]
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::automation::transition_accessor::TransitionAccessor;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::automaton::automaton_test_util::AutomatonTestUtil;
use crate::test::core::util::automaton::minimization_operation::MinimizationOperations;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{at_least, random};
#[allow(dead_code)] // for quick search
/// This test builds some randomish NFA/DFA and minimizes them.
struct TestMinimize;
/// the minimal and non-minimal are compared to ensure they are the same.
#[test]
fn test_basic() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 200);

  for _ in 0..num {
    let a = AutomatonTestUtil::random_automaton(&mut random)?;
    let v = Operations::remove_dead_states(&a)?;
    let la = Operations::determinize(&v, i32::MAX as usize)?;
    let lb = MinimizationOperations::minimize(&a, i32::MAX as usize)?;
    assert!(AutomatonTestUtil::same_language(&la, &lb)?);
  }

  Ok(())
}
///  compare minimized against minimized with a slower, simple impl. we
/// check not only that they are  the same, but that
/// #states/#transitions are the same.
#[test]
fn test_against_brzozowski() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 200);

  for _ in 0..num {
    let a = AutomatonTestUtil::random_automaton(&mut random)?;
    let a = AutomatonTestUtil::minimize_simple(&a)?;

    let b = MinimizationOperations::minimize(&a, i32::MAX as usize)?;
    assert!(AutomatonTestUtil::same_language(&a, &b)?);
    assert_eq!(a.get_num_states(), b.get_num_states());

    let num_states = a.get_num_states();
    let sum1: i32 = (0..num_states)
      .map(|s| a.get_num_transitions_with_state(s))
      .sum();
    let sum2: i32 = (0..num_states)
      .map(|s| b.get_num_transitions_with_state(s))
      .sum();

    assert_eq!(sum1, sum2);
  }

  Ok(())
}
#[cfg(feature = "nightly")]
#[test]
#[ignore = "nightly"]
fn test_minimize_huge() -> Result<()> {
  let a = RegExp::parse("+-*(A|.....|BC)*]", RegExp::NONE, 0)?.to_automaton()?;
  let b = MinimizationOperations::minimize(&a, 1_000_000)?;
  assert!(b.is_deterministic());
  Ok(())
}
