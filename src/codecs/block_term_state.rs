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

use crate::codecs::lucene101::lucene101_postings_format::IntBlockTermState;
use crate::index::ord_term_state::OrdTermState;
use crate::index::term_state::{TermState, TermStateEnum};
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;

/// Holds all state required for
/// [`PostingsReaderBase`](crate::codecs::postings_reader_base::PostingsReaderBase)
/// to produce a [`PostingsEnum`](crate::index::postings_enum::PostingsEnum)
/// without re-seeking the terms dict.
#[derive(Default, Clone)]
pub struct BlockTermState {
    /// how many docs have this term
    pub doc_freq: i32,
    /// total number of occurrences of this term
    pub total_term_freq: i64,
    /// the term's ord in the current block
    pub term_block_ord: i32,
    /// fp into the terms dict primary file (_X.tim) that holds this term
    // TODO: update BTR to nuke this
    pub block_file_pointer: i64,
    ord: OrdTermState,
}

impl Display for BlockTermState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} docFreq={} totalTermFreq={} termBlockOrd={} blockFP={}",
            self.ord,
            self.doc_freq,
            self.total_term_freq,
            self.term_block_ord,
            self.block_file_pointer
        )
    }
}

impl TermState for BlockTermState {
    fn copy_from(&mut self, other: &TermStateEnum) -> Result<()> {
        match other {
            TermStateEnum::Block(other) => match other {
                BlockTermStateEnum::Block(block) => {
                    self.doc_freq = block.doc_freq;
                    self.total_term_freq = block.total_term_freq;
                    self.term_block_ord = block.term_block_ord;
                    self.block_file_pointer = block.block_file_pointer;
                    self.ord = block.ord.clone();
                    Ok(())
                },
                _ => Err(LuceneError::illegal_state(
                    "enum other should be BlockTermState",
                )),
            },
            _ => Err(LuceneError::illegal_state(
                "enum other should be BlockTermState",
            )),
        }
    }
}

#[derive(Clone)]
pub enum BlockTermStateEnum {
    Int(IntBlockTermState),
    Block(BlockTermState),
}
impl BlockTermStateEnum {
    pub fn get_block_term_state(&mut self) -> &mut BlockTermState {
        match self {
            BlockTermStateEnum::Int(int) => &mut int.base,
            BlockTermStateEnum::Block(block) => block,
        }
    }
}

impl Display for BlockTermStateEnum {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl TermState for BlockTermStateEnum {
    fn copy_from(&mut self, other: &TermStateEnum) -> Result<()> {
        match self {
            BlockTermStateEnum::Int(int) => int.copy_from(other),
            BlockTermStateEnum::Block(block) => block.copy_from(other),
        }
    }
}
