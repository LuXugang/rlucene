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
use std::collections::HashMap;

use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::{Automaton, Builder};
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::unicode_util::{UTF8CodePoint, UnicodeUtil};
/// Builds a minimal, deterministic [`Automaton`] that accepts a set of strings
/// using the algorithm described in [Incremental Construction of Minimal Acyclic Finite-State Automata by Daciuk, Mihov, Watson and Watson](https://aclanthology.org/J00-1002.pdf).
///
/// This requires sorted input data, but is very fast (nearly linear with the
/// input size). Also offers the ability to directly build a binary
/// [`Automaton`] representation. Users should access this functionality through
/// [`Automata`] static methods.
///
/// See also:
/// - [`Automata::make_string_union_bytes`](Automaton::make_string_union_bytes)
/// - [`Automata::make_binary_string_union`](Automaton::make_binary_string_union)
/// - [`Automata::make_string_union_iter`](Automaton::make_string_union_iter)
/// - [`Automata::make_binary_string_union_iter`](Automaton::make_binary_string_union_iter)
pub(crate) struct StringsToAutomaton {
  /// A "registry" for state interning.
  pub(crate) state_registry: HashMap<StateKey, usize>,
  /// All automaton states. State references are stored as indexes into this
  /// vector.
  pub(crate) all_states: Vec<State>,
  /// Root automaton state.
  pub(crate) root: usize,
  /// Used for input order checking (only through assertions right now)
  pub(crate) previous: Option<BytesRefBuilder<Vec<u8>>>,
}

impl StringsToAutomaton {
  pub(crate) fn new() -> Self {
    StringsToAutomaton {
      state_registry: HashMap::new(),
      all_states: vec![State::new()],
      root: 0,
      previous: None,
    }
  }
  /// Copies `current` into an internal buffer.
  fn set_previous(&mut self, current: &BytesRef<Vec<u8>>) -> Result<()> {
    match &mut self.previous {
      Some(prev) => {
        prev.copy_bytes_from_ref(current)?;
      },
      None => {
        let mut builder = BytesRefBuilder::new();
        builder.copy_bytes_from_ref(current)?;
        self.previous = Some(builder);
      },
    }
    Ok(())
  }
  fn create_state(&mut self) -> usize {
    self.all_states.push(State::new());
    self.all_states.len() - 1
  }
  /// Internal recursive traversal for conversion.
  fn convert(
    a: &mut Builder,
    state: usize,
    all_states: &[State],
    visited: &mut HashMap<usize, i32>,
  ) -> Result<i32> {
    if let Some(&converted) = visited.get(&state) {
      return Ok(converted);
    }

    let converted = a.create_state();
    let s = &all_states[state];
    a.set_accept(converted, s.is_final);

    visited.insert(state, converted);

    for (i, &target) in s.states.iter().enumerate() {
      let v = Self::convert(a, target, all_states, visited)?;
      a.add_transition_label(converted, v, s.labels[i])?;
    }

    Ok(converted)
  }
  /// Called after adding all terms. Performs final minimization and converts
  /// to a standard [`Automaton`] instance.
  fn complete_and_convert(&mut self) -> Result<Automaton> {
    if self.all_states[self.root].has_children() {
      self.replace_or_register(self.root)?;
    }

    self.state_registry.clear();

    let mut a = Builder::new();
    Self::convert(&mut a, self.root, &self.all_states, &mut HashMap::new())?;
    a.finish()
  }
  /// Builds a minimal, deterministic automaton from a sorted list of
  /// [`BytesRef`] representing strings in UTF-8. These strings must be
  /// binary-sorted. Creates an [`Automaton`] with either UTF-8 codepoints
  /// as transition labels or binary (compiled) transition labels based on
  /// `as_binary`.
  pub(crate) fn build(input: &[BytesRef<Vec<u8>>], as_binary: bool) -> Result<Automaton> {
    let mut builder = StringsToAutomaton::new();

    for b in input {
      builder.add(b, as_binary)?;
    }

    builder.complete_and_convert()
  }
  /// Builds a minimal, deterministic automaton from a sorted list of
  /// [`BytesRef`] representing strings in UTF-8. These strings must be
  /// binary-sorted. Creates an [`Automaton`] with either UTF-8 codepoints
  /// as transition labels or binary (compiled) transition labels based on
  /// `as_binary`.
  pub(crate) fn build_from_iterator<B>(input: &mut B, as_binary: bool) -> Result<Automaton>
  where
    B: BytesRefIterator,
  {
    let mut builder = StringsToAutomaton::new();

    while let Some(b) = input.next()? {
      builder.add(&b, as_binary)?; // b: Cow<'_, BytesRef<Vec<u8>>> ->
      // &BytesRef<Vec<u8>>
    }

    builder.complete_and_convert()
  }

  fn add(&mut self, current: &BytesRef<Vec<u8>>, as_binary: bool) -> Result<()> {
    if current.length > Automata::MAX_STRING_UNION_TERM_LENGTH as usize {
      return Err(LuceneError::illegal_argument(format!(
        "This builder doesn't allow terms that are larger than {} UTF-8 bytes, got {:?}",
        Automata::MAX_STRING_UNION_TERM_LENGTH,
        current
      )));
    }

    if let Some(prev) = &mut self.previous
      && prev.bytes_ref.cmp(current) == std::cmp::Ordering::Greater
    {
      return Err(LuceneError::illegal_argument(format!(
        "Input must be in sorted UTF-8 order: {} >= {}",
        prev.bytes_ref, current
      )));
    }
    self.set_previous(current)?;
    let mut code_point = UTF8CodePoint::default();

    let bytes = &current.bytes;
    let mut pos = current.offset;
    let max = current.offset + current.length;
    let mut state = self.root;
    let mut next;

    if as_binary {
      while pos < max {
        let b = bytes[pos] as i32;
        next = self.all_states[state].last_child_with_label(b);
        if let Some(child) = next {
          state = child;
          pos += 1;
        } else {
          break;
        }
      }
    } else {
      while pos < max {
        code_point = *UnicodeUtil::code_point_at(bytes, pos, &mut code_point)?;
        next = self.all_states[state].last_child_with_label(code_point.code_point);
        if let Some(child) = next {
          state = child;
          pos += code_point.num_bytes;
        } else {
          break;
        }
      }
    }

    if self.all_states[state].has_children() {
      self.replace_or_register(state)?;
    }

    if as_binary {
      while pos < max {
        let b = bytes[pos] as i32;
        let new_state = self.new_state(state, b)?;
        state = new_state;
        pos += 1;
      }
    } else {
      while pos < max {
        code_point = *UnicodeUtil::code_point_at(bytes, pos, &mut code_point)?;
        let new_state = self.new_state(state, code_point.code_point)?;
        state = new_state;
        pos += code_point.num_bytes;
      }
    }

    self.all_states[state].is_final = true;

    Ok(())
  }
  fn new_state(&mut self, state: usize, label: i32) -> Result<usize> {
    let new_state = self.create_state();
    self.all_states[state].new_state(label, new_state)
  }
  /// Replaces the last child of `state` with an already registered state or
  /// registers the last child state into the state registry.
  fn replace_or_register(&mut self, state: usize) -> Result<()> {
    let child = self.all_states[state].last_child();

    if self.all_states[child].has_children() {
      self.replace_or_register(child)?;
    }
    let state_key = StateKey::from(&self.all_states[child]);
    if let Some(&registered) = self.state_registry.get(&state_key) {
      self.all_states[state].replace_last_child(registered);
    } else {
      self.state_registry.insert(state_key, child);
    }
    Ok(())
  }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct State {
  /// Labels of outgoing transitions. Indexed identically to [`states`].
  /// Labels must be sorted lexicographically.
  pub labels: Vec<i32>,
  /// States reachable from outgoing transitions. Indexed identically to
  /// [`labels`].
  pub states: Vec<usize>,
  /// `true` if this state corresponds to the end of at least one input
  /// sequence.
  pub is_final: bool,
}
// for padding

impl State {
  pub(crate) fn new() -> Self {
    State {
      labels: Vec::new(),
      states: Vec::new(),
      is_final: false,
    }
  }
  /// Returns the target state of a transition leaving this state and labeled
  /// with `label`. If no such transition exists, returns `None`.
  pub(crate) fn get_state(&self, label: i32) -> Option<usize> {
    match self.labels.binary_search(&label) {
      Ok(index) => self.states.get(index).copied(),
      Err(_) => None,
    }
  }
  /// Returns `true` if this state has any children (outgoing transitions).
  pub(crate) fn has_children(&self) -> bool {
    !self.labels.is_empty()
  }
  /// Creates a new outgoing transition labeled `label` and returns the newly
  /// created target state for this transition.
  pub(crate) fn new_state(&mut self, label: i32, new_state: usize) -> Result<usize> {
    debug_assert!(
      self.labels.binary_search(&label).is_err(),
      "State already has transition labeled: {label}"
    );
    let mut labels_len = self.labels.len();
    let mut states_len = self.states.len();
    ArrayUtil::grow_exact(&mut self.labels, labels_len + 1)?;
    ArrayUtil::grow_exact(&mut self.states, states_len + 1)?;
    labels_len = self.labels.len();
    states_len = self.states.len();
    self.labels[labels_len - 1] = label;
    self.states[states_len - 1] = new_state;
    Ok(new_state)
  }
  /// Returns the most recent transition's target state.
  pub(crate) fn last_child(&self) -> usize {
    debug_assert!(self.has_children(), "No outgoing transitions.");
    *self.states.last().unwrap()
  }
  /// Returns the associated state if the most recent transition is labeled
  /// with `label`.
  pub(crate) fn last_child_with_label(&self, label: i32) -> Option<usize> {
    let index = self.labels.len();
    let state = if index > 0 && self.labels[index - 1] == label {
      Some(self.states[index - 1])
    } else {
      None
    };
    debug_assert_eq!(state, self.get_state(label));
    state
  }
  /// Replace the last added outgoing transition's target state with the given
  /// state.
  pub(crate) fn replace_last_child(&mut self, state: usize) {
    debug_assert!(self.has_children(), "No outgoing transitions.");
    let len = self.states.len();
    self.states[len - 1] = state;
  }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct StateKey {
  is_final: bool,
  labels: Vec<i32>,
  states: Vec<usize>,
}

impl From<&State> for StateKey {
  fn from(state: &State) -> Self {
    StateKey {
      is_final: state.is_final,
      labels: state.labels.clone(),
      states: state.states.clone(),
    }
  }
}
