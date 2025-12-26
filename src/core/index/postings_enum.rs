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
use crate::core::index::BytesRef;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;

/// Iterates through the postings.
/// NOTE: you must first call [`next_doc`](DocIdSetIterator::next_doc) before
/// using any of the per-doc methods.
pub trait PostingsEnum: DocIdSetIterator {
    /// Returns term frequency in the current document, or 1 if the field was
    /// indexed with [`DOCS`](crate::core::index::index_options::IndexOptions::Docs)
    /// only.  Do not call this before
    /// [`nextDoc`](DocIdSetIterator::next_doc) is first called, nor after
    /// [`nextDoc`](DocIdSetIterator::next_doc) returns
    /// [`DocIdSetIterator#
    /// NO_MORE_DOCS`](crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS)
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
    fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>>;
}

// PostingsEnum
pub enum PostingsEnumEnum2<A, B> {
    A(A),
    B(B),
}

impl<A, B> DocIdSetIterator for PostingsEnumEnum2<A, B>
where
    A: PostingsEnum,
    B: PostingsEnum,
{
    fn doc_id(&self) -> i32 {
        match self {
            PostingsEnumEnum2::A(t) => t.doc_id(),
            PostingsEnumEnum2::B(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            PostingsEnumEnum2::A(t) => t.next_doc(),
            PostingsEnumEnum2::B(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            PostingsEnumEnum2::A(t) => t.advance(target),
            PostingsEnumEnum2::B(s) => s.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            PostingsEnumEnum2::A(t) => t.slow_advance(target),
            PostingsEnumEnum2::B(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            PostingsEnumEnum2::A(t) => t.cost(),
            PostingsEnumEnum2::B(s) => s.cost(),
        }
    }
}

impl<A, B> PostingsEnum for PostingsEnumEnum2<A, B>
where
    A: PostingsEnum,
    B: PostingsEnum,
{
    fn freq(&mut self) -> Result<i32> {
        match self {
            PostingsEnumEnum2::A(t) => t.freq(),
            PostingsEnumEnum2::B(s) => s.freq(),
        }
    }

    fn next_position(&mut self) -> Result<i32> {
        match self {
            PostingsEnumEnum2::A(t) => t.next_position(),
            PostingsEnumEnum2::B(s) => s.next_position(),
        }
    }

    fn start_offset(&self) -> Result<i32> {
        match self {
            PostingsEnumEnum2::A(t) => t.start_offset(),
            PostingsEnumEnum2::B(s) => s.start_offset(),
        }
    }

    fn end_offset(&self) -> Result<i32> {
        match self {
            PostingsEnumEnum2::A(t) => t.end_offset(),
            PostingsEnumEnum2::B(s) => s.end_offset(),
        }
    }

    fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        match self {
            PostingsEnumEnum2::A(t) => t.get_payload(),
            PostingsEnumEnum2::B(s) => s.get_payload(),
        }
    }
}

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
