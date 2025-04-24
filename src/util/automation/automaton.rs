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
use std::collections::HashSet;

use bit_set::BitSet;
use num_traits::ToPrimitive;

use crate::util::accountable::Accountable;
use crate::util::array_util::ArrayUtil;
use crate::util::automation::transition::Transition;
use crate::util::automation::transition_accessor::TransitionAccessor;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{SliceCopyOps, Sorter};

pub struct Automaton {
    next_state: i32,
    next_transition: i32,
    cur_state: i32,
    states: Vec<i32>,
    is_accept: BitSet,
    transitions: Vec<i32>,
    deterministic: bool,
}

impl Automaton {
    pub fn new() -> Self {
        Self::with_capacity(2, 2)
    }

    pub fn with_capacity(num_states: usize, num_transitions: usize) -> Self {
        Automaton {
            next_state: 0,
            next_transition: 0,
            cur_state: -1,
            states: vec![-1; num_states * 2],
            is_accept: BitSet::with_capacity(num_states),
            transitions: vec![0; num_transitions * 3],
            deterministic: false,
        }
    }
    pub fn create_state(&mut self) -> i32 {
        self.grow_states();
        let state = self.next_state / 2;
        self.states[self.next_state as usize] = -1;
        self.next_state += 2;
        state
    }

    pub fn set_accept(&mut self, state: usize, accept: bool) {
        debug_assert!(
            (0..self.get_num_states() as usize).contains(&state),
            "state {} out of bounds",
            state
        );
        if accept {
            self.is_accept.insert(state);
        } else {
            self.is_accept.remove(state);
        }
    }
    pub fn get_sorted_transitions(&self) -> Vec<Vec<Transition>> {
        let num_states = self.get_num_states();
        let mut result = Vec::with_capacity(num_states as usize);
        for s in 0..num_states {
            let cnt = self.get_num_transitions_with_state(s) as usize;
            let mut row = Vec::with_capacity(cnt);
            for i in 0..cnt {
                let mut t = Transition::default();
                self.get_transition(s, i as i32, &mut t);
                row.push(t);
            }
            result.push(row);
        }
        result
    }

    pub(crate) fn get_accept_states(&mut self) -> &mut BitSet {
        &mut self.is_accept
    }
    pub fn is_accept(&self, state: i32) -> bool {
        self.is_accept.contains(state as usize)
    }

    pub fn add_transition_label(&mut self, source: i32, dest: i32, label: i32) {
        self.add_transition(source, dest, label, label);
    }

    pub fn add_transition(&mut self, source: i32, dest: i32, min: i32, max: i32) -> Result<()> {
        debug_assert!(self.next_transition % 3 == 0);
        let bounds = self.next_state / 2;
        debug_assert!((0..bounds).contains(&source));
        debug_assert!((0..bounds).contains(&dest));

        self.grow_transitions();

        if self.cur_state != source {
            if self.cur_state != -1 {
                self.finish_current_state()?;
            }
            self.cur_state = source;
            let source = source as usize;
            if self.states[2 * source] == -1 {
                return Err(LuceneError::illegal_state(format!(
                    "from state ({}) already had transitions added",
                    source
                )));
            }
            debug_assert!(self.states[2 * source + 1] == 0);
            self.states[2 * source] = self.next_transition;
        }

        let tp = self.next_transition as usize;
        self.transitions[tp] = dest;
        self.transitions[tp + 1] = min;
        self.transitions[tp + 2] = max;
        self.next_transition += 3;
        // Increment transition count for this state
        self.states[2 * source as usize + 1] += 1;
        Ok(())
    }

    pub fn add_epsilon(&mut self, source: i32, dest: i32) {
        let mut t = Transition::default();
        let count = self.init_transition(dest, &mut t);
        for _ in 0..count {
            self.get_next_transition(&mut t);
            self.add_transition(source, t.min, t.max, t.dest);
        }
        if self.is_accept(dest) {
            self.set_accept(source as usize, true);
        }
    }
    pub fn copy_from(&mut self, other: &mut Automaton) {
        // Bulk copy and fix up state pointers
        let state_offset = self.get_num_states();
        let total_states = self.next_state + other.next_state;
        ArrayUtil::grow_with_len(&mut self.states, total_states as usize);
        self.states.copy_from(
            &other.states[0..other.next_state as usize],
            self.next_state as usize,
        );

        let next_state = self.next_state as usize;
        for i in (0..other.next_state as usize).step_by(2) {
            let idx = next_state + i;
            if self.states[idx] != -1 {
                self.states[idx] += self.next_transition;
            }
        }
        self.next_state += other.next_state;

        let other_num_states = other.get_num_states();
        let other_accept_states = other.get_accept_states();
        let mut state = 0;
        while state < other_num_states as usize {
            if let Some(next) = other_accept_states.iter().find(|&i| i >= state) {
                self.set_accept(state_offset as usize + next, true);
                state = next + 1;
            } else {
                break;
            }
        }

        // Bulk copy and fix up transition destinations
        let len = self.next_transition + other.next_transition;
        ArrayUtil::grow_with_len(&mut self.transitions, len as usize);
        self.transitions.copy_from(
            &other.transitions[0..other.next_transition as usize],
            self.next_transition as usize,
        );

        let next_transition = self.next_transition as usize;
        for i in (0..other.next_transition as usize).step_by(3) {
            let idx = next_transition + i;
            self.transitions[idx] += state_offset;
        }
        self.next_transition += other.next_transition;

        if !other.deterministic {
            self.deterministic = false;
        }
    }
    fn finish_current_state(&mut self) -> Result<()> {
        let state = self.cur_state as usize;
        let num_transitions = self.states[2 * state + 1];
        assert!(num_transitions > 0, "no transitions to finish");

        let offset = self.states[2 * state];
        let start = offset / 3;
        // sort by dest, then min, then max
        let mut sort = MinMaxDestSorter {
            transitions: &mut self.transitions,
        };
        sort.sort(start, start + num_transitions)?;

        // merge adjacent transitions
        let mut upto = 0;
        let mut min = -1;
        let mut max = -1;
        let mut dest = -1;

        let offset = offset as usize;
        for i in 0..num_transitions as usize {
            let base = offset + 3 * i;
            let t_dest = self.transitions[base];
            let t_min = self.transitions[base + 1];
            let t_max = self.transitions[base + 2];

            if dest == t_dest {
                if t_min <= max + 1 {
                    if t_max > max {
                        max = t_max;
                    }
                } else {
                    if dest != -1 {
                        self.transitions[offset + 3 * upto] = dest;
                        self.transitions[offset + 3 * upto + 1] = min;
                        self.transitions[offset + 3 * upto + 2] = max;
                        upto += 1;
                    }

                    min = t_min;
                    max = t_max;
                }
            } else {
                if dest != -1 {
                    self.transitions[offset + 3 * upto] = dest;
                    self.transitions[offset + 3 * upto + 1] = min;
                    self.transitions[offset + 3 * upto + 2] = max;
                    upto += 1;
                }
                dest = t_dest;
                min = t_min;
                max = t_max;
            }
        }
        // flush last
        if dest != -1 {
            self.transitions[offset + 3 * upto] = dest;
            self.transitions[offset + 3 * upto + 1] = min;
            self.transitions[offset + 3 * upto + 2] = max;
            upto += 1;
        }

        // adjust counters
        debug_assert!(upto.to_i32().is_some());
        self.next_transition -= (num_transitions - upto as i32) * 3;
        self.states[2 * state + 1] = upto as i32;

        // Sort transitions by min/max/dest:
        let mut sort = MinMaxDestSorter {
            transitions: &mut self.transitions,
        };
        sort.sort(start, start + upto as i32)?;

        // check determinism
        if self.deterministic && upto > 1 {
            let mut last_max = self.transitions[offset + 2];
            for i in 1..upto {
                let next_min = self.transitions[offset + 3 * i + 1];
                if next_min <= last_max {
                    self.deterministic = false;
                    break;
                }
                last_max = self.transitions[offset + 3 * i + 2];
            }
        }
        Ok(())
    }
    pub fn is_deterministic(&self) -> bool {
        self.deterministic
    }
    pub fn finish_state(&mut self) {
        if self.cur_state != -1 {
            self.finish_current_state();
            self.cur_state = -1;
        }
    }

    pub fn get_num_states(&self) -> i32 {
        self.next_state / 2
    }

    pub fn get_num_transitions(&self) -> i32 {
        self.next_transition / 3
    }
    fn grow_states(&mut self) {
        let len = (self.next_state + 2) as usize;
        if len > self.states.len() {
            ArrayUtil::grow_with_len(&mut self.states, len);
        }
    }

    fn grow_transitions(&mut self) {
        let len = (self.next_transition + 3) as usize;
        if len > self.transitions.len() {
            ArrayUtil::grow_with_len(&mut self.transitions, len);
        }
    }

    fn transition_sorted(&self, t: &Transition) -> bool {
        let upto = t.transition_upto;
        // Transition isn't initialized yet (this is the first transition)
        if upto == self.states[2 * t.source as usize] {
            return true;
        }
        let upto = upto as usize;

        let next_dest = self.transitions[upto];
        let next_min = self.transitions[upto + 1];
        let next_max = self.transitions[upto + 2];

        if next_min > t.min {
            true
        } else if next_min < t.min {
            false
        } else if next_max > t.max {
            true
        } else if next_max < t.max {
            false
        } else if next_dest > t.dest {
            true
        } else {
            // We should never see fully equal transitions here:
            false
        }
    }

    pub fn get_start_points(&self) -> Vec<i32> {
        let mut pointset = HashSet::new();
        pointset.insert(0);

        for s in 0..self.next_state as usize {
            let base_idx = s * 2;
            let mut trans = self.states[base_idx] as usize;
            let limit = trans + 3 * (self.states[base_idx + 1] as usize);
            while trans < limit {
                let min = self.transitions[trans + 1];
                let max = self.transitions[trans + 2];
                pointset.insert(min);
                if max < 0x10FFFF {
                    pointset.insert(max + 1);
                }
                trans += 3;
            }
        }

        let mut points: Vec<i32> = pointset.into_iter().collect();
        points.sort();
        points
    }
    pub fn step(&self, state: i32, label: i32) -> i32 {
        self.next_impl(state, 0, label, None)
    }
    pub fn next(&self, transition: &mut Transition, label: i32) -> i32 {
        self.next_impl(
            transition.source,
            transition.transition_upto,
            label,
            Some(transition),
        )
    }
    fn next_impl(
        &self,
        state: i32,
        from_transition_idx: i32,
        label: i32,
        transition: Option<&mut Transition>,
    ) -> i32 {
        debug_assert!(label >= 0);

        let state_index = 2 * state as usize;
        let first_transition = self.states[state_index];
        let num_transitions = self.states[state_index + 1];

        let mut low = from_transition_idx.max(0);
        let mut high = num_transitions - 1;
        // Since transitions are sorted,
        // binary search the transition for which label is within [minLabel, maxLabel].
        while low <= high {
            let mid = ((low + high) as u32 >> 1) as i32;
            let transition_index = (first_transition + 3 * mid) as usize;
            let min_label = self.transitions[transition_index + 1];
            if min_label > label {
                high = mid - 1;
            } else {
                let max_label = self.transitions[transition_index + 2];
                if max_label < label {
                    low = mid + 1;
                } else {
                    let dest = self.transitions[transition_index];
                    if let Some(tr) = transition {
                        tr.dest = dest;
                        tr.min = min_label;
                        tr.max = max_label;
                        tr.transition_upto = mid;
                    }
                    return dest;
                }
            }
        }

        let dest_state = -1;
        if let Some(tr) = transition {
            tr.dest = dest_state;
            tr.transition_upto = low;
        }
        dest_state
    }
}
impl TransitionAccessor for Automaton {
    fn init_transition(&self, state: i32, t: &mut Transition) -> i32 {
        debug_assert!(
            state < self.next_state / 2,
            "state {} next_state {}",
            state,
            self.next_state
        );
        t.source = state;
        t.transition_upto = self.states[2 * state as usize];
        self.get_num_transitions_with_state(state)
    }

    fn get_next_transition(&self, t: &mut Transition) {
        // Make sure there is still a transition left:
        debug_assert!(
            (t.transition_upto + 3 - self.states[2 * t.source as usize])
                <= 3 * self.states[2 * t.source as usize + 1]
        );
        // Make sure transitions are in fact sorted:
        debug_assert!(self.transition_sorted(t));
        let base = t.transition_upto as usize;
        t.dest = self.transitions[base];
        t.min = self.transitions[base + 1];
        t.max = self.transitions[base + 2];
        t.transition_upto += 3;
    }

    fn get_num_transitions_with_state(&self, state: i32) -> i32 {
        debug_assert!(state >= 0);
        debug_assert!(state < self.get_num_states());
        let count = self.states[2 * state as usize + 1];
        if count == -1 {
            0
        } else {
            count
        }
    }

    fn get_transition(&self, state: i32, index: i32, t: &mut Transition) {
        let base = self.states[2 * state as usize] as usize;
        let offset = base + 3 * index as usize;
        t.source = state;
        t.dest = self.transitions[offset];
        t.min = self.transitions[offset + 1];
        t.max = self.transitions[offset + 2];
    }
}
impl Accountable for Automaton {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

pub struct Builder {
    next_state: i32,
    is_accept: BitSet,
    transitions: Vec<i32>,
    next_transition: i32,
}
impl Builder {
    pub fn new() -> Self {
        Self::with_capacity(16, 16)
    }
    pub fn with_capacity(num_states: usize, num_transitions: usize) -> Self {
        let is_accept = BitSet::with_capacity(num_states);
        let transitions = vec![0; num_transitions * 4];
        Builder {
            next_state: 0,
            is_accept,
            transitions,
            next_transition: 0,
        }
    }
    /// Add a new transition with the specified source, dest, min, max.
    pub fn add_transition(&mut self, source: i32, dest: i32, min: i32, max: i32) {
        let new_len = (self.next_transition + 4) as usize;
        if self.transitions.len() < new_len {
            ArrayUtil::grow_with_len(&mut self.transitions, new_len);
        }
        let mut next_transition = self.next_transition as usize;
        self.transitions[next_transition] = source;
        next_transition += 1;
        self.transitions[next_transition] = dest;
        next_transition += 1;
        self.transitions[next_transition] = min;
        next_transition += 1;
        self.transitions[next_transition] = max;
        next_transition += 1;
        self.next_transition = next_transition as i32;
    }
    /// Add a `virtual` epsilon transition between source and dest. Dest state
    /// must already have all transitions added because this method simply
    /// copies those same transitions over to source.
    pub fn add_epsilon(&mut self, source: i32, dest: i32) {
        let mut upto = 0;
        while upto < self.next_transition as usize {
            if self.transitions[upto] == dest {
                self.add_transition(
                    source,
                    self.transitions[upto + 1],
                    self.transitions[upto + 2],
                    self.transitions[upto + 3],
                );
            }
            upto += 4;
        }
        if self.is_accept(dest as usize) {
            self.set_accept(source, true);
        }
    }
    pub fn finish(&mut self) -> Result<Automaton> {
        let num_states = self.next_state;
        let num_transitions = self.transitions.len() / 4;
        let mut a = Automaton::with_capacity(num_states as usize, num_transitions);

        for state in 0..num_states as usize {
            a.create_state();
            a.set_accept(state, self.is_accept(state));
        }
        let mut sort = InPlaceMergeSorterImpl {
            transitions: &mut self.transitions,
        };
        debug_assert!(num_transitions.to_i32().is_some());
        sort.sort(0, num_transitions as i32)?;
        let mut upto = 0;
        while upto < self.next_transition as usize {
            a.add_transition(
                self.transitions[upto],
                self.transitions[upto + 1],
                self.transitions[upto + 2],
                self.transitions[upto + 3],
            )?;
            upto += 4;
        }

        a.finish_state();
        Ok(a)
    }
    /// Create a new state
    pub fn create_state(&mut self) -> i32 {
        let s = self.next_state;
        self.next_state += 1;
        s
    }

    /// Set or clear this state as an accept state.
    pub fn set_accept(&mut self, state: i32, accept: bool) {
        debug_assert!(
            (0..self.next_state).contains(&state),
            "state {} out of bounds",
            state
        );
        if accept {
            self.is_accept.insert(state as usize);
        } else {
            self.is_accept.remove(state as usize);
        }
    }

    /// Returns true if this state is an accept state.
    pub fn is_accept(&self, state: usize) -> bool {
        self.is_accept.contains(state)
    }

    /// How many states this automaton has.
    pub fn get_num_states(&self) -> i32 {
        self.next_state
    }

    /// Copies over all states/transitions from other.
    pub fn copy(&mut self, other: &Automaton) {
        let offset = self.get_num_states();
        let other_num_states = other.get_num_states();

        // Copy all states
        self.copy_states(other);

        // Copy all transitions
        let mut t = Transition::default();
        for s in 0..other_num_states {
            let count = other.init_transition(s, &mut t);
            for _ in 0..count {
                other.get_next_transition(&mut t);
                self.add_transition(offset + s, offset + t.dest, t.min, t.max);
            }
        }
    }

    /// Copies over all states from other.
    pub fn copy_states(&mut self, other: &Automaton) {
        let other_num_states = other.get_num_states();
        for s in 0..other_num_states {
            let new_state = self.create_state();
            let is_accept = other.is_accept(s);
            self.set_accept(new_state, is_accept);
        }
    }
}

pub struct InPlaceMergeSorterImpl<'a> {
    transitions: &'a mut [i32],
}

impl<'a> InPlaceMergeSorterImpl<'a> {
    fn swap_one(&mut self, i: usize, j: usize) {
        self.transitions.swap(i, j);
    }
}
impl Sorter for InPlaceMergeSorterImpl<'_> {
    fn compare(&mut self, i: i32, j: i32) -> Result<i32> {
        let i_start = i as usize * 4;
        let j_start = j as usize * 4;

        // First src
        let i_src = self.transitions[i_start];
        let j_src = self.transitions[j_start];
        if i_src < j_src {
            return Ok(-1);
        }
        if i_src > j_src {
            return Ok(1);
        }

        // Then min
        let i_min = self.transitions[i_start + 2];
        let j_min = self.transitions[j_start + 2];
        if i_min < j_min {
            return Ok(-1);
        }
        if i_min > j_min {
            return Ok(1);
        }

        // Then max
        let i_max = self.transitions[i_start + 3];
        let j_max = self.transitions[j_start + 3];
        if i_max < j_max {
            return Ok(-1);
        }
        if i_max > j_max {
            return Ok(1);
        }

        // Finally dest
        let i_dest = self.transitions[i_start + 1];
        let j_dest = self.transitions[j_start + 1];
        if i_dest < j_dest {
            Ok(-1)
        } else if i_dest > j_dest {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        let i_start = i as usize * 4;
        let j_start = j as usize * 4;
        self.swap_one(i_start, j_start);
        self.swap_one(i_start + 1, j_start + 1);
        self.swap_one(i_start + 2, j_start + 2);
        self.swap_one(i_start + 3, j_start + 3);
        Ok(())
    }
}
pub struct MinMaxDestSorter<'a> {
    transitions: &'a mut [i32],
}
impl<'a> MinMaxDestSorter<'a> {
    fn swap_one(&mut self, i: usize, j: usize) -> Result<()> {
        self.transitions.swap(i, j);
        Ok(())
    }
}
impl Sorter for MinMaxDestSorter<'_> {
    fn compare(&mut self, i: i32, j: i32) -> Result<i32> {
        let i_start = 3 * i as usize;
        let j_start = 3 * j as usize;

        // First compare min
        let i_min = self.transitions[i_start + 1];
        let j_min = self.transitions[j_start + 1];
        if i_min < j_min {
            return Ok(-1);
        } else if i_min > j_min {
            return Ok(1);
        }

        // Then compare max
        let i_max = self.transitions[i_start + 2];
        let j_max = self.transitions[j_start + 2];
        if i_max < j_max {
            return Ok(-1);
        } else if i_max > j_max {
            return Ok(1);
        }

        // Finally compare dest
        let i_dest = self.transitions[i_start];
        let j_dest = self.transitions[j_start];
        if i_dest < j_dest {
            Ok(-1)
        } else if i_dest > j_dest {
            return Ok(1);
        } else {
            return Ok(0);
        }
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        let i_start = 3 * i as usize;
        let j_start = 3 * j as usize;
        self.swap_one(i_start, j_start)?;
        self.swap_one(i_start + 1, j_start + 1)?;
        self.swap_one(i_start + 2, j_start + 2)
    }
}
pub struct DestMinMaxSorter<'a> {
    transitions: &'a mut [i32],
}
impl<'a> DestMinMaxSorter<'a> {
    fn swap_one(&mut self, i: usize, j: usize) -> Result<()> {
        self.transitions.swap(i, j);
        Ok(())
    }
}
impl Sorter for DestMinMaxSorter<'_> {
    fn compare(&mut self, i: i32, j: i32) -> Result<i32> {
        let i_start = (3 * i) as usize;
        let j_start = (3 * j) as usize;

        // First dest:
        let i_dest = self.transitions[i_start];
        let j_dest = self.transitions[j_start];
        if i_dest < j_dest {
            return Ok(-1);
        } else if i_dest > j_dest {
            return Ok(1);
        }

        // Then min:
        let i_min = self.transitions[i_start + 1];
        let j_min = self.transitions[j_start + 1];
        if i_min < j_min {
            return Ok(-1);
        } else if i_min > j_min {
            return Ok(1);
        }

        // Then max:
        let i_max = self.transitions[i_start + 2];
        let j_max = self.transitions[j_start + 2];
        if i_max < j_max {
            return Ok(-1);
        } else if i_max > j_max {
            return Ok(1);
        }
        Ok(0)
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        let i_start = (3 * i) as usize;
        let j_start = (3 * j) as usize;
        self.swap_one(i_start, j_start)?;
        self.swap_one(i_start + 1, j_start + 1)?;
        self.swap_one(i_start + 2, j_start + 2)
    }
}
