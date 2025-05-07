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
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::rc::Rc;

use bit_set::BitSet;

use crate::util::automation::automaton::Automaton;
use crate::util::automation::operations::Operations;
use crate::util::automation::transition::Transition;
use crate::util::automation::transition_accessor::TransitionAccessor;
use crate::util::error::lucene_error::Result;
use crate::util::BitSetExt;

/// Operations for minimizing automata.
pub struct MinimizationOperations;
impl MinimizationOperations {
    /// Minimizes (and determinizes if not already deterministic) the given
    /// automaton using Hopcroft's algorithm.
    ///
    /// Parameters:
    /// - `determinize_work_limit`: Maximum effort to spend determinizing the
    ///   automaton. Set higher to allow more complex queries and lower to
    ///   prevent memory exhaustion. Use
    ///   [`Operations::DEFAULT_DETERMINIZE_WORK_LIMIT`](Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)
    ///   as a decent default if you don't otherwise know what to specify.
    pub(crate) fn minimize(a: &Automaton, determinize_work_limit: usize) -> Result<Cow<Automaton>> {
        if a.get_num_states() == 0 || (!a.is_accept(0) && a.get_num_transitions_with_state(0) == 0)
        {
            return Ok(Cow::Owned(Automaton::new()));
        }
        let a1 = Operations::determinize(a, determinize_work_limit)?;

        if a1.get_num_transitions_with_state(0) == 1 {
            let mut t = Transition::default();
            a.get_transition(0, 0, &mut t);
            if t.dest == 0 && t.min == char::MIN as i32 && t.max == char::MAX as i32 {
                match a1 {
                    Cow::Borrowed(_) => return Ok(Cow::Borrowed(a)),
                    Cow::Owned(o) => return Ok(Cow::Owned(o)),
                }
            }
        }
        let a = Operations::totalize(&a1)?;

        let sigma = a.get_start_points();
        let sigma_len = sigma.len();
        let states_len = a.get_num_states() as usize;

        let mut reverse = vec![vec![vec![]; sigma_len]; states_len];
        let mut partition = vec![HashSet::new(); states_len];
        let mut splitblock = vec![vec![]; states_len];
        let mut block = vec![0; states_len];
        let mut active = vec![vec![StateList::default(); sigma_len]; states_len];
        let mut active2 = vec![vec![None; sigma_len]; states_len];
        let mut pending = VecDeque::new();
        let mut pending2 = BitSet::with_capacity(sigma_len * states_len);
        let mut split = BitSet::with_capacity(states_len);
        let mut refine = BitSet::with_capacity(states_len);
        let mut refine2 = BitSet::with_capacity(states_len);

        let mut transition = Transition::default();
        for q in 0..states_len {
            let j = if a.is_accept(q as i32) { 0 } else { 1 };
            partition[j].insert(q);
            block[q] = j;
            transition.source = q as i32;
            transition.transition_upto = -1;
            for x in 0..sigma_len {
                let next = a.next(&mut transition, sigma[x]);
                let r = &mut reverse[next as usize];
                r[x].push(q);
            }
        }
        // initialize active sets
        for j in 0..=1 {
            for x in 0..sigma_len {
                for &q in &partition[j] {
                    if !reverse[q][x].is_empty() {
                        let state_list = &mut active[j][x];
                        if j == 1 && x == 7 {
                            print!("");
                        }
                        // size == -1 means empty
                        if state_list.size == -1 {
                            *state_list = StateList::new();
                        }
                        active2[q][x] = Some(state_list.add(q as i32));
                        print!("");
                    }
                }
            }
        }

        for x in 0..sigma_len {
            let j = if active[0][x].size <= active[1][x].size {
                0
            } else {
                1
            };
            pending.push_back(IntPair(j, x));
            pending2.insert(x * states_len + j);
        }

        let mut k = 2;
        while let Some(ip) = pending.pop_front() {
            let p = ip.0;
            let x = ip.1;
            pending2.remove(x * states_len + p);
            // find states that need to be split off their blocks
            let mut m = active[p][x].first.clone();
            while let Some(m_rc) = m {
                let r = &reverse[m_rc.borrow().q as usize][x];
                if !r.is_empty() {
                    for &i in r.iter() {
                        if !split.contains(i) {
                            split.insert(i);
                            let j = block[i];
                            splitblock[j].push(i);
                            if !refine2.contains(j) {
                                refine2.insert(j);
                                refine.insert(j);
                            }
                        }
                    }
                }
                m = m_rc.borrow().next.clone();
            }

            let mut k1 = refine.next_set_bit(0);
            while k1 >= 0 {
                let j = k1 as usize;
                let sb = &splitblock[j];
                if sb.len() < partition[j].len() {
                    for &s in sb {
                        partition[j].remove(&s);
                        partition[k].insert(s);
                        block[s] = k;

                        for c in 0..sigma_len {
                            if let Some(sn) = &active2[s][c] {
                                let sl_ptr = sn.borrow().sl;
                                if std::ptr::eq(sl_ptr, &active[j][c]) {
                                    StateListNode::remove(sn);
                                    if active[k][c].size == -1 {
                                        active[k][c] = StateList::new();
                                    }
                                    active2[s][c] = Some(active[k][c].add(s as i32));
                                }
                            }
                        }
                    }

                    for c in 0..sigma_len {
                        let aj = active[j][c].size;
                        let ak = active[k][c].size;
                        let ofs = c * states_len;
                        if !pending2.contains(ofs + j) && aj > 0 && aj <= ak {
                            pending2.insert(ofs + j);
                            pending.push_back(IntPair(j, c));
                        } else {
                            pending2.insert(ofs + k);
                            pending.push_back(IntPair(k, c));
                        }
                    }
                    k += 1;
                }
                refine2.remove(j);
                for &s in sb {
                    split.remove(s);
                }
                splitblock[j].clear();
                k1 = refine.next_set_bit(j + 1);
            }
            refine.clear();
        }

        let mut result = Automaton::new();
        let mut t = Transition::default();

        let mut state_map = vec![0; states_len];
        let mut state_rep = vec![0; k];

        result.create_state();

        for n in 0..k {
            let is_initial = partition[n].contains(&0);
            let new_state = if is_initial { 0 } else { result.create_state() };
            for &q in &partition[n] {
                state_map[q] = new_state;
                result.set_accept(new_state, a.is_accept(q as i32));
                state_rep[new_state as usize] = q as i32;
            }
        }

        for n in 0..k {
            let num_transitions = a.init_transition(state_rep[n] as i32, &mut t);
            for _ in 0..num_transitions {
                a.get_next_transition(&mut t);
                result.add_transition(n as i32, state_map[t.dest as usize], t.min, t.max)?;
            }
        }

        result.finish_state()?;
        match Operations::remove_dead_states(&result)? {
            Cow::Borrowed(_) => Ok(Cow::Owned(result)),
            Cow::Owned(o) => Ok(Cow::Owned(o)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct IntPair(pub usize, pub usize);
#[derive(Debug, Clone)]
pub(crate) struct StateList {
    pub(crate) size: i32,
    pub(crate) first: Option<Rc<RefCell<StateListNode>>>,
    pub(crate) last: Option<Rc<RefCell<StateListNode>>>,
}
// for padding
impl Default for StateList {
    fn default() -> Self {
        Self {
            size: -1,
            first: None,
            last: None,
        }
    }
}
impl StateList {
    pub(crate) fn new() -> Self {
        StateList {
            size: 0,
            first: None,
            last: None,
        }
    }

    /// Add a new node with value `q`, return Rc to the created node
    pub(crate) fn add(&mut self, q: i32) -> Rc<RefCell<StateListNode>> {
        let node = Rc::new(RefCell::new(StateListNode {
            q,
            next: None,
            prev: None,
            sl: self as *mut _,
        }));

        if self.size == 0 {
            self.first = Some(Rc::clone(&node));
            self.last = Some(Rc::clone(&node));
        } else {
            let last = self.last.as_ref().unwrap();
            last.borrow_mut().next = Some(Rc::clone(&node));
            node.borrow_mut().prev = Some(Rc::clone(last));
            self.last = Some(Rc::clone(&node));
        }

        self.size += 1;
        node
    }
}

#[derive(Debug)]
pub(crate) struct StateListNode {
    pub(crate) q: i32,
    // TODO: memory leak risk?
    pub(crate) next: Option<Rc<RefCell<StateListNode>>>,
    pub(crate) prev: Option<Rc<RefCell<StateListNode>>>,
    sl: *mut StateList,
}
impl StateListNode {
    pub fn remove(this_rc: &Rc<RefCell<StateListNode>>) {
        let mut this = this_rc.borrow_mut();

        unsafe {
            let sl = &mut *this.sl;
            sl.size -= 1;
            if let Some(first) = &sl.first {
                if Rc::ptr_eq(first, this_rc) {
                    sl.first = this.next.clone();
                } else if let Some(prev) = &this.prev {
                    prev.borrow_mut().next = this.next.clone();
                }
            }

            if let Some(last) = &sl.last {
                if Rc::ptr_eq(last, this_rc) {
                    sl.last = this.prev.clone();
                } else if let Some(next) = &this.next {
                    next.borrow_mut().prev = this.prev.clone();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::util::automaton::automaton_test_util::AutomatonTestUtil;
    use crate::test::util::automaton::minimization_operation::MinimizationOperations;
    use crate::test::util::lucene_test_case::{at_least, random};
    use crate::util::automation::operations::Operations;
    use crate::util::automation::transition_accessor::TransitionAccessor;
    use crate::util::error::lucene_error::Result;
    #[allow(dead_code)] // for quick search
    /// This test builds some randomish NFA/DFA and minimizes them.
    struct TestMinimize;
    /// the minimal and non-minimal are compared to ensure they are the same.
    #[test]
    fn test_basic() -> Result<()> {
        let mut random = random();
        let num = at_least(&mut random, 200);

        for _ in 0..num {
            let a = AutomatonTestUtil::random_automaton(&mut random)?;
            let v = Operations::remove_dead_states(&a)?;
            let la = Operations::determinize(&v, i32::MAX as usize)?;
            let lb = MinimizationOperations::minimize(&a, i32::MAX as usize)?;
            assert!(AutomatonTestUtil::same_language(&la, &lb)?);
        }

        Ok(())
    }
    ///  compare minimized against minimized with a slower, simple impl. we
    /// check not only that they are  the same, but that
    /// #states/#transitions are the same.
    #[test]
    fn test_against_brzozowski() -> Result<()> {
        let mut random = random();
        let num = at_least(&mut random, 200);

        for _ in 0..num {
            let a = AutomatonTestUtil::random_automaton(&mut random)?;
            let a = AutomatonTestUtil::minimize_simple(&a)?;

            let b = MinimizationOperations::minimize(&a, i32::MAX as usize)?;
            assert!(AutomatonTestUtil::same_language(&a, &b)?);
            assert_eq!(a.get_num_states(), b.get_num_states());

            let num_states = a.get_num_states();
            let sum1: i32 = (0..num_states)
                .map(|s| a.get_num_transitions_with_state(s))
                .sum();
            let sum2: i32 = (0..num_states)
                .map(|s| b.get_num_transitions_with_state(s))
                .sum();

            assert_eq!(sum1, sum2);
        }

        Ok(())
    }
    #[test]
    #[ignore]
    fn test_minimize_huge() -> Result<()> {
        // TODO: RegExp not Implement
        Ok(())
    }
}
