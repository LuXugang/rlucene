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
/// Not thorough, but tries to test determinism correctness somewhat randomly,
/// by determinizing a huge random lexicon.
#[cfg(test)]
use crate::test::core::util::lucene_test_case::{at_least, is_night_mode, random};
use rand::Rng;
use rand::prelude::SliceRandom;

use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::byte_run_automaton::ByteRunAutomaton;
use crate::core::util::automation::byte_runnable::ByteRunnable;
use crate::core::util::automation::operations::Operations;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::automaton::automaton_test_util::AutomatonTestUtil;
use crate::test::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestDeterminizeLexicon;
#[test]
fn test_determinize_lexicon() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 1);

  for _ in 0..num {
    let mut automata = Vec::with_capacity(5000);
    let mut terms = Vec::with_capacity(5000);

    for _ in 0..5000 {
      let s = TestUtil::random_unicode_string(&mut random);
      let a = Automata::make_string(&s)?;
      automata.push(a);
      terms.push(s);
    }

    assert_lexicon(&mut random, terms, automata)?;
  }
  Ok(())
}

fn assert_lexicon<R>(random: &mut R, terms: Vec<String>, mut automata: Vec<Automaton>) -> Result<()>
where
  R: Rng + ?Sized,
{
  automata.shuffle(random);
  let lex = Operations::union_list(&automata.iter().collect::<Vec<_>>())?;
  let lex = Operations::determinize(&lex, 1_000_000)?;
  assert!(AutomatonTestUtil::is_finite(&lex)?);

  for s in terms.iter() {
    assert!(Operations::run_str(&lex, s));
  }
  if is_night_mode() {
    let mut lex_byte = ByteRunAutomaton::new(lex.into_owned())?;
    for s in terms {
      let bytes = s.as_bytes();
      assert!(lex_byte.run(bytes, 0, bytes.len())?);
    }
  }
  Ok(())
}
