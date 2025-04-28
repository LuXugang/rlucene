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
use bit_set::BitSet;

use crate::util::automation::automaton::Automaton;
use crate::util::automation::transition::Transition;
use crate::util::automation::transition_accessor::TransitionAccessor;
use crate::util::ints_ref_builder::IntsRefBuilder;
/// Iterates all accepted strings.
///
/// If the [`Automaton`] has cycles, then this iterator may throw an error,
/// but this is not guaranteed.
///
/// Be aware that the iteration order is implementation dependent and may change across releases.
///
/// If the automaton is not determinized, then it is possible this iterator will return duplicates.
pub struct FiniteStringsIterator {
    /// Automaton to create finite string from.
    a: Automaton,
    /// The state where each path should stop or -1 if only accepted states
    /// should be final.
    end_state: i32,
    /// Tracks which states are in the current path, for cycle detection.
    path_states: BitSet,
    /// Builder for current finite string.
    string: IntsRefBuilder<Vec<i32>>,
    /// Stack to hold our current state in the recursion/iteration.
    nodes: Vec<PathNode>,
    /// Emit empty string?
    emit_empty_string: bool,
}

impl FiniteStringsIterator {
    /// Constructs an iterator for all finite strings of the automaton starting
    /// from 0
    pub fn new(a: Automaton) -> Self {
        Self::new_with_start_end(a, 0, -1)
    }

    /// Constructs an iterator for all finite strings of the automaton starting
    /// from given state
    pub fn new_with_start_end(a: Automaton, start_state: i32, end_state: i32) -> Self {
        let num_states = a.get_num_states();
        let mut nodes = Vec::with_capacity(16);
        for _ in 0..16 {
            nodes.push(PathNode::new());
        }

        let mut path_states = BitSet::with_capacity(num_states as usize);
        let mut string = IntsRefBuilder::new();

        let emit_empty_string = a.is_accept(start_state);

        if num_states > start_state && a.get_num_transitions_with_state(start_state) > 0 {
            path_states.insert(start_state as usize);
            nodes[0].reset_state(&a, start_state);
            string.append(start_state);
        }

        Self {
            a,
            end_state,
            path_states,
            string,
            nodes,
            emit_empty_string,
        }
    }
}

#[derive(Debug)]
pub(crate) struct PathNode {
    /// Which state the path node ends on, whose transitions we are enumerating.
    pub(crate) state: i32,
    /// Which state the current transition leads to.
    pub(crate) to: i32,
    /// Which transition we are on.
    pub(crate) transition: i32,
    /// Which label we are on, in the min-max range of the current Transition
    pub(crate) label: i32,
    t: Transition,
}

impl PathNode {
    pub fn new() -> Self {
        Self {
            state: 0,
            to: 0,
            transition: 0,
            label: 0,
            t: Transition::default(),
        }
    }

    /// Resets this node to start enumerating transitions leaving given state.
    pub fn reset_state(&mut self, a: &Automaton, state: i32) {
        debug_assert!(a.get_num_transitions_with_state(state) != 0);
        self.state = state;
        self.transition = 0;
        a.get_transition(state, 0, &mut self.t);
        self.label = self.t.min;
        self.to = self.t.dest;
    }

    /// Returns next label of current transition, or advances to next
    /// transition. If there are no more transitions, returns -1.
    pub fn next_label(&mut self, a: &Automaton) -> i32 {
        if self.label > self.t.max {
            // We've exhaused the current transition's labels;
            // move to next transitions:
            self.transition += 1;
            if self.transition >= a.get_num_transitions_with_state(self.state) {
                // We're done iterating transitions leaving this state
                self.label = -1;
                return -1;
            }
            a.get_transition(self.state, self.transition, &mut self.t);
            self.label = self.t.min;
            self.to = self.t.dest;
        }
        let ret = self.label;
        self.label += 1;
        ret
    }
}
