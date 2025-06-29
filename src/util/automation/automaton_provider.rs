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
use crate::util::automation::automaton::Automaton;
use crate::util::error::lucene_error::{LuceneError, Result};
/// Automaton provider for [`RegExp`](crate::util::automation::reg_exp::RegExp)
/// used by [`RegExp::get_automaton`](crate::util::automation::reg_exp::RegExp::get_automaton).
pub trait AutomatonProvider {
    /// Returns the automaton associated with the given name.
    fn get_automaton(&self, name: &str) -> Result<Automaton>;
}

pub struct EmptyAutomatonProvider;
impl AutomatonProvider for EmptyAutomatonProvider {
    fn get_automaton(&self, _name: &str) -> Result<Automaton> {
        Err(LuceneError::illegal_argument(
            "this method should never be called",
        ))
    }
}
