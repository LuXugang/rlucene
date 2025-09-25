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
use crate::core::index::doc_values::{DocValues, EmptyNumeric};
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::numeric_doc_values::{Either2NumericDocValues, NumericDocValues};
use crate::core::index::point_values::{
    IntersectVisitor, PointValues, Relation, is_estimated_point_count_greater_than_or_equal_to,
};
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::{
    AllDocIdSetIterator, DocIdSetIterator, Either3DocIdSetIterator,
};
use crate::core::search::field_comparator::FieldComparator;
use crate::core::search::leaf_field_comparator::LeafFieldComparator;
use crate::core::search::pruning::Pruning;
use crate::core::search::scorable::Scorable;
use crate::core::util::ToInt;
use crate::core::util::doc_id_set_builder::{DocIdSetBuilder, DocIdSetBuilderIterator};
use crate::core::util::error::lucene_error::{LuceneError, Result};

const MIN_SKIP_INTERVAL: i32 = 32;
const MAX_SKIP_INTERVAL: i32 = 8192;
/// Base numeric comparator for comparing numeric values.
/// This comparator provides a skipping functionality – an iterator that can skip over
/// non-competitive documents.
///
/// The parameter `field` provided in the constructor is used as a field name in the default
/// implementations of the methods `get_numeric_doc_values` and `get_point_values` to retrieve
/// doc values and points.
///
/// You can pass a dummy value for a field name (e.g. when sorting by script),
/// but in this case you must override both of these methods.
pub struct NumericComparator<S>
where
    S: NumericComparatorBase + FieldComparator + Default,
{
    pub(crate) field: String,
    pub(crate) missing_value: S::V,
    missing_value_as_long: i64,
    pub(crate) reverse: bool,
    bytes_count: i32, // how many bytes are used to encode this number

    pub(crate) top_value_set: bool,
    pub(crate) single_sort: bool, // true if sort is based on a single sort field
    pub(crate) hits_threshold_reached: bool,
    pub(crate) queue_full: bool,
    pub(crate) pruning: Pruning,
    sub: S,
}

impl<S> NumericComparator<S>
where
    S: NumericComparatorBase + FieldComparator + Default,
{
    pub fn new(
        field: String,
        missing_value: S::V,
        reverse: bool,
        pruning: Pruning,
        bytes_count: i32,
        sub: S,
    ) -> Self {
        let missing_value_as_long = sub.missing_value_as_comparable_long();
        Self {
            field,
            missing_value,
            missing_value_as_long,
            reverse,
            bytes_count,
            top_value_set: false,
            single_sort: false,
            hits_threshold_reached: false,
            queue_full: false,
            pruning,
            sub,
        }
    }
}
impl<S> FieldComparator for NumericComparator<S>
where
    S: NumericComparatorBase + FieldComparator + Default,
{
    type V = S::V;

    fn compare(&self, slot1: i32, slot2: i32) -> i32 {
        self.sub.compare(slot1, slot2)
    }

    fn set_top_value(&mut self, value: Self::V) {
        self.top_value_set = true;
        self.sub.set_top_value(value)
    }

    fn value(&self, slot: i32) -> &Self::V {
        self.sub.value(slot)
    }

    type LeafFieldComparator = S::LeafFieldComparator;

    fn get_leaf_comparator<LR>(self, _context: &LeafReaderContext<LR>) -> Self::LeafFieldComparator
    where
        LR: LeafReader,
    {
        todo!()
    }

    fn compare_values(&self, first: Option<&Self::V>, second: Option<&Self::V>) -> Result<i32> {
        self.sub.compare_values(first, second)
    }

    fn set_single_sort(&mut self) {
        self.single_sort = true;
    }

    fn disable_skipping(&mut self) {
        self.pruning = Pruning::None;
    }
}
pub trait NumericComparatorBase {
    fn missing_value_as_comparable_long(&self) -> i64;
    fn sortable_bytes_to_long(&self, bytes: &[u8]) -> i64;
}
pub struct NumericLeafComparator<LR, NCB, NC>
where
    LR: LeafReader,
    NCB: NumericLeafComparatorBase + LeafFieldComparator,
    NC: NumericComparatorBase + FieldComparator + Default,
{
    pub(crate) doc_values: NCB::NumericDocValues,
    point_values: Option<LR::PointValues>,
    // lazily constructed to avoid performance overhead when this is not used
    point_tree: Option<<LR::PointValues as PointValues>::PointTree>,
    // if skipping functionality should be enabled on this segment
    enable_skipping: bool,
    max_doc: i32,
    leaf_top_set: bool,
    min_value_as_long: i64,
    max_value_as_long: i64,
    competitive_iterator: Option<CompetitiveIteratorType<NCB::NumericDocValues>>,
    iterator_cost: i64,
    max_doc_visited: i32,
    update_counter: i32,
    current_skip_interval: i32,
    // helps to be conservative about increasing the sampling interval
    try_update_fail_count: i32,
    sub: NCB,
    parent: NumericComparator<NC>,
    candidate: Option<NCB::NumericDocValues>,
}
impl<LR, NCB, NC> NumericLeafComparator<LR, NCB, NC>
where
    LR: LeafReader,
    NCB: NumericLeafComparatorBase + LeafFieldComparator,
    NC: NumericComparatorBase + FieldComparator + Default,
{
    pub fn new(
        context: &LeafReaderContext<LR>,
        sub: NCB,
        parent: NumericComparator<NC>,
    ) -> Result<Self> {
        let field = &parent.field;
        let doc_values = sub.get_numeric_doc_values(context, field)?;
        let candidate = Some(sub.get_numeric_doc_values(context, field)?);

        let point_values = if parent.pruning != Pruning::None {
            context.reader().get_point_values(field)?
        } else {
            None
        };

        let (enable_skipping, max_doc, competitive_iterator) = if let Some(ref pv) = point_values {
            if let Some(info) = context
                .reader()
                .get_field_infos()?
                .field_info_by_name(field)
            {
                if info.get_point_dimension_count() == 0 {
                    return Err(LuceneError::illegal_state(format!(
                        "Field {} doesn't index points according to FieldInfos yet returns non-null PointValues",
                        field
                    )));
                } else if info.get_point_dimension_count() > 1 {
                    return Err(LuceneError::illegal_argument(format!(
                        "Field {} is indexed with multiple dimensions, sorting is not supported",
                        field
                    )));
                } else if info.get_point_num_bytes() != parent.bytes_count {
                    return Err(LuceneError::illegal_argument(format!(
                        "Field {} is indexed with {} bytes per dimension, but expected {}",
                        field,
                        info.get_point_num_bytes(),
                        parent.bytes_count
                    )));
                }
            } else {
                return Err(LuceneError::illegal_state(format!(
                    "Field {} has no FieldInfo but returned non-null PointValues",
                    field
                )));
            }

            let max_doc = context.reader().max_doc()?;
            let competitive_iterator = Some(CompetitiveIteratorType::A(AllDocIdSetIterator::new(
                max_doc,
            )));
            (true, max_doc, competitive_iterator)
        } else {
            (false, 0, None)
        };

        let mut v = Self {
            doc_values,
            point_values,
            point_tree: None,
            enable_skipping,
            max_doc,
            leaf_top_set: parent.top_value_set,
            min_value_as_long: i64::MIN,
            max_value_as_long: i64::MAX,
            competitive_iterator,
            iterator_cost: -1,
            max_doc_visited: -1,
            update_counter: 0,
            current_skip_interval: MIN_SKIP_INTERVAL,
            try_update_fail_count: 0,
            sub,
            parent,
            candidate,
        };
        if v.point_values.is_some() && v.leaf_top_set {
            v.encode_top();
        }
        Ok(v)
    }
    fn update_competitive_iterator(&mut self) -> Result<()> {
        if !self.enable_skipping
            || !self.parent.hits_threshold_reached
            || (!self.leaf_top_set && !self.parent.queue_full)
        {
            return Ok(());
        }
        // if some documents have missing points, check that missing values prohibits optimization
        if let Some(ref pv) = self.point_values
            && pv.get_doc_count()? < self.max_doc
            && self.is_missing_value_competitive()
        {
            return Ok(()); // we can't filter out documents, as documents with missing values are competitive
        }

        self.update_counter += 1;

        // Start sampling if we get called too much
        if self.update_counter > 256
            && (self.update_counter & (self.current_skip_interval - 1))
                != self.current_skip_interval - 1
        {
            return Ok(());
        }

        if self.parent.queue_full {
            self.encode_bottom();
        }

        let result = DocIdSetBuilder::new(self.max_doc);

        self.init_point_tree()?;
        let mut visitor = IntersectVisitorImpl::new(
            result,
            self.max_doc_visited,
            self.min_value_as_long,
            self.max_value_as_long,
            &self.parent.sub,
        );

        let threshold = ((self.iterator_cost as u64) >> 3) as i64;

        if self.point_values.is_some() {
            if is_estimated_point_count_greater_than_or_equal_to(
                &visitor,
                self.point_tree.as_mut().unwrap(),
                threshold,
            )? {
                // the new range is not selective enough to be worth materializing, it doesn't reduce number
                // of docs at least 8x
                self.update_skip_interval(false);

                let pv = self.point_values.as_ref().unwrap();
                if (pv.get_doc_count()? as i64) < self.iterator_cost {
                    debug_assert!(self.candidate.is_some());
                    self.competitive_iterator =
                        Some(CompetitiveIteratorType::B(self.candidate.take().unwrap()));

                    self.iterator_cost = pv.get_doc_count()? as i64
                }
                return Ok(());
            }
            self.point_values
                .as_ref()
                .unwrap()
                .intersect(&mut visitor)?;
        }

        match visitor.result.build()?.iterator()? {
            Some(it) => {
                self.iterator_cost = it.cost()?;
                self.competitive_iterator = Some(CompetitiveIteratorType::C(it));
                self.update_skip_interval(true);
                Ok(())
            },
            None => Err(LuceneError::illegal_state(
                "DocIdSetBuilder returned None iterator",
            ))?,
        }
    }
    fn init_point_tree(&mut self) -> Result<()> {
        if self.point_tree.is_none() {
            if let Some(ref mut pv) = self.point_values {
                self.point_tree = Some(pv.get_point_tree()?);
            } else {
                return Err(LuceneError::illegal_state(
                    "point_values is None but get_point_tree() was called",
                ));
            }
        }
        Ok(())
    }
    fn update_skip_interval(&mut self, success: bool) {
        if self.update_counter > 256 {
            if success {
                self.current_skip_interval =
                    (self.current_skip_interval / 2).max(MIN_SKIP_INTERVAL);
                self.try_update_fail_count = 0;
            } else if self.try_update_fail_count >= 3 {
                self.current_skip_interval =
                    (self.current_skip_interval * 2).min(MAX_SKIP_INTERVAL);
                self.try_update_fail_count = 0;
            } else {
                self.try_update_fail_count += 1;
            }
        }
    }
    fn encode_bottom(&mut self) {
        if !self.parent.reverse {
            // ascending order
            self.max_value_as_long = self.sub.bottom_as_comparable_long();
            if self.parent.pruning == Pruning::GreaterThanOrEqualTo
                && self.max_value_as_long != i64::MIN
            {
                self.max_value_as_long -= 1;
            }
        } else {
            // descending order
            self.min_value_as_long = self.sub.bottom_as_comparable_long();
            if self.parent.pruning == Pruning::GreaterThanOrEqualTo
                && self.min_value_as_long != i64::MAX
            {
                self.min_value_as_long += 1;
            }
        }
    }
    fn encode_top(&mut self) {
        if !self.parent.reverse {
            self.min_value_as_long = self.sub.top_as_comparable_long();
            if self.parent.single_sort
                && self.parent.pruning == Pruning::GreaterThanOrEqualTo
                && self.parent.queue_full
                && self.min_value_as_long != i64::MAX
            {
                self.min_value_as_long += 1;
            }
        } else {
            // descending order
            self.max_value_as_long = self.sub.top_as_comparable_long();
            if self.parent.single_sort
                && self.parent.pruning == Pruning::GreaterThanOrEqualTo
                && self.parent.queue_full
                && self.max_value_as_long != i64::MIN
            {
                self.max_value_as_long -= 1;
            }
        }
    }
    #[allow(clippy::collapsible_else_if)]
    fn is_missing_value_competitive(&self) -> bool {
        // if queue is full, compare with bottom first,
        // if competitive, then check if we can compare with topValue
        if self.parent.queue_full {
            let result = self
                .parent
                .missing_value_as_long
                .cmp(&self.sub.bottom_as_comparable_long())
                .to_int();
            // in reverse (desc) sort missingValue is competitive when it's greater or equal to bottom,
            // in asc sort missingValue is competitive when it's smaller or equal to bottom
            let competitive = if self.parent.reverse {
                if self.parent.pruning == Pruning::GreaterThanOrEqualTo {
                    result > 0
                } else {
                    result >= 0
                }
            } else {
                if self.parent.pruning == Pruning::GreaterThanOrEqualTo {
                    result < 0
                } else {
                    result <= 0
                }
            };

            if !competitive {
                return false;
            }
        }

        if self.leaf_top_set {
            let result = self
                .parent
                .missing_value_as_long
                .cmp(&self.sub.top_as_comparable_long())
                .to_int();
            // in reverse (desc) sort missingValue is competitive when it's smaller or equal to
            // topValue,
            // in asc sort missingValue is competitive when it's greater or equal to topValue

            return if self.parent.reverse {
                result <= 0
            } else {
                result >= 0
            };
        }
        // by default competitive
        true
    }
}
impl<LR, NCB, NC> LeafFieldComparator for NumericLeafComparator<LR, NCB, NC>
where
    LR: LeafReader,
    NCB: NumericLeafComparatorBase + LeafFieldComparator,
    NC: NumericComparatorBase + FieldComparator + Default,
{
    fn set_bottom(&mut self, slot: usize) -> Result<()> {
        self.sub.set_bottom(slot)?;
        self.parent.queue_full = true; // if we are setting bottom, it means that we have collected enough hits
        self.update_competitive_iterator()?; // update an iterator if we set a new bottom
        Ok(())
    }

    fn compare_bottom(&self, doc: i32) -> Result<i32> {
        self.sub.compare_bottom(doc)
    }

    fn compare_top(&self, doc: i32) -> Result<i32> {
        self.sub.compare_top(doc)
    }

    fn copy(&mut self, slot: usize, doc: i32) -> Result<()> {
        self.sub.copy(slot, doc)?;
        self.max_doc_visited = doc;
        Ok(())
    }

    type Scorable = NCB::Scorable;

    fn set_scorer<S: Scorable>(&mut self, _scorer: Self::Scorable) -> Result<()> {
        todo!()
    }

    type DocIdSetIterator = CompetitiveIterator<CompetitiveIteratorType<NCB::NumericDocValues>>;

    fn competitive_iterator(&mut self) -> Option<Self::DocIdSetIterator> {
        debug_assert!(self.competitive_iterator.is_some());
        match self.enable_skipping {
            true => Some(CompetitiveIterator::new(
                self.competitive_iterator.take().unwrap(),
            )),
            false => None,
        }
    }

    fn set_hits_threshold_reached(&mut self) -> Result<()> {
        self.parent.hits_threshold_reached = true;
        self.update_competitive_iterator()
    }
}

pub struct CompetitiveIterator<D>
where
    D: DocIdSetIterator,
{
    competitive_iterator: D,
    doc_id: i32,
}
impl<D> CompetitiveIterator<D>
where
    D: DocIdSetIterator,
{
    pub fn new(competitive_iterator: D) -> Self {
        let doc_id = competitive_iterator.doc_id();
        Self {
            competitive_iterator,
            doc_id,
        }
    }
}
impl<D> DocIdSetIterator for CompetitiveIterator<D>
where
    D: DocIdSetIterator,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc_id + 1)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.doc_id = self.competitive_iterator.advance(target)?;
        Ok(self.doc_id)
    }

    fn cost(&self) -> Result<i64> {
        self.competitive_iterator.cost()
    }
}

struct IntersectVisitorImpl<'a, T>
where
    T: NumericComparatorBase,
{
    result: DocIdSetBuilder,
    max_doc_visited: i32,
    min_value_as_long: i64,
    max_value_as_long: i64,
    sub_comparator: &'a T,
}
impl<'a, T> IntersectVisitorImpl<'a, T>
where
    T: NumericComparatorBase,
{
    fn new(
        result: DocIdSetBuilder,
        max_doc_visited: i32,
        min_value_as_long: i64,
        max_value_as_long: i64,
        sub_comparator: &'a T,
    ) -> Self {
        Self {
            result,
            max_doc_visited,
            min_value_as_long,
            max_value_as_long,
            sub_comparator,
        }
    }
}
impl<T> IntersectVisitor for IntersectVisitorImpl<'_, T>
where
    T: NumericComparatorBase,
{
    fn visit(&mut self, doc_id: i32) -> Result<()> {
        if doc_id <= self.max_doc_visited {
            return Ok(()); // Already visited or skipped
        }
        self.result.add_doc(doc_id);
        Ok(())
    }

    fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
        if doc_id <= self.max_doc_visited {
            return Ok(()); // Already visited or skipped
        }
        let l = self.sub_comparator.sortable_bytes_to_long(packed_value);
        if l >= self.min_value_as_long && l <= self.max_value_as_long {
            self.result.add_doc(doc_id); // doc is competitive
        }
        Ok(())
    }

    fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
        let min = self.sub_comparator.sortable_bytes_to_long(min_packed_value);
        let max = self.sub_comparator.sortable_bytes_to_long(max_packed_value);

        if min > self.max_value_as_long || max < self.min_value_as_long {
            // 1. cmp ==0 and pruning==Pruning.GREATER_THAN_OR_EQUAL_TO : if the sort is
            // ascending then maxValueAsLong is bottom's next less value, so it is competitive
            // 2. cmp ==0 and pruning==Pruning.GREATER_THAN: maxValueAsLong equals to
            // bottom, but there are multiple comparators, so it could be competitive
            Ok(Relation::CellOutsideQuery)
        } else if min < self.min_value_as_long || max > self.max_value_as_long {
            Ok(Relation::CellCrossesQuery)
        } else {
            Ok(Relation::CellInsideQuery)
        }
    }

    fn grow(&mut self, count: i32) -> Result<()> {
        self.result.grow(count);
        Ok(())
    }
}
pub trait NumericLeafComparatorBase {
    type NumericDocValues: NumericDocValues;
    fn get_numeric_doc_values<LR>(
        &self,
        context: &LeafReaderContext<LR>,
        field: &str,
    ) -> Result<Self::NumericDocValues>
    where
        LR: LeafReader;
    fn default_get_numeric_doc_values<LR>(
        &self,
        context: &LeafReaderContext<LR>,
        field: &str,
    ) -> Result<Either2NumericDocValues<LR::NumericDocValues, EmptyNumeric>>
    where
        LR: LeafReader,
    {
        DocValues::get_numeric(context.reader(), field)
    }

    fn bottom_as_comparable_long(&self) -> i64;
    fn top_as_comparable_long(&self) -> i64;
}
pub type CompetitiveIteratorType<T> =
    Either3DocIdSetIterator<AllDocIdSetIterator, T, DocIdSetBuilderIterator>;
