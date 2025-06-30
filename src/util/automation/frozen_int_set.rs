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
use std::rc::Rc;

use crate::util::automation::int_set::IntSet;

#[derive(Eq)]
pub(crate) struct FrozenIntSet {
    pub(crate) values: Rc<Vec<i32>>,
    pub(crate) state: i32,
    pub(crate) hash_code: i64,
}

impl FrozenIntSet {
    pub(crate) fn new(values: Rc<Vec<i32>>, hash_code: i64, state: i32) -> Self {
        FrozenIntSet {
            values,
            hash_code,
            state,
        }
    }
}

impl PartialEq for FrozenIntSet {
    fn eq(&self, other: &Self) -> bool {
        self.hash_code == other.hash_code && *self.values == *other.values
    }
}
impl IntSet for FrozenIntSet {
    fn get_array(&mut self) -> &Rc<Vec<i32>> {
        &self.values
    }

    fn size(&self) -> usize {
        self.values.len()
    }

    fn long_hash_code(&mut self) -> i64 {
        self.hash_code
    }
}

impl fmt::Display for FrozenIntSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.values)
    }
}
impl Hash for FrozenIntSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_code.hash(state);
    }
}
