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
use crate::core::index::freq_prox_terms_writer::DocOffsetSorter;
use crate::core::util::Sorter;
use crate::test_framework::core::util::lucene_test_case::{is_night_mode, random};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;
use rand::prelude::SliceRandom;
use std::collections::HashMap;

fn generate_doc_offset_data<R>(random: &mut R, len: usize) -> (Vec<i32>, Vec<i64>)
where
  R: Rng + ?Sized,
{
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
  sorter.sort(0, len).unwrap();

  assert_sorted_and_synced(&docs, &offsets, &original_map);
}
