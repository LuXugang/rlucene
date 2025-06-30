/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use crate::util::automation::byte_run_automaton::ByteRunAutomaton;
use crate::util::automation::nfa_run_automaton::NFARunAutomaton;
use crate::util::automation::transition::Transition;

/// Interface accessing the transitions of an automaton.
pub trait TransitionAccessor {
    /// Initialize the provided `Transition` to iterate through all transitions
    /// leaving the specified state. Returns the number of transitions
    /// leaving this state.
    fn init_transition(&self, state: i32, t: &mut Transition) -> i32;

    /// Advance the provided `Transition` to the next transition.
    fn get_next_transition(&self, t: &mut Transition);

    /// How many transitions this state has.
    fn get_num_transitions_with_state(&self, state: i32) -> i32;

    /// Fill the provided `Transition` with the index‑th transition leaving the
    /// specified state.
    fn get_transition(&self, state: i32, index: i32, t: &mut Transition);
}
pub enum TransitionAccessorEnum {
    Byte(ByteRunAutomaton),
    NFA(NFARunAutomaton),
}
impl TransitionAccessor for TransitionAccessorEnum {
    fn init_transition(&self, state: i32, t: &mut Transition) -> i32 {
        match self {
            TransitionAccessorEnum::Byte(byte) => byte.base.automaton.init_transition(state, t),
            TransitionAccessorEnum::NFA(nfa) => nfa.automaton.init_transition(state, t),
        }
    }

    fn get_next_transition(&self, t: &mut Transition) {
        match self {
            TransitionAccessorEnum::Byte(byte) => byte.base.automaton.get_next_transition(t),
            TransitionAccessorEnum::NFA(nfa) => nfa.automaton.get_next_transition(t),
        }
    }

    fn get_num_transitions_with_state(&self, state: i32) -> i32 {
        match self {
            TransitionAccessorEnum::Byte(byte) => {
                byte.base.automaton.get_num_transitions_with_state(state)
            },
            TransitionAccessorEnum::NFA(nfa) => nfa.automaton.get_num_transitions_with_state(state),
        }
    }

    fn get_transition(&self, state: i32, index: i32, t: &mut Transition) {
        match self {
            TransitionAccessorEnum::Byte(byte) => {
                byte.base.automaton.get_transition(state, index, t)
            },
            TransitionAccessorEnum::NFA(nfa) => nfa.automaton.get_transition(state, index, t),
        }
    }
}
