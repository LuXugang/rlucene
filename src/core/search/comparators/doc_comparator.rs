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
use crate::core::util::error::lucene_error::Result;
/// Comparator that sorts by asc _doc
pub struct DocComparator {
    doc_ids: Vec<i32>,
    // if skipping functionality should be enabled
    enable_skipping: bool,
    bottom: i32,
    top_value: i32,
    top_value_set: bool,
    bottom_value_set: bool,
    hits_threshold_reached: bool,
}

impl DocComparator {
    /// Creates a new comparator based on document ids for `num_hits`.
    pub fn new(num_hits: usize, reverse: bool, pruning: Pruning) -> Self {
        // skipping functionality is enabled if we are sorting by _doc in asc order as a primary sort
        let enable_skipping = !reverse && pruning != Pruning::None;
        Self {
            doc_ids: vec![0; num_hits],
            enable_skipping,
            bottom: 0,
            top_value: 0,
            top_value_set: false,
            bottom_value_set: false,
            hits_threshold_reached: false,
        }
    }
}
impl FieldComparator for DocComparator {
    type V = i32;

    fn compare(&self, slot1: i32, slot2: i32) -> i32 {
        self.doc_ids[slot1 as usize] - self.doc_ids[slot2 as usize]
    }

    fn set_top_value(&mut self, value: Self::V) {
        self.top_value = value;
        self.top_value_set = true;
    }

    fn value(&self, slot: i32) -> &Self::V {
        &self.doc_ids[slot as usize]
    }

    type LeafFieldComparator<LR>
        = DummyLeafFieldComparator
    where
        LR: LeafReader;

    fn get_leaf_comparator<LR>(
        self,
        _context: &LeafReaderContext<LR>,
    ) -> Result<Self::LeafFieldComparator<LR>>
    where
        LR: LeafReader,
    {
        todo!()
    }
}
