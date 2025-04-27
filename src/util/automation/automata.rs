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
use crate::index::BytesRef;
use crate::util::automation::automaton::{Automaton, Builder};
use crate::util::automation::operations::Operations;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{StringHelper, ToInt};

pub struct Automata;
impl Automata {
    pub const MAX_STRING_UNION_TERM_LENGTH: i32 = 1000;
    /// Returns a new (deterministic) automaton with the empty language.
    pub fn make_empty() -> Result<Automaton> {
        let mut a = Automaton::new();
        a.finish_state()?;
        Ok(a)
    }
    /// Returns a new (deterministic) automaton that accepts only the empty
    /// string.
    pub fn make_empty_string() -> Result<Automaton> {
        let mut a = Automaton::new();
        a.create_state();
        a.set_accept(0, true);
        Ok(a)
    }
    /// Returns a new (deterministic) automaton that accepts all Unicode
    /// strings.
    pub fn make_any_string() -> Result<Automaton> {
        let mut a = Automaton::new();
        let s = a.create_state();
        a.set_accept(s, true);
        a.add_transition(s, s, char::MIN as i32, char::MAX as i32)?;
        a.finish_state()?;
        Ok(a)
    }

    /// Returns a new (deterministic) automaton that accepts all binary terms
    /// (0..=255).
    pub fn make_any_binary() -> Result<Automaton> {
        let mut a = Automaton::new();
        let s = a.create_state();
        a.set_accept(s, true);
        a.add_transition(s, s, 0, 255)?;
        a.finish_state()?;
        Ok(a)
    }

    /// Returns a new (deterministic) automaton that accepts all binary terms
    /// except the empty string.
    pub fn make_non_empty_binary() -> Result<Automaton> {
        let mut a = Automaton::new();
        let s1 = a.create_state();
        let s2 = a.create_state();
        a.set_accept(s2, true);
        a.add_transition(s1, s2, 0, 255)?;
        a.add_transition(s2, s2, 0, 255)?;
        a.finish_state()?;
        Ok(a)
    }
    /// Returns a new (deterministic) automaton that accepts any single Unicode
    /// codepoint.
    pub fn make_any_char() -> Result<Automaton> {
        Self::make_char_range(char::MIN as i32, char::MAX as i32)
    }

    /// Appends a transition accepting any single Unicode codepoint from the
    /// given state, returning the new state.
    #[allow(unused)]
    pub fn append_any_char(a: &mut Automaton, state: i32) -> Result<i32> {
        let new_state = a.create_state();
        a.add_transition(state, new_state, char::MIN as i32, char::MAX as i32)?;
        Ok(new_state)
    }

    /// Returns a new (deterministic) automaton that accepts a single codepoint
    /// with the given value.
    pub fn make_char(c: i32) -> Result<Automaton> {
        Self::make_char_range(c, c)
    }

    /// Appends a transition accepting a specific codepoint from the given
    /// state, returning the new state.
    #[allow(unused)]
    pub fn append_char(a: &mut Automaton, state: i32, c: i32) -> Result<i32> {
        let new_state = a.create_state();
        a.add_transition(state, new_state, c, c)?;
        Ok(new_state)
    }
    /// Returns a new (deterministic) automaton that accepts a single codepoint
    /// within the given range [min, max].
    pub fn make_char_range(min: i32, max: i32) -> Result<Automaton> {
        if min > max {
            return Self::make_empty();
        }
        let mut a = Automaton::new();
        let s1 = a.create_state();
        let s2 = a.create_state();
        a.set_accept(s2, true);
        a.add_transition(s1, s2, min, max)?;
        a.finish_state()?;
        Ok(a)
    }

    fn any_of_right_length(builder: &mut Builder, x: &str, n: usize) -> Result<i32> {
        let s = builder.create_state();
        if x.len() == n {
            builder.set_accept(s, true);
        } else {
            let next = Self::any_of_right_length(builder, x, n + 1)?;
            builder.add_transition(s, next, '0' as i32, '9' as i32);
        }
        Ok(s)
    }
    fn at_least(
        builder: &mut Builder,
        x: &str,
        n: usize,
        initials: &mut Vec<i32>,
        zeros: bool,
    ) -> Result<i32> {
        let s = builder.create_state();
        if x.len() == n {
            builder.set_accept(s, true);
        } else {
            if zeros {
                initials.push(s);
            }
            let c = x.as_bytes()[n] as char;
            let v = Self::at_least(builder, x, n + 1, initials, zeros && c == '0')?;
            builder.add_transition_label(s, v, c as i32);
            if c < '9' {
                let v = Self::any_of_right_length(builder, x, n + 1)?;
                builder.add_transition(s, v, (c as u8 + 1) as i32, '9' as i32);
            }
        }
        Ok(s)
    }

    fn at_most(builder: &mut Builder, x: &str, n: usize) -> Result<i32> {
        let s = builder.create_state();
        if x.len() == n {
            builder.set_accept(s, true);
        } else {
            let c = x.as_bytes()[n] as char;
            let v = Self::at_most(builder, x, n + 1)?;
            builder.add_transition(s, v, c as i32, c as i32);
            if c > '0' {
                let v = Self::any_of_right_length(builder, x, n + 1)?;
                builder.add_transition(s, v, '0' as i32, (c as u8 - 1) as i32);
            }
        }
        Ok(s)
    }
    pub(crate) fn between(
        builder: &mut Builder,
        x: &str,
        y: &str,
        n: usize,
        initials: &mut Vec<i32>,
        zeros: bool,
    ) -> Result<i32> {
        let s = builder.create_state();
        if x.len() == n {
            builder.set_accept(s, true);
        } else {
            if zeros {
                initials.push(s);
            }
            let cx = x.as_bytes()[n] as char;
            let cy = y.as_bytes()[n] as char;

            if cx == cy {
                let v = Self::between(builder, x, y, n + 1, initials, zeros && cx == '0')?;
                builder.add_transition(s, v, cx as i32, cx as i32);
            } else {
                let v = Self::at_least(builder, x, n + 1, initials, zeros && cx == '0')?;
                builder.add_transition(s, v, cx as i32, cx as i32);
                let v = Self::at_most(builder, y, n + 1)?;
                builder.add_transition(s, v, cy as i32, cy as i32);
                if (cx as u8) + 1 < (cy as u8) {
                    let v = Self::any_of_right_length(builder, x, n + 1)?;
                    builder.add_transition(s, v, (cx as u8 + 1) as i32, (cy as u8 - 1) as i32);
                }
            }
        }
        Ok(s)
    }
    fn suffix_is_zeros(br: &BytesRef<Vec<u8>>, len: usize) -> bool {
        for i in len..br.length {
            if br.bytes[br.offset + i] != 0 {
                return false;
            }
        }
        true
    }
    pub fn make_binary_interval(
        mut min: Option<BytesRef<Vec<u8>>>,
        mut min_inclusive: bool,
        max: Option<BytesRef<Vec<u8>>>,
        max_inclusive: bool,
    ) -> Result<Automaton> {
        if min.is_none() && !min_inclusive {
            return Err(LuceneError::illegal_argument(
                "minInclusive must be true when min is None".to_string(),
            ));
        }

        if max.is_none() && !max_inclusive {
            return Err(LuceneError::illegal_argument(
                "maxInclusive must be true when max is None".to_string(),
            ));
        }

        if min.is_none() {
            min = Some(BytesRef::new());
            min_inclusive = true;
        }
        let min = min.as_ref().unwrap();
        let cmp = if let Some(max_ref) = &max {
            min.cmp(max_ref).to_int()
        } else {
            if min.length == 0 {
                if min_inclusive {
                    return Self::make_any_binary();
                } else {
                    return Self::make_non_empty_binary();
                }
            }
            -1
        };

        if cmp == 0 {
            if !min_inclusive || !max_inclusive {
                return Automata::make_empty();
            } else {
                return Automata::make_binary(min);
            }
        } else if cmp > 0 {
            return Automata::make_empty();
        }

        let max_ref = max.as_ref();

        if let Some(max_ref) = max_ref {
            if StringHelper::starts_with_byte_ref(max_ref, min)
                && Automata::suffix_is_zeros(max_ref, min.length)
            {
                let mut max_length = max_ref.length;

                debug_assert!(max_length > min.length);
                if !max_inclusive {
                    max_length -= 1;
                }

                if max_length == min.length {
                    if !min_inclusive {
                        return Automata::make_empty();
                    } else {
                        return Automata::make_binary(min);
                    }
                }

                let mut a = Automaton::new();
                let mut last_state = a.create_state();
                for i in 0..min.length {
                    let state = a.create_state();
                    let label = min.bytes[i] as i32;
                    a.add_transition_label(last_state, state, label)?;
                    last_state = state;
                }

                if min_inclusive {
                    a.set_accept(last_state, true);
                }

                for _ in min.length..max_length {
                    let state = a.create_state();
                    a.add_transition_label(last_state, state, 0)?;
                    a.set_accept(state, true);
                    last_state = state;
                }

                a.finish_state()?;
                return Ok(a);
            }
        }

        // General case:
        let mut a = Automaton::new();
        let start_state = a.create_state();
        let sink_state = a.create_state();
        a.set_accept(sink_state, true);
        a.add_transition(sink_state, sink_state, 0, 255)?;

        let mut equal_prefix = true;
        let mut last_state = start_state;
        let first_max_state = -1;
        let mut shared_prefix_length = 0;

        for i in 0..min.length {
            let min_label = min.bytes[i] as i32;

            let max_label = if let Some(max_ref) = max_ref {
                if equal_prefix && i < max_ref.length {
                    max_ref.bytes[i] as i32
                } else {
                    -1
                }
            } else {
                -1
            };

            let next_state = if min_inclusive
                && i == min.length - 1
                && (!equal_prefix || min_label != max_label)
            {
                sink_state
            } else {
                a.create_state()
            };

            if equal_prefix {
                if min_label == max_label {
                    a.add_transition_label(last_state, next_state, min_label)?;
                } else if max.is_none() {
                    equal_prefix = false;
                    shared_prefix_length = 0;
                    a.add_transition(last_state, sink_state, min_label + 1, 255)?;
                    a.add_transition_label(last_state, next_state, min_label)?;
                } else {
                    assert!(max_label > min_label);

                    a.add_transition_label(last_state, next_state, min_label)?;
                    if max_label > min_label + 1 {
                        a.add_transition(last_state, sink_state, min_label + 1, max_label - 1)?;
                    }

                    if max_inclusive || i < max_ref.as_ref().unwrap().length - 1 {
                        let first_max_state = a.create_state();
                        if i < max_ref.as_ref().unwrap().length - 1 {
                            a.set_accept(first_max_state, true);
                        }
                        a.add_transition_label(last_state, first_max_state, max_label)?;
                    }
                    equal_prefix = false;
                    shared_prefix_length = i;
                }
            } else {
                a.add_transition_label(last_state, next_state, min_label)?;
                if min_label < 255 {
                    a.add_transition(last_state, sink_state, min_label + 1, 255)?;
                }
            }

            last_state = next_state;
        }

        if !equal_prefix && last_state != sink_state && last_state != start_state {
            a.add_transition(last_state, sink_state, 0, 255)?;
        }

        if min_inclusive {
            a.set_accept(last_state, true);
        }

        if let Some(max_ref) = max_ref {
            if first_max_state == -1 {
                shared_prefix_length = min.length;
            } else {
                last_state = first_max_state;
                shared_prefix_length += 1;
            }

            for i in shared_prefix_length..max_ref.length {
                let max_label = max_ref.bytes[i] as i32;
                if max_label > 0 {
                    a.add_transition(last_state, sink_state, 0, max_label - 1)?;
                }
                if max_inclusive || i < max_ref.length - 1 {
                    let next_state = a.create_state();
                    if i < max_ref.length - 1 {
                        a.set_accept(next_state, true);
                    }
                    a.add_transition_label(last_state, next_state, max_label)?;
                    last_state = next_state;
                }
            }

            if max_inclusive {
                a.set_accept(last_state, true);
            }
        }

        a.finish_state()?;

        debug_assert!(a.is_deterministic());

        Ok(a)
    }
    pub fn make_decimal_interval(min: i32, max: i32, digits: i32) -> Result<Automaton> {
        let mut x = min.to_string();
        let mut y = max.to_string();

        if min > max || (digits > 0 && y.len() as i32 > digits) {
            return Err(LuceneError::illegal_argument(
                "invalid min/max/digits for make_decimal_interval".to_string(),
            ));
        }

        let d = if digits > 0 { digits as usize } else { y.len() };
        let mut bx = String::new();
        for _ in x.len()..d {
            bx.push('0');
        }
        bx.push_str(&x);
        x = bx;

        let mut by = String::new();
        for _ in y.len()..d {
            by.push('0');
        }
        by.push_str(&y);
        y = by;

        let mut builder = Builder::new();

        if digits <= 0 {
            builder.create_state();
        }

        let mut initials = Vec::new();
        Self::between(&mut builder, &x, &y, 0, &mut initials, digits <= 0)?;

        let mut a1 = builder.finish()?;

        if digits <= 0 {
            a1.add_transition_label(0, 0, '0' as i32)?;
            for &p in &initials {
                a1.add_epsilon(0, p)?;
            }
            a1.finish_state()?;
        }

        Ok(a1)
    }
    /// Returns a new (deterministic) automaton that accepts exactly the given
    /// UTF-8 string.
    pub fn make_string(s: &str) -> Result<Automaton> {
        let mut a = Automaton::new();
        let mut last_state = a.create_state();

        let mut i = 0;
        let s_bytes = s.as_bytes();
        while i < s.len() {
            let (cp, size) = match std::str::from_utf8(&s_bytes[i..])
                .ok()
                .and_then(|s| s.chars().next().map(|ch| (ch as i32, ch.len_utf8())))
            {
                Some(t) => t,
                None => {
                    return Err(LuceneError::illegal_argument(
                        "invalid UTF-8 string".to_string(),
                    ));
                },
            };

            let state = a.create_state();
            a.add_transition_label(last_state, state, cp)?;
            last_state = state;
            i += size;
        }

        a.set_accept(last_state, true);
        a.finish_state()?;

        debug_assert!(a.is_deterministic());
        debug_assert!(!Operations::has_dead_states(&a)?);

        Ok(a)
    }

    /// Returns a new (deterministic) automaton that accepts exactly the given
    /// binary term.
    pub fn make_binary(term: &BytesRef<Vec<u8>>) -> Result<Automaton> {
        let mut a = Automaton::new();
        let mut last_state = a.create_state();

        for i in 0..term.length {
            let state = a.create_state();
            let label = term.bytes[term.offset + i] as i32;
            a.add_transition_label(last_state, state, label)?;
            last_state = state;
        }

        a.set_accept(last_state, true);
        a.finish_state()?;

        debug_assert!(a.is_deterministic());
        debug_assert!(!Operations::has_dead_states(&a)?);

        Ok(a)
    }
    /// Returns a new (deterministic) automaton that accepts exactly the given
    /// string, specified as Unicode codepoints.
    pub fn make_string_from_codepoints(
        word: &[i32],
        offset: usize,
        length: usize,
    ) -> Result<Automaton> {
        let mut a = Automaton::new();
        a.create_state();
        let mut s = 0;
        for i in offset..offset + length {
            let s2 = a.create_state();
            a.add_transition_label(s, s2, word[i])?;
            s = s2;
        }
        a.set_accept(s, true);
        a.finish_state()?;
        Ok(a)
    }
}
