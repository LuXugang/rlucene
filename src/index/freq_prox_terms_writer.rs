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
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::Result;
use crate::util::{SliceCopyOps, Sorter, TimSorter, TimSorterBase};

pub(crate) struct FreqProxTermsWriter;

pub struct DocOffsetSorter<'a> {
    docs: &'a mut [i32],
    offsets: &'a mut [i64],
    tmp_docs: Vec<i32>,
    tmp_offsets: Vec<i64>,
    pivot_index: i32,
}

impl<'a> DocOffsetSorter<'a> {
    pub fn new(
        docs: &'a mut [i32],
        offsets: &'a mut [i64],
        max_temp_slots: usize,
    ) -> TimSorter<DocOffsetSorter<'a>> {
        let tmp_docs = Vec::new();
        let tmp_offsets = Vec::new();
        let sorter = DocOffsetSorter {
            docs,
            offsets,
            tmp_docs,
            tmp_offsets,
            pivot_index: 0,
        };
        TimSorter::new(max_temp_slots as i32, sorter)
    }
}

impl Sorter for DocOffsetSorter<'_> {
    fn compare(&mut self, i: i32, j: i32) -> Result<i32> {
        Ok(self.docs[i as usize] - self.docs[j as usize])
    }

    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        let i = i as usize;
        let j = j as usize;
        self.docs.swap(i, j);
        self.offsets.swap(i, j);
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

impl TimSorterBase for DocOffsetSorter<'_> {
    fn copy(&mut self, src: i32, dest: i32) {
        let src = src as usize;
        let dest = dest as usize;
        self.docs[dest] = self.docs[src];
        self.offsets[dest] = self.offsets[src];
    }

    fn save(&mut self, i: i32, len: i32) {
        if self.tmp_docs.len() < len as usize {
            let new_len = ArrayUtil::oversize(len as usize, 0);
            self.tmp_docs = vec![0; new_len];
            self.tmp_offsets = vec![0; new_len];
        }
        let i = i as usize;
        let len = len as usize;

        self.tmp_docs.copy_from(&self.docs[i..i + len], 0);
        self.tmp_offsets.copy_from(&self.offsets[i..i + len], 0);
    }

    fn restore(&mut self, i: i32, j: i32) {
        let i = i as usize;
        let j = j as usize;
        self.docs[j] = self.tmp_docs[i];
        self.offsets[j] = self.tmp_offsets[i];
    }

    fn compare_saved(&self, i: i32, j: i32) -> i32 {
        self.tmp_docs[i as usize] - self.docs[j as usize]
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rand::prelude::SliceRandom;
    use rand::Rng;

    use crate::index::freq_prox_terms_writer::DocOffsetSorter;
    use crate::test::util::lucene_test_case::{is_night_mode, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::Sorter;

    fn generate_doc_offset_data<R: Rng + ?Sized>(
        random: &mut R,
        len: usize,
    ) -> (Vec<i32>, Vec<i64>) {
        let mut docs = Vec::with_capacity(len);
        let mut offsets = Vec::with_capacity(len);

        let mut doc_id = 0;
        for _ in 0..len {
            doc_id += random.random_range(1..10);
            docs.push(doc_id);
            offsets.push(random.random_range(1000..10_000));
        }
        docs.shuffle(random);

        (docs, offsets)
    }

    fn assert_sorted_and_synced(docs: &[i32], offsets: &[i64], original_map: &HashMap<i32, i64>) {
        assert_eq!(docs.len(), offsets.len());

        for i in 0..docs.len() {
            if i > 0 {
                assert!(
                    docs[i - 1] <= docs[i],
                    "docs not sorted at index {}: {} > {}",
                    i,
                    docs[i - 1],
                    docs[i]
                );
            }

            let doc = docs[i];
            let expected_offset = original_map.get(&doc).expect("missing doc in map");

            assert_eq!(
                offsets[i], *expected_offset,
                "offset mismatch at index {}: doc={} expected={} actual={}",
                i, doc, expected_offset, offsets[i]
            );
        }
    }

    #[test]
    fn test_doc_offset_sorter_basic() {
        let mut random = random();
        let len = if is_night_mode() {
            random.random_range(1000..5000)
        } else {
            random.random_range(10000..20000)
        };

        let (mut docs, mut offsets) = generate_doc_offset_data(&mut random, len);
        assert_eq!(docs.len(), offsets.len());

        let mut original_map: HashMap<i32, i64> = HashMap::with_capacity(len);
        for (doc, offset) in docs.iter().cloned().zip(offsets.iter().cloned()) {
            original_map.insert(doc, offset);
        }

        let max_temp_slots = TestUtil::next_int(&mut random, 0, len as i32);
        let mut sorter = DocOffsetSorter::new(&mut docs, &mut offsets, max_temp_slots as usize);
        sorter.sort(0, len as i32).unwrap();

        assert_sorted_and_synced(&docs, &offsets, &original_map);
    }
}
