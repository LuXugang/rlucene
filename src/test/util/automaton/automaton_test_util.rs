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

use crate::util::automation::automaton::Automaton;
use crate::util::automation::operations::Operations;
use crate::util::automation::state_pair::StatePair;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;

pub struct AutomatonTestUtil;
impl AutomatonTestUtil {
    /// Returns `true` if these two automata accept exactly the same language.
    /// This is a costly computation!
    ///
    /// Both automata must be determinized and have no dead states.
    pub(crate) fn same_language(a1: &Rc<Automaton>, a2: &Rc<Automaton>) -> Result<bool> {
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
    pub(crate) fn subset_of(a1: &Rc<Automaton>, a2: &Rc<Automaton>) -> Result<bool> {
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
