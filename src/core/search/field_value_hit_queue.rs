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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::field_comparator::{FieldComparator, FieldComparatorEnum};
use crate::core::search::leaf_field_comparator::LeafFieldComparatorEnum;
use crate::core::search::pruning::Pruning;
use crate::core::search::score_doc::{ScoreDoc, ScoreDocLike};
use crate::core::search::sort_field::SortFiledBase;
use crate::core::search::sort_field_enum::SortFieldEnum;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, Either2Compare, PriorityQueue};
use std::fmt;
/// A hit queue for sorting by hits by terms in more than one field
pub struct FieldValueHitQueue;
impl FieldValueHitQueue {
    pub fn new(
        fields: &[SortFieldEnum],
        size: i32,
    ) -> Result<PriorityQueue<Entry, FieldValueHitQueueComparator>> {
        let num_comparators = fields.len();
        let mut comparators = Vec::with_capacity(num_comparators);
        let mut reverse_mul = Vec::with_capacity(num_comparators);

        for (i, field) in fields.iter().enumerate() {
            let reverse = if field.get_reverse() { -1 } else { 1 };
            reverse_mul.push(reverse);

            let pruning = if i == 0 {
                if num_comparators > 1 {
                    Pruning::GreaterThan
                } else {
                    Pruning::GreaterThanOrEqualTo
                }
            } else {
                Pruning::None
            };

            let comparator = field.get_comparator(size as usize, pruning)?;
            comparators.push(comparator);
        }
        let comparator = if num_comparators == 1 {
            Either2Compare::A(OneComparatorComparator::new(comparators, reverse_mul))
        } else {
            Either2Compare::B(MultiComparatorsComparator::new(comparators, reverse_mul))
        };
        PriorityQueue::new(size, comparator)
    }
}
/// Creates a hit queue sorted by the given list of fields.
///
/// **NOTE:**
/// The instances returned by this method pre-allocate a full array of length `num_hits`.
///
/// # Arguments
///
/// * `fields` – SortField array we are sorting by in priority order (highest priority first);
///   cannot be empty.
/// * `size` – The number of hits to retain.
/// Must be greater than zero.

pub fn create(
    fields: &[SortFieldEnum],
    size: i32,
) -> Result<PriorityQueue<Entry, FieldValueHitQueueComparator>> {
    if fields.is_empty() {
        return Err(LuceneError::illegal_state(
            "Sort must contain at least one field",
        ));
    }
    FieldValueHitQueue::new(fields, size)
}

#[derive(Clone, Default)]
pub struct Entry {
    pub base: ScoreDoc,
    pub slot: i32,
}
impl Entry {
    pub fn new(slot: i32, doc: i32) -> Self {
        let base = ScoreDoc::new(doc, f32::NAN);
        Self { base, slot }
    }
}
impl ScoreDocLike for Entry {
    fn doc(&self) -> i32 {
        self.base.doc
    }

    fn score(&self) -> f32 {
        self.base.score
    }

    fn shard_index(&self) -> i32 {
        self.base.shard_index
    }
}
impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "slot:{} {}", self.slot, self.base)
    }
}

/// An implementation of FieldValueHitQueue which is optimized in case there is just one comparator.
pub struct OneComparatorComparator {
    one_comparator: Vec<FieldComparatorEnum>,
    one_reverse_mul: Vec<i32>,
}
impl OneComparatorComparator {
    pub fn new(one_comparator: Vec<FieldComparatorEnum>, one_reverse_mul: Vec<i32>) -> Self {
        debug_assert_eq!(one_comparator.len(), one_reverse_mul.len());
        debug_assert!(one_comparator.len() == 1);
        Self {
            one_comparator,
            one_reverse_mul,
        }
    }
}
impl Compare<Entry> for OneComparatorComparator {
    fn less_than(&self, hit_a: &Entry, hit_b: &Entry) -> Result<bool> {
        debug_assert!(self.one_comparator.len() == self.one_reverse_mul.len());
        debug_assert_ne!(hit_a as *const _, hit_b as *const _);
        debug_assert_ne!(hit_a.slot, hit_b.slot);

        let cmp_result = self.one_comparator[0].compare(hit_a.slot, hit_b.slot);
        let c = self.one_reverse_mul[0] * cmp_result;

        if c != 0 {
            Ok(c > 0)
        } else {
            Ok(hit_a.base.doc > hit_b.base.doc)
        }
    }
}
/// An implementation of FieldValueHitQueue which is optimized in case there is more than one comparator.
pub struct MultiComparatorsComparator {
    comparators: Vec<FieldComparatorEnum>,
    reverse_mul: Vec<i32>,
}

impl MultiComparatorsComparator {
    pub fn new(comparators: Vec<FieldComparatorEnum>, reverse_mul: Vec<i32>) -> Self {
        debug_assert_eq!(
            comparators.len(),
            reverse_mul.len(),
            "comparators and reverse_mul length must match"
        );
        Self {
            comparators,
            reverse_mul,
        }
    }
}

impl Compare<Entry> for MultiComparatorsComparator {
    fn less_than(&self, hit_a: &Entry, hit_b: &Entry) -> Result<bool> {
        debug_assert_eq!(
            self.comparators.len(),
            self.reverse_mul.len(),
            "comparators/reverse_mul length mismatch"
        );
        debug_assert_ne!(hit_a as *const _, hit_b as *const _);
        debug_assert_ne!(hit_a.slot, hit_b.slot);

        let num_comparators = self.comparators.len();

        for i in 0..num_comparators {
            let cmp_result = self.comparators[i].compare(hit_a.slot, hit_b.slot);
            let c = self.reverse_mul[i] * cmp_result;

            if c != 0 {
                return Ok(c > 0);
            }
        }
        Ok(hit_a.base.doc > hit_b.base.doc)
    }
}
pub type FieldValueHitQueueComparator =
    Either2Compare<OneComparatorComparator, MultiComparatorsComparator>;

impl PriorityQueue<Entry, FieldValueHitQueueComparator> {
    pub fn get_leaf_comparator<LR>(
        self,
        context: &LeafReaderContext<LR>,
    ) -> Result<Vec<LeafFieldComparatorEnum<LR>>>
    where
        LR: LeafReader,
    {
        match self.compare {
            Either2Compare::A(one_comp) => {
                debug_assert!(one_comp.one_comparator.len() == 1);
                let comp = one_comp.one_comparator.into_iter().next().unwrap();
                Ok(vec![comp.get_leaf_comparator(context)?])
            },
            Either2Compare::B(multi_comp) => multi_comp
                .comparators
                .into_iter()
                .map(|c| c.get_leaf_comparator(context))
                .collect(),
        }
    }
    pub fn get_reverse_mul(self) -> Vec<i32> {
        match self.compare {
            Either2Compare::A(one_comp) => one_comp.one_reverse_mul,
            Either2Compare::B(multi_comp) => multi_comp.reverse_mul,
        }
    }
    pub fn get_comparators(&self) -> &[FieldComparatorEnum] {
        match &self.compare {
            Either2Compare::A(one_comp) => &one_comp.one_comparator,
            Either2Compare::B(multi_comp) => &multi_comp.comparators,
        }
    }
}
