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
use std::char::decode_utf16;

use crate::util::automation::automaton::Automaton;
use crate::util::automation::run_automaton::RunAutomaton;
use crate::util::error::lucene_error::Result;

/// Automaton representation for matching char[].
pub struct CharacterRunAutomaton {
    pub base: RunAutomaton,
}

impl CharacterRunAutomaton {
    /// Constructs the automaton. error if the input is not deterministic.
    pub fn new(automaton: Automaton) -> Result<Self> {
        Ok(Self {
            base: RunAutomaton::new(automaton, char::MAX as usize + 1)?,
        })
    }

    /// Returns true if the given string is accepted by this automaton.
    pub fn run_str(&self, s: &str) -> bool {
        let utf16_vec: Vec<u16> = s.encode_utf16().collect();
        let length = utf16_vec.len();
        self.run_chars(utf16_vec.as_slice(), 0, length)
    }

    /// Returns true if the given UTF-16 `char` buffer is accepted.
    pub fn run_chars(&self, chars: &[u16], offset: usize, length: usize) -> bool {
        let mut state: i32 = 0;

        let iter = decode_utf16(chars[offset..offset + length].iter().cloned());

        for result in iter {
            match result {
                Ok(ch) => {
                    state = self.base.step(state, ch as i32);
                    if state == -1 {
                        return false;
                    }
                },
                Err(_) => return false,
            }
        }

        self.base.is_accept(state)
    }
}
