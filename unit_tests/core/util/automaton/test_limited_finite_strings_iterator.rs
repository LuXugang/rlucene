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
use crate::core::util::automation::limited_finite_strings_iterator::LimitedFiniteStringsIterator;
use crate::core::util::automation::operations::Operations;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fst_impl::util::Util;
use crate::core::util::ints_ref_builder::IntsRefBuilder;
use crate::test::support::core::util::automaton::automaton_test_util::AutomatonTestUtil;
use crate::test::support::core::util::lucene_test_case::{at_least, random};
use crate::test::support::core::util::test_util::TestUtil;
use crate::util_tests::automaton::test_finite_strings_iterator::get_finite_strings;
#[allow(dead_code)] // for quick search
struct TestLimitedFiniteStringsIterator;
#[test]
fn test_random_finite_strings() -> Result<()> {
  let mut random = random();
  // Just makes sure we can run on any random finite
  // automaton:
  let iters = at_least(&mut random, 1000);
  for _ in 0..iters {
    let limit = TestUtil::next_int(&mut random, 1, 1000);
    let a = AutomatonTestUtil::random_automaton(&mut random)?;
    let mut v = LimitedFiniteStringsIterator::new(&a, limit)?;
    // Must pass a limit because the random automaton
    // can accept MANY strings:
    let result = get_finite_strings(&mut v);
    // NOTE: cannot do this, because the method is not
    // guaranteed to detect cycles when you have a limit
    // assertTrue(AutomatonTestUtil.isFinite(a));
    if result.is_err() {
      assert!(!AutomatonTestUtil::is_finite(&a)?);
    }
  }

  Ok(())
}

#[test]
fn test_invalid_limit_negative() -> Result<()> {
  let mut random = random();
  let a = AutomatonTestUtil::random_automaton(&mut random)?;

  let err = LimitedFiniteStringsIterator::new(&a, -7);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert!(err.unwrap_err().to_string().contains("limit must be -1"));
  Ok(())
}

#[test]
fn test_invalid_limit_null() -> Result<()> {
  let mut random = random();
  let a = AutomatonTestUtil::random_automaton(&mut random)?;

  let err = LimitedFiniteStringsIterator::new(&a, 0);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  assert!(err.unwrap_err().to_string().contains("limit must be -1"));
  Ok(())
}

#[test]
fn test_singleton() -> Result<()> {
  let a = Automata::make_string("foobar")?;
  let mut iterator = LimitedFiniteStringsIterator::new(&a, 1)?;
  let actual = get_finite_strings(&mut iterator)?;
  assert_eq!(1, actual.len());

  let mut scratch = IntsRefBuilder::new();
  Util::to_utf32_with_slice("foobar", 0, 6, &mut scratch)?;
  assert!(actual.contains(scratch.get()));

  Ok(())
}

#[test]
fn test_limit() -> Result<()> {
  let a = Operations::union(
    &Automata::make_string("foo")?,
    &Automata::make_string("bar")?,
  )?;

  // Test without limit
  let mut without_limit = LimitedFiniteStringsIterator::new(&a, -1)?;
  let actual1 = get_finite_strings(&mut without_limit)?;
  assert_eq!(2, actual1.len());

  // Test with limit
  let mut with_limit = LimitedFiniteStringsIterator::new(&a, 1)?;
  let actual2 = get_finite_strings(&mut with_limit)?;
  assert_eq!(1, actual2.len());

  Ok(())
}

#[test]
fn test_size() -> Result<()> {
  let a = Operations::union(
    &Automata::make_string("foo")?,
    &Automata::make_string("bar")?,
  )?;

  let mut iterator = LimitedFiniteStringsIterator::new(&a, -1)?;
  let actual = get_finite_strings(&mut iterator)?;
  assert_eq!(2, actual.len());
  assert_eq!(2, iterator.size());

  Ok(())
}
