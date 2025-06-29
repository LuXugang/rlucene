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
use std::borrow::Cow;

use crate::util::automation::automaton::Automaton;
use crate::util::automation::byte_runnable::ByteRunnable;
use crate::util::automation::operations::Operations;
use crate::util::automation::run_automaton::RunAutomaton;
use crate::util::automation::utf32_to_utf8::UTF32ToUTF8;
use crate::util::error::lucene_error::Result;

pub struct ByteRunAutomaton {
    pub base: RunAutomaton,
}

impl ByteRunAutomaton {
    /// Converts the incoming automaton to a byte-based one (via UTF-32 to UTF-8
    /// conversion).
    ///
    /// Errors:
    /// - Returns an error if the automaton is not deterministic.
    pub fn new_with_bool(a: Automaton, is_binary: bool) -> Result<Self> {
        let automaton = if is_binary {
            a
        } else {
            match Self::convert(&a)? {
                Cow::Borrowed(_) => a,
                Cow::Owned(o) => o,
            }
        };

        Ok(ByteRunAutomaton {
            base: RunAutomaton::new(automaton, 256)?,
        })
    }
    /// Expert use only: if `is_binary` is `true`, the input is already
    /// byte-based.
    ///
    /// Errors:
    /// - Returns an error if the automaton is not deterministic.
    pub fn new(a: Automaton) -> Result<Self> {
        Self::new_with_bool(a, false)
    }

    fn convert(a: &Automaton) -> Result<Cow<Automaton>> {
        if !a.is_deterministic() {
            panic!("Automaton must be deterministic");
        }
        let converted = UTF32ToUTF8::default().convert(a)?;
        match Operations::determinize(&converted, i32::MAX as usize)? {
            Cow::Borrowed(_) => Ok(converted),
            Cow::Owned(o) => Ok(Cow::Owned(o)),
        }
    }
}
impl ByteRunnable for ByteRunAutomaton {
    fn step(&self, state: i32, c: i32) -> i32 {
        self.base.step(state, c)
    }

    fn is_accept(&self, state: i32) -> bool {
        self.base.is_accept(state)
    }

    fn get_size(&self) -> i32 {
        self.base.size()
    }
}
