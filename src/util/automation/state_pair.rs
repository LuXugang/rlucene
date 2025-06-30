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
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
/// Pair of states
pub(crate) struct StatePair {
    pub(crate) s1: i32,
    pub(crate) s2: i32,
    // only mike knows what it does (do not expose)
    pub(crate) s: i32,
}

impl StatePair {
    pub(crate) fn new_with_s(s: i32, s1: i32, s2: i32) -> Self {
        StatePair { s1, s2, s }
    }

    /// Constructs a new state pair.
    pub(crate) fn new(s1: i32, s2: i32) -> Self {
        StatePair { s1, s2, s: -1 }
    }
}

impl PartialEq for StatePair {
    fn eq(&self, other: &Self) -> bool {
        self.s1 == other.s1 && self.s2 == other.s2
    }
}

impl Eq for StatePair {}

impl Hash for StatePair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.s1.hash(state);
        self.s2.hash(state);
    }
}

impl fmt::Display for StatePair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StatePair(s1={} s2={})", self.s1, self.s2)
    }
}
