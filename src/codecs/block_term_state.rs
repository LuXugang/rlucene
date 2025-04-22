/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate::codecs::lucene101::lucene101_postings_format::IntBlockTermState;
use crate::index::ord_term_state::OrdTermState;
use crate::index::term_state::{TermState, TermStateEnum};
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

/// Holds all state required for [`PostingsReaderBase`](crate::codecs::postings_reader_base::PostingsReaderBase) to produce a
/// [`PostingsEnum`](crate::index::postings_enum::PostingsEnum) without re-seeking the terms dict.
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
            TermStateEnum::Block(other) => {
                self.doc_freq = other.doc_freq;
                self.total_term_freq = other.total_term_freq;
                self.term_block_ord = other.term_block_ord;
                self.block_file_pointer = other.block_file_pointer;
                self.ord = other.ord.clone();
                Ok(())
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
}
impl BlockTermStateEnum {
    pub fn get_block_term_state(&mut self) -> &mut BlockTermState {
        match self {
            BlockTermStateEnum::Int(int) => &mut int.base,
        }
    }
}
