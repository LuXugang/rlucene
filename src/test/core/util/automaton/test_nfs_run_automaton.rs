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
  new_directory_shared, new_searcher_with_reader, new_text_field, random,
};
use std::collections::{HashMap, HashSet};

use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::index::directory_reader;
use crate::core::index::term::Term;
use crate::core::search::automaton_query::AutomatonQuery;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::nfa_run_automaton::NFARunAutomaton;
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::automation::transition::Transition;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::ints_ref::IntsRef;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::automaton::automaton_test_util::{
  AutomatonTestUtil, RandomAcceptedStrings,
};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;

const FIELD: &str = "field";
#[allow(dead_code)] // for quick search
struct TestNFARunAutomaton;
#[test]
fn test_ram_usage_estimation() -> Result<()> {
  // TODO: memory calculate not implement
  Ok(())
}
#[test]
fn test_with_random_regex() -> Result<()> {
  let mut random = random();

  for _ in 0..100 {
    let regexp_str = AutomatonTestUtil::random_regexp(&mut random)?;
    let re = RegExp::from_str_with_flags(&regexp_str, RegExp::NONE)?;
    let nfa = re.to_automaton()?;

    if nfa.is_deterministic() {
      continue;
    }

    let dfa = Operations::determinize(&nfa, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?;
    let mut candidate = NFARunAutomaton::new(nfa.clone());

    let generator = match RandomAcceptedStrings::new(&dfa) {
      Ok(g) => g,
      Err(_) => continue, /* sometimes the automaton accept nothing and throw this
                           * error  */
    };

    for _ in 0..20 {
      // test order of accepted strings and random (likely rejected) strings
      // alternatively to make sure caching system works correctly
      if random.random_bool(0.5) {
        test_accepted_string(&mut random, &re, &generator, &mut candidate, 10)?;
        test_random_string(&mut random, &re, &dfa, &mut candidate, 10)?;
      } else {
        test_random_string(&mut random, &re, &dfa, &mut candidate, 10)?;
        test_accepted_string(&mut random, &re, &generator, &mut candidate, 10)?;
      }
    }
  }

  Ok(())
}
#[test]
fn test_random_access_transition() -> Result<()> {
  let mut random = random();
  let s = AutomatonTestUtil::random_regexp(&mut random)?;
  let mut nfa = RegExp::from_str_with_flags(&s, RegExp::NONE)?.to_automaton()?;
  while nfa.is_deterministic() {
    let s = AutomatonTestUtil::random_regexp(&mut random)?;
    nfa = RegExp::from_str_with_flags(&s, RegExp::NONE)?.to_automaton()?;
  }

  let mut run_automaton1 = NFARunAutomaton::new(nfa.clone());
  let mut run_automaton2 = NFARunAutomaton::new(nfa);

  let mut visited = HashSet::new();
  assert_random_access_transition(
    &mut random,
    &mut run_automaton1,
    &mut run_automaton2,
    0,
    &mut visited,
  )?;

  Ok(())
}

fn assert_random_access_transition<R>(
  random: &mut R,
  automaton1: &mut NFARunAutomaton,
  automaton2: &mut NFARunAutomaton,
  state: i32,
  visited: &mut HashSet<i32>,
) -> Result<()>
where
  R: Rng + ?Sized,
{
  if !visited.insert(state) {
    return Ok(());
  }

  let mut t1 = Transition::default();
  let mut t2 = Transition::default();

  automaton1.init_transition(state, &mut t1)?;
  if random.random_bool(0.5) {
    automaton2.init_transition(state, &mut t2)?;
  }

  let num_transitions = automaton2.get_num_transitions_with_state(state)?;
  for i in 0..num_transitions {
    automaton1.get_next_transition(&mut t1);
    automaton2.get_transition(state, i, &mut t2)?;

    assert_eq!(format!("{:?}", t1), format!("{:?}", t2));

    assert_random_access_transition(random, automaton1, automaton2, t1.dest, visited)?;
  }

  Ok(())
}
#[test]
fn test_random_automaton_query() -> Result<()> {
  let mut random = random();

  let doc_num: usize = 50;
  let automaton_num: usize = 50;

  let directory = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, directory.clone());

  let mut vocab = HashSet::new();
  let mut per_doc_vocab = HashSet::new();
  let mut field_to_type = HashMap::new();
  for _ in 0..doc_num {
    per_doc_vocab.clear();

    let term_num: usize = random.random_range(0..20) + 30;
    while per_doc_vocab.len() < term_num {
      let mut s = TestUtil::random_unicode_string(&mut random);
      while s.is_empty() {
        s = TestUtil::random_unicode_string(&mut random);
      }
      per_doc_vocab.insert(s.clone());
      vocab.insert(s);
    }

    let text = per_doc_vocab
      .iter()
      .fold(String::new(), |s1, s2| s1 + " " + s2);

    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      FIELD,
      &text,
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(&mut random, doc)?;
  }

  writer.commit(&mut random)?;

  let reader = directory_reader::open(directory.clone())?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut foreign_vocab = HashSet::new();
  while foreign_vocab.len() < vocab.len() {
    let mut s = TestUtil::random_unicode_string(&mut random);
    while s.is_empty() {
      s = TestUtil::random_unicode_string(&mut random);
    }
    foreign_vocab.insert(s);
  }

  let vocab_list: Vec<String> = vocab.into_iter().collect();
  let foreign_vocab_list: Vec<String> = foreign_vocab.into_iter().collect();

  let mut per_query_vocab = HashSet::new();

  let mut i = 0;
  while i < automaton_num {
    per_query_vocab.clear();

    let term_num: usize = random.random_range(0..40) + 30;
    while per_query_vocab.len() < term_num {
      if random.random_bool(0.5) {
        let idx = random.random_range(0..vocab_list.len());
        per_query_vocab.insert(vocab_list[idx].clone());
      } else {
        let idx = random.random_range(0..foreign_vocab_list.len());
        per_query_vocab.insert(foreign_vocab_list[idx].clone());
      }
    }

    let mut a = None;
    for term in per_query_vocab.iter() {
      let s = Automata::make_string(term)?;
      a = match a {
        None => Some(s),
        Some(prev) => Some(Operations::union(&prev, &s)?),
      };
    }
    let a = a.unwrap();

    if a.is_deterministic() {
      continue;
    }

    let dfa = Operations::determinize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?.into_owned();

    let dfa_query = AutomatonQuery::from_automaton(Term::from_empty(FIELD), dfa)?;
    let nfa_query = AutomatonQuery::from_automaton(Term::from_empty(FIELD), a)?;

    assert!(nfa_query.get_compiled().nfa_run_automaton.is_some());

    assert_eq!(searcher.count(dfa_query)?, searcher.count(nfa_query)?);

    i += 1;
  }

  writer.close(&mut random)?;
  Ok(())
}

fn test_accepted_string<R>(
  random: &mut R,
  reg_exp: &RegExp,
  random_string_gen: &RandomAcceptedStrings,
  candidate: &mut NFARunAutomaton,
  repeat: usize,
) -> Result<()>
where
  R: Rng + ?Sized,
{
  for _ in 0..repeat {
    let accepted_string = random_string_gen.get_random_accepted_string(random)?;
    assert!(
      candidate.run(&accepted_string),
      "regExp: {} testString: {:?}",
      reg_exp,
      accepted_string
    );
  }

  Ok(())
}

fn test_random_string<R>(
  random: &mut R,
  reg_exp: &RegExp,
  dfa: &Automaton,
  candidate: &mut NFARunAutomaton,
  repeat: usize,
) -> Result<()>
where
  R: Rng + ?Sized,
{
  for _ in 0..repeat {
    let len = random.random_range(0..50);
    let random_string: Vec<i32> = (0..len)
      .map(|_| random.random_range(0..=char::MAX as i32))
      .collect();

    let s = format!("{:?}", random_string);
    let actual = candidate.run(&random_string);
    let expected = Operations::run_ints_ref(dfa, &IntsRef::from_slice(random_string, 0, len));

    assert_eq!(expected, actual, "regExp: {} testString: {:?}", reg_exp, s);
  }

  Ok(())
}
