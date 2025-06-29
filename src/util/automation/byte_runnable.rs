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

/// A runnable automaton accepting byte array as input
pub trait ByteRunnable {
    /// Returns the state obtained by reading the given byte from the given
    /// state.
    ///
    /// Returns -1 if not obtaining any such state.
    ///
    /// # Parameters
    /// - `state`: the last state
    /// - `c`: the input codepoint
    ///
    /// # Returns
    /// The next state, or -1 if no such transition.
    fn step(&self, state: i32, c: i32) -> i32;

    /// Returns acceptance status for given state.
    ///
    /// # Parameters
    /// - `state`: the state
    ///
    /// # Returns
    /// Whether the state is accepted.
    fn is_accept(&self, state: i32) -> bool;

    /// Returns number of states this automaton has.
    ///
    /// Note: This may not be an accurate number in case of an NFA.
    ///
    /// # Returns
    /// Number of states.
    fn get_size(&self) -> i32;

    /// Returns true if the given byte array is accepted by this automaton.
    ///
    /// # Parameters
    /// - `s`: input byte slice
    /// - `offset`: start index
    /// - `length`: number of bytes to read
    ///
    /// # Returns
    /// Whether the automaton accepts the input.
    fn run(&self, s: &[u8], offset: usize, length: usize) -> bool {
        let mut p = 0;
        let end = offset + length;
        for &b in &s[offset..end] {
            p = self.step(p, b as i32);
            if p == -1 {
                return false;
            }
        }
        self.is_accept(p)
    }
}

pub enum ByteRunnableEnum {
    Byte(ByteRunAutomaton),
    NFA(NFARunAutomaton),
}
impl ByteRunnable for ByteRunnableEnum {
    fn step(&self, state: i32, c: i32) -> i32 {
        match self {
            ByteRunnableEnum::Byte(bra) => bra.step(state, c),
            ByteRunnableEnum::NFA(nfa) => nfa.step(state, c),
        }
    }

    fn is_accept(&self, state: i32) -> bool {
        match self {
            ByteRunnableEnum::Byte(bra) => bra.is_accept(state),
            ByteRunnableEnum::NFA(nfa) => nfa.is_accept(state),
        }
    }

    fn get_size(&self) -> i32 {
        match self {
            ByteRunnableEnum::Byte(bra) => bra.get_size(),
            ByteRunnableEnum::NFA(nfa) => nfa.get_size(),
        }
    }

    fn run(&self, s: &[u8], offset: usize, length: usize) -> bool {
        match self {
            ByteRunnableEnum::Byte(bra) => bra.run(s, offset, length),
            ByteRunnableEnum::NFA(nfa) => nfa.run(s, offset, length),
        }
    }
}
