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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::sort::Sort;
use crate::core::search::collector::Collector;
use crate::core::search::doc_id_set_iterator::Either2DocIdSetIterator;
use crate::core::search::dummy::dummy_doc_id_set_iterator::DummyDocIdSetIterator;
use crate::core::search::dummy::dummy_leaf_collector::DummyLeafCollector;
use crate::core::search::field_comparator::{FieldComparator, FieldComparatorEnum};
use crate::core::search::field_value_hit_queue::{
    Entry, FieldValueHitQueueComparator, TopFieldScoreDoc,
};
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::leaf_field_comparator::{
    LeafFieldComparator, LeafFieldComparatorDocIdSetIterator, LeafFieldComparatorEnum,
};
use crate::core::search::max_score_accumulator::MaxScoreAccumulator;
use crate::core::search::multi_leaf_field_comparator::MultiLeafFieldComparator;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::sort_field::SortField;
use crate::core::search::sort_field_enum::SortFieldEnum;
use crate::core::search::top_docs::TopDocs;
use crate::core::search::top_docs_collector::{TopDocsCollector, TopDocsCollectorBase};
use crate::core::search::total_hits::Relation;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::PriorityQueue;

pub struct TopFieldCollector {
    base: TopDocsCollectorBase<TopFieldScoreDoc, FieldValueHitQueueComparator>,
    num_hits: i32,
    total_hits_threshold: i32,
    can_set_min_score: bool,
    search_sort_part_of_index_sort: Option<bool>,
    min_score_acc: Option<MaxScoreAccumulator>,
    min_competitive_score: f32,
    num_comparators: i32,
    queue_full: bool,
    doc_base: i32,
    needs_scores: bool,
    score_mode: ScoreMode,
}
impl TopFieldCollector {
    pub fn new(
        pq: PriorityQueue<TopFieldScoreDoc, FieldValueHitQueueComparator>,
        num_hits: i32,
        total_hits_threshold: i32,
        needs_scores: bool,
        min_score_acc: Option<MaxScoreAccumulator>,
    ) -> Result<Self> {
        let total_hits_threshold = std::cmp::max(total_hits_threshold, num_hits);
        debug_assert!(total_hits_threshold >= 0);

        let num_comparators = pq.get_comparators().len() as i32;

        let first_comparator = &pq.get_comparators()[0];
        let reverse_mul = pq.get_reverse_mul()[0];

        let (score_mode, can_set_min_score) = if matches!(first_comparator, FieldComparatorEnum::Relevance(_))
                && reverse_mul == 1// if the natural sort is preserved (sort by descending relevance)
                && total_hits_threshold != i32::MAX
        {
            (ScoreMode::TopScores, true)
        } else {
            let can_set_min_score = false;
            let score_mode = if total_hits_threshold != i32::MAX {
                if needs_scores {
                    ScoreMode::TopDocsWithScores
                } else {
                    ScoreMode::TopDocs
                }
            } else if needs_scores {
                ScoreMode::Complete
            } else {
                ScoreMode::CompleteNoScores
            };
            (score_mode, can_set_min_score)
        };

        let base = TopDocsCollectorBase::new(pq);
        Ok(Self {
            base,
            num_hits,
            total_hits_threshold,
            can_set_min_score,
            search_sort_part_of_index_sort: None,
            min_score_acc,
            min_competitive_score: 0.0,
            num_comparators,
            queue_full: false,
            doc_base: 0,
            needs_scores,
            score_mode,
        })
    }
    pub(crate) fn update_global_min_competitive_score<S: Scorable>(
        &mut self,
        scorer: &mut S,
    ) -> Result<()> {
        match &self.min_score_acc {
            Some(acc) if self.can_set_min_score => {
                // we can start checking the global maximum score even if the local queue
                // is not full or the threshold is not reached on the local competitor:
                // the fact that there is a shared min competitive score implies that one
                // of the collectors hit its totalHitsThreshold already
                let max_min_score = acc.get_raw();

                if max_min_score != i64::MIN {
                    let score = MaxScoreAccumulator::to_score(max_min_score);
                    if score > self.min_competitive_score {
                        scorer.set_min_competitive_score(score)?;
                        self.min_competitive_score = score;
                        self.base.total_hits_relation = Relation::GreaterThanOrEqualTo;
                    }
                }
                Ok(())
            },
            _ => Ok(()),
        }
    }

    pub(crate) fn update_min_competitive_score<S: Scorable>(
        &mut self,
        scorer: &mut S,
    ) -> Result<()> {
        debug_assert!(self.total_hits_threshold >= 0);
        if self.can_set_min_score
            && self.queue_full
            && self.base.total_hits > self.total_hits_threshold as usize
        {
            let bottom = self.bottom()?;

            let first_comparator = &self.base.pq.get_comparators()[0];
            let min_score = *first_comparator
                .value(bottom.slot()?)
                .as_f32()
                .expect("first comparator is not a float");

            if min_score > self.min_competitive_score {
                scorer.set_min_competitive_score(min_score)?;
                self.min_competitive_score = min_score;
                self.base.total_hits_relation = Relation::GreaterThanOrEqualTo;

                if let Some(acc) = &self.min_score_acc {
                    acc.accumulate(self.doc_base, min_score);
                }
            }
        }
        Ok(())
    }
    pub(crate) fn add(&mut self, slot: i32, doc: i32) -> Result<()> {
        let global_doc = doc + self.doc_base;
        self.pq_mut().add(Entry::new(slot, global_doc).into())?;

        // The queue is full either when total_hits == num_hits (in SimpleFieldCollector),
        // in which case slot = total_hits - 1, or when hits_collected == num_hits (in
        // PagingFieldCollector this is hits on the current page) and slot = hits_collected - 1.
        debug_assert!(slot < self.num_hits);

        self.queue_full = slot == self.num_hits - 1;
        Ok(())
    }
    pub(crate) fn update_bottom(&mut self, doc: i32) -> Result<()> {
        let global_doc = doc + self.doc_base;
        let bottom = self.bottom_mut()?;
        bottom.base().doc = global_doc;
        let pq = self.pq_mut();
        pq.update_top()?;
        Ok(())
    }
    #[inline]
    fn bottom(&self) -> Result<&TopFieldScoreDoc> {
        self.base
            .pq
            .top()
            .ok_or_else(|| LuceneError::illegal_state("priority queue bottom missing"))
    }
    #[inline]
    fn bottom_mut(&mut self) -> Result<&mut TopFieldScoreDoc> {
        self.base
            .pq
            .top_mut()
            .ok_or_else(|| LuceneError::illegal_state("priority queue bottom missing"))
    }
}

impl Collector for TopFieldCollector {
    type LeafCollector<'a>
        = DummyLeafCollector
    where
        Self: 'a;

    fn get_leaf_collector<'a, W, LR>(
        &'a mut self,
        _context: &LeafReaderContext<LR>,
        _weight: Option<&mut W>,
    ) -> Result<Self::LeafCollector<'a>>
    where
        LR: LeafReader,
        W: Weight<LR>,
    {
        todo!()
    }

    fn score_mode(&self) -> ScoreMode {
        self.score_mode
    }
}

impl TopDocsCollector for TopFieldCollector {
    type Item = TopFieldScoreDoc;
    type Cmp = FieldValueHitQueueComparator;

    fn pq(&self) -> &PriorityQueue<Self::Item, Self::Cmp> {
        &self.base.pq
    }

    fn pq_mut(&mut self) -> &mut PriorityQueue<Self::Item, Self::Cmp> {
        &mut self.base.pq
    }

    fn total_hits(&self) -> usize {
        self.base.total_hits
    }

    fn get_total_hits_relation(&self) -> Relation {
        self.base.total_hits_relation
    }

    fn populate_results(&mut self, results: &mut [Self::Item], how_many: usize) -> Result<()> {
        let pq = &mut self.base.pq;
        for i in (0..how_many).rev() {
            let entry = pq.pop_unchecked()?;
            results[i] = pq.fill_fields(entry)?;
        }
        Ok(())
    }

    fn new_top_docs(&self, _results: Option<Vec<Self::Item>>, _start: i32) -> TopDocs<Self::Item>
    where
        Self: Sized,
    {
        todo!()
    }
}

pub struct TopFieldLeafCollector<'a, LR>
where
    LR: LeafReader,
{
    base: &'a mut TopFieldCollector,
    reverse_mul: i32,
    collected_all_competitive_hits: bool,
    comparator: TopFieldLeafComparatorEnum<LR>,
}
impl<'a, LR> TopFieldLeafCollector<'a, LR>
where
    LR: LeafReader,
{
    pub fn new(
        base: &'a mut TopFieldCollector,
        sort: &Sort,
        context: &LeafReaderContext<LR>,
    ) -> Result<Self> {
        // as all segments are sorted in the same way, enough to check only the 1st segment for
        // indexSort
        if base.search_sort_part_of_index_sort.is_none()
            && let Some(index_sort) = context.reader().get_metadata()?.get_sort()
        {
            let can_early_terminate = can_early_terminate(sort, Some(index_sort))?;
            base.search_sort_part_of_index_sort = Some(can_early_terminate);

            if can_early_terminate {
                let pq = &mut base.base.pq;
                let first_comparator = &mut pq.get_comparators_mut()[0];
                first_comparator.disable_skipping();
            }
        }

        let leaf_comparators = base.base.pq.get_leaf_comparator(context)?;
        let reverse_muls = base.base.pq.get_reverse_mul_shared();

        let (reverse_mul, comparator) = if leaf_comparators.len() == 1 {
            (
                reverse_muls[0],
                TopFieldLeafComparatorEnum::Single(leaf_comparators.into_iter().next().unwrap()),
            )
        } else {
            (
                1,
                TopFieldLeafComparatorEnum::Multi(MultiLeafFieldComparator::new(
                    leaf_comparators,
                    reverse_muls,
                )?),
            )
        };

        Ok(Self {
            base,
            reverse_mul,
            collected_all_competitive_hits: false,
            comparator,
        })
    }
    pub(crate) fn count_hit<S: Scorable>(&mut self, scorer: &mut S, _doc: i32) -> Result<()> {
        self.base.base.total_hits += 1;
        debug_assert!(self.base.base.total_hits <= i32::MAX as usize);
        let hit_count_so_far = self.base.base.total_hits as i32;

        if let Some(acc) = &self.base.min_score_acc {
            debug_assert!(acc.mod_interval <= i32::MAX as i64);
            if (hit_count_so_far & acc.mod_interval as i32) == 0 {
                self.base.update_global_min_competitive_score(scorer)?;
            }
        }

        if !self.base.score_mode.is_exhaustive()
            && self.base.base.total_hits_relation == Relation::EqualTo
            && hit_count_so_far > self.base.total_hits_threshold
        {
            let comparators = self.base.base.pq.get_comparators_mut();
            self.comparator.set_hits_threshold_reached(comparators)?;
            self.base.base.total_hits_relation = Relation::GreaterThanOrEqualTo;
        }

        Ok(())
    }
    pub(crate) fn threshold_check<S>(&mut self, doc: i32, scorer: &mut S) -> Result<bool>
    where
        S: Scorable,
    {
        let cmp_check = if self.collected_all_competitive_hits {
            true
        } else {
            let comparators = self.base.base.pq.get_comparators_mut();
            let cmp = self.comparator.compare_bottom(doc, scorer, comparators)?;
            self.reverse_mul * cmp <= 0
        };

        if cmp_check {
            // since docs are visited in doc Id order, if compare is 0, it means
            // this document is larger than anything else in the queue, and
            // therefore not competitive.
            if self.base.search_sort_part_of_index_sort.unwrap_or(false) {
                if self.base.base.total_hits > self.base.total_hits_threshold as usize {
                    self.base.base.total_hits_relation = Relation::GreaterThanOrEqualTo;
                    return Err(LuceneError::collection_terminated(
                        "collection terminated due to early termination threshold",
                    ));
                } else {
                    self.collected_all_competitive_hits = true;
                }
            } else if self.base.base.total_hits_relation == Relation::EqualTo {
                // we can start setting the min competitive score if the
                // threshold is reached for the first time here.
                self.base.update_min_competitive_score(scorer)?;
            }
            return Ok(true);
        }

        Ok(false)
    }
    pub(crate) fn collect_competitive_hit<S>(&mut self, doc: i32, scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        {
            let bottom = self.bottom()?;
            self.comparator.copy(
                bottom.slot()? as usize,
                doc,
                scorer,
                self.base.base.pq.get_comparators_mut(),
            )?;
        }
        self.base.update_bottom(doc)?;
        let bottom = self.bottom()?;
        self.comparator.set_bottom(
            bottom.slot()? as usize,
            self.base.base.pq.get_comparators_mut(),
        )?;
        self.base.update_min_competitive_score(scorer)?;

        Ok(())
    }
    pub(crate) fn collect_any_hit<S>(
        &mut self,
        doc: i32,
        hits_collected: i32,
        scorer: &mut S,
    ) -> Result<()>
    where
        S: Scorable,
    {
        // Startup transient: queue hasn't gathered numHits yet
        let slot = hits_collected - 1;
        // Copy hit into queue
        self.comparator.copy(
            slot as usize,
            doc,
            scorer,
            self.base.base.pq.get_comparators_mut(),
        )?;
        self.base.add(slot, doc)?;
        if self.base.queue_full {
            let bottom = self.bottom()?;
            self.comparator.set_bottom(
                bottom.slot()? as usize,
                self.base.base.pq.get_comparators_mut(),
            )?;
            self.base.update_min_competitive_score(scorer)?;
        }
        Ok(())
    }
    #[inline]
    fn bottom(&self) -> Result<&TopFieldScoreDoc> {
        self.base.bottom()
    }
    #[inline]
    fn bottom_mut(&mut self) -> Result<&mut TopFieldScoreDoc> {
        self.base.bottom_mut()
    }
}
impl<'a, LR> LeafCollector for TopFieldLeafCollector<'a, LR>
where
    LR: LeafReader,
{
    fn set_scorer<S>(&mut self, scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        let comparators = self.base.base.pq.get_comparators_mut();
        self.comparator.set_scorer(scorer, comparators)?;

        if self.base.min_score_acc.is_none() {
            self.base.update_min_competitive_score(scorer)?;
        } else {
            self.base.update_global_min_competitive_score(scorer)?;
        }

        Ok(())
    }

    fn collect<S>(&mut self, doc: i32, scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        todo!()
    }

    type DocIdSetIterator = DummyDocIdSetIterator;

    fn competitive_iterator(&mut self) -> Result<Option<&mut Self::DocIdSetIterator>> {
        todo!()
    }
}
fn can_early_terminate(search_sort: &Sort, index_sort: Option<&Sort>) -> Result<bool> {
    Ok(can_early_terminate_on_doc_id(search_sort)?
        || can_early_terminate_on_prefix(search_sort, index_sort)?)
}

fn can_early_terminate_on_doc_id(search_sort: &Sort) -> Result<bool> {
    let fields = search_sort.get_sort();
    if let Some(SortFieldEnum::Sorter(field)) = fields.first() {
        let field_doc = SortField::get_field_doc()?;
        Ok(*field == field_doc)
    } else {
        Ok(false)
    }
}
fn can_early_terminate_on_prefix(search_sort: &Sort, index_sort: Option<&Sort>) -> Result<bool> {
    if let Some(index_sort) = index_sort {
        let fields1 = search_sort.get_sort();
        let fields2 = index_sort.get_sort();

        if fields1.len() > fields2.len() {
            return Ok(false);
        }

        Ok(fields1.iter().zip(fields2.iter()).all(|(a, b)| a == b))
    } else {
        Ok(false)
    }
}
pub enum TopFieldLeafComparatorEnum<LR>
where
    LR: LeafReader,
{
    Multi(MultiLeafFieldComparator<LR>),
    Single(LeafFieldComparatorEnum<LR>),
}
impl<LR> TopFieldLeafComparatorEnum<LR>
where
    LR: LeafReader,
{
    pub(crate) fn set_bottom(
        &mut self,
        slot: usize,
        comparator: &mut [FieldComparatorEnum],
    ) -> Result<()> {
        match self {
            Self::Multi(inner) => inner.set_bottom(slot, comparator),
            Self::Single(inner) => inner.set_bottom(slot, &mut comparator[0]),
        }
    }

    pub(crate) fn compare_bottom<S>(
        &mut self,
        doc: i32,
        scorer: &mut S,
        comparators: &mut [FieldComparatorEnum],
    ) -> Result<i32>
    where
        S: Scorable,
    {
        match self {
            Self::Multi(inner) => inner.compare_bottom(doc, scorer, comparators),
            Self::Single(inner) => inner.compare_bottom(doc, scorer, &mut comparators[0]),
        }
    }

    pub(crate) fn compare_top<S>(
        &mut self,
        doc: i32,
        scorer: &mut S,
        comparators: &mut [FieldComparatorEnum],
    ) -> Result<i32>
    where
        S: Scorable,
    {
        match self {
            Self::Multi(inner) => inner.compare_top(doc, scorer, comparators),
            Self::Single(inner) => inner.compare_top(doc, scorer, &mut comparators[0]),
        }
    }

    pub(crate) fn copy<S>(
        &mut self,
        slot: usize,
        doc: i32,
        scorer: &mut S,
        comparators: &mut [FieldComparatorEnum],
    ) -> Result<()>
    where
        S: Scorable,
    {
        match self {
            Self::Multi(inner) => inner.copy(slot, doc, scorer, comparators),
            Self::Single(inner) => inner.copy(slot, doc, scorer, &mut comparators[0]),
        }
    }

    pub(crate) fn set_scorer<S>(
        &mut self,
        scorer: &mut S,
        comparators: &mut [FieldComparatorEnum],
    ) -> Result<()>
    where
        S: Scorable,
    {
        match self {
            Self::Multi(inner) => inner.set_scorer(scorer, comparators),
            Self::Single(inner) => inner.set_scorer(scorer, &mut comparators[0]),
        }
    }

    pub(crate) fn competitive_iterator(
        &mut self,
        comparators: &mut [FieldComparatorEnum],
    ) -> Option<
        Either2DocIdSetIterator<
            LeafFieldComparatorDocIdSetIterator<LR>,
            <LeafFieldComparatorEnum<LR> as LeafFieldComparator>::DocIdSetIterator,
        >,
    > {
        match self {
            Self::Multi(inner) => inner
                .competitive_iterator(comparators)
                .map(Either2DocIdSetIterator::A),
            Self::Single(inner) => inner
                .competitive_iterator(&mut comparators[0])
                .map(Either2DocIdSetIterator::B),
        }
    }

    pub(crate) fn set_hits_threshold_reached(
        &mut self,
        comparators: &mut [FieldComparatorEnum],
    ) -> Result<()> {
        match self {
            Self::Multi(inner) => inner.set_hits_threshold_reached(comparators),
            Self::Single(inner) => inner.set_hits_threshold_reached(&mut comparators[0]),
        }
    }
}
