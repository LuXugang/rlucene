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
use crate::core::internal::hppc::bit_mixer::BitMixer;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::byte_runnable::ByteRunnable;
use crate::core::util::automation::int_set::IntSet;
use crate::core::util::automation::operations::PointTransitionSet;
use crate::core::util::automation::state_set::StateSet;
use crate::core::util::automation::transition::Transition;
use crate::core::util::automation::transition_accessor::TransitionAccessor;
use crate::core::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
/// A [`RunAutomaton`](crate::core::util::automation::run_automaton::RunAutomaton)
/// that does not require a precomputed DFA. It will lazily determinize
/// on-demand, memorizing the DFA states that have been explored.
///
/// **Note:** The current implementation is **not thread-safe**.
///
/// Implemented based on: <https://swtch.com/~rsc/regexp/regexp1.html>
pub struct NFARunAutomaton {
  pub(crate) automaton: Automaton,
  points: Vec<i32>,
  alphabet_size: i32,
  classmap: Vec<usize>, // map from char number to class
  // TODO IMPORTANT 这里需要使用内部可变吗
  // Due to trait `TransitionAccessor`'s method is not mutable,so the methods of NFARunAutomaton
  // needs not mutable too, so using RefCell to make the necessary fields internally mutable.
  dstates: Mutex<Vec<DState>>,
  state: Mutex<State>,
  states_set: Mutex<StateSet>, // reusable
}
struct State {
  dstate_to_ord: HashMap<DStateKey, i32>,
  transition_set: PointTransitionSet, // reusable
}
impl NFARunAutomaton {
  const MISSING: i32 = -1;
  const NOT_COMPUTED: i32 = -2;
  pub fn new(automaton: Automaton) -> Self {
    Self::with_alphabet_size(automaton, 0x10FFFF + 1)
  }
  pub fn with_alphabet_size(automaton: Automaton, alphabet_size: i32) -> Self {
    let points = automaton.get_start_points();
    let classmap_len = std::cmp::min(256, alphabet_size) as usize;
    let mut classmap = vec![0; classmap_len];
    let mut i = 0;
    for (j, class) in classmap.iter_mut().enumerate() {
      if i + 1 < points.len() && j as i32 == points[i + 1] {
        i += 1;
      }
      *class = i;
    }

    let state = State {
      dstate_to_ord: HashMap::new(),
      transition_set: PointTransitionSet::new(),
    };

    let automaton_instance = NFARunAutomaton {
      automaton,
      points,
      alphabet_size,
      classmap,
      dstates: Mutex::new(Vec::with_capacity(10)),
      state: Mutex::new(state),
      states_set: Mutex::new(StateSet::new(5)),
    };

    let initial_state = DState::new(Arc::new(vec![0]), &automaton_instance);
    {
      let mut dstate = automaton_instance.dstates.lock();
      let mut state = automaton_instance.state.lock();
      automaton_instance.find_dstate(Some(initial_state), &mut dstate, &mut state);
    }

    automaton_instance
  }
  /// Runs through a given codepoint array and returns whether it is accepted
  /// by the automaton. This should only be used in tests.
  ///
  /// Parameters:
  /// - `s`: A string represented by an array of Unicode codepoints (`i32`)
  ///
  /// Returns:
  /// - `true` if the input is accepted; `false` otherwise.
  pub(crate) fn run(&self, input: &[i32]) -> bool {
    let mut p = 0;
    for &c in input {
      p = self.step(p, c);
      if p == Self::MISSING {
        return false;
      }
    }

    self.dstates.lock()[p as usize].is_accept
  }
  /// From an existing DFA state, steps to the next DFA state given character
  /// `c`.
  ///
  /// If the transition was previously computed, this operation will use the
  /// cached result; otherwise, it will call [`Self::step_with_index`] to
  /// compute the next state and then cache it.
  fn step_with_dstate_index(&self, dstate_index: usize, c: i32) -> i32 {
    let char_class = self.get_char_class(c);
    self.next_state(char_class, dstate_index)
  }
  /// return the ordinal of given DFA state, generate a new ordinal if the
  /// given DFA state is a new one
  fn find_dstate(
    &self,
    dstate: Option<DState>,
    dstates: &mut Vec<DState>,
    state: &mut State,
  ) -> i32 {
    match dstate {
      Some(dstate) => {
        let dstate_key = DStateKey {
          nfa_states: dstate.nfa_states.clone(),
          hash_code: dstate.hash_code as i32,
        };
        if let Some(&ord) = state.dstate_to_ord.get(&dstate_key) {
          return ord;
        }
        debug_assert!(state.dstate_to_ord.len() <= i32::MAX as usize);
        let ord = state.dstate_to_ord.len();
        state.dstate_to_ord.insert(dstate_key, ord as i32);

        if ord >= dstates.len() {
          ArrayUtil::grow_with_len(dstates, ord + 1);
        }

        dstates[ord] = dstate;

        ord as i32
      },
      None => Self::MISSING,
    }
  }
  /// Gets character class of given codepoint
  pub(crate) fn get_char_class(&self, c: i32) -> usize {
    debug_assert!(c < self.alphabet_size);

    if (c as usize) < self.classmap.len() {
      return self.classmap[c as usize];
    }

    // binary search
    let mut a = 0;
    let mut b = self.points.len();

    while b - a > 1 {
      let d = (a + b) / 2;
      if self.points[d] > c {
        b = d;
      } else if self.points[d] < c {
        a = d;
      } else {
        return d;
      }
    }
    a
  }
  fn set_transition_accordingly(&self, t: &mut Transition) {
    let transition_upto = t.transition_upto as usize;
    let state = &self.dstates.lock()[t.source as usize];
    t.dest = state.transitions[transition_upto];
    t.min = self.points[transition_upto];

    if transition_upto == self.points.len() - 1 {
      t.max = self.alphabet_size - 1;
    } else {
      t.max = self.points[transition_upto + 1] - 1;
    }
  }
  fn next_state(&self, char_class: usize, index: usize) -> i32 {
    let v = {
      let mut dstates = self.dstates.lock();
      let dstate = &mut dstates[index];
      debug_assert!(char_class < dstate.transitions.len());
      dstate.transitions[char_class]
    };
    if v == NFARunAutomaton::NOT_COMPUTED {
      let next_dstate = self.step_with_index(self.points[char_class], index);
      let mut dstates = self.dstates.lock();
      let ord = self.find_dstate(next_dstate, &mut dstates, &mut self.state.lock());
      let dstate = &mut dstates[index];
      dstate.assign_transition(char_class, ord);
      // we could potentially update more than one char classes
      if let Some(minimal_transition) = dstate.minimal_transition.take() {
        let mut cls = char_class;
        while cls > 0 && self.points[cls - 1] >= minimal_transition.min {
          cls -= 1;
          debug_assert!(
            dstate.transitions[cls] == NFARunAutomaton::NOT_COMPUTED
              || dstate.transitions[cls] == dstate.transitions[char_class]
          );
          dstate.assign_transition(cls, dstate.transitions[char_class]);
        }

        let mut cls = char_class;
        {
          while cls + 1 < self.points.len() && self.points[cls + 1] <= minimal_transition.max {
            cls += 1;
            debug_assert!(
              dstate.transitions[cls] == NFARunAutomaton::NOT_COMPUTED
                || dstate.transitions[cls] == dstate.transitions[char_class]
            );
            dstate.assign_transition(cls, dstate.transitions[char_class]);
          }
        }
      }
    }
    self.dstates.lock()[index].transitions[char_class]
  }
  ///  given a list of NFA states and a character c, compute the output list
  /// of NFA state which is wrapped as a DFA state
  fn step_with_index(&self, c: i32, index: usize) -> Option<DState> {
    let mut states_set = self.states_set.lock();
    states_set.reset();

    let dstate = &mut self.dstates.lock()[index];
    let mut left = -1;
    let mut right = self.alphabet_size;
    let step_transition = &mut dstate.step_transition;

    for &nfa_state in dstate.nfa_states.iter() {
      let num_transitions = self.automaton.init_transition(nfa_state, step_transition);

      for _ in 0..num_transitions {
        self.automaton.get_next_transition(step_transition);

        if (step_transition.min..=step_transition.max).contains(&c) {
          states_set.incr(step_transition.dest);
          left = left.max(step_transition.min);
          right = right.min(step_transition.max);
        }

        if step_transition.max < c {
          left = left.max(step_transition.max + 1);
        }

        if step_transition.min > c {
          right = right.min(step_transition.min - 1);
          break; // transitions are sorted
        }
      }
    }

    if states_set.size() == 0 {
      return None;
    }

    dstate.minimal_transition = Some(Transition {
      min: left,
      max: right,
      ..Default::default()
    });

    Some(DState::new(states_set.get_array().clone(), self))
  }
  fn determinize(&self, index: usize) {
    {
      let mut state = self.state.lock();
      let mut dstates = self.dstates.lock();
      let dstate = &mut dstates[index];
      if dstate.computed_transitions == dstate.transitions.len() as i32 {
        return;
      }
      state.transition_set.reset();

      for &nfa_state in dstate.nfa_states.iter() {
        let num_transitions = self
          .automaton
          .init_transition(nfa_state, &mut dstate.step_transition);
        for _ in 0..num_transitions {
          self
            .automaton
            .get_next_transition(&mut dstate.step_transition);
          state.transition_set.add(&dstate.step_transition);
        }
      }

      {
        if state.transition_set.count == 0 {
          dstate.transitions.fill(NFARunAutomaton::MISSING);
          dstate.computed_transitions = dstate.transitions.len() as i32;
          return;
        }
      }
      state.transition_set.sort().expect("should not fail");
    }
    let mut states_set = self.states_set.lock();
    states_set.reset();
    let mut last_point = -1;
    let mut char_class = 0;

    let count = self.state.lock().transition_set.count;
    for i in 0..count {
      let point = self.state.lock().transition_set.points[i].point;

      if states_set.size() > 0 {
        debug_assert_ne!(last_point, -1);

        let v = states_set.get_array().clone();
        let new_dstate = DState::new(v, self);
        let mut dstates = self.dstates.lock();
        let ord = self.find_dstate(Some(new_dstate), &mut dstates, &mut self.state.lock());
        let dstate = &mut dstates[index];

        while self.points[char_class] < last_point {
          dstate.assign_transition(char_class, Self::MISSING);
          char_class += 1;
        }

        debug_assert_eq!(self.points[char_class], last_point);

        while char_class < self.points.len() && self.points[char_class] < point {
          debug_assert!(
            dstate.transitions[char_class] == NFARunAutomaton::NOT_COMPUTED
              || dstate.transitions[char_class] == ord
          );
          dstate.assign_transition(char_class, ord);
          char_class += 1;
        }

        debug_assert!(
          (char_class == self.points.len() && point == self.alphabet_size)
            || self.points[char_class] == point
        );
      }

      // process transitions that end on this point
      // (closes an overlapping interval)
      let mut state = self.state.lock();
      let ends = &mut state.transition_set.points[i].ends;
      let limit = ends.next;
      for j in (0..limit).step_by(3) {
        let dest = ends.transitions[j];
        states_set.decr(dest);
      }
      ends.next = 0;

      // process transitions that start on this point
      // (opens a new interval)
      let starts = &mut state.transition_set.points[i].starts;
      let limit = starts.next;
      for j in (0..limit).step_by(3) {
        let dest = starts.transitions[j];
        states_set.incr(dest);
      }

      last_point = point;
      starts.next = 0;
    }
    let mut dstates = self.dstates.lock();
    let dstate = &mut dstates[index];
    debug_assert_eq!(states_set.size(), 0);
    debug_assert!(dstate.computed_transitions >= char_class as i32);
    // it's also possible that some transitions after the charClass has already
    // been explored
    // no more outgoing transitions, set rest of transition to MISSING
    if char_class < dstate.transitions.len() {
      debug_assert!(
        dstate.transitions[char_class] == NFARunAutomaton::MISSING
          || dstate.transitions[char_class] == NFARunAutomaton::NOT_COMPUTED
      );
    }

    let len = dstate.transitions.len();
    dstate.transitions[char_class..len].fill(NFARunAutomaton::NOT_COMPUTED);

    dstate.computed_transitions = dstate.transitions.len() as i32;
  }
}
impl ByteRunnable for NFARunAutomaton {
  /// For a given state and an incoming character (codepoint), returns the
  /// next state.
  ///
  /// Parameters:
  /// - `state`: The incoming state. It should either be `0` or a state
  ///   previously returned by this function.
  /// - `c`: The Unicode codepoint to transition on
  ///
  /// Returns:
  /// - The next state, or `Self::MISSING` if the transition doesn't exist.
  fn step(&self, state: i32, c: i32) -> i32 {
    debug_assert!(self.dstates.lock().get(state as usize).is_some());
    self.step_with_dstate_index(state as usize, c)
  }

  fn is_accept(&self, state: i32) -> Result<bool> {
    debug_assert!(self.dstates.lock().get(state as usize).is_some());
    Ok(self.dstates.lock()[state as usize].is_accept)
  }

  fn get_size(&self) -> i32 {
    debug_assert!(self.dstates.lock().len() <= i32::MAX as usize);
    self.dstates.lock().len() as i32
  }
}
impl TransitionAccessor for NFARunAutomaton {
  fn init_transition(&self, state: i32, t: &mut Transition) -> i32 {
    t.source = state;
    t.transition_upto = -1;
    self.get_num_transitions_with_state(state)
  }

  fn get_next_transition(&self, t: &mut Transition) {
    debug_assert!(t.transition_upto >= -1 && t.transition_upto < self.points.len() as i32 - 1);
    {
      let transitions = &self.dstates.lock()[t.source as usize].transitions;
      loop {
        // this shouldn't throw AIOOBE as long as this function is only called
        // numTransitions times
        t.transition_upto += 1;
        let idx = t.transition_upto as usize;
        if transitions[idx] != Self::MISSING {
          break;
        }
      }

      debug_assert!(transitions[t.transition_upto as usize] != Self::NOT_COMPUTED);
    }
    self.set_transition_accordingly(t);
  }

  fn get_num_transitions_with_state(&self, state: i32) -> i32 {
    self.determinize(state as usize);
    self.dstates.lock()[state as usize].outgoing_transitions
  }

  fn get_transition(&self, state: i32, index: i32, t: &mut Transition) {
    self.determinize(state as usize);
    {
      let transitions = &self.dstates.lock()[state as usize].transitions;

      let mut outgoing_transitions = -1;
      t.transition_upto = -1;
      t.source = state;

      while outgoing_transitions < index && (t.transition_upto) < self.points.len() as i32 - 1 {
        t.transition_upto += 1;
        let idx = t.transition_upto as usize;
        if transitions[idx] != Self::MISSING {
          outgoing_transitions += 1;
        }
      }

      debug_assert_eq!(outgoing_transitions, index);
    }
    self.set_transition_accordingly(t);
  }
}

struct DState {
  nfa_states: Arc<Vec<i32>>,
  transitions: Vec<i32>,
  hash_code: u32,
  is_accept: bool,
  step_transition: Transition,
  minimal_transition: Option<Transition>,
  computed_transitions: i32,
  outgoing_transitions: i32,
}
// The purpose of implementing Default is to enable use with Vec::default(),
// where the default value acts as a placeholder.
impl Default for DState {
  fn default() -> Self {
    Self {
      nfa_states: Arc::new(vec![]),
      transitions: vec![],
      hash_code: 0,
      is_accept: false,
      step_transition: Transition::default(),
      minimal_transition: None,
      computed_transitions: 0,
      outgoing_transitions: 0,
    }
  }
}
impl DState {
  fn new(nfa_states: Arc<Vec<i32>>, nfa: &NFARunAutomaton) -> Self {
    debug_assert!(!nfa_states.is_empty());

    debug_assert!(nfa_states.len() <= i32::MAX as usize);
    let mut hash_code = nfa_states.len() as u32;
    let mut is_accept = false;

    for s in nfa_states.iter() {
      hash_code += BitMixer::mix_i32(*s);
      if nfa.automaton.is_accept(*s) {
        is_accept = true;
      }
    }
    let transitions = vec![NFARunAutomaton::NOT_COMPUTED; nfa.points.len()];
    DState {
      nfa_states,
      transitions,
      hash_code,
      is_accept,
      step_transition: Transition::default(),
      minimal_transition: None,
      computed_transitions: 0,
      outgoing_transitions: 0,
    }
  }

  fn assign_transition(&mut self, char_class: usize, dest: i32) {
    if self.transitions[char_class] == NFARunAutomaton::NOT_COMPUTED {
      self.computed_transitions += 1;
      self.transitions[char_class] = dest;
      if dest != NFARunAutomaton::MISSING {
        self.outgoing_transitions += 1;
      }
    }
  }

  #[allow(dead_code)]
  fn init_transitions() {
    // no need lazily init'd transitions in Rust Lucene
  }
}
struct DStateKey {
  nfa_states: Arc<Vec<i32>>,
  hash_code: i32,
}
impl PartialEq for DStateKey {
  fn eq(&self, other: &Self) -> bool {
    self.hash_code == other.hash_code && self.nfa_states == other.nfa_states
  }
}
impl Eq for DStateKey {}
impl Hash for DStateKey {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.hash_code.hash(state);
  }
}
