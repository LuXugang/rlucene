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
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::Result;

/// Holds one transition from an automaton. This is typically used temporarily
/// when iterating through transitions via
/// [`TransitionAccessor::init_transition`](crate::util::automation::transition_accessor::TransitionAccessor::init_transition)
/// and [`TransitionAccessor::get_next_transition`](crate::util::automation::transition_accessor::TransitionAccessor::get_next_transition).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Transition {
    /// Source state.
    pub source: i32,
    /// Destination state.
    pub dest: i32,
    /// Minimum accepted label (inclusive).
    pub min: i32,
    /// Maximum accepted label (inclusive).
    pub max: i32,
    /// Remembers where we are in the iteration; initialized to -1 to provoke
    /// an error if `get_next_transition` is called without first
    /// `init_transition`.
    pub transition_upto: i32,
}
/// Static estimation of bytes used by a `Transition` instance.
// TODO: memory calculation not implemented
pub const BYTES_USED: usize = std::mem::size_of::<Transition>();

impl Default for Transition {
    /// Creates a `Transition` with zeroed fields and `transition_upto` set to
    /// -1.
    fn default() -> Self {
        Transition {
            source: 0,
            dest: 0,
            min: 0,
            max: 0,
            transition_upto: -1,
        }
    }
}

impl Accountable for Transition {
    fn ram_bytes_used(&self) -> Result<i64> {
        Ok(BYTES_USED as i64)
    }
}

impl std::fmt::Display for Transition {
    /// Formats the transition as `source --> dest minChar-maxChar`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} --> {} {}-{}",
            self.source, self.dest, self.min as u8 as char, self.max as u8 as char
        )
    }
}
