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

use crate::util::automation::automata::Automata;
use crate::util::automation::automaton::{Automaton, Builder};
use crate::util::automation::transition::Transition;
use crate::util::automation::transition_accessor::TransitionAccessor;
use crate::util::error::lucene_error::Result;
use crate::util::BitSetExt;

pub struct Operations;
impl Operations {
    pub fn concatenate(a1: Rc<Automaton>, a2: Rc<Automaton>) -> Result<Automaton> {
        Operations::concatenate_with_list(&[a1, a2])
    }

    pub fn concatenate_with_list(list: &[Rc<Automaton>]) -> Result<Automaton> {
        let mut result = Automaton::new();
        // First pass: create all states
        for a in list {
            if a.get_num_states() == 0 {
                result.finish_state()?;
                return Ok(result);
            }
            let num_states = a.get_num_states();
            for _ in 0..num_states {
                result.create_state();
            }
        }

        // Second pass: add transitions, linking accept states of each automaton to the
        // start of the next
        let mut state_offset = 0;
        let mut t = Transition::default();
        for i in 0..list.len() {
            let a = &list[i];
            let num_states = a.get_num_states();
            let next_a = if i + 1 < list.len() {
                Some(&list[i + 1])
            } else {
                None
            };

            for s in 0..num_states {
                let count = a.init_transition(s, &mut t);
                for _ in 0..count {
                    a.get_next_transition(&mut t);
                    result.add_transition(state_offset + s, state_offset + t.dest, t.min, t.max)?;
                }

                if a.is_accept(s) {
                    let mut follow_offset = state_offset;
                    let mut upto = i + 1;
                    let mut follow_a = next_a;

                    loop {
                        if let Some(fa) = follow_a {
                            let num_transitions = fa.init_transition(0, &mut t);
                            for _ in 0..num_transitions {
                                fa.get_next_transition(&mut t);
                                result.add_transition(
                                    state_offset + s,
                                    follow_offset + num_states + t.dest,
                                    t.min,
                                    t.max,
                                )?;
                            }
                            if fa.is_accept(0) {
                                follow_offset += fa.get_num_states();
                                if upto + 1 < list.len() {
                                    follow_a = Some(&list[upto + 1]);
                                } else {
                                    follow_a = None;
                                }
                                upto += 1;
                            } else {
                                break;
                            }
                        } else {
                            result.set_accept((state_offset + s) as usize, true);
                            break;
                        }
                    }
                }
            }

            state_offset += num_states;
        }

        if result.get_num_states() == 0 {
            result.create_state();
        }
        result.finish_state()?;
        Ok(result)
    }
    pub fn optional(mut a: Automaton) -> Result<Automaton> {
        // If the initial state already accepts, return as is
        if a.is_accept(0) {
            return Ok(a);
        }

        // Check for any transition back to the initial state
        let mut has_transitions_to_initial = false;
        let mut t = Transition::default();
        'outer: for s in 0..a.get_num_states() {
            let count = a.init_transition(s, &mut t) as usize;
            for _ in 0..count {
                a.get_next_transition(&mut t);
                if t.dest == 0 {
                    has_transitions_to_initial = true;
                    break 'outer;
                }
            }
        }

        // If no transitions to initial, just mark initial as accept
        if !has_transitions_to_initial {
            let mut result = Automaton::new();
            result.copy(&mut a);
            if result.get_num_states() == 0 {
                result.create_state();
            }
            result.set_accept(0, true);
            return Ok(result);
        }
        let mut result = Automaton::new();
        result.create_state();
        result.set_accept(0, true);
        if a.get_num_states() > 0 {
            result.copy(&mut a);
            result.add_epsilon(0, 1)?;
        }
        result.finish_state()?;
        Ok(result)
    }
    pub fn repeat(a: Rc<Automaton>) -> Result<Rc<Automaton>> {
        if a.get_num_states() == 0 {
            // Repeating the empty automata will still only accept the empty automata.
            return Ok(a);
        }

        // If state 0 is the only accept state, and it already repeats itself
        if a.is_accept(0) && Operations::get_live_states_to_accept(&a)?.len() == 1 {
            return Ok(a);
        }

        let mut builder = Builder::new();
        builder.create_state(); // initial state
        builder.set_accept(0, true);

        let num_states = a.get_num_states();
        let mut state_map = vec![0; num_states as usize];
        for state in 0..num_states {
            if !a.is_accept(state) {
                state_map[state as usize] = builder.create_state();
            } else if a.get_num_transitions_with_state(state) == 0 {
                state_map[state as usize] = 0; // merge into initial state
            } else {
                let new_state = builder.create_state();
                state_map[state as usize] = new_state;
                builder.set_accept(new_state, true);
            }
        }

        // Copy transitions with remapped states
        let mut t = Transition::default();
        for state in 0..a.get_num_states() {
            let src = state_map[state as usize];
            let count = a.init_transition(state, &mut t) as usize;
            for _ in 0..count {
                a.get_next_transition(&mut t);
                let dest = state_map[t.dest as usize];
                builder.add_transition(src, dest, t.min, t.max);
            }
        }

        // Copy initial transitions to new initial state (state 0)
        let count = a.init_transition(0, &mut t) as usize;
        for _ in 0..count {
            a.get_next_transition(&mut t);
            builder.add_transition(0, state_map[t.dest as usize], t.min, t.max);
        }

        // Add transitions from each accept state to repeat the initial transitions
        let accept_set = a.get_accept_states();
        for s in accept_set.iter() {
            if state_map[s] != 0 {
                let count = a.init_transition(0, &mut t) as usize;
                for _ in 0..count {
                    a.get_next_transition(&mut t);
                    builder.add_transition(state_map[s], state_map[t.dest as usize], t.min, t.max);
                }
            }
        }

        Operations::remove_dead_states(Rc::new(builder.finish()?))
    }
    pub fn repeat_count(a: Rc<Automaton>, count: i32) -> Result<Rc<Automaton>> {
        if count == 0 {
            return Operations::repeat(a);
        }

        let mut automata = Vec::with_capacity(count as usize + 1);
        for _ in 0..count {
            automata.push(a.clone());
        }
        automata.push(Operations::repeat(a)?);

        Ok(Rc::new(Operations::concatenate_with_list(&automata)?))
    }
    pub fn repeat_min_max(a: Rc<Automaton>, min: i32, max: i32) -> Result<Rc<Automaton>> {
        if min > max {
            return Ok(Rc::new(Automata::make_empty()?));
        }

        let b: Rc<Automaton> = if min == 0 {
            Rc::new(Automata::make_empty_string()?)
        } else if min == 1 {
            let mut base = Automaton::new();
            base.copy(&a);
            Rc::new(base)
        } else {
            let min = min as usize;
            let mut reps = Vec::with_capacity(min);
            for _ in 0..min {
                reps.push(a.clone());
            }
            Rc::new(Operations::concatenate_with_list(&reps)?)
        };

        let mut prev_accept = Operations::get_set(&b, 0);
        let mut builder = Builder::new();
        builder.copy(&b);

        for _ in min..max {
            let offset = builder.get_num_states();
            builder.copy(&a);
            for s in prev_accept.iter() {
                builder.add_epsilon(*s, offset);
            }
            prev_accept = Operations::get_set(&a, offset);
        }

        Ok(Rc::new(builder.finish()?))
    }
    fn get_set(a: &Automaton, offset: i32) -> HashSet<i32> {
        let mut result = HashSet::new();
        for s in 0..a.get_num_states() {
            if a.is_accept(s) {
                result.insert(offset + s);
            }
        }
        result
    }

    pub fn has_dead_states(a: &Automaton) -> Result<bool> {
        let live_states = Operations::get_live_states(a)?;
        let num_live = live_states.len();
        let num_states = a.get_num_states();
        debug_assert!(
            num_live <= num_states as usize,
            "num_live = {}, num_states = {}, live = {:?}",
            num_live,
            num_states,
            live_states
        );
        Ok(num_live < num_states as usize)
    }

    pub fn get_live_states(a: &Automaton) -> Result<BitSet> {
        let mut live = Operations::get_live_states_from_initial(a);
        live.intersect_with(&Operations::get_live_states_to_accept(a)?);
        Ok(live)
    }
    pub fn get_live_states_from_initial(a: &Automaton) -> BitSet {
        let num_states = a.get_num_states();
        let mut live = BitSet::with_capacity(num_states as usize);
        if num_states == 0 {
            return live;
        }
        let mut work_list = VecDeque::new();
        live.insert(0);
        work_list.push_back(0);

        let mut t = Transition::default();
        while let Some(s) = work_list.pop_front() {
            let count = a.init_transition(s, &mut t) as usize;
            for _ in 0..count {
                a.get_next_transition(&mut t);
                let dest = t.dest as usize;
                if !live.contains(dest) {
                    live.insert(dest);
                    work_list.push_back(dest as i32);
                }
            }
        }
        live
    }
    fn get_live_states_to_accept(a: &Automaton) -> Result<BitSet> {
        let num_states = a.get_num_states();
        // build reversed automaton
        let mut builder = Builder::new();
        for _ in 0..num_states {
            builder.create_state();
        }
        let mut t = Transition::default();
        for s in 0..num_states {
            let count = a.init_transition(s, &mut t) as usize;
            for _ in 0..count {
                a.get_next_transition(&mut t);
                builder.add_transition(t.dest, s, t.min, t.max);
            }
        }
        let a2 = builder.finish()?;

        // collect accept states and traverse backwards
        let mut live = BitSet::with_capacity(num_states as usize);
        let mut work_list = VecDeque::new();
        let accept_bits = a.get_accept_states();
        let mut s = 0;
        while s < num_states {
            s = accept_bits.next_set_bit(s as usize);
            if s == -1 {
                break;
            }
            let su = s as usize;
            live.insert(su);
            work_list.push_back(su);
            s += 1;
        }
        while let Some(s) = work_list.pop_front() {
            let count = a2.init_transition(s as i32, &mut t) as usize;
            for _ in 0..count {
                a2.get_next_transition(&mut t);
                let dest = t.dest as usize;
                if !live.contains(dest) {
                    live.insert(dest);
                    work_list.push_back(dest);
                }
            }
        }
        Ok(live)
    }
    pub fn remove_dead_states(a: Rc<Automaton>) -> Result<Rc<Automaton>> {
        let num_states = a.get_num_states() as usize;
        let live_set = Operations::get_live_states(&a)?;
        if live_set.len() == num_states {
            return Ok(a);
        }

        let mut map = vec![0; num_states];
        let mut result = Automaton::new();

        for i in 0..num_states {
            if live_set.contains(i) {
                let s = result.create_state();
                map[i] = s;
                result.set_accept(s as usize, a.is_accept(i as i32));
            }
        }

        let mut t = Transition::default();
        for i in 0..num_states {
            if live_set.contains(i) {
                let count = a.init_transition(i as i32, &mut t) as usize;
                for _ in 0..count {
                    a.get_next_transition(&mut t);
                    let d = t.dest as usize;
                    if live_set.contains(d) {
                        result.add_transition(map[i], map[d], t.min, t.max)?;
                    }
                }
            }
        }

        result.finish_state()?;
        debug_assert!(!Operations::has_dead_states(&result)?);
        Ok(Rc::new(result))
    }
}
