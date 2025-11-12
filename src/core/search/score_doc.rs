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
use std::fmt;
use std::fmt::Display;

/// Holds one hit in [`TopDocs`](crate::core::search::top_docs::TopDocs).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScoreDoc {
    /// The score of this document for the query.
    pub score: f32,

    /// A hit document's number.
    ///
    /// See [`StoredFields::document`](crate::core::index::stored_fields::StoredFields::document).
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
impl ScoreDocLike for ScoreDoc {
    fn doc(&self) -> i32 {
        self.doc
    }

    fn score(&self) -> f32 {
        self.score
    }

    fn shard_index(&self) -> i32 {
        self.shard_index
    }

    fn set_shard_index(&mut self, shard_index: i32) {
        self.shard_index = shard_index
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
pub trait ScoreDocLike: Display + Clone + Default {
    fn doc(&self) -> i32;
    fn score(&self) -> f32;
    fn shard_index(&self) -> i32;
    fn set_shard_index(&mut self, shard_index: i32);
}
