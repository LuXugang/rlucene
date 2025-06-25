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
use crate::index::index_sorter::DocComparator;
use crate::util::error::lucene_error::Result;
use crate::util::{SliceCopyOps, TimSorter, TimSorterBase};

pub struct Sorter;

pub trait DocMap {
    /// Given a doc ID from the original index, return its ordinal in the sorted
    /// index.
    fn old_to_new(&self, doc_id: i32) -> i32;

    /// Given the ordinal of a doc ID, return its doc ID in the original index.
    fn new_to_old(&self, doc_id: i32) -> i32;

    /// Return the number of documents in this map.
    /// This must equal the number of documents in the sorted `LeafReader`.
    fn size(&self) -> usize;
}

pub struct DummyDocMap;
impl DocMap for DummyDocMap {
    fn old_to_new(&self, _doc_id: i32) -> i32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn new_to_old(&self, _doc_id: i32) -> i32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn size(&self) -> usize {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

struct DocValueSorter<'a, DC>
where
    DC: DocComparator,
{
    docs: &'a mut [i32],
    comparator: DC,
    tmp: Vec<i32>,
    pivot_index: i32,
}
impl<'a, DC> DocValueSorter<'a, DC>
where
    DC: DocComparator,
{
    pub fn new(docs: &'a mut [i32], comparator: DC) -> TimSorter<DocValueSorter<DC>> {
        let max_temp_slots = docs.len() / 64;
        let tmp = vec![0i32; max_temp_slots];
        let sub = DocValueSorter {
            docs,
            comparator,
            tmp,
            pivot_index: 0,
        };
        TimSorter::new(max_temp_slots as i32, sub)
    }
}
impl<'a, DC> TimSorterBase for DocValueSorter<'a, DC>
where
    DC: DocComparator,
{
    fn copy(&mut self, src: i32, dest: i32) {
        self.docs[dest as usize] = self.docs[src as usize];
    }

    fn save(&mut self, i: i32, len: i32) {
        self.tmp
            .copy_from(&self.docs[i as usize..(i + len) as usize], 0);
    }

    fn restore(&mut self, i: i32, j: i32) {
        self.docs[j as usize] = self.tmp[i as usize];
    }

    fn compare_saved(&self, i: i32, j: i32) -> i32 {
        self.comparator
            .compare(self.tmp[i as usize], self.docs[j as usize])
    }
}
impl<'a, DC> crate::util::Sorter for DocValueSorter<'a, DC>
where
    DC: DocComparator,
{
    fn compare(&mut self, i: i32, j: i32) -> Result<i32> {
        Ok(self
            .comparator
            .compare(self.docs[i as usize], self.docs[j as usize]))
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        self.docs.swap(i as usize, j as usize);
        Ok(())
    }

    fn set_pivot(&mut self, i: i32) -> Result<()> {
        self.pivot_index = i;
        Ok(())
    }

    fn compare_pivot(&mut self, j: i32) -> Result<i32> {
        self.compare(self.pivot_index, j)
    }
}
