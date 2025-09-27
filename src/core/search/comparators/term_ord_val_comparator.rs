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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::dummy::dummy_leaf_field_comparator::DummyLeafFieldComparator;
use crate::core::search::field_comparator::FieldComparator;
use crate::core::search::pruning::Pruning;
use crate::core::util::ToInt;
/// Sorts by field's natural Term sort order, using ordinals.
///
/// This is functionally equivalent to
/// [`TermValComparator`](crate::core::search::field_comparator::TermValComparator),
/// but it first resolves the string to their relative ordinal positions
/// (using the index returned by
/// [`LeafReader::getSortedDocValues`](LeafReader::get_sorted_doc_values)),
/// and does most comparisons using the ordinals.
///
/// For medium to large results, this comparator will be much faster than
/// [`TermValComparator`](crate::core::search::field_comparator::TermValComparator).
/// For very small result sets it may be slower.
pub struct TermOrdValComparator {
    /// Ords for each slot.
    pub(crate) ords: Vec<i32>,
    /// Values for each slot.
    pub(crate) values: Vec<Option<BytesRef<Vec<u8>>>>,
    /// Which reader last copied a value into the slot. When
    ///   we compare two slots, we just compare-by-ord if the
    ///  readerGen is the same; else we must compare the
    ///  values (slower).
    pub(crate) reader_gen: Vec<i32>,
    /// Gen of current reader we are on.
    pub(crate) current_reader_gen: i32,
    field: String,
    reverse: bool,
    sort_missing_last: bool,
    /// Bottom value (same as `values[bottomSlot]` once bottomSlot is set).  Cached for faster compares.
    pub(crate) bottom_value: Option<BytesRef<Vec<u8>>>,
    /* Bottom slot, or -1 if queue isn't full yet */
    pub(crate) bottom_slot: i32,
    /// Set by setTopValue.
    pub(crate) top_value: Option<BytesRef<Vec<u8>>>,
    /// -1 if missing values are sorted first, 1 if they are sorted last
    pub(crate) missing_sort_cmp: i32,
    /// Whether this is the only comparator.
    single_sort: bool,
    /// Whether this comparator is allowed to skip documents.
    can_skip_documents: bool,
    /// Whether the collector is done with counting hits so that we can start skipping documents.
    hits_threshold_reached: bool,
}
impl TermOrdValComparator {
    pub fn new(
        num_hits: i32,
        field: String,
        sort_missing_last: bool,
        reverse: bool,
        pruning: Pruning,
    ) -> Self {
        let can_skip_documents = pruning != Pruning::None;
        Self {
            ords: vec![0; num_hits as usize],
            values: vec![None; num_hits as usize],
            reader_gen: vec![0; num_hits as usize],
            current_reader_gen: -1,
            field,
            reverse,
            sort_missing_last,
            bottom_value: None,
            bottom_slot: -1,
            top_value: None,
            missing_sort_cmp: if sort_missing_last { 1 } else { -1 },
            single_sort: false,
            can_skip_documents,
            hits_threshold_reached: false,
        }
    }
}
impl FieldComparator for TermOrdValComparator {
    type V = BytesRef<Vec<u8>>;

    fn compare(&self, slot1: i32, slot2: i32) -> i32 {
        let slot1 = slot1 as usize;
        let slot2 = slot2 as usize;
        if self.reader_gen[slot1] == self.reader_gen[slot2] {
            return self.ords[slot1] - self.ords[slot2];
        }

        let val1 = self.values[slot1].as_ref();
        let val2 = self.values[slot2].as_ref();

        match (val1, val2) {
            (None, None) => 0,
            (None, Some(_)) => self.missing_sort_cmp,
            (Some(_), None) => -self.missing_sort_cmp,
            (Some(v1), Some(v2)) => v1.cmp(v2).to_int(),
        }
    }

    fn set_top_value(&mut self, value: Self::V) {
        // None is fine: it means the last doc of the prior
        // search was missing this value
        self.top_value = Some(value);
    }

    fn value(&self, slot: i32) -> &Self::V {
        self.values[slot as usize]
            .as_ref()
            .expect("value in slot must be present")
    }

    type LeafFieldComparator<LR>
        = DummyLeafFieldComparator
    where
        LR: LeafReader;

    fn get_leaf_comparator<LR>(
        self,
        context: &LeafReaderContext<LR>,
    ) -> crate::core::util::error::lucene_error::Result<Self::LeafFieldComparator<LR>>
    where
        LR: LeafReader,
    {
        todo!()
    }

    fn compare_values(&self, val1: Option<&Self::V>, val2: Option<&Self::V>) -> i32 {
        match (val1, val2) {
            (None, None) => 0,
            (None, Some(_)) => self.missing_sort_cmp,
            (Some(_), None) => -self.missing_sort_cmp,
            (Some(v1), Some(v2)) => v1.cmp(v2).to_int(),
        }
    }

    fn set_single_sort(&mut self) {
        self.single_sort = true;
    }

    fn disable_skipping(&mut self) {
        self.can_skip_documents = false;
    }
}
