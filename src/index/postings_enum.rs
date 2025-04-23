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
use crate::codecs::lucene101::lucene101_postings_reader::BlockPostingsEnum;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::IndexInput;
use crate::util::error::lucene_error::Result;

/// Iterates through the postings.
/// NOTE: you must first call [`next_doc`](DocIdSetIterator::next_doc) before
/// using any of the per-doc methods.
pub trait PostingsEnum: DocIdSetIterator {
    /// Returns term frequency in the current document, or 1 if the field was
    /// indexed with [`DOCS`](crate::index::index_options::IndexOptions::DOCS)
    /// only.  Do not call this before
    /// [`nextDoc`](DocIdSetIterator::next_doc) is first called, nor after
    /// [`nextDoc`](DocIdSetIterator::next_doc) returns
    /// [`DocIdSetIterator#
    /// NO_MORE_DOCS`](crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS)
    ///
    /// NOTE: if this enum was obtained with `NONE`, the result of this method
    /// is undefined.
    fn freq(&mut self) -> Result<i32>;

    /// Returns the next position, or -1 if positions were not indexed.
    /// Calling this more than [`freq()`](Self::freq) times is undefined.
    fn next_position(&mut self) -> Result<i32>;

    /// Returns start offset for the current position, or -1 if offsets were not
    /// indexed.
    fn start_offset(&self) -> Result<i32>;

    /// Returns end offset for the current position, or -1 if offsets were not
    /// indexed.
    fn end_offset(&self) -> Result<i32>;

    /// Returns the payload at this position, or None if no payload was indexed.
    /// Do not modify the returned bytes.
    fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>>;
}

pub mod postings_enum_util {
    pub const NONE: i16 = 0;
    pub const FREQS: i16 = 1 << 3;
    pub const POSITIONS: i16 = FREQS | 1 << 4;
    pub const OFFSETS: i16 = POSITIONS | 1 << 5;
    pub const PAYLOADS: i16 = POSITIONS | 1 << 6;
    pub const ALL: i16 = OFFSETS | PAYLOADS;

    /// Returns true if the given feature is requested in the flags.
    pub fn feature_requested(flags: i32, feature: i16) -> bool {
        (flags & (feature as i32)) == (feature as i32)
    }
}

pub enum PostingsEnums<I>
where
    I: IndexInput,
{
    Block(BlockPostingsEnum<I>),
}
impl<I> DocIdSetIterator for PostingsEnums<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        match self {
            PostingsEnums::Block(block) => block.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            PostingsEnums::Block(block) => block.next_doc(),
        }
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        match self {
            PostingsEnums::Block(block) => block.advance(_target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            PostingsEnums::Block(block) => block.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            PostingsEnums::Block(block) => block.cost(),
        }
    }
}

impl<I> PostingsEnum for PostingsEnums<I>
where
    I: IndexInput,
{
    fn freq(&mut self) -> Result<i32> {
        match self {
            PostingsEnums::Block(block) => block.freq(),
        }
    }

    fn next_position(&mut self) -> Result<i32> {
        match self {
            PostingsEnums::Block(block) => block.next_position(),
        }
    }

    fn start_offset(&self) -> Result<i32> {
        match self {
            PostingsEnums::Block(block) => block.start_offset(),
        }
    }

    fn end_offset(&self) -> Result<i32> {
        match self {
            PostingsEnums::Block(block) => block.end_offset(),
        }
    }

    fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
        match self {
            PostingsEnums::Block(block) => block.get_payload(),
        }
    }
}
