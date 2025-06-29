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

/// Holds one hit in [`TopDocs`](crate::search::top_docs::TopDocs).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScoreDoc {
    /// The score of this document for the query.
    pub score: f32,

    /// A hit document's number.
    ///
    /// See [`StoredFields::document`](crate::index::stored_fields::StoredFields::document).
    pub doc: i32,

    /// Only set by `TopDocs::merge`.
    pub shard_index: i32,
}

impl ScoreDoc {
    /// Constructs a `ScoreDoc`.
    pub fn new(doc: i32, score: f32) -> Self {
        Self::with_shard_index(doc, score, -1)
    }

    /// Constructs a `ScoreDoc` with a given `shard_index`.
    pub fn with_shard_index(doc: i32, score: f32, shard_index: i32) -> Self {
        Self {
            doc,
            score,
            shard_index,
        }
    }
}

impl fmt::Display for ScoreDoc {
    /// A convenience method for debugging.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "doc={} score={} shardIndex={}",
            self.doc, self.score, self.shard_index
        )
    }
}
