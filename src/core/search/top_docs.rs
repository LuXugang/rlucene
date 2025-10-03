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
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::total_hits::TotalHits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::{Comparator, ToInt};

/// Represents hits returned.
#[derive(Debug, Clone, PartialEq)]
pub struct TopDocs {
    /// The total number of hits for the query.
    pub total_hits: TotalHits,

    /// The top hits for the query.
    pub score_docs: Vec<ScoreDoc>,
}

impl TopDocs {
    /// Constructs a new `TopDocs`.
    pub fn new(total_hits: TotalHits, score_docs: Vec<ScoreDoc>) -> Self {
        Self {
            total_hits,
            score_docs,
        }
    }
}

struct ShardIndexTieBreaker;
impl Comparator<ScoreDoc> for ShardIndexTieBreaker {
    const TYPE: &'static str = "ShardIndexTieBreaker";

    fn compare(&self, a: &ScoreDoc, b: &ScoreDoc) -> Result<i32> {
        Ok(a.shard_index.cmp(&b.shard_index).to_int())
    }
}

struct DocIdTieBreaker;
impl Comparator<ScoreDoc> for DocIdTieBreaker {
    const TYPE: &'static str = "DocIdTieBreaker";

    fn compare(&self, a: &ScoreDoc, b: &ScoreDoc) -> Result<i32> {
        Ok(a.doc.cmp(&b.doc).to_int())
    }
}

struct DefaultTieBreaker {
    shard_cmp: ShardIndexTieBreaker,
    doc_cmp: DocIdTieBreaker,
}

impl Comparator<ScoreDoc> for DefaultTieBreaker {
    const TYPE: &'static str = "DefaultTieBreaker";

    fn compare(&self, a: &ScoreDoc, b: &ScoreDoc) -> Result<i32> {
        let res = self.shard_cmp.compare(a, b)?;
        if res != 0 {
            Ok(res)
        } else {
            self.doc_cmp.compare(a, b)
        }
    }
}
#[derive(Debug, Clone)]
pub(crate) struct ShardRef {
    /// Which shard (index into shardHits[]).
    pub(crate) shard_index: i32,

    /// Which hit within the shard.
    pub(crate) hit_index: i32,
}

impl ShardRef {
    pub fn new(shard_index: i32) -> Self {
        ShardRef {
            shard_index,
            hit_index: 0,
        }
    }
}

impl std::fmt::Display for ShardRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ShardRef(shard_index={} hit_index={})",
            self.shard_index, self.hit_index
        )
    }
}
pub(crate) fn tie_break_less_than<C>(
    first: &ShardRef,
    first_doc: &ScoreDoc,
    second: &ShardRef,
    second_doc: &ScoreDoc,
    tie_breaker: &C,
) -> bool
where
    C: Comparator<ScoreDoc>,
{
    let value = tie_breaker.compare_unchecked(first_doc, second_doc);

    if value == 0 {
        // Equal Values
        // Tie break in same shard: resolve however the
        // shard had resolved it:
        debug_assert!(first.hit_index != second.hit_index);
        return first.hit_index < second.hit_index;
    }

    value < 0
}
