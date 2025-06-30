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
use std::fmt::{Display, Formatter};

use crate::codecs::block_term_state::BlockTermStateEnum;
use crate::index::base_terms_enum::TermStateImpl1;
use crate::index::dummy::dummy_term_state_type::DummyTermState;
use crate::index::ord_term_state::OrdTermState;
use crate::util::error::lucene_error::Result;

/// Encapsulates all required internal state to position the associated
/// [`TermsEnum`](crate::index::terms_enum::TermsEnum) without re-seeking.
pub trait TermState: Display + Clone {
    /// Copies the content of the given `TermState` to this instance.
    fn copy_from(&mut self, other: &TermStateEnum) -> Result<()>;
}

pub enum TermStateEnum {
    Dummy(DummyTermState),
    Impl1(TermStateImpl1),
    Ord(OrdTermState),
    Block(BlockTermStateEnum),
}

impl Display for TermStateEnum {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Clone for TermStateEnum {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl TermState for TermStateEnum {
    fn copy_from(&mut self, _other: &TermStateEnum) -> Result<()> {
        todo!()
    }
}
