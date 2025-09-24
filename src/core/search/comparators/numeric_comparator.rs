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
use crate::core::search::dummy::dummy_leaf_field_comparator::DummyLeafFieldComparator;
use crate::core::search::field_comparator::FieldComparator;
use crate::core::search::pruning::Pruning;
use crate::core::util::Comparator;
use crate::core::util::error::lucene_error::{LuceneError, Result};

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

    fn compare_values(&self, first: Option<&Self::V>, second: Option<&Self::V>) -> i32 {
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
