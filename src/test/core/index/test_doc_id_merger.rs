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
use std::rc::Rc;

use rand::RngExt;

use crate::core::index::doc_id_merger::{DocIDMerger, Sub, SubBase};
use crate::core::index::merge_state::DocMap;
use crate::core::index::of;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random;
use crate::test::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestDocIDMerger;
pub struct TestSubUnsorted {
  doc_id: i32,
  value_start: i32,
  max_doc: i32,
  doc_map: Rc<DocMapMock1>,
}

impl TestSubUnsorted {
  pub fn new(doc_map: Rc<DocMapMock1>, max_doc: i32, value_start: i32) -> Self {
    Self {
      doc_id: -1,
      value_start,
      max_doc,
      doc_map,
    }
  }

  pub fn get_value(&self) -> i32 {
    self.value_start + self.doc_id
  }
}

impl SubBase for TestSubUnsorted {
  fn next_doc(&mut self) -> Result<i32> {
    self.doc_id += 1;
    if self.doc_id == self.max_doc {
      Ok(NO_MORE_DOCS)
    } else {
      Ok(self.doc_id)
    }
  }

  type DocMap = DocMapMock1;

  fn get_doc_map(&self) -> Result<&Self::DocMap> {
    Ok(&self.doc_map)
  }
}
pub struct DocMapMock1 {
  doc_base: usize,
}
impl DocMap for DocMapMock1 {
  fn get(&self, doc_id: i32) -> Result<i32> {
    Ok(self.doc_base as i32 + doc_id)
  }
}

#[test]
fn test_no_sort() -> Result<()> {
  let mut random = random();
  let sub_count = TestUtil::next_int(&mut random, 1, 200);
  let mut subs = vec![];
  let mut value_start: i32 = 0;

  for _ in 0..sub_count {
    let max_doc = TestUtil::next_int(&mut random, 1, 1000);
    let doc_base = value_start;
    let doc_map = Rc::new(DocMapMock1 {
      doc_base: doc_base as usize,
    });
    let sub = Sub::new(TestSubUnsorted::new(doc_map.clone(), max_doc, value_start));
    subs.push(sub);
    value_start += max_doc;
  }

  let mut merger = of(subs, false)?;

  let mut count = 0;

  while let Some(sub) = merger.next()? {
    assert_eq!(count, merger.get_subs()[sub].mapped_doc_id);
    assert_eq!(count, merger.get_subs()[sub].sub.get_value());
    count += 1;
  }

  assert_eq!(value_start, count);
  Ok(())
}

pub struct TestSubSorted {
  doc_id: i32,
  max_doc: i32,
  #[allow(dead_code)]
  index: i32,
  pub(crate) doc_map: Rc<DocMapMock2>,
}

impl TestSubSorted {
  pub fn new(doc_map: Rc<DocMapMock2>, max_doc: i32, index: i32) -> Self {
    Self {
      doc_id: -1,
      max_doc,
      index,
      doc_map,
    }
  }
}

impl SubBase for TestSubSorted {
  fn next_doc(&mut self) -> Result<i32> {
    self.doc_id += 1;
    if self.doc_id == self.max_doc {
      Ok(NO_MORE_DOCS)
    } else {
      Ok(self.doc_id)
    }
  }

  type DocMap = DocMapMock2;

  fn get_doc_map(&self) -> Result<&Self::DocMap> {
    Ok(&self.doc_map)
  }
}

pub struct DocMapMock2 {
  doc_map: Vec<i32>,
  live_docs: Option<Rc<FixedBitSet>>,
}
impl DocMapMock2 {
  fn new(doc_map: Vec<i32>, live_docs: Option<Rc<FixedBitSet>>) -> Self {
    Self { doc_map, live_docs }
  }
}
impl DocMap for DocMapMock2 {
  fn get(&self, doc_id: i32) -> Result<i32> {
    let mapped = self.doc_map[doc_id as usize];
    if self.live_docs.is_none() || self.live_docs.as_ref().unwrap().get(mapped as usize)? {
      Ok(mapped)
    } else {
      Ok(-1)
    }
  }
}
#[test]
fn test_with_sort() -> Result<()> {
  let mut random = random();
  let sub_count = TestUtil::next_int(&mut random, 1, 20);
  let mut old_to_new: Vec<Vec<i32>> = Vec::new();
  // how many docs we've written to each sub:
  let mut uptos: Vec<usize> = Vec::new();
  let mut tot_doc_count = 0;

  for _ in 0..sub_count {
    let max_doc = TestUtil::next_usize(&mut random, 1, 1000);
    uptos.push(0);
    old_to_new.push(vec![0; max_doc]);
    tot_doc_count += max_doc;
  }

  let mut completed_subs: Vec<Vec<i32>> = vec![];

  // Randomly assign global docIDs to subs
  for doc_id in 0..tot_doc_count {
    let sub = random.random_range(0..old_to_new.len());
    let mut upto = uptos[sub];
    old_to_new[sub][upto] = doc_id as i32;
    upto += 1;
    if upto == old_to_new[sub].len() {
      completed_subs.push(old_to_new[sub].clone());
      old_to_new.remove(sub);
      uptos.remove(sub);
    } else {
      uptos[sub] = upto;
    }
  }

  assert_eq!(old_to_new.len(), 0);

  // Optional deletions
  let mut live_docs: Option<Rc<FixedBitSet>> = None;
  if random.random_bool(0.5) {
    let mut bitset = FixedBitSet::new(tot_doc_count);
    bitset.set_with_range(0, tot_doc_count);
    let delete_attempts = TestUtil::next_int(&mut random, 1, tot_doc_count as i32);
    for _ in 0..delete_attempts {
      bitset.clear_with_index(random.random_range(0..tot_doc_count));
    }
    live_docs = Some(Rc::new(bitset));
  }

  let mut subs: Vec<Sub<TestSubSorted>> = Vec::new();

  for (i, doc_map) in completed_subs.iter().enumerate() {
    let len = doc_map.len();
    let doc_map_enum = Rc::new(DocMapMock2::new(doc_map.clone(), live_docs.clone()));

    let sub = Sub::new(TestSubSorted::new(doc_map_enum, len as i32, i as i32));

    subs.push(sub);
  }

  let mut merger = of(subs, true)?;

  let mut count = 0;
  while let Some(sub) = merger.next()? {
    if let Some(ref live) = live_docs {
      count = live.next_set_bit(count);
    }
    assert_eq!(
      count,
      merger.get_subs()[sub].mapped_doc_id as usize,
      "doc mismatch at count {}",
      count
    );
    count += 1;
  }

  if let Some(ref live) = live_docs {
    if count < tot_doc_count {
      assert_eq!(live.next_set_bit(count), NO_MORE_DOCS as usize);
    } else {
      assert_eq!(count, tot_doc_count);
    }
  } else {
    assert_eq!(count, tot_doc_count);
  }

  Ok(())
}
