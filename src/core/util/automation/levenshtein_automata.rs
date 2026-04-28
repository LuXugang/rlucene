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
use std::collections::BTreeSet;

use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::lev1_parametric_description::Lev1ParametricDescription;
use crate::core::util::automation::lev1t_parametric_description::Lev1TParametricDescription;
use crate::core::util::automation::lev2_parametric_description::Lev2ParametricDescription;
use crate::core::util::automation::lev2t_parametric_description::Lev2TParametricDescription;
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::{
  lev1_parametric_description, lev1t_parametric_description, lev2_parametric_description,
  lev2t_parametric_description,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::impl_from_for_enum;

/// Class to construct DFAs that match a word within some edit distance.
///
/// Implements the algorithm described in: Schulz and Mihov: Fast String Correction with
/// Levenshtein Automata.
pub struct LevenshteinAutomata {
  /// Input word.
  word: Vec<i32>,
  /// The automata alphabet.
  alphabet: Vec<i32>,
  /// The maximum symbol in the alphabet (e.g. 255 for UTF-8 or 10FFFF for UTF-32).
  alpha_max: i32,
  /// Lower bounds for ranges outside of alphabet.
  range_lower: Vec<i32>,
  /// Upper bounds for ranges outside of alphabet.
  range_upper: Vec<i32>,
  num_ranges: usize,
  descriptions: Vec<Option<ParametricDescription>>,
}

impl LevenshteinAutomata {
  /// Maximum edit distance this class can generate an automaton for.
  pub const MAXIMUM_SUPPORTED_DISTANCE: usize = 2;

  /// Create a new `LevenshteinAutomata` for some input string. Optionally count transpositions as a
  /// primitive edit.
  pub fn new(input: &str, with_transpositions: bool) -> Result<Self> {
    Self::from_word(
      Self::code_points(input),
      char::MAX as i32,
      with_transpositions,
    )
  }

  /// Expert: specify a custom maximum possible symbol (`alpha_max`); default is
  /// `char::MAX`.
  pub fn from_word(word: Vec<i32>, alpha_max: i32, with_transpositions: bool) -> Result<Self> {
    let mut set = BTreeSet::new();
    for &v in &word {
      if v > alpha_max {
        return Err(LuceneError::illegal_argument(format!(
          "alphaMax exceeded by symbol {v} in word"
        )));
      }
      set.insert(v);
    }
    let alphabet: Vec<i32> = set.into_iter().collect();

    let mut range_lower = vec![0; alphabet.len() + 2];
    let mut range_upper = vec![0; alphabet.len() + 2];
    let mut num_ranges = 0;

    let mut lower = 0;
    for &higher in &alphabet {
      if higher > lower {
        range_lower[num_ranges] = lower;
        range_upper[num_ranges] = higher - 1;
        num_ranges += 1;
      }
      lower = higher + 1;
    }

    if lower <= alpha_max {
      range_lower[num_ranges] = lower;
      range_upper[num_ranges] = alpha_max;
      num_ranges += 1;
    }

    let w = word.len() as i32;
    let descriptions = vec![
      None,
      Some(if with_transpositions {
        lev1t_parametric_description::new(w)
      } else {
        lev1_parametric_description::new(w)
      }),
      Some(if with_transpositions {
        lev2t_parametric_description::new(w)
      } else {
        lev2_parametric_description::new(w)
      }),
    ];

    Ok(Self {
      word,
      alphabet,
      alpha_max,
      range_lower,
      range_upper,
      num_ranges,
      descriptions,
    })
  }

  fn code_points(input: &str) -> Vec<i32> {
    input.chars().map(|ch| ch as i32).collect()
  }

  /// Compute a DFA that accepts all strings within an edit distance of `n`.
  ///
  /// All automata have the following properties:
  ///
  /// - They are deterministic (DFA).
  /// - There are no transitions to dead states.
  /// - They are not minimal (some transitions could be combined).
  pub fn to_automaton(&self, n: usize) -> Result<Option<Automaton>> {
    self.to_automaton_with_prefix(n, "")
  }

  /// Compute a DFA that accepts all strings within an edit distance of `n`, matching the specified
  /// exact prefix.
  ///
  /// All automata have the following properties:
  ///
  /// - They are deterministic (DFA).
  /// - There are no transitions to dead states.
  /// - They are not minimal (some transitions could be combined).
  pub fn to_automaton_with_prefix(&self, n: usize, prefix: &str) -> Result<Option<Automaton>> {
    if n == 0 {
      let mut word = String::with_capacity(prefix.len() + self.word.len());
      word.push_str(prefix);
      for &cp in &self.word {
        match char::from_u32(cp as u32) {
          Some(ch) => word.push(ch),
          None => {
            return Err(LuceneError::illegal_argument(format!(
              "invalid Unicode code point {cp} in word"
            )));
          },
        }
      }
      return Ok(Some(Automata::make_string(&word)?));
    }

    if n >= self.descriptions.len() {
      return Ok(None);
    }

    let Some(description) = &self.descriptions[n] else {
      return Ok(None);
    };
    let range = 2 * n as i32 + 1;
    let num_states = description.size();
    let num_transitions = num_states * std::cmp::min(1 + 2 * n as i32, self.alphabet.len() as i32);
    let prefix_states = prefix.chars().count() as i32;

    let mut a = Automaton::with_capacity(
      (num_states + prefix_states) as usize,
      num_transitions as usize,
    );
    let mut last_state = a.create_state();
    for cp in prefix.chars().map(|ch| ch as i32) {
      let state = a.create_state();
      a.add_transition_label(last_state, state, cp)?;
      last_state = state;
    }

    let state_offset = last_state;
    a.set_accept(last_state, description.is_accept(0));

    for i in 1..num_states {
      let state = a.create_state();
      a.set_accept(state, description.is_accept(i));
    }

    for k in 0..num_states {
      let xpos = description.get_position(k);
      if xpos < 0 {
        continue;
      }
      let end = xpos + std::cmp::min(self.word.len() as i32 - xpos, range);

      for &ch in &self.alphabet {
        let cvec = self.get_vector(ch, xpos, end);
        let dest = description.transition(k, xpos, cvec);
        if dest >= 0 {
          a.add_transition_label(state_offset + k, state_offset + dest, ch)?;
        }
      }
      // add transitions for all other chars in unicode
      // by definition, their characteristic vectors are always 0,
      // because they do not exist in the input string.
      let dest = description.transition(k, xpos, 0);
      if dest >= 0 {
        for r in 0..self.num_ranges {
          a.add_transition(
            state_offset + k,
            state_offset + dest,
            self.range_lower[r],
            self.range_upper[r],
          )?;
        }
      }
    }

    a.finish_state()?;
    let automaton = Operations::remove_dead_states(&a)?.into_owned();
    debug_assert!(automaton.is_deterministic());
    Ok(Some(automaton))
  }

  /// Get the characteristic vector `X(x, V)` where V is `substring(pos, end)`.
  fn get_vector(&self, x: i32, pos: i32, end: i32) -> i32 {
    let mut vector = 0;
    for i in pos..end {
      vector <<= 1;
      if self.word[i as usize] == x {
        vector |= 1;
      }
    }
    vector
  }
}

/// A `ParametricDescription` describes the structure of a Levenshtein DFA for some degree `n`.
///
/// There are four components of a parametric description, all parameterized on the length of
/// the word `w`:
///
/// 1. The number of states: [`ParametricDescription::size`]
/// 2. The set of final states: [`ParametricDescription::is_accept`]
/// 3. The transition function: [`ParametricDescription::transition`]
/// 4. Minimal boundary function: [`ParametricDescription::get_position`]
pub(crate) struct ParametricDescription {
  pub(crate) w: i32,
  n: i32,
  min_errors: Vec<i32>,
  sub: ParametricDescriptionBaseEnum,
}

impl ParametricDescription {
  pub(crate) fn new<T>(w: i32, n: i32, min_errors: Vec<i32>, sub: T) -> Self
  where
    T: Into<ParametricDescriptionBaseEnum>,
  {
    Self {
      w,
      n,
      min_errors,
      sub: sub.into(),
    }
  }
  /// Return the number of states needed to compute a Levenshtein DFA.
  pub(crate) fn size(&self) -> i32 {
    self.min_errors.len() as i32 * (self.w + 1)
  }

  /// Returns `true` if the `state` in any Levenshtein DFA is an accept state (final state).
  pub(crate) fn is_accept(&self, abs_state: i32) -> bool {
    // decode absState -> state, offset
    let state = abs_state / (self.w + 1);
    let offset = abs_state % (self.w + 1);
    debug_assert!(offset >= 0);

    self.w - offset + self.min_errors[state as usize] <= self.n
  }

  /// Returns the position in the input word for a given `state`. This is the minimal boundary for
  /// the state.
  pub(crate) fn get_position(&self, abs_state: i32) -> i32 {
    abs_state % (self.w + 1)
  }

  pub(crate) fn transition(&self, state: i32, position: i32, vector: i32) -> i32 {
    self.sub.transition(state, position, vector, self)
  }
}
pub trait ParametricDescriptionBase {
  /// Returns the state number for a transition from the given `state`, assuming `position` and
  /// characteristic vector `vector`.
  fn transition(&self, state: i32, position: i32, vector: i32, base: &ParametricDescription)
  -> i32;
}
pub enum ParametricDescriptionBaseEnum {
  Lev1(Lev1ParametricDescription),
  Lev1T(Lev1TParametricDescription),
  Lev2(Lev2ParametricDescription),
  Lev2T(Lev2TParametricDescription),
}
impl_from_for_enum!(
ParametricDescriptionBaseEnum,
Lev1ParametricDescription=> Lev1,
Lev1TParametricDescription=> Lev1T,
Lev2ParametricDescription=> Lev2,
Lev2TParametricDescription=> Lev2T,
);
impl ParametricDescriptionBase for ParametricDescriptionBaseEnum {
  fn transition(
    &self,
    state: i32,
    position: i32,
    vector: i32,
    base: &ParametricDescription,
  ) -> i32 {
    match self {
      ParametricDescriptionBaseEnum::Lev1(lev1) => lev1.transition(state, position, vector, base),
      ParametricDescriptionBaseEnum::Lev1T(lev1t) => {
        lev1t.transition(state, position, vector, base)
      },
      ParametricDescriptionBaseEnum::Lev2(lev2) => lev2.transition(state, position, vector, base),
      ParametricDescriptionBaseEnum::Lev2T(lev2t) => {
        lev2t.transition(state, position, vector, base)
      },
    }
  }
}
const MASKS: [i64; 63] = [
  0x1,
  0x3,
  0x7,
  0xf,
  0x1f,
  0x3f,
  0x7f,
  0xff,
  0x1ff,
  0x3ff,
  0x7ff,
  0xfff,
  0x1fff,
  0x3fff,
  0x7fff,
  0xffff,
  0x1ffff,
  0x3ffff,
  0x7ffff,
  0xfffff,
  0x1fffff,
  0x3fffff,
  0x7fffff,
  0xffffff,
  0x1ffffff,
  0x3ffffff,
  0x7ffffff,
  0xfffffff,
  0x1fffffff,
  0x3fffffff,
  0x7fffffff,
  0xffffffff,
  0x1ffffffff,
  0x3ffffffff,
  0x7ffffffff,
  0xfffffffff,
  0x1fffffffff,
  0x3fffffffff,
  0x7fffffffff,
  0xffffffffff,
  0x1ffffffffff,
  0x3ffffffffff,
  0x7ffffffffff,
  0xfffffffffff,
  0x1fffffffffff,
  0x3fffffffffff,
  0x7fffffffffff,
  0xffffffffffff,
  0x1ffffffffffff,
  0x3ffffffffffff,
  0x7ffffffffffff,
  0xfffffffffffff,
  0x1fffffffffffff,
  0x3fffffffffffff,
  0x7fffffffffffff,
  0xffffffffffffff,
  0x1ffffffffffffff,
  0x3ffffffffffffff,
  0x7ffffffffffffff,
  0xfffffffffffffff,
  0x1fffffffffffffff,
  0x3fffffffffffffff,
  0x7fffffffffffffff,
];
pub(crate) fn unpack(data: &[i64], index: i32, bits_per_value: i32) -> i32 {
  let bit_loc = bits_per_value as i64 * index as i64;
  let data_loc = (bit_loc >> 6) as usize;
  let bit_start = (bit_loc & 63) as i32;

  if bit_start + bits_per_value <= 64 {
    ((data[data_loc] >> bit_start) & MASKS[(bits_per_value - 1) as usize]) as i32
  } else {
    let part = 64 - bit_start;
    (((data[data_loc] >> bit_start) & MASKS[(part - 1) as usize])
      + ((data[1 + data_loc] & MASKS[(bits_per_value - part - 1) as usize]) << part)) as i32
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::util::automation::character_run_automaton::CharacterRunAutomaton;
  use crate::test::core::util::automaton::automaton_test_util::AutomatonTestUtil;
  use crate::test::core::util::automaton::minimization_operation::MinimizationOperations;
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
}
