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
use crate::core::index::sort::Sort;
use crate::core::search::field_comparator::{FieldComparator, FieldComparatorEnum};
use crate::core::search::field_doc::FieldDoc;
use crate::core::search::pruning::Pruning;
use crate::core::search::score_doc::{ScoreDoc, ScoreDocLike};
use crate::core::search::sort_field::SortFiledBase;
use crate::core::search::total_hits::{Relation, TotalHits};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};
use crate::core::util::{Comparator, ToInt};

/// Represents hits returned.
pub struct TopDocs<S>
where
    S: ScoreDocLike,
{
    /// The total number of hits for the query.
    pub total_hits: TotalHits,

    /// The top hits for the query.
    pub score_docs: Vec<S>,
}

impl<S> TopDocs<S>
where
    S: ScoreDocLike,
{
    /// Constructs a new `TopDocs`.
    pub fn new(total_hits: TotalHits, score_docs: Vec<S>) -> Self {
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
#[derive(Debug, Clone, Default)]
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
pub(crate) fn tie_break_less_than<C, S>(
    first: &ShardRef,
    first_doc: &S,
    second: &ShardRef,
    second_doc: &S,
    tie_breaker: &C,
) -> bool
where
    C: Comparator<S>,
    S: ScoreDocLike,
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
fn merge_aux<C, S>(
    mut queue: PriorityQueue<ShardRef, C>,
    start: i32,
    size: i32,
    shard_hits: Vec<TopDocs<S>>,
) -> Result<(TotalHits, Vec<ShardRef>)>
where
    C: Compare<ShardRef>,
    S: ScoreDocLike,
{
    let mut total_hit_count: i64 = 0;
    let mut total_hits_relation = Relation::EqualTo;
    let mut avail_hit_count = 0;

    for (shard_idx, shard) in shard_hits.iter().enumerate() {
        total_hit_count += shard.total_hits.value as i64;
        if shard.total_hits.relation == Relation::GreaterThanOrEqualTo {
            total_hits_relation = Relation::GreaterThanOrEqualTo;
        }
        if !shard.score_docs.is_empty() {
            avail_hit_count += shard.score_docs.len() as i32;
            queue.add(ShardRef::new(shard_idx as i32))?;
        }
    }

    let mut hits: Vec<ShardRef>;
    let mut unset_shard_index = false;
    if avail_hit_count <= start {
        hits = Vec::new();
    } else {
        let len = std::cmp::min(size, avail_hit_count - start);
        hits = vec![ShardRef::default(); len as usize];

        let requested_result_window = start + size;
        let num_iter_on_hits = std::cmp::min(avail_hit_count, requested_result_window);
        let mut hit_upto = 0;

        while hit_upto < num_iter_on_hits {
            assert!(queue.size() > 0);
            let ref_ = match queue.top_mut() {
                None => return Err(LuceneError::illegal_state("queue is empty")),
                Some(v) => v,
            };

            let shard = &shard_hits[ref_.shard_index as usize];
            let hit = &shard.score_docs[ref_.hit_index as usize];
            let shard_ref = ref_.clone();
            ref_.hit_index += 1;

            // Irrespective of whether we use shard indices for tie breaking or not, we check for
            // consistent
            // order in shard indices to defend against potential bugs
            if hit_upto > 0 && unset_shard_index != (hit.shard_index() == -1) {
                return Err(LuceneError::illegal_argument(
                    "Inconsistent order of shard indices",
                ));
            }
            unset_shard_index |= hit.shard_index() == -1;

            if hit_upto >= start {
                hits[(hit_upto - start) as usize] = shard_ref;
            }

            hit_upto += 1;

            if ref_.hit_index < shard.score_docs.len() as i32 {
                queue.update_top()?;
            } else {
                queue.pop()?;
            }
        }
    }
    Ok((
        TotalHits::new(total_hit_count as usize, total_hits_relation),
        hits,
    ))
}

pub(crate) struct ScoreMergeSortQueueCmp<C, S>
where
    C: Comparator<S>,
    S: ScoreDocLike,
{
    shard_hits: Vec<TopDocs<S>>,
    tie_breaker_comparator: C,
}
impl<C, S> ScoreMergeSortQueueCmp<C, S>
where
    C: Comparator<S>,
    S: ScoreDocLike,
{
    pub fn new(shard_hits: Vec<TopDocs<S>>, tie_breaker_comparator: C) -> Self {
        Self {
            shard_hits,
            tie_breaker_comparator,
        }
    }
}

impl<C, S> Compare<ShardRef> for ScoreMergeSortQueueCmp<C, S>
where
    C: Comparator<S>,
    S: ScoreDocLike,
{
    fn less_than(&self, first: &ShardRef, second: &ShardRef) -> Result<bool> {
        let first_shard_hits = &self.shard_hits[first.shard_index as usize];
        let second_shard_hits = &self.shard_hits[second.shard_index as usize];

        let first_scorer_doc = &first_shard_hits.score_docs[first.hit_index as usize];
        let second_scorer_doc = &second_shard_hits.score_docs[second.hit_index as usize];
        let first_scorer_doc_score = first_scorer_doc.score();
        let second_scorer_doc_score = second_scorer_doc.score();
        if first_scorer_doc_score < second_scorer_doc_score {
            Ok(false)
        } else if first_scorer_doc_score < second_scorer_doc_score {
            Ok(true)
        } else {
            Ok(tie_break_less_than(
                first,
                first_scorer_doc,
                second,
                second_scorer_doc,
                &self.tie_breaker_comparator,
            ))
        }
    }
}

pub(crate) struct MergeSortQueueCmp<C>
where
    C: Comparator<FieldDoc>,
{
    shard_hits: Vec<TopDocs<FieldDoc>>,
    comparators: Vec<FieldComparatorEnum>,
    reverse_mul: Vec<i32>,
    tie_breaker: C,
}

impl<C> MergeSortQueueCmp<C>
where
    C: Comparator<FieldDoc>,
{
    pub fn new(sort: &Sort, shard_hits: Vec<TopDocs<FieldDoc>>, tie_breaker: C) -> Result<Self> {
        let mut comparators = Vec::new();
        let mut reverse_mul = Vec::new();
        for sf in &sort.fields {
            comparators.push(sf.get_comparator(1, Pruning::None)?);
            reverse_mul.push(if sf.get_reverse() { -1 } else { 1 });
        }

        Ok(Self {
            shard_hits,
            comparators,
            reverse_mul,
            tie_breaker,
        })
    }
}

impl<C> Compare<ShardRef> for MergeSortQueueCmp<C>
where
    C: Comparator<FieldDoc>,
{
    fn less_than(&self, first: &ShardRef, second: &ShardRef) -> Result<bool> {
        let first_fd =
            &self.shard_hits[first.shard_index as usize].score_docs[first.hit_index as usize];
        let second_fd =
            &self.shard_hits[second.shard_index as usize].score_docs[second.hit_index as usize];

        for (i, comp) in self.comparators.iter().enumerate() {
            let cmp = self.reverse_mul[i]
                * comp.compare_values(first_fd.fields.get(i), second_fd.fields.get(i));
            if cmp != 0 {
                return Ok(cmp < 0);
            }
        }
        Ok(tie_break_less_than(
            first,
            first_fd,
            second,
            second_fd,
            &self.tie_breaker,
        ))
    }
}
