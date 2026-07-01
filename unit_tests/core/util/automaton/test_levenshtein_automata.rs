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
use crate::core::util::automation::character_run_automaton::CharacterRunAutomaton;
use crate::core::util::automation::levenshtein_automata::LevenshteinAutomata;
use crate::core::util::automation::operations::Operations;
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::util::automaton::automaton_test_util::AutomatonTestUtil;
use crate::test::support::core::util::automaton::minimization_operation::MinimizationOperations;
struct TestLevenshteinAutomata;

#[test]
fn test_lev0() -> Result<()> {
  TestLevenshteinAutomata::assert_lev("", 0)?;
  TestLevenshteinAutomata::assert_char_vectors(0)
}

#[test]
fn test_lev1() -> Result<()> {
  TestLevenshteinAutomata::assert_lev("", 1)?;
  TestLevenshteinAutomata::assert_char_vectors(1)
}

#[test]
fn test_lev2() -> Result<()> {
  TestLevenshteinAutomata::assert_lev("", 2)?;
  TestLevenshteinAutomata::assert_char_vectors(2)
}

#[test]
fn test_no_wasted_states() -> Result<()> {
  let automaton = LevenshteinAutomata::new("abc", false)?
    .to_automaton(1)?
    .expect("distance 1 should be supported");
  assert!(!Operations::has_dead_states_from_initial(&automaton)?);
  Ok(())
}

impl TestLevenshteinAutomata {
  /// Tests all possible characteristic vectors for some n This exhaustively tests the parametric transitions tables.
  fn assert_char_vectors(n: usize) -> Result<()> {
    let k = 2 * n + 1;
    // use k + 2 as the exponent: the formula generates different transitions
    // for w, w-1, w-2
    let limit = 1 << (k + 2);
    for i in 0..limit {
      Self::assert_lev(&format!("{i:b}"), n)?;
    }
    Ok(())
  }
  /// Builds a DFA for some string, and checks all Lev automata up to some maximum distance.
  fn assert_lev(s: &str, max_distance: usize) -> Result<()> {
    let builder = LevenshteinAutomata::new(s, false)?;
    let tbuilder = LevenshteinAutomata::new(s, true)?;
    let mut automata = Vec::with_capacity(max_distance + 1);
    let mut tautomata = Vec::with_capacity(max_distance + 1);

    for n in 0..=max_distance {
      automata.push(
        builder
          .to_automaton(n)?
          .unwrap_or_else(|| panic!("distance {n} should be supported")),
      );
      tautomata.push(
        tbuilder
          .to_automaton(n)?
          .unwrap_or_else(|| panic!("distance {n} should be supported")),
      );

      assert!(automata[n].is_deterministic());
      assert!(tautomata[n].is_deterministic());
      assert!(AutomatonTestUtil::is_finite(&automata[n])?);
      assert!(AutomatonTestUtil::is_finite(&tautomata[n])?);
      assert!(!Operations::has_dead_states_from_initial(&automata[n])?);
      assert!(!Operations::has_dead_states_from_initial(&tautomata[n])?);

      if n > 0 {
        let a1 = Operations::remove_dead_states(&automata[n - 1])?;
        let a2 = Operations::remove_dead_states(&automata[n])?;
        assert!(AutomatonTestUtil::subset_of(&a1, &a2)?);

        let a1 = Operations::remove_dead_states(&automata[n - 1])?;
        let a2 = Operations::remove_dead_states(&tautomata[n])?;
        assert!(AutomatonTestUtil::subset_of(&a1, &a2)?);

        let a1 = Operations::remove_dead_states(&tautomata[n - 1])?;
        let a2 = Operations::remove_dead_states(&automata[n])?;
        assert!(AutomatonTestUtil::subset_of(&a1, &a2)?);

        let a1 = Operations::remove_dead_states(&tautomata[n - 1])?;
        let a2 = Operations::remove_dead_states(&tautomata[n])?;
        assert!(AutomatonTestUtil::subset_of(&a1, &a2)?);
      }
      let a1 = Operations::remove_dead_states(&automata[n])?;
      let a2 = Operations::remove_dead_states(&tautomata[n])?;
      assert!(AutomatonTestUtil::subset_of(&a1, &a2)?);

      match n {
        0 => {
          let expected = Automata::make_string(s)?;
          let _a1 = Operations::remove_dead_states(&automata[0])?;
          AutomatonTestUtil::same_language(&expected, &a2)?;

          let expected = Automata::make_string(s)?;
          let a2 = Operations::remove_dead_states(&tautomata[0])?;
          AutomatonTestUtil::same_language(&expected, &a2)?;
        },
        1 => {
          let expected = Self::naive_lev1(s)?;
          let a1 = Operations::remove_dead_states(&automata[0])?;
          AutomatonTestUtil::same_language(&expected, &a1)?;

          let expected = Self::naive_lev1_t(s)?;
          let a2 = Operations::remove_dead_states(&tautomata[0])?;
          AutomatonTestUtil::same_language(&expected, &a2)?;
        },
        _ => {
          Self::assert_brute_force(s, &automata[n], n)?;
          Self::assert_brute_force_t(s, &tautomata[n], n)?;
        },
      }
    }
    Ok(())
  }
  /// Return an automaton that accepts all 1-character insertions, deletions, and substitutions of s.
  fn naive_lev1(s: &str) -> Result<Automaton> {
    let mut a = Automata::make_string(s)?;
    a = Operations::union(&a, &Self::insertions_of(s)?)?;
    a = MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
      .into_owned();
    a = Operations::union(&a, &Self::deletions_of(s)?)?;
    a = MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
      .into_owned();
    a = Operations::union(&a, &Self::substitutions_of(s)?)?;
    Ok(
      MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
        .into_owned(),
    )
  }
  /// Return an automaton that accepts all 1-character insertions, deletions, substitutions, and transpositions of s.
  fn naive_lev1_t(s: &str) -> Result<Automaton> {
    let a = Self::naive_lev1(s)?;
    let a = Operations::union(&a, &Self::transpositions_of(s)?)?;
    Ok(
      MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
        .into_owned(),
    )
  }
  /// Return an automaton that accepts all 1-character insertions of s (inserting one character)
  fn insertions_of(s: &str) -> Result<Automaton> {
    let chars: Vec<char> = s.chars().collect();
    let mut list = Vec::new();

    for i in 0..=chars.len() {
      let prefix = Self::string_from_chars(&chars[..i]);
      let suffix = Self::string_from_chars(&chars[i..]);

      let mut a = Automata::make_string(&prefix)?;
      a = Operations::concatenate(&a, &Automata::make_any_char()?)?;
      a = Operations::concatenate(&a, &Automata::make_string(&suffix)?)?;
      list.push(a);
    }
    let list: Vec<&Automaton> = list.iter().collect();
    let a = Operations::union_list(list.as_slice())?;
    Ok(
      MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
        .into_owned(),
    )
  }
  /// Return an automaton that accepts all 1-character deletions of s (deleting one character).
  fn deletions_of(s: &str) -> Result<Automaton> {
    let chars: Vec<char> = s.chars().collect();
    let mut list = Vec::new();

    for i in 0..chars.len() {
      let prefix = Self::string_from_chars(&chars[..i]);
      let suffix = Self::string_from_chars(&chars[i + 1..]);
      list.push(Operations::concatenate(
        &Automata::make_string(&prefix)?,
        &Automata::make_string(&suffix)?,
      )?);
    }
    let list: Vec<&Automaton> = list.iter().collect();
    let a = Operations::union_list(list.as_slice())?;
    Ok(
      MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
        .into_owned(),
    )
  }
  /// Return an automaton that accepts all 1-character substitutions of s (replacing one character)
  fn substitutions_of(s: &str) -> Result<Automaton> {
    let chars: Vec<char> = s.chars().collect();
    let mut list = Vec::new();

    for i in 0..chars.len() {
      let prefix = Self::string_from_chars(&chars[..i]);
      let suffix = Self::string_from_chars(&chars[i + 1..]);
      let a = Operations::concatenate(
        &Automata::make_string(&prefix)?,
        &Automata::make_any_char()?,
      )?;
      list.push(Operations::concatenate(
        &a,
        &Automata::make_string(&suffix)?,
      )?);
    }
    let list: Vec<&Automaton> = list.iter().collect();
    let a = Operations::union_list(list.as_slice())?;
    Ok(
      MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
        .into_owned(),
    )
  }
  /// Return an automaton that accepts all transpositions of s (transposing two adjacent characters)
  fn transpositions_of(s: &str) -> Result<Automaton> {
    let mut split_points: Vec<usize> = s.char_indices().map(|(i, _)| i).collect();
    split_points.push(s.len());

    if split_points.len() - 1 < 2 {
      return Automata::make_empty();
    }

    let mut list = Vec::new();

    for i in 0..split_points.len() - 2 {
      let mut st = String::new();
      st.push_str(&s[..split_points[i]]);
      st.push_str(&s[split_points[i + 1]..split_points[i + 2]]);
      st.push_str(&s[split_points[i]..split_points[i + 1]]);
      st.push_str(&s[split_points[i + 2]..]);

      if st != s {
        list.push(Automata::make_string(&st)?);
      }
    }

    let list: Vec<&Automaton> = list.iter().collect();
    let a = Operations::union_list(list.as_slice())?;
    Ok(
      MinimizationOperations::minimize(&a, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)?
        .into_owned(),
    )
  }

  fn assert_brute_force(input: &str, dfa: &Automaton, distance: usize) -> Result<()> {
    let ra = CharacterRunAutomaton::new(dfa.clone())?;
    let max_len = input.len() + distance + 1;
    let max_num = 2_i32.pow(max_len as u32);

    for i in 0..max_num {
      let encoded = format!("{:b}", i);
      let accepts = ra.run_str(&encoded)?;

      if accepts {
        assert!(Self::get_distance(input, &encoded) <= distance as i32);
      } else {
        assert!(Self::get_distance(input, &encoded) > distance as i32);
      }
    }

    Ok(())
  }

  fn assert_brute_force_t(input: &str, dfa: &Automaton, distance: usize) -> Result<()> {
    let ra = CharacterRunAutomaton::new(dfa.clone())?;
    let max_len = input.len() + distance + 1;
    let max_num = 2_i32.pow(max_len as u32);

    for i in 0..max_num {
      let encoded = format!("{:b}", i);
      let accepts = ra.run_str(&encoded)?;

      if accepts {
        assert!(Self::get_t_distance(input, &encoded) <= distance as i32);
      } else {
        assert!(Self::get_t_distance(input, &encoded) > distance as i32);
      }
    }

    Ok(())
  }

  fn get_distance(target: &str, other: &str) -> i32 {
    let sa: Vec<char> = target.chars().collect();
    let n = sa.len();
    let mut p = vec![0; n + 1];
    let mut d = vec![0; n + 1];

    let other_chars: Vec<char> = other.chars().collect();
    let m = other_chars.len();

    if n == 0 || m == 0 {
      if n == m {
        return 0;
      } else {
        return n.max(m) as i32;
      }
    }

    for (i, value) in p.iter_mut().enumerate().take(n + 1) {
      *value = i as i32;
    }

    for j in 1..=m {
      let t_j = other_chars[j - 1];
      d[0] = j as i32;

      for i in 1..=n {
        let cost = if sa[i - 1] == t_j { 0 } else { 1 };

        // minimum of cell to the left+1, to the top+1, diagonally left and up +cost
        d[i] = (d[i - 1] + 1).min(p[i] + 1).min(p[i - 1] + cost);
      }

      // copy current distance counts to 'previous row' distance counts
      std::mem::swap(&mut p, &mut d);
    }

    // our last action in the above loop was to switch d and p, so p now
    // actually has the most recent cost counts
    p[n].abs()
  }
  fn get_t_distance(target: &str, other: &str) -> i32 {
    let sa: Vec<char> = target.chars().collect();
    let n = sa.len();

    let other_chars: Vec<char> = other.chars().collect();
    let m = other_chars.len();

    let mut d = vec![vec![0; m + 1]; n + 1];

    if n == 0 || m == 0 {
      if n == m {
        return 0;
      } else {
        return n.max(m) as i32;
      }
    }

    for (i, row) in d.iter_mut().enumerate().take(n + 1) {
      row[0] = i as i32;
    }

    for (j, value) in d[0].iter_mut().enumerate().take(m + 1) {
      *value = j as i32;
    }

    for j in 1..=m {
      let t_j = other_chars[j - 1];

      for i in 1..=n {
        let cost = if sa[i - 1] == t_j { 0 } else { 1 };

        // minimum of cell to the left+1, to the top+1, diagonally left and up +cost
        d[i][j] = (d[i - 1][j] + 1)
          .min(d[i][j - 1] + 1)
          .min(d[i - 1][j - 1] + cost);

        // transposition
        if i > 1 && j > 1 && sa[i - 1] == other_chars[j - 2] && sa[i - 2] == other_chars[j - 1] {
          d[i][j] = d[i][j].min(d[i - 2][j - 2] + cost);
        }
      }
    }

    // our last action in the above loop was to switch d and p, so p now
    // actually has the most recent cost counts
    d[n][m].abs()
  }

  fn string_from_chars(chars: &[char]) -> String {
    chars.iter().collect()
  }
}
