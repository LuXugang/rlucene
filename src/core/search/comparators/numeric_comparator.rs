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
use crate::core::index::point_values::{IntersectVisitor, Relation};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::field_comparator::FieldComparator;
use crate::core::search::pruning::Pruning;
use crate::core::util::doc_id_set_builder::DocIdSetBuilder;
use crate::core::util::error::lucene_error::Result;

pub(crate) const MIN_SKIP_INTERVAL: i32 = 32;
pub(crate) const MAX_SKIP_INTERVAL: i32 = 8192;
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
    S: NumericComparatorBase + FieldComparator,
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
    S: NumericComparatorBase + FieldComparator,
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
    S: NumericComparatorBase + FieldComparator,
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

    fn get_leaf_comparator<LR>(&self, _context: &LeafReaderContext<LR>) -> Self::LeafFieldComparator
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
