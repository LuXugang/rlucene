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
use std::collections::{HashSet, VecDeque};
use std::rc::Rc;

use bit_set::BitSet;

use crate::util::automation::automaton::Automaton;
use crate::util::automation::operations::Operations;
use crate::util::automation::state_pair::StatePair;
use crate::util::automation::transition::Transition;
use crate::util::automation::transition_accessor::TransitionAccessor;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use crate::util::ints_ref::IntsRef;
use crate::util::ints_ref_builder::IntsRefBuilder;

pub struct AutomatonTestUtil;
impl AutomatonTestUtil {
    /// Default maximum number of states that {@link Operations#determinize}
    /// should create.
    pub const DEFAULT_MAX_DETERMINIZED_STATES: usize = 1000000;
    ///  Maximum level of recursion allowed in recursive operations.
    pub const MAX_RECURSION_LEVEL: usize = 1000;
    /// Simple, original implementation of `get_finite_strings`.
    ///
    /// Returns the set of accepted strings, assuming that at most `limit`
    /// strings are accepted. If more than `limit` strings are accepted, the
    /// first `limit` strings found are returned. If `limit < 0`, then the
    /// limit is considered infinite.
    ///
    /// This implementation is recursive: it uses one stack frame for each
    /// character in the returned strings (i.e., the maximum is the maximum
    /// length of the returned strings).
    pub fn get_finite_strings_recursive(a: &Automaton, limit: i32) -> HashSet<IntsRef<Vec<i32>>> {
        let mut strings = HashSet::new();
        let mut path_states = HashSet::new();
        let mut path = IntsRefBuilder::new();

        if !Self::get_finite_strings(a, 0, &mut path_states, &mut strings, &mut path, limit) {
            return strings;
        }
        strings
    }
    /// Returns the strings that can be produced from the given state,
    /// or `false` if more than `limit` strings are found.
    ///
    /// A `limit` less than `0` means "infinite".
    fn get_finite_strings(
        a: &Automaton,
        s: i32,
        path_states: &mut HashSet<i32>,
        strings: &mut HashSet<IntsRef<Vec<i32>>>,
        path: &mut IntsRefBuilder<Vec<i32>>,
        limit: i32,
    ) -> bool {
        path_states.insert(s);

        let mut t = Transition::default();
        let count = a.init_transition(s, &mut t);

        for _ in 0..count {
            a.get_next_transition(&mut t);
            if path_states.contains(&t.dest) {
                return false;
            }
            for label in t.min..=t.max {
                path.append(label);

                if a.is_accept(t.dest) {
                    strings.insert(path.to_ints_ref());
                    if limit >= 0 && strings.len() > limit as usize {
                        return false;
                    }
                }

                if !Self::get_finite_strings(a, t.dest, path_states, strings, path, limit) {
                    return false;
                }
                path.set_length(path.length() - 1);
            }
        }

        path_states.remove(&s);
        true
    }
    /// Returns `true` if the language of this automaton is finite.
    /// The automaton must not have any dead states.
    pub(crate) fn is_finite(a: &Automaton) -> Result<bool> {
        if a.get_num_states() == 0 {
            return Ok(true);
        }
        let mut scratch = Transition::default();
        let mut path = BitSet::with_capacity(a.get_num_states() as usize);
        let mut visited = BitSet::with_capacity(a.get_num_states() as usize);
        Self::is_finite_inner(&mut scratch, a, 0, &mut path, &mut visited, 0)
    }

    /// Checks whether there is a loop containing the given state.
    /// (This is sufficient since there are never transitions to dead states.)
    pub(crate) fn is_finite_inner(
        scratch: &mut Transition,
        a: &Automaton,
        state: i32,
        path: &mut BitSet,
        visited: &mut BitSet,
        level: usize,
    ) -> Result<bool> {
        if level > Self::MAX_RECURSION_LEVEL {
            return Err(LuceneError::illegal_argument(format!(
                "input automaton is too large: level={}",
                level
            )));
        }

        path.insert(state as usize);
        let num_transitions = a.init_transition(state, scratch);
        let state = state as usize;

        for _ in 0..num_transitions {
            a.get_next_transition(scratch);
            let dest = scratch.dest;
            if path.contains(dest as usize)
                || (!visited.contains(dest as usize)
                    && !Self::is_finite_inner(scratch, a, dest, path, visited, level + 1)?)
            {
                return Ok(false);
            }
        }

        path.remove(state);
        visited.insert(state);
        Ok(true)
    }

    /// Returns `true` if these two automata accept exactly the same language.
    /// This is a costly computation!
    ///
    /// Both automata must be determinized and have no dead states.
    pub(crate) fn same_language(a1: &Automaton, a2: &Automaton) -> Result<bool> {
        if std::ptr::eq(a1, a2) {
            return Ok(true);
        }
        Ok(AutomatonTestUtil::subset_of(a2, a1)? && AutomatonTestUtil::subset_of(a1, a2)?)
    }

    /// Returns `true` if the language of `a1` is a subset of the language of
    /// `a2`. Both automata must be determinized and must have no dead
    /// states.
    ///
    /// Complexity: quadratic in the number of states.
    pub(crate) fn subset_of(a1: &Automaton, a2: &Automaton) -> Result<bool> {
        if !a1.is_deterministic() {
            return Err(LuceneError::illegal_argument(
                "a1 must be deterministic".to_string(),
            ));
        }
        if !a2.is_deterministic() {
            return Err(LuceneError::illegal_argument(
                "a2 must be deterministic".to_string(),
            ));
        }
        debug_assert!(!Operations::has_dead_states_from_initial(a1)?);
        debug_assert!(!Operations::has_dead_states_from_initial(a2)?);

        if a1.get_num_states() == 0 {
            return Ok(true);
        } else if a2.get_num_states() == 0 {
            return Ok(Operations::is_empty(a1));
        }

        let transitions1 = a1.get_sorted_transitions();
        let transitions2 = a2.get_sorted_transitions();

        let mut worklist = VecDeque::new();
        let mut visited = HashSet::new();

        let p = Rc::new(StatePair::new(0, 0));
        worklist.push_back(p.clone());
        visited.insert(p);

        while let Some(p) = worklist.pop_front() {
            if a1.is_accept(p.s1) && !a2.is_accept(p.s2) {
                return Ok(false);
            }

            let t1 = &transitions1[p.s1 as usize];
            let t2 = &transitions2[p.s2 as usize];

            let mut b2 = 0;
            for n1 in 0..t1.len() {
                while b2 < t2.len() && t2[b2].max < t1[n1].min {
                    b2 += 1;
                }

                let mut min1 = t1[n1].min;
                let mut max1 = t1[n1].max;

                for n2 in b2..t2.len() {
                    if t1[n1].max < t2[n2].min {
                        break;
                    }
                    if t2[n2].min > min1 {
                        return Ok(false);
                    }

                    if t2[n2].max < char::MAX as i32 {
                        min1 = t2[n2].max + 1;
                    } else {
                        min1 = char::MAX as i32;
                        max1 = char::MIN as i32;
                    }

                    let q = Rc::new(StatePair::new(t1[n1].dest, t2[n2].dest));
                    if visited.insert(q.clone()) {
                        worklist.push_back(q);
                    }
                }

                if min1 <= max1 {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }
}
